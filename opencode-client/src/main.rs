use std::sync::Arc;
use std::cell::RefCell;
use futures_util::StreamExt;
use tokio::sync::Mutex;

#[cfg(feature = "gui")]
use cstr::cstr;
#[cfg(feature = "gui")]
use qmetaobject::{QmlEngine, QObjectPinned};

mod api;
mod audio;
#[cfg(feature = "gui")]
mod bridge;
mod event;
mod server;
mod state;
mod types;

#[cfg(feature = "gui")]
use bridge::app::AppBridge;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::try_init().ok();
    log::info!("opencode-client starting");

    // Сервер — часть приложения: поднять в момент старта (RAII).
    let server_guard = match server::ServerGuard::start(server::ServerConfig::default()).await {
        Ok(g) => g,
        Err(e) => {
            log::error!("не удалось поднять opencode serve: {e}");
            return Ok(());
        }
    };
    let serve_url = server_guard.base_url().to_string();
    log::info!("opencode serve поднят: {serve_url}");

    // Клиент + состояние + SSE.
    let base = std::env::var("OPENCODE_SERVER_URL").unwrap_or(serve_url);
    let auth = std::env::var("OPENCODE_SERVER_PASSWORD").ok().map(|pw| {
        let user = std::env::var("OPENCODE_SERVER_USERNAME")
            .unwrap_or_else(|_| "opencode".to_string());
        (user, pw)
    });

    let client = api::OpenCodeClient::new(base.clone(), auth.clone());
    let healthy = client.health().await?;
    log::info!("server health: {healthy}");

    let dispatcher: event::dispatcher::SharedState = Arc::new(Mutex::new(Default::default()));

    #[cfg(feature = "gui")]
    {
        run_gui(base, auth, dispatcher).await?;
    }
    #[cfg(not(feature = "gui"))]
    {
        run_headless(base, auth, dispatcher).await?;
    }

    Ok(())
}

/// Консольный режим (без QML): подписка на SSE + логирование событий.
#[cfg(not(feature = "gui"))]
async fn run_headless(
    base: String,
    auth: Option<(String, String)>,
    dispatcher: event::dispatcher::SharedState,
) -> Result<(), Box<dyn std::error::Error>> {
    let sse = event::sse::SseClient::new(base, auth);
    log::info!("подключаюсь к /global/event...");
    let mut stream = sse.stream().await?;
    while let Some(item) = stream.next().await {
        match item {
            Ok(ev) => {
                let mut st = dispatcher.lock().await;
                let changed = event::dispatcher::dispatch(&ev.payload, &mut st).await;
                drop(st);
                if changed {
                    log::info!("event: {:?}", ev.payload);
                }
            }
            Err(e) => log::error!("sse error: {e}"),
        }
    }
    Ok(())
}

/// GUI-режим: QML-окно + сервер + SSE-воркер в фоне.
#[cfg(feature = "gui")]
async fn run_gui(
    base: String,
    auth: Option<(String, String)>,
    dispatcher: event::dispatcher::SharedState,
) -> Result<(), Box<dyn std::error::Error>> {
    // Мост для QML.
    qmetaobject::qml_register_type::<AppBridge>(
        cstr!("OpenCodeClient"),
        1,
        0,
        cstr!("AppBridge"),
    );
    let bridge = RefCell::new(AppBridge::default());
    let bridge_pinned = unsafe { QObjectPinned::new(&bridge) };
    bridge_pinned.borrow_mut().set_session_status_rs("idle".to_string());

    // Очереди: воркер пишет/читает без QML (Send + 'static), QML-мост работает
    // с теми же Arc на главном потоке.
    let (pending_queue, prompts_queue) = bridge_pinned.borrow().queues();
    let push_msg = pending_queue.clone();
    drop(push_msg);

    // Фоновый воркер: (1) SSE-события → QML-мост, (2) промпты из QML → сервер.
    let worker_pending = pending_queue.clone();
    let worker_prompts = prompts_queue.clone();
    tokio::spawn(async move {
        let client = api::OpenCodeClient::new(base.clone(), auth.clone());

        let sse = event::sse::SseClient::new(base, auth);
        log::info!("подключаюсь к /global/event...");
        let mut stream = match sse.stream().await {
            Ok(s) => s,
            Err(e) => {
                log::error!("sse connect: {e}");
                return;
            }
        };

        // Копим текст ответов по part.id (см. example full_cycle).
        let mut parts: std::collections::HashMap<String, String> = Default::default();
        let mut current_session: Option<String> = None;
        // id ассистентских сообщений — только их текстовые части идят в ответ.
        let mut assistant_messages: std::collections::HashSet<String> = Default::default();

        // Цикл: слушаем SSE и параллельно опрашиваем мост на новые промпты.
        loop {
            tokio::select! {
                item = stream.next() => {
                    match item {
                        Some(Ok(ev)) => {
                            let mut st = dispatcher.lock().await;
                            let changed = event::dispatcher::dispatch(&ev.payload, &mut st).await;
                            drop(st);
                            if let Some(mid) = ev.payload.assistant_message_id() {
                                assistant_messages.insert(mid.to_string());
                            }
                            if let Some((mid, id, text)) = ev.payload.text_part() {
                                if assistant_messages.contains(mid) {
                                    parts.insert(id.to_string(), text);
                                }
                            } else if let Some(session_id) = ev.payload.idle_session_id() {
                                if current_session.as_deref() == Some(session_id) {
                                    // Собрать накопленный ответ и отдать в QML.
                                    let mut ans: Vec<&String> = parts.values().collect();
                                    ans.sort();
                                    let text: String = ans.iter().map(|s| s.as_str()).collect();
                                    if !text.trim().is_empty() {
                                        log::info!("push answer: {:?}", text);
                                        if let Ok(mut q) = worker_pending.lock() {
                                            q.push(text);
                                        }
                                    }
                                    parts.clear();
                                    current_session = None;
                                }
                            }
                            let _ = changed;
                        }
                        Some(Err(e)) => log::error!("sse error: {e}"),
                        None => { log::info!("SSE закрылся"); return; }
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(200)) => {
                    for prompt in worker_prompts.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default() {
                        log::info!("QML промпт: {prompt}");
                        // Создаём сессию если ещё нет.
                        if current_session.is_none() {
                            match client.create_session().await {
                                Ok(sess) => {
                                    if let Some(id) = sess["id"].as_str() {
                                        current_session = Some(id.to_string());
                                    }
                                }
                                Err(e) => {
                                    log::error!("create_session: {e}");
                                    if let Ok(mut q) = worker_pending.lock() {
                                        q.push(format!("[ошибка сессии] {e}"));
                                    }
                                    continue;
                                }
                            }
                        }
                        if let Some(sid) = current_session.clone() {
                            if let Err(e) = client.prompt_async(&sid, &prompt).await {
                                log::error!("prompt_async: {e}");
                                if let Ok(mut q) = worker_pending.lock() {
                                    q.push(format!("[ошибка промпта] {e}"));
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    // QML-движок (главный поток; exec блокирует до закрытия окна).
    let mut engine = QmlEngine::new();
    let _ = bridge_pinned.get_or_create_cpp_object();
    engine.set_object_property("bridge".into(), bridge_pinned);

    let qml_path = std::env::var("OPENCODE_QML")
        .unwrap_or_else(|_| "qml/opencode-client.qml".to_string());
    engine.load_file(qml_path.clone().into());
    engine.exec();

    log::info!("QML закрыт, выходим");
    Ok(())
}