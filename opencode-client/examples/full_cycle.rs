use std::collections::HashMap;
use std::time::Duration;

use futures_util::StreamExt;
use opencode_client::api::OpenCodeClient;
use opencode_client::event::sse::SseClient;
use opencode_client::server::{ServerConfig, ServerGuard};
use opencode_client::types::event::Event;
use opencode_client::types::message::Part;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::try_init().ok();

    // 1. RAII-сервер: авто-порт.
    let guard = ServerGuard::start(ServerConfig::default()).await?;
    let base = guard.base_url().to_string();
    println!("PASS: сервер {base}");

    // 2. Клиент.
    let client = OpenCodeClient::new(base.clone(), None);
    assert!(client.health().await?);
    println!("PASS: health ok");

    // 3. SSE.
    let sse = SseClient::new(base, None);
    let mut stream = sse.stream().await?;
    println!("PASS: SSE подключён");

    // 4. Сессия.
    let sess = client.create_session().await?;
    let session_id = sess["id"].as_str().unwrap().to_string();
    println!("PASS: сессия {session_id}");

    // 5. Промпт.
    client
        .prompt_async(&session_id, "Ответь одним числом: 2+2=?")
        .await?;
    println!("PASS: промпт отправлен");

    // 6. Копим текст по part.id.
    let mut parts: HashMap<String, String> = HashMap::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(25);
    let mut idle_for_this = false;

    while std::time::Instant::now() < deadline {
        match stream.next().await {
            Some(Ok(ev)) => match &ev.payload {
                Event::MessagePartUpdated { properties } => {
                    if let Part::Text { id, text, ignored, .. } = &properties.part {
                        if *ignored != Some(true) {
                            parts.insert(id.clone(), text.clone());
                        }
                    }
                }
                Event::SessionIdle { properties } => {
                    if properties.sessionID == session_id {
                        idle_for_this = true;
                        break;
                    }
                }
                _ => {}
            },
            Some(Err(err)) => {
                eprintln!("sse err: {err}");
            }
            None => break,
        }
    }

    let mut answer: Vec<&String> = parts.values().collect();
    answer.sort();
    let text: String = answer.iter().map(|s| s.as_str()).collect();

    if idle_for_this && !text.trim().is_empty() {
        println!("PASS: ответ получен:");
        println!("    {text}");
    } else {
        println!("ANSWER(partial): >{text}<");
        println!("(idle={idle_for_this})");
    }

    println!("...закрываю (Drop убьёт сервер)");
    Ok(())
}