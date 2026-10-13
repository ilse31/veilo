//! "Read my screen": capture a monitor on a global shortcut and ask the vision
//! model about it. Reuses the orchestrator's `copilot-suggest-*` events so the
//! AnswerCard shows the result as one more Q&A in the thread.

use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

use crate::screen_capture::{self, MonitorChoice};

pub const QUESTION: &str = "Baca layar";

#[derive(Default)]
pub struct ScreenState {
    /// Bumped by every request; a streaming task stops when it no longer matches.
    pub generation: AtomicU64,
    /// Bumped by every cycle press; only the latest press captures.
    pub cycle_ticket: AtomicU64,
    /// Monitor index chosen by the cycle shortcut (None = nothing chosen yet).
    pub selection: Mutex<Option<usize>>,
}

pub fn vision_gate(model: &str, overrides: &[String]) -> Result<(), String> {
    let model = model.trim();
    if model.is_empty() {
        return Err("Belum ada model dipilih. Pilih model di Pengaturan → Koneksi.".into());
    }
    if overrides.iter().any(|o| o == model) || crate::vision_models::is_vision_model(model) {
        return Ok(());
    }
    Err(format!(
        "Model {model} tidak mendukung gambar. Pilih model bertanda 👁 di Pengaturan → Koneksi \
         (atau tandai manual jika model ini sebenarnya mendukung)."
    ))
}

pub fn system_prompt(force_indonesian: bool) -> String {
    let mut s = String::from(
        "Kamu asisten yang membaca tangkapan layar pengguna. Pahami apa yang tampil \
         (soal, kode, pesan, form, dokumen) lalu langsung beri jawaban atau bantuan yang \
         paling berguna, singkat dan to the point. Jika ada pertanyaan/soal di layar, jawab itu.",
    );
    if force_indonesian {
        s.push_str("\n\nSELALU tulis jawabanmu dalam Bahasa Indonesia.");
    }
    s
}

pub fn build_messages(system: &str, context: &str, data_uri: &str) -> Vec<serde_json::Value> {
    let mut text = String::from("Baca layar ini dan jawab/bantu apa yang terlihat.");
    if !context.trim().is_empty() {
        text = format!(
            "Konteks percakapan terakhir (latar saja):\n{context}\n\n{text}"
        );
    }
    vec![
        serde_json::json!({ "role": "system", "content": system }),
        serde_json::json!({
            "role": "user",
            "content": [
                { "type": "text", "text": text },
                { "type": "image_url", "image_url": { "url": data_uri } },
            ],
        }),
    ]
}

pub fn friendly_http_error(status: u16, body: &str) -> String {
    let snippet: String = body.chars().take(200).collect();
    match status {
        400 | 415 | 422 => format!(
            "Endpoint ini tampaknya tidak menerima gambar (HTTP {status}). \
             Coba model/provider lain yang mendukung vision. {snippet}"
        ),
        _ if snippet.is_empty() => format!("HTTP {status}"),
        _ => format!("HTTP {status}: {snippet}"),
    }
}

