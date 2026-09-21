//! Hub "Obrolan AI" streaming chat — runs via Rust to bypass browser CORS.
//!
//! The frontend calls `chat_send` (awaited for the final text) while listening
//! to `chat-delta` events to render tokens incrementally. `chat_stop` cancels
//! an in-flight stream. Mirrors the streaming pattern in `copilot::orchestrator`.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
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

/// API base URLs whose server rejected `stream_options` (HTTP 400/422) but
/// accepted the same request without it. Learned at runtime, per process.
fn no_usage_option() -> &'static Mutex<HashSet<String>> {
    static SET: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    SET.get_or_init(Default::default)
}

fn build_request(
    client: &reqwest::Client,
    base: &str,
    api_key: &str,
    model: &str,
    messages: &[ChatMessage],
    include_usage: bool,
) -> reqwest::RequestBuilder {
    let mut body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": true,
    });
    if include_usage {
        body["stream_options"] = serde_json::json!({ "include_usage": true });
    }
    let mut req = client.post(format!("{base}/v1/chat/completions")).json(&body);
    if !api_key.is_empty() {
        req = req.bearer_auth(api_key);
    }
    req
}

/// Send the streaming request, asking for token `usage` when the server allows it.
/// Some OpenAI-compatible servers reject the unknown `stream_options` field; on
/// 400/422 retry once without it, and remember the server only if that succeeds
/// (so an unrelated 400 such as a bad model name doesn't disable usage tracking).
async fn open_stream(
    client: &reqwest::Client,
    base: &str,
    api_key: &str,
    model: &str,
    messages: &[ChatMessage],
) -> reqwest::Result<reqwest::Response> {
    let want_usage = !no_usage_option().lock().map(|s| s.contains(base)).unwrap_or(false);
    let resp = build_request(client, base, api_key, model, messages, want_usage).send().await?;
    if want_usage && matches!(resp.status().as_u16(), 400 | 422) {
        let retry = build_request(client, base, api_key, model, messages, false).send().await?;
        if retry.status().is_success() {
            if let Ok(mut s) = no_usage_option().lock() {
                s.insert(base.to_string());
            }
        }
        return Ok(retry);
    }
    Ok(resp)
}

/// Rough token estimate (~4 chars per token) for servers that don't report usage.
fn estimate_tokens(text: &str) -> i64 {
    text.chars().count().div_ceil(4) as i64
}

/// Token counts to record: the server's numbers when it reported any, else an
/// estimate from the text. The bool is true when the numbers are estimated.
fn resolve_usage(
    prompt: Option<i64>,
    completion: Option<i64>,
    messages: &[ChatMessage],
    reply: &str,
) -> (i64, i64, bool) {
    if prompt.is_none() && completion.is_none() {
        let p = messages.iter().map(|m| estimate_tokens(&m.content)).sum();
        return (p, estimate_tokens(reply), true);
    }
    (prompt.unwrap_or(0), completion.unwrap_or(0), false)
}

fn remove_abort(state: &State<'_, ChatAbortState>, id: &str) {
    if let Ok(mut g) = state.0.lock() {
        g.remove(id);
    }
}

