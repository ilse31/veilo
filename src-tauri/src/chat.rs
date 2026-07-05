//! Hub "Obrolan AI" streaming chat — runs via Rust to bypass browser CORS.
//!
//! The frontend calls `chat_send` (awaited for the final text) while listening
//! to `chat-delta` events to render tokens incrementally. `chat_stop` cancels
//! an in-flight stream. Mirrors the streaming pattern in `copilot::orchestrator`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::oneshot;

/// Holds a cancellation sender per active chat stream, keyed by client id.
#[derive(Default)]
pub struct ChatAbortState(pub Mutex<HashMap<String, oneshot::Sender<()>>>);

#[derive(Deserialize, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Clone)]
struct ChatDelta {
    id: String,
    delta: String,
}

fn remove_abort(state: &State<'_, ChatAbortState>, id: &str) {
    if let Ok(mut g) = state.0.lock() {
        g.remove(id);
    }
}

/// Stream a chat completion. Emits `chat-delta` per token and returns the full
/// assembled text on success. On abort returns the partial text streamed so far
/// (Ok); on HTTP/request errors returns Err(message).
#[tauri::command]
pub async fn chat_send(
    app: AppHandle,
    id: String,
    api_url: String,
    api_key: String,
    model: String,
    messages: Vec<ChatMessage>,
    state: State<'_, ChatAbortState>,
) -> Result<String, String> {
    let base = api_url.trim_end_matches('/').to_string();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": true,
    });

    let mut req = client.post(format!("{base}/v1/chat/completions")).json(&body);
    if !api_key.is_empty() {
        req = req.bearer_auth(api_key);
    }

    // Register abort handle so chat_stop can signal this stream.
    let (abort_tx, mut abort_rx) = oneshot::channel::<()>();
    {
        let mut guard = state.0.lock().map_err(|e| e.to_string())?;
        guard.insert(id.clone(), abort_tx);
    }

    let resp = req.send().await;
    let resp = match resp {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            let status = r.status().as_u16();
            let text = r.text().await.unwrap_or_default();
            remove_abort(&state, &id);
            let snippet: String = text.chars().take(300).collect();
            return Err(if snippet.is_empty() {
                format!("HTTP {status}")
            } else {
                format!("HTTP {status}: {snippet}")
            });
        }
        Err(e) => {
            remove_abort(&state, &id);
            return Err(e.to_string());
        }
    };

    use futures_util::StreamExt;
    let mut stream = resp.bytes_stream();
    let mut full = String::new();
    let mut leftover = String::new();

    loop {
        tokio::select! {
            biased;
            _ = &mut abort_rx => {
                remove_abort(&state, &id);
                return Ok(full);
            }
            chunk = stream.next() => {
                match chunk {
                    Some(Ok(bytes)) => {
                        leftover.push_str(&String::from_utf8_lossy(&bytes));
                        // Split into complete lines; retain the trailing partial.
                        let mut lines: Vec<String> = std::mem::take(&mut leftover)
                            .split('\n')
                            .map(String::from)
                            .collect();
                        leftover = lines.pop().unwrap_or_default();
                        for line in lines {
                            let Some(data) = line.trim().strip_prefix("data: ") else { continue };
                            if data == "[DONE]" { continue; }
                            let Ok(j) = serde_json::from_str::<serde_json::Value>(data) else { continue };
                            if let Some(d) = j["choices"][0]["delta"]["content"].as_str() {
                                full.push_str(d);
                                let _ = app.emit("chat-delta", ChatDelta {
                                    id: id.clone(),
                                    delta: d.into(),
                                });
                            }
                        }
                    }
                    Some(Err(e)) => {
                        remove_abort(&state, &id);
                        return Err(e.to_string());
                    }
                    None => break,
                }
            }
        }
    }

    remove_abort(&state, &id);
    Ok(full)
}

/// Cancel an in-flight `chat_send` stream by id. Returns true if a stream was
/// found and signalled.
#[tauri::command]
pub fn chat_stop(id: String, state: State<'_, ChatAbortState>) -> Result<bool, String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(tx) = guard.remove(&id) {
        let _ = tx.send(());
        Ok(true)
    } else {
        Ok(false)
    }
}