#[derive(Serialize, Clone)]
struct CaptureStarted { id: String, question: String, thumbnail: String }
#[derive(Serialize, Clone)]
struct Delta { id: String, question: String, delta: String, format: &'static str }
#[derive(Serialize, Clone)]
struct Done { id: String, question: String, full_text: String }

fn show_card(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("copilot-card") {
        crate::protection::show_no_activate(&w);
    }
}

/// Surface a problem as a finished answer in the card (no new event type).
fn notice(app: &AppHandle, id: &str, question: &str, text: &str) {
    show_card(app);
    let _ = app.emit("copilot-suggest-delta", Delta {
        id: id.into(), question: question.into(), delta: text.into(), format: "Bullets",
    });
    let _ = app.emit("copilot-suggest-done", Done {
        id: id.into(), question: question.into(), full_text: text.into(),
    });
}

fn profile(app: &AppHandle, key: &str) -> Option<String> {
    let db = app.try_state::<crate::db::Db>()?;
    crate::db::get_value(&db, key).ok().flatten().filter(|v| !v.trim().is_empty())
}

pub async fn run(app: AppHandle, choice: MonitorChoice) {
    let id = uuid::Uuid::new_v4().to_string();
    let state = app.state::<ScreenState>();
    let my_gen = state.generation.fetch_add(1, Ordering::SeqCst) + 1;
    *state.selection.lock().unwrap() = None;

    // 1. Credentials + capability gate (before touching the screen).
    let base = profile(&app, "api_url")
        .unwrap_or_else(|| "https://api.openai.com".into())
        .trim_end_matches('/')
        .to_string();
    let key = profile(&app, "api_key").unwrap_or_default();
    let model = profile(&app, "model").unwrap_or_default();
    let overrides: Vec<String> = profile(&app, "vision_overrides")
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    if let Err(msg) = vision_gate(&model, &overrides) {
        notice(&app, &id, QUESTION, &msg);
        return;
    }

    // 2. Capture (blocking OS calls).
    if !screen_capture::screen_access_granted() {
        notice(&app, &id, QUESTION,
            "Izin Perekaman Layar belum diberikan. Buka Pengaturan Sistem → Privasi & Keamanan → \
             Perekaman Layar, aktifkan Veilo, lalu tekan pintasan lagi.");
        return;
    }
    let shot = match tauri::async_runtime::spawn_blocking(move || screen_capture::capture(choice)).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => { notice(&app, &id, QUESTION, &e); return; }
        Err(e) => { notice(&app, &id, QUESTION, &format!("Gagal menangkap layar: {e}")); return; }
    };

    // 3. Optional transcript context + language preference.
    let include = profile(&app, "screen_include_transcript").as_deref() != Some("0");
    let context = if include {
        app.try_state::<crate::copilot::session::CopilotState>()
            .and_then(|st| st.0.lock().ok().and_then(|g| g.as_ref().map(|s| s.orchestrator.recent_context())))
            .unwrap_or_default()
    } else {
        String::new()
    };
    let force_id = app.try_state::<crate::copilot::session::CopilotState>()
        .and_then(|st| st.0.lock().ok().and_then(|g| g.as_ref().map(|s|
            s.orchestrator.force_indonesian.load(Ordering::Relaxed))))
        .unwrap_or_else(|| {
            app.path().app_data_dir().ok()
                .map(|d| crate::config::read_config(&d).force_indonesian)
                .unwrap_or(false)
        });

    let question = format!("{QUESTION}: {}", shot.monitor_name);
    show_card(&app);
    let _ = app.emit("screen-capture-started", CaptureStarted {
        id: id.clone(), question: question.clone(), thumbnail: shot.thumbnail.clone(),
    });

    // 4. Stream.
    let messages = build_messages(&system_prompt(force_id), &context, &shot.data_uri);
    let body = serde_json::json!({ "model": model, "messages": messages, "stream": true });
    let client = match reqwest::Client::builder().timeout(Duration::from_secs(90)).build() {
        Ok(c) => c,
        Err(e) => { notice(&app, &id, &question, &e.to_string()); return; }
    };
    let mut req = client.post(format!("{base}/v1/chat/completions")).json(&body);
    if !key.is_empty() { req = req.bearer_auth(&key); }
    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => { notice(&app, &id, &question, &format!("Gagal menghubungi endpoint: {e}")); return; }
    };
    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        notice(&app, &id, &question, &friendly_http_error(status, &text));
        return;
    }

    use futures_util::StreamExt;
    let mut stream = resp.bytes_stream();
    let mut leftover = String::new();
    let mut full = String::new();
    'outer: while let Some(chunk) = stream.next().await {
        if state.generation.load(Ordering::SeqCst) != my_gen { break; } // superseded
        let Ok(bytes) = chunk else { break };
        leftover.push_str(&String::from_utf8_lossy(&bytes));
        let mut lines: Vec<String> = std::mem::take(&mut leftover).split('\n').map(String::from).collect();
        leftover = lines.pop().unwrap_or_default();
        for line in lines {
            let Some(data) = line.trim().strip_prefix("data: ") else { continue };
            if data == "[DONE]" { break 'outer; }
            let Ok(j) = serde_json::from_str::<serde_json::Value>(data) else { continue };
            if let Some(d) = j["choices"][0]["delta"]["content"].as_str() {
                full.push_str(d);
                let _ = app.emit("copilot-suggest-delta", Delta {
                    id: id.clone(), question: question.clone(), delta: d.into(), format: "Bullets",
                });
            }
        }
    }
    let _ = app.emit("copilot-suggest-done", Done { id, question, full_text: full });
}

