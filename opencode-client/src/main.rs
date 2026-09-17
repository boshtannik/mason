use std::cell::RefCell;
use std::ffi::CStr;
use std::sync::{mpsc, Arc};
use std::time::Duration;

use futures_util::StreamExt;
use qmetaobject::{qml_register_type, QObjectPinned};
use sailors::sailfishapp::QmlApp;
use tokio::sync::Mutex;

use opencode_client::api;
use opencode_client::bridge::app::AppBridge;
use opencode_client::event;
use opencode_client::server;

/// Единственный QML-файл приложения (лежит в /usr/share/harbour-opencode/qml).
const MAIN_QML: &str = "qml/harbour-opencode.qml";
/// Имя приложения: определяет и data-каталог `SailfishApp::pathTo`, и argv[0].
const APP_NAME: &str = "harbour-opencode";
/// Сколько ждём готовности opencode serve перед показом окна.
const SERVER_WAIT: Duration = Duration::from_secs(12);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::try_init().ok();
    log::info!("harbour-opencode starting");

    // Мост QML ←→ Rust (создаём до Qt, чтобы отдать очереди воркеру).
    qml_register_type::<AppBridge>(
        CStr::from_bytes_with_nul(b"OpenCodeClient\0")?,
        1,
        0,
        CStr::from_bytes_with_nul(b"AppBridge\0")?,
    );
    let bridge = RefCell::new(AppBridge::default());
    let bridge_pinned = unsafe { QObjectPinned::new(&bridge) };
    let (pending, prompts) = bridge_pinned.borrow().queues();

    let dispatcher: event::dispatcher::SharedState = Arc::new(Mutex::new(Default::default()));

    // Важно: tokio-runtime нельзя поднимать в главном потоке — при запуске через
    // mapplauncherd/booster (`invoker`, lipstick) он паникует («RefCell already
    // borrowed»). Поэтому весь async-стек живёт в отдельном std-потоке, а Qt —
    // в главном.
    let (url_tx, url_rx) = mpsc::channel::<Option<String>>();
    let env_base = std::env::var("OPENCODE_SERVER_URL").ok();
    let env_auth = std::env::var("OPENCODE_SERVER_PASSWORD").ok().map(|pw| {
        let user =
            std::env::var("OPENCODE_SERVER_USERNAME").unwrap_or_else(|_| "opencode".to_string());
        (user, pw)
    });

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        rt.block_on(async move {
            let guard = server::ServerGuard::start(server::ServerConfig::default()).await;
            let base = match &guard {
                Ok(g) => {
                    log::info!("opencode serve поднят: {}", g.base_url());
                    Some(env_base.clone().unwrap_or_else(|| g.base_url().to_string()))
                }
                Err(e) => {
                    log::error!("не удалось поднять opencode serve: {e}");
                    env_base.clone()
                }
            };
            let _ = url_tx.send(base.clone());
            if let Some(base) = base {
                run_worker(base, env_auth, dispatcher, pending, prompts).await;
            }
            drop(guard);
        });
    });

    let has_server = match url_rx.recv_timeout(SERVER_WAIT) {
        Ok(Some(_)) => true,
        _ => {
            log::warn!("opencode serve не готов за {:?} — стартуем в offline", SERVER_WAIT);
            false
        }
    };
    bridge_pinned.borrow_mut().set_session_status_rs(
        if has_server { "connecting" } else { "error" }.to_string(),
    );

    // QML в главном потоке; exec() блокирует до закрытия окна.
    let mut app = QmlApp::application(APP_NAME.to_string());
    app.set_object_property("bridge".into(), bridge_pinned);
    app.set_source(QmlApp::path_to(MAIN_QML.into()));
    app.show_full_screen();
    app.exec();

    log::info!("QML закрыт, выходим");
    Ok(())
}

/// Фоновый воркер: (1) SSE-события → QML-мост, (2) промпты из QML → сервер.
async fn run_worker(
    base: String,
    auth: Option<(String, String)>,
    dispatcher: event::dispatcher::SharedState,
    worker_pending: Arc<std::sync::Mutex<Vec<String>>>,
    worker_prompts: Arc<std::sync::Mutex<Vec<String>>>,
) {
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
}
