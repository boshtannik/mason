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

    // Фоновый SSE-воркер: глотает события и логирует.
    // Push в QML-мост из этого потока небезопасно (Qt требует главный поток);
    // позже — через QMetaObject::invokeMethod на объект.
    tokio::spawn(async move {
        let sse = event::sse::SseClient::new(base, auth);
        log::info!("подключаюсь к /global/event...");
        let mut stream = match sse.stream().await {
            Ok(s) => s,
            Err(e) => {
                log::error!("sse connect: {e}");
                return;
            }
        };
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