/// Shortcut A: capture the monitor under the cursor now.
pub fn trigger_cursor(app: AppHandle) {
    let point = app.cursor_position().ok().map(|p| (p.x as i32, p.y as i32));
    tauri::async_runtime::spawn(async move { run(app, MonitorChoice::Cursor(point)).await });
}

/// Shortcut B: advance the chosen monitor; capture it once presses stop for 1.5 s.
pub fn trigger_cycle(app: AppHandle) {
    let count = screen_capture::monitor_count();
    let state = app.state::<ScreenState>();
    if count == 0 {
        notice(&app, &uuid::Uuid::new_v4().to_string(), QUESTION, "Tidak ada layar yang terdeteksi.");
        return;
    }
    let idx = {
        let mut sel = state.selection.lock().unwrap();
        let next = screen_capture::next_selection(*sel, count);
        *sel = Some(next);
        next
    };
    let ticket = state.cycle_ticket.fetch_add(1, Ordering::SeqCst) + 1;
    show_card(&app);
    let _ = app.emit("screen-monitor-selected", serde_json::json!({
        "index": idx, "total": count, "name": screen_capture::monitor_name(idx),
    }));
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(1500)).await;
        let still_latest = app.state::<ScreenState>().cycle_ticket.load(Ordering::SeqCst) == ticket;
        if still_latest {
            run(app, MonitorChoice::Index(idx)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_rejects_unknown_text_model_and_names_it() {
        let err = vision_gate("llama-3.1-70b-versatile", &[]).unwrap_err();
        assert!(err.contains("llama-3.1-70b-versatile"));
        assert!(err.contains("gambar"));
    }

    #[test]
    fn gate_accepts_vision_model_and_manual_override() {
        assert!(vision_gate("gpt-4o", &[]).is_ok());
        assert!(vision_gate("my-custom-vlm", &["my-custom-vlm".to_string()]).is_ok());
    }

    #[test]
    fn gate_rejects_empty_model() {
        let err = vision_gate("  ", &[]).unwrap_err();
        assert!(err.contains("model"));
    }

    #[test]
    fn messages_have_system_then_user_with_image_part() {
        let m = build_messages("SYS", "Q: apa itu X?", "data:image/jpeg;base64,AAAA");
        assert_eq!(m.len(), 2);
        assert_eq!(m[0]["role"], "system");
        assert_eq!(m[0]["content"], "SYS");
        assert_eq!(m[1]["role"], "user");
        let parts = m[1]["content"].as_array().expect("content is an array of parts");
        assert_eq!(parts[0]["type"], "text");
        assert!(parts[0]["text"].as_str().unwrap().contains("Q: apa itu X?"));
        assert_eq!(parts[1]["type"], "image_url");
        assert_eq!(parts[1]["image_url"]["url"], "data:image/jpeg;base64,AAAA");
    }

    #[test]
    fn messages_without_context_still_ask_about_the_screen() {
        let m = build_messages("SYS", "", "data:image/jpeg;base64,AAAA");
        let text = m[1]["content"][0]["text"].as_str().unwrap();
        assert!(text.to_lowercase().contains("layar"));
        assert!(!text.contains("Konteks percakapan"));
    }

    #[test]
    fn http_errors_are_friendly() {
        assert!(friendly_http_error(400, "image_url is not supported").contains("gambar"));
        assert!(friendly_http_error(415, "").contains("gambar"));
        assert!(friendly_http_error(401, "bad key").contains("401"));
    }

    #[test]
    fn newer_request_invalidates_older_generation() {
        let s = ScreenState::default();
        let a = s.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let b = s.generation.fetch_add(1, Ordering::SeqCst) + 1;
        assert_ne!(s.generation.load(Ordering::SeqCst), a);
        assert_eq!(s.generation.load(Ordering::SeqCst), b);
    }
}
