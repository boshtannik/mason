mod api;
mod event;
mod state;
mod types;

use std::sync::Arc;

use futures_util::StreamExt;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::try_init().ok();
    log::info!("opencode-client starting");

    // Через env: OPENCODE_SERVER_URL (default http://127.0.0.1:4096)
    let base = std::env::var("OPENCODE_SERVER_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:4096".to_string());
    let auth = std::env::var("OPENCODE_SERVER_PASSWORD").ok().map(|pw| {
        let user = std::env::var("OPENCODE_SERVER_USERNAME")
            .unwrap_or_else(|_| "opencode".to_string());
        (user, pw)
    });

    // UI-мост (qmetaobject-rs) — добавится на этапе интеграции с QML.
    // Пока: консольный smoke-test: health + SSE-подписка.
    let client = api::OpenCodeClient::new(base.clone(), auth.clone());
    let healthy = client.health().await?;
    log::info!("server health: {healthy}");
    if !healthy {
        log::error!("opencode server не отвечает на {base}");
        return Ok(());
    }

    let dispatcher: event::dispatcher::SharedState = Arc::new(Mutex::new(Default::default()));

    let sse = event::sse::SseClient::new(base, auth);
    log::info!("подключаюсь к /global/event...");
    let mut stream = sse.stream().await?;

    while let Some(item) = stream.next().await {
        match item {
            Ok(ev) => {
                let changed = {
                    let mut st = dispatcher.lock().await;
                    event::dispatcher::dispatch(&ev.payload, &mut st).await
                };
                if changed {
                    log::info!("event: {:?}", ev.payload);
                }
            }
            Err(e) => log::error!("sse error: {e}"),
        }
    }

    Ok(())
}