use std::collections::HashMap;
use std::time::Duration;

use futures_util::StreamExt;
use opencode_client::api::OpenCodeClient;
use opencode_client::event::sse::SseClient;
use opencode_client::server::{ServerConfig, ServerGuard};

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

    // 6. Копим текст по part.id только из ассистентских сообщений.
    let mut parts: HashMap<String, String> = HashMap::new();
    let mut assistant_messages: std::collections::HashSet<String> = Default::default();
    let deadline = std::time::Instant::now() + Duration::from_secs(25);
    let mut idle_for_this = false;

    while std::time::Instant::now() < deadline {
        match stream.next().await {
            Some(Ok(ev)) => {
                if let Some(mid) = ev.payload.assistant_message_id() {
                    assistant_messages.insert(mid.to_string());
                }
                if let Some((mid, id, text)) = ev.payload.text_part() {
                    if assistant_messages.contains(mid) {
                        parts.insert(id.to_string(), text);
                    }
                } else if let Some(sid) = ev.payload.idle_session_id() {
                    if sid == session_id {
                        idle_for_this = true;
                        break;
                    }
                }
            }
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