//! Listens for transcript chunks, debounces, rate-limits, and calls the
//! OpenAI-compatible LLM endpoint with streaming. Emits `copilot-suggest-delta`
//! and `copilot-suggest-done` events.

use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Listener, Manager};
use tokio::sync::mpsc;
use tokio::time::sleep;

use crate::copilot::preset::{Preset, ResponseFormat};
use crate::copilot::transcript::{TranscriptBuffer, TranscriptChunk};

#[derive(Serialize, Clone)]
struct SuggestDelta {
    id: String,
    delta: String,
    format: ResponseFormat,
}

#[derive(Serialize, Clone)]
struct SuggestDone {
    id: String,
    full_text: String,
}

pub struct OrchestratorConfig {
    pub session_id:       i64,
    pub preset:           Preset,
    pub context_window_s: u64,
    pub min_interval_s:   u64,
    pub debounce_ms:      u64,
    pub api_url:          String,
    pub api_key:          String,
    pub model:            String,
    pub save_transcript:  bool,
}

pub struct Orchestrator {
    pub config: OrchestratorConfig,
    pub buffer: Arc<Mutex<TranscriptBuffer>>,
    pub last_trigger: Arc<Mutex<Option<Instant>>>,
    force_tx: mpsc::UnboundedSender<()>,
    force_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<()>>>>,
}

impl Orchestrator {
    pub fn new(config: OrchestratorConfig) -> Self {
        let max_age = (config.context_window_s * 2).max(60);
        let (force_tx, force_rx) = mpsc::unbounded_channel();
        Self {
            config,
            buffer: Arc::new(Mutex::new(TranscriptBuffer::new(max_age))),
            last_trigger: Arc::new(Mutex::new(None)),
            force_tx,
            force_rx: Arc::new(Mutex::new(Some(force_rx))),
        }
    }

    /// Trigger a manual regeneration (bypasses debounce + rate-limit).
    pub fn force_regenerate(&self) {
        let _ = self.force_tx.send(());
    }

    /// Spawn the orchestrator task. Returns immediately.
    pub fn start(self: Arc<Self>, app: AppHandle) {
        let me = self.clone();
        tokio::spawn(async move {
            let _listener = app.listen(
                "copilot-transcript-final",
                {
                    let me = me.clone();
                    move |event| {
                        if let Ok(chunk) = serde_json::from_str::<TranscriptChunk>(event.payload()) {
                            if chunk.session_id == me.config.session_id {
                                me.buffer.lock().unwrap().append(chunk);
                            }
                        }
                    }
                },
            );

            let mut force_rx = me.force_rx.lock().unwrap().take().unwrap();

            loop {
                tokio::select! {
                    _ = sleep(Duration::from_millis(200)) => {
                        // Debounce check: only fire if we have new chunks AND none arrived recently
                        let has_new = !me.buffer.lock().unwrap().since_last_trigger().is_empty();
                        if !has_new { continue; }
                        if me.is_rate_limited() { continue; }
                        me.trigger(&app).await;
                    }
                    Some(()) = force_rx.recv() => {
                        me.trigger(&app).await;
                    }
                }
            }
        });
    }

    pub fn is_rate_limited(&self) -> bool {
        let lt = self.last_trigger.lock().unwrap();
        match *lt {
            Some(t) => t.elapsed() < Duration::from_secs(self.config.min_interval_s),
            None => false,
        }
    }

    pub fn build_messages(&self, context_text: &str) -> Vec<serde_json::Value> {
        vec![
            serde_json::json!({
                "role": "system",
                "content": self.config.preset.system_prompt,
            }),
            serde_json::json!({
                "role": "user",
                "content": format!(
                    "Context terbaru (last {}s of transcript):\n\n{}\n\nBerikan saran/jawaban singkat:",
                    self.config.context_window_s, context_text,
                ),
            }),
        ]
    }

