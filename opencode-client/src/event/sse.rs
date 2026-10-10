use std::collections::VecDeque;
use std::pin::Pin;

use futures_util::{stream::unfold, Stream, StreamExt};
use reqwest::Client;

use crate::types::GlobalEvent;

/// SSE client: reads /global/event and yields a stream of GlobalEvent.
pub struct SseClient {
    http: Client,
    base: String,
    auth: Option<(String, String)>,
}

impl SseClient {
    pub fn new(base: String, auth: Option<(String, String)>) -> Self {
        let builder = Client::builder().connect_timeout(std::time::Duration::from_secs(10));
        Self {
            http: builder.build().expect("reqwest client"),
            base,
            auth,
        }
    }

    pub async fn stream(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<GlobalEvent, String>> + Send>>, String> {
        let url = format!("{}/global/event", self.base);
        let mut req = self.http.get(&url);
        if let Some((user, pass)) = &self.auth {
            req = req.basic_auth(user, Some(pass));
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("SSE status: {}", resp.status()));
        }

        // Concatenate chunks and split by lines: "data: <json>"
        let stream = unfold(
            (resp.bytes_stream(), Vec::new(), VecDeque::new()),
            move |(mut chunks, mut buffer, mut pending)| async move {
                loop {
                    if let Some(ev) = pending.pop_front() {
                        return Some((ev, (chunks, buffer, pending)));
                    }
                    match chunks.next().await {
                        Some(Ok(bytes)) => {
                            buffer.extend_from_slice(&bytes);
                            parse_events(&mut buffer, &mut pending);
                        }
                        Some(Err(e)) => return Some((Err(e.to_string()), (chunks, buffer, pending))),
                        None => return None,
                    }
                }
            },
        );

        Ok(Box::pin(stream))
    }
}

fn parse_events(buffer: &mut Vec<u8>, out: &mut VecDeque<Result<GlobalEvent, String>>) {
    // Parse only complete lines ending with \n
    let text = String::from_utf8_lossy(buffer);
    let mut complete_idx = 0;
    for line in text.split_inclusive('\n') {
        if line.ends_with('\n') {
            complete_idx += line.len();
            if let Some(data) = line.strip_prefix("data: ") {
                let data = data.trim_end_matches('\n');
                match serde_json::from_str::<GlobalEvent>(data) {
                    Ok(ev) => out.push_back(Ok(ev)),
                    Err(e) => out.push_back(Err(format!("parse: {e}"))),
                }
            }
        } else {
            break;
        }
    }
    if complete_idx > 0 {
        buffer.drain(..complete_idx);
    }
}