/// Stream a chat completion. Emits `chat-delta` per token and returns the full
/// assembled text on success. On abort returns the partial text streamed so far
/// (Ok); on HTTP/request errors returns Err(message). A completed stream is
/// recorded in `chat_usage` (time to first token + token counts, estimated when the server reports none).
#[tauri::command]
pub async fn chat_send(
    app: AppHandle,
    id: String,
    api_url: String,
    api_key: String,
    model: String,
    messages: Vec<ChatMessage>,
    conv_id: Option<i64>,
    state: State<'_, ChatAbortState>,
    db: State<'_, crate::db::Db>,
) -> Result<String, String> {
    let base = api_url.trim_end_matches('/').to_string();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    // Register abort handle so chat_stop can signal this stream.
    let (abort_tx, mut abort_rx) = oneshot::channel::<()>();
    {
        let mut guard = state.0.lock().map_err(|e| e.to_string())?;
        guard.insert(id.clone(), abort_tx);
    }

    let started = Instant::now();
    let resp = open_stream(&client, &base, &api_key, &model, &messages).await;
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
    let mut first_token_ms: Option<i64> = None;
    let mut prompt_tokens: Option<i64> = None;
    let mut completion_tokens: Option<i64> = None;

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
                            if let Some(n) = j["usage"]["prompt_tokens"].as_i64() {
                                prompt_tokens = Some(n);
                            }
                            if let Some(n) = j["usage"]["completion_tokens"].as_i64() {
                                completion_tokens = Some(n);
                            }
                            if let Some(d) = j["choices"][0]["delta"]["content"].as_str() {
                                if first_token_ms.is_none() && !d.is_empty() {
                                    first_token_ms = Some(started.elapsed().as_millis() as i64);
                                }
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
    if let Some(latency_ms) = first_token_ms {
        let (prompt, completion, estimated) = resolve_usage(prompt_tokens, completion_tokens, &messages, &full);
        if let Err(e) = crate::db::record_chat_usage(&db, conv_id, prompt, completion, estimated, latency_ms) {
            eprintln!("[chat] failed to record usage: {e}");
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn msg(content: &str) -> ChatMessage {
        ChatMessage { role: "user".into(), content: content.into() }
    }

    #[test]
    fn estimates_about_four_chars_per_token_rounding_up() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
        assert_eq!(estimate_tokens("héllo wörld!"), 3); // counts chars, not bytes
    }

    #[test]
    fn resolve_usage_prefers_reported_numbers() {
        let m = [msg("hello there")];
        assert_eq!(resolve_usage(Some(7), Some(9), &m, "reply"), (7, 9, false));
        assert_eq!(resolve_usage(Some(7), None, &m, "reply"), (7, 0, false));
    }

    #[test]
    fn resolve_usage_estimates_when_nothing_reported() {
        let m = [msg("12345678"), msg("1234")];
        assert_eq!(resolve_usage(None, None, &m, "12345"), (3, 2, true));
    }

    /// Serve one canned HTTP response per connection; returns the base URL and
    /// a handle yielding every raw request body received.
    fn fake_server(responses: Vec<&'static str>) -> (String, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let handle = thread::spawn(move || {
            let mut bodies = Vec::new();
            for resp in responses {
                let (mut sock, _) = listener.accept().unwrap();
                let mut buf = vec![0u8; 8192];
                let n = sock.read(&mut buf).unwrap();
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                bodies.push(req.split("\r\n\r\n").nth(1).unwrap_or("").to_string());
                sock.write_all(resp.as_bytes()).unwrap();
            }
            bodies
        });
        (base, handle)
    }

    const OK: &str = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    const BAD: &str = "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

    #[tokio::test]
    async fn requests_usage_and_keeps_it_when_server_accepts() {
        let (base, srv) = fake_server(vec![OK]);
        let client = reqwest::Client::new();
        let r = open_stream(&client, &base, "", "m", &[msg("hi")]).await.unwrap();
        assert!(r.status().is_success());
        let bodies = srv.join().unwrap();
        assert!(bodies[0].contains("include_usage"));
        assert!(!no_usage_option().lock().unwrap().contains(&base));
    }

    #[tokio::test]
    async fn retries_without_stream_options_on_400_and_remembers() {
        let (base, srv) = fake_server(vec![BAD, OK]);
        let client = reqwest::Client::new();
        let r = open_stream(&client, &base, "", "m", &[msg("hi")]).await.unwrap();
        assert!(r.status().is_success());
        let bodies = srv.join().unwrap();
        assert!(bodies[0].contains("include_usage"));
        assert!(!bodies[1].contains("stream_options"));
        assert!(no_usage_option().lock().unwrap().contains(&base));

        // Next request skips stream_options straight away.
        let (base2, srv2) = fake_server(vec![OK]);
        no_usage_option().lock().unwrap().insert(base2.clone());
        open_stream(&client, &base2, "", "m", &[msg("hi")]).await.unwrap();
        assert!(!srv2.join().unwrap()[0].contains("stream_options"));
    }

    #[tokio::test]
    async fn unrelated_400_is_returned_and_server_not_marked() {
        let (base, srv) = fake_server(vec![BAD, BAD]);
        let client = reqwest::Client::new();
        let r = open_stream(&client, &base, "", "bad-model", &[msg("hi")]).await.unwrap();
        assert_eq!(r.status().as_u16(), 400);
        srv.join().unwrap();
        assert!(!no_usage_option().lock().unwrap().contains(&base));
    }
}