    async fn trigger(&self, app: &AppHandle) {
        let context = self.buffer.lock().unwrap().window(self.config.context_window_s);
        if context.trim().is_empty() { return; }
        self.buffer.lock().unwrap().mark_triggered();
        *self.last_trigger.lock().unwrap() = Some(Instant::now());

        let messages = self.build_messages(&context);
        let id = uuid::Uuid::new_v4().to_string();
        let format = self.config.preset.response_format;
        let api_url = self.config.api_url.trim_end_matches('/').to_string();
        let api_key = self.config.api_key.clone();
        let model   = self.config.model.clone();
        let app_inner = app.clone();
        let id_inner = id.clone();
        let session_id = self.config.session_id;
        let save = self.config.save_transcript;
        let context_for_db = context.clone();

        tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("reqwest client");
            let body = serde_json::json!({
                "model": model,
                "messages": messages,
                "stream": true,
            });
            let resp = client.post(format!("{api_url}/v1/chat/completions"))
                .bearer_auth(api_key)
                .json(&body)
                .send().await;

            let mut full = String::new();
            match resp {
                Ok(r) if r.status().is_success() => {
                    use futures_util::StreamExt;
                    let mut stream = r.bytes_stream();
                    while let Some(chunk) = stream.next().await {
                        let Ok(bytes) = chunk else { break };
                        let text = String::from_utf8_lossy(&bytes);
                        for line in text.lines() {
                            let Some(data) = line.strip_prefix("data: ") else { continue };
                            if data == "[DONE]" { continue; }
                            let Ok(j) = serde_json::from_str::<serde_json::Value>(data) else { continue };
                            if let Some(d) = j["choices"][0]["delta"]["content"].as_str() {
                                full.push_str(d);
                                let _ = app_inner.emit("copilot-suggest-delta", SuggestDelta {
                                    id:    id_inner.clone(),
                                    delta: d.into(),
                                    format,
                                });
                            }
                        }
                    }
                }
                Ok(r) => eprintln!("[orchestrator] HTTP {}", r.status()),
                Err(e) => eprintln!("[orchestrator] request error: {e}"),
            }

            let _ = app_inner.emit("copilot-suggest-done", SuggestDone {
                id: id_inner,
                full_text: full.clone(),
            });

            if save && !full.is_empty() {
                if let Some(db) = app_inner.try_state::<crate::db::Db>() {
                    let fmt = format!("{:?}", format);
                    let _ = crate::db::insert_copilot_suggestion(
                        &db, session_id, &context_for_db, &full, &fmt,
                    );
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copilot::preset::builtin_presets;

    fn cfg() -> OrchestratorConfig {
        OrchestratorConfig {
            session_id:       1,
            preset:           builtin_presets()[0].clone(),
            context_window_s: 90,
            min_interval_s:   4,
            debounce_ms:      1200,
            api_url:          "https://api.openai.com".into(),
            api_key:          "test".into(),
            model:            "gpt-4o-mini".into(),
            save_transcript:  false,
        }
    }

    #[test]
    fn rate_limit_blocks_within_interval() {
        let o = Orchestrator::new(cfg());
        *o.last_trigger.lock().unwrap() = Some(Instant::now());
        assert!(o.is_rate_limited(), "should rate-limit right after a trigger");
    }

    #[test]
    fn rate_limit_clears_after_interval() {
        let o = Orchestrator::new(cfg());
        *o.last_trigger.lock().unwrap() = Some(Instant::now() - Duration::from_secs(10));
        assert!(!o.is_rate_limited(), "should not rate-limit after 10s with 4s interval");
    }

    #[test]
    fn build_messages_includes_system_and_user() {
        let o = Orchestrator::new(cfg());
        let messages = o.build_messages("hello world");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[1]["role"], "user");
        assert!(messages[1]["content"].as_str().unwrap().contains("hello world"));
    }

    #[test]
    fn force_regenerate_does_not_panic() {
        let o = Orchestrator::new(cfg());
        o.force_regenerate();
    }
}
