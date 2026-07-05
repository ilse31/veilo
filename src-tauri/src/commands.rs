use crate::config::{read_config, write_config, Config};
use crate::stt::{start_stt, SttHandle};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Listener};

pub struct SttState(pub Mutex<Option<SttHandle>>);

#[tauri::command]
pub fn start_stt_cmd(
    app: AppHandle,
    state: tauri::State<SttState>,
) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    // A handle whose worker thread already died (e.g. Windows speech
    // recognition disabled) is stale — clear it so STT can restart.
    if let Some(h) = guard.as_ref() {
        if !h.is_finished() {
            return Ok(());
        }
        *guard = None;
    }
    let handle = start_stt(app.clone())?;
    *guard = Some(handle);
    Ok(())
}

#[tauri::command]
pub fn stop_stt_cmd(state: tauri::State<SttState>) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(handle) = guard.take() {
        handle.stop();
    }
    Ok(())
}

/// Inject transcribed text into the AI webview via the bootstrap JS function.
#[tauri::command]
pub fn inject_text(app: AppHandle, text: String) -> Result<(), String> {
    let window = app
        .get_webview_window("ai-view")
        .ok_or_else(|| "ai-view window not found".to_string())?;
    let escaped = text
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r");
    window
        .eval(&format!("window.__noscreen_inject('{escaped}')"))
        .map_err(|e| e.to_string())
}

/// Show or hide both ai-view and control-bar windows together.
///
/// A minimized window still reports is_visible() = true on Windows,
/// so we treat "visible AND not minimized" as the "showing" state.
pub fn toggle_visibility(app: AppHandle) {
    let hub = match app.get_webview_window("ai-view") {
        Some(w) => w,
        None => return,
    };

    // Use OS-level check because we show via SW_SHOWNOACTIVATE which bypasses
    // Tauri's internal visible flag — is_visible() would wrongly return false.
    let currently_shown = crate::protection::is_os_visible(&hub);

    if currently_shown {
        let _ = hub.set_ignore_cursor_events(false);
        // Use Win32 directly — Tauri's hide() is a no-op when Tauri's internal
        // visible flag is already false (which happens after show_no_activate).
        crate::protection::hide_window(&hub);
        for service in ["gpt", "claude", "translate"] {
            if let Some(w) = app.get_webview_window(&format!("svc-{service}")) {
                crate::protection::hide_window(&w);
            }
        }
    } else {
        // SW_SHOWNOACTIVATE: shows without stealing foreground → browser never blurs.
        crate::protection::show_no_activate(&hub);
    }
}

/// Show and focus the settings window.
pub fn open_settings(app: AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Tauri command wrapper for toggle_visibility (callable from frontend JS).
#[tauri::command]
pub fn toggle_visibility_cmd(app: AppHandle) {
    toggle_visibility(app);
}

/// Tauri command wrapper for open_settings (callable from frontend JS).
#[tauri::command]
pub fn open_settings_cmd(app: AppHandle) {
    open_settings(app);
}

/// Read persisted config from disk.
#[tauri::command]
pub fn get_config(app: AppHandle) -> Result<Config, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(read_config(&dir))
}

/// Write config to disk.
#[tauri::command]
pub fn save_config(app: AppHandle, mut config: Config) -> Result<(), String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let old_config = read_config(&dir);

    // 1. Dynamic hotkey registration — register the NEW one first so that if it
    // fails (invalid/conflicting accelerator) the old hotkey keeps working and
    // the hidden hub can still be summoned.
    if old_config.hotkey != config.hotkey {
        use tauri_plugin_global_shortcut::GlobalShortcutExt;

        let handle = app.clone();
        let new_hotkey = config.hotkey.clone();
        app.global_shortcut()
            .on_shortcut(new_hotkey.as_str(), move |_app, _shortcut, event| {
                if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                    toggle_visibility(handle.clone());
                }
            })
            .map_err(|e| format!("Gagal mendaftarkan hotkey baru '{}': {}", new_hotkey, e))?;

        // Only after the new one is live, drop the old one.
        if let Ok(old_shortcut) = old_config.hotkey.parse::<tauri_plugin_global_shortcut::Shortcut>() {
            let _ = app.global_shortcut().unregister(old_shortcut);
        }
    }

    // 2. Autostart sync
    use tauri_plugin_autostart::ManagerExt;
    let autostart_manager = app.autolaunch();
    if config.autostart {
        let _ = autostart_manager.enable();
    } else {
        let _ = autostart_manager.disable();
    }

    // 3. Preserve window position if incoming is None
    if config.position.is_none() {
        config.position = old_config.position;
    }

    write_config(&dir, &config)
}

/// Generic command to read value from SQLite profile table.
#[tauri::command]
pub fn get_profile_value(db: tauri::State<crate::db::Db>, key: String) -> Option<String> {
    crate::db::get_value(&db, &key).unwrap_or(None)
}

/// Generic command to write value to SQLite profile table.
#[tauri::command]
pub fn set_profile_value(db: tauri::State<crate::db::Db>, key: String, value: String) -> Result<(), String> {
    crate::db::set_value(&db, &key, &value).map_err(|e| e.to_string())
}

/// Navigate Window 1 (ai-view) to a new AI site URL.
#[tauri::command]
pub fn set_site(app: AppHandle, site: String) -> Result<(), String> {
    let parsed = url::Url::parse(&site).map_err(|_| format!("invalid URL: {site}"))?;
    match parsed.scheme() {
        "https" | "http" => {}
        other => return Err(format!("disallowed scheme: {other}")),
    }
    let window = app
        .get_webview_window("ai-view")
        .ok_or_else(|| "ai-view window not found".to_string())?;
    window
        .eval(&format!("window.location.href = '{}'", parsed.as_str().replace('\'', "\\'")))
        .map_err(|e| e.to_string())
}

/// Compute the hub window's logical origin on screen (physical position ÷ scale factor).
/// On Windows, we use ClientToScreen to obtain the exact screen coordinates of the client area's top-left corner,
/// which correctly accounts for native maximize borders and margins.
fn hub_logical_origin(app: &AppHandle) -> Result<(f64, f64), String> {
    let hub = app
        .get_webview_window("ai-view")
        .ok_or_else(|| "hub window not found".to_string())?;

    #[cfg(windows)]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        if let Ok(handle) = hub.window_handle() {
            if let RawWindowHandle::Win32(h) = handle.as_raw() {
                let hwnd = windows::Win32::Foundation::HWND(h.hwnd.get() as *mut core::ffi::c_void);
                let mut point = windows::Win32::Foundation::POINT { x: 0, y: 0 };
                unsafe {
                    use windows::Win32::Graphics::Gdi::ClientToScreen;
                    if ClientToScreen(hwnd, &mut point).as_bool() {
                        let scale = hub.scale_factor().map_err(|e| e.to_string())?;
                        return Ok((point.x as f64 / scale, point.y as f64 / scale));
                    }
                }
            }
        }
    }

    let phys = hub.outer_position().map_err(|e| e.to_string())?;
    let scale = hub.scale_factor().map_err(|e| e.to_string())?;
    Ok((phys.x as f64 / scale, phys.y as f64 / scale))
}

/// Open (or show + reposition) a service webview.
///
/// x/y are content-area-relative logical coords (from getBoundingClientRect in the hub).
/// Service windows are pre-created hidden at startup (lib.rs).
/// This command simply repositions and shows the already-initialized window.
#[tauri::command]
pub fn open_service_webview(
    app: AppHandle,
    service: String,
    _url: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    if width <= 0.0 || height <= 0.0 {
        return Ok(());
    }

    if let Some(hub) = app.get_webview_window("ai-view") {
        if hub.is_minimized().unwrap_or(false) || !crate::protection::is_os_visible(&hub) {
            return Ok(());
        }
    }

    let label = format!("svc-{service}");

    let (hub_lx, hub_ly) = hub_logical_origin(&app)?;
    let screen_x = hub_lx + x;
    let screen_y = hub_ly + y;
    eprintln!("[svc] show: service={service} screen=({screen_x},{screen_y}) size=({width}×{height})");

    let win = app
        .get_webview_window(&label)
        .ok_or_else(|| format!("'{label}' not found — was it pre-created at startup?"))?;

    win.set_position(LogicalPosition::new(screen_x, screen_y)).map_err(|e| e.to_string())?;
    win.set_size(LogicalSize::new(width, height)).map_err(|e| e.to_string())?;
    // Win32 show, not Tauri show(): the hotkey hide path uses protection::hide_window
    // which leaves Tauri's internal visible flag `true`, making Tauri's show() a no-op.
    crate::protection::show_no_activate(&win);

    Ok(())
}

/// Hide a service window (keeps it alive for fast re-show).
#[tauri::command]
pub fn hide_service_webview(app: AppHandle, service: String) -> Result<(), String> {
    let label = format!("svc-{service}");
    if let Some(w) = app.get_webview_window(&label) {
        // Win32 hide — Tauri's hide() is a no-op after show_no_activate.
        crate::protection::hide_window(&w);
    }
    Ok(())
}

/// Enable or disable content protection on every app window at runtime.
///
/// protected = true  → hidden from all screen capture / recording tools (default)
/// protected = false → visible in screenshots and screen sharing
#[tauri::command]
pub fn set_all_content_protected(app: AppHandle, protected: bool) -> Result<(), String> {
    for label in ["ai-view", "settings", "copilot-card"] {
        if let Some(w) = app.get_webview_window(label) {
            w.set_content_protected(protected).map_err(|e| e.to_string())?;
        }
    }
    for service in ["gpt", "claude", "translate"] {
        let label = format!("svc-{service}");
        if let Some(w) = app.get_webview_window(&label) {
            w.set_content_protected(protected).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Reposition and resize a service window (e.g. on hub window resize or move).
#[tauri::command]
pub fn resize_service_webview(
    app: AppHandle,
    service: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    if width <= 0.0 || height <= 0.0 {
        return Ok(());
    }

    if let Some(hub) = app.get_webview_window("ai-view") {
        if hub.is_minimized().unwrap_or(false) || !crate::protection::is_os_visible(&hub) {
            return Ok(());
        }
    }

    let label = format!("svc-{service}");

    let (hub_lx, hub_ly) = hub_logical_origin(&app)?;
    let screen_x = hub_lx + x;
    let screen_y = hub_ly + y;

    if let Some(w) = app.get_webview_window(&label) {
        w.set_position(LogicalPosition::new(screen_x, screen_y))
            .map_err(|e| e.to_string())?;
        w.set_size(LogicalSize::new(width, height))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Inject text into a service webview (svc-gpt, svc-claude, svc-translate).
/// Uses a JS retry loop so it works even if the page is still loading.
#[tauri::command]
pub fn inject_to_service(app: AppHandle, service: String, text: String) -> Result<(), String> {
    let label = format!("svc-{service}");
    let win = app
        .get_webview_window(&label)
        .ok_or_else(|| format!("'{label}' not found"))?;

    let escaped = text
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r");

    // Retry up to 15× with 300 ms gaps (4.5 s total) to handle pages still loading
    let script = format!(
        r#"(function tryInject(t, n) {{
            if (typeof window.__noscreen_inject === 'function') {{
                window.__noscreen_inject(t);
            }} else if (n > 0) {{
                setTimeout(function() {{ tryInject(t, n - 1); }}, 300);
            }}
        }})('{escaped}', 15);"#
    );

    win.eval(&script).map_err(|e| e.to_string())
}

/// Read the user's display name from the SQLite profile database.
#[tauri::command]
pub fn get_profile_name(db: tauri::State<crate::db::Db>) -> Option<String> {
    crate::db::get_value(&db, "name").unwrap_or(None)
}

/// Write (upsert) the user's display name to the SQLite profile database.
#[tauri::command]
pub fn set_profile_name(db: tauri::State<crate::db::Db>, name: String) -> Result<(), String> {
    crate::db::set_value(&db, "name", &name).map_err(|e| e.to_string())
}

/// List all conversations, newest first.
#[tauri::command]
pub fn list_conversations(db: tauri::State<crate::db::Db>) -> Result<Vec<crate::db::ConvRow>, String> {
    crate::db::list_conversations(&db).map_err(|e| e.to_string())
}

/// Create a new conversation and return its id.
#[tauri::command]
pub fn create_conversation(db: tauri::State<crate::db::Db>, title: String) -> Result<i64, String> {
    crate::db::create_conversation(&db, &title).map_err(|e| e.to_string())
}

/// Rename a conversation.
#[tauri::command]
pub fn update_conversation_title(db: tauri::State<crate::db::Db>, conv_id: i64, title: String) -> Result<(), String> {
    crate::db::update_conversation_title(&db, conv_id, &title).map_err(|e| e.to_string())
}

/// Delete a conversation and all its messages.
#[tauri::command]
pub fn delete_conversation(db: tauri::State<crate::db::Db>, conv_id: i64) -> Result<(), String> {
    crate::db::delete_conversation(&db, conv_id).map_err(|e| e.to_string())
}

/// Load all messages for a conversation.
#[tauri::command]
pub fn get_messages(db: tauri::State<crate::db::Db>, conv_id: i64) -> Result<Vec<crate::db::MsgRow>, String> {
    crate::db::get_messages(&db, conv_id).map_err(|e| e.to_string())
}

/// Append a message to a conversation.
#[tauri::command]
pub fn append_message(db: tauri::State<crate::db::Db>, conv_id: i64, role: String, body: String) -> Result<(), String> {
    crate::db::append_message(&db, conv_id, &role, &body).map_err(|e| e.to_string())
}

/// Toggle ghost typing mode — called by the Ctrl+Alt+G global shortcut.
/// Applies WS_EX_NOACTIVATE so the hub never steals focus, then starts
/// the low-level keyboard hook that intercepts all keystrokes.
pub fn toggle_ghost_typing(app: AppHandle) {
    if crate::ghost_typing::is_active() {
        crate::ghost_typing::stop();
        if let Some(w) = app.get_webview_window("ai-view") {
            crate::protection::remove_noactivate(&w);
        }
        let _ = app.emit("ghost-typing-state", false);
    } else {
        if let Some(w) = app.get_webview_window("ai-view") {
            crate::protection::apply_noactivate(&w);
        }
        if let Err(e) = crate::ghost_typing::start(app.clone()) {
            eprintln!("[ghost] start failed: {e}");
            return;
        }
        let _ = app.emit("ghost-typing-state", true);
    }
}

/// Tauri command: start ghost typing from the frontend button.
#[tauri::command]
pub fn start_ghost_typing_cmd(app: AppHandle) -> Result<(), String> {
    if crate::ghost_typing::is_active() {
        return Ok(());
    }
    if let Some(w) = app.get_webview_window("ai-view") {
        crate::protection::apply_noactivate(&w);
    }
    crate::ghost_typing::start(app.clone())?;
    let _ = app.emit("ghost-typing-state", true);
    Ok(())
}

/// Tauri command: stop ghost typing from the frontend (after send or cancel).
#[tauri::command]
pub fn stop_ghost_typing_cmd(app: AppHandle) {
    crate::ghost_typing::stop();
    if let Some(w) = app.get_webview_window("ai-view") {
        crate::protection::remove_noactivate(&w);
    }
    let _ = app.emit("ghost-typing-state", false);
}

/// Toggle click-through (passthrough) mode on the ai-view window.
///
/// enabled = true  → mouse events pass through to whatever is behind the overlay;
///                   the browser underneath stays focused (no blur/visibilitychange).
/// enabled = false → normal interactive mode.
#[tauri::command]
pub fn set_click_through(app: AppHandle, enabled: bool) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("ai-view") {
        w.set_ignore_cursor_events(enabled).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── Copilot commands ─────────────────────────────────────────────────────────

/// Convert config stt_language ("" = auto-detect → None, "en" → Some("en")).
fn stt_language_opt(lang: &str) -> Option<String> {
    if lang.is_empty() { None } else { Some(lang.to_string()) }
}

use std::sync::Arc;
use crate::copilot::orchestrator::Orchestrator;
use crate::copilot::preset::{builtin_presets, find_preset, Preset};
use crate::copilot::session::{ActiveSession, CopilotState, SessionStatus};
use crate::copilot::stt::{whisper_cloud::WhisperCloudStt, SttBackend};

#[tauri::command]
pub fn copilot_get_presets() -> Vec<Preset> {
    builtin_presets()
}

#[tauri::command]
pub fn copilot_session_status(state: tauri::State<CopilotState>) -> Option<SessionStatus> {
    crate::copilot::session::current_status(&state)
}

#[tauri::command]
pub async fn copilot_start_session(
    app: AppHandle,
    state: tauri::State<'_, CopilotState>,
    db: tauri::State<'_, crate::db::Db>,
    preset_id: String,
    context_window_s: u64,
    save: bool,
    api_url: String,
    api_key: String,
    model: String,
    custom_context: String,
) -> Result<i64, String> {
    {
        let guard = state.0.lock().map_err(|e| e.to_string())?;
        if guard.is_some() {
            return Err("Session already running".into());
        }
    }
    let preset = find_preset(&preset_id).ok_or_else(|| format!("unknown preset {preset_id}"))?;
    let app_cfg = crate::config::read_config(
        &app.path().app_data_dir().map_err(|e| e.to_string())?,
    );
    let sid = crate::db::create_copilot_session(
        &db, &preset_id, context_window_s as i64, save,
    ).map_err(|e| e.to_string())?;

    let mut audio = crate::copilot::audio::start_loopback()?;
    let audio_rx = audio.take_rx().ok_or_else(|| "audio rx already taken".to_string())?;

    let mut stt: Box<dyn SttBackend> = match app_cfg.stt_backend.as_str() {
        "whisper-local" => {
            let local_url = if app_cfg.whisper_local_url.is_empty() {
                "http://localhost:8080".to_string()
            } else {
                app_cfg.whisper_local_url.clone()
            };
            let model_name = if app_cfg.whisper_model.is_empty() {
                "whisper-1".to_string()
            } else {
                app_cfg.whisper_model.clone()
            };
            let mut whisper_stt = WhisperCloudStt::new(local_url, api_key.clone(), model_name);
            whisper_stt.language = stt_language_opt(&app_cfg.stt_language);
            Box::new(whisper_stt)
        }
        "windows-sr" => {
            Box::new(crate::copilot::stt::windows_sr::WindowsSrStt::new())
        }
        _ => {
            let cloud_url = if api_url.is_empty() { "https://api.openai.com".to_string() } else { api_url.clone() };
            let model_name = if app_cfg.whisper_model.is_empty() { "whisper-1".to_string() } else { app_cfg.whisper_model.clone() };
            let mut whisper_stt = WhisperCloudStt::new(cloud_url, api_key.clone(), model_name);
            whisper_stt.language = stt_language_opt(&app_cfg.stt_language);
            Box::new(whisper_stt)
        }
    };
    stt.start(sid, audio_rx, app.clone())?;

    let mut orch_cfg = crate::copilot::session::build_orchestrator_config(
        sid, preset.clone(), context_window_s, save, custom_context, &app_cfg,
    );
    orch_cfg.api_url = api_url;
    orch_cfg.api_key = api_key;
    orch_cfg.model   = model;
    let orchestrator = Arc::new(Orchestrator::new(orch_cfg));
    let (orchestrator_handle, listener_id) = orchestrator.clone().start(app.clone());

    {
        let mut guard = state.0.lock().map_err(|e| e.to_string())?;
        // Re-check under the lock: a concurrent start may have won the race
        // during the async setup above. Tear this one down instead of silently
        // dropping the winner (whose orchestrator/listener would leak).
        if guard.is_some() {
            audio.stop();
            stt.stop();
            orchestrator_handle.abort();
            app.unlisten(listener_id);
            let _ = crate::db::end_copilot_session(&db, sid);
            let _ = crate::db::purge_copilot_session_data(&db, sid);
            return Err("Session already running".into());
        }
        *guard = Some(ActiveSession {
            id:                  sid,
            preset_id:           preset_id.clone(),
            started_at:          chrono::Utc::now().timestamp(),
            save,
            audio,
            stt,
            orchestrator,
            orchestrator_handle,
            listener_id,
        });
    }

    // Show copilot-card without activating it — SW_SHOW would focus the card,
    // firing `blur` in the shared browser/meeting window (stealth leak).
    if let Some(w) = app.get_webview_window("copilot-card") {
        crate::protection::show_no_activate(&w);
    }

    let _ = app.emit("copilot-session-started", serde_json::json!({
        "id": sid, "preset_id": preset_id,
    }));
    Ok(sid)
}

#[tauri::command]
pub async fn copilot_stop_session(
    app: AppHandle,
    state: tauri::State<'_, CopilotState>,
    db: tauri::State<'_, crate::db::Db>,
) -> Result<(), String> {
    let session = {
        let mut guard = state.0.lock().map_err(|e| e.to_string())?;
        guard.take()
    };
    let Some(session) = session else { return Ok(()); };

    session.audio.stop();
    session.stt.stop();
    session.orchestrator_handle.abort();
    app.unlisten(session.listener_id);

    let _ = crate::db::end_copilot_session(&db, session.id);
    if !session.save {
        let _ = crate::db::purge_copilot_session_data(&db, session.id);
    }

    if let Some(w) = app.get_webview_window("copilot-card") {
        // Win32 hide — Tauri's hide() is a no-op after show_no_activate.
        crate::protection::hide_window(&w);
    }

    let _ = app.emit("copilot-session-ended", serde_json::json!({ "id": session.id }));
    Ok(())
}

#[tauri::command]
pub fn copilot_force_regenerate(state: tauri::State<CopilotState>) -> Result<(), String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(session) = guard.as_ref() {
        session.orchestrator.force_regenerate();
    }
    Ok(())
}

#[tauri::command]
pub fn copilot_list_sessions(
    db: tauri::State<crate::db::Db>,
) -> Result<Vec<crate::db::CopilotSessionRow>, String> {
    crate::db::list_copilot_sessions(&db).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn copilot_set_custom_instruction(
    state: tauri::State<'_, CopilotState>,
    instruction: String,
) -> Result<(), String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(session) = guard.as_ref() {
        if let Ok(mut inst_lock) = session.orchestrator.custom_instruction.lock() {
            *inst_lock = instruction;
        }
        session.orchestrator.force_regenerate();
    }
    Ok(())
}

/// Test AI endpoint connectivity by sending GET /v1/models.
///
/// Runs from Rust (not the webview) so it bypasses browser CORS enforcement.
/// Mirrors the test path used by the settings UI.
#[tauri::command]
pub async fn test_ai_connection(
    api_url: String,
    api_key: String,
) -> Result<u16, String> {
    let base = api_url.trim_end_matches('/').to_string();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let mut req = client.get(format!("{base}/v1/models"));
    if !api_key.is_empty() {
        req = req.bearer_auth(api_key);
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    Ok(resp.status().as_u16())
}

#[cfg(test)]
mod tests {
    #[test]
    fn inject_text_escapes_single_quotes() {
        let text = "it's a test";
        let escaped = text.replace('\\', "\\\\").replace('\'', "\\'");
        assert_eq!(escaped, "it\\'s a test");
    }

    #[test]
    fn inject_text_escapes_backslashes() {
        let text = "path\\to\\thing";
        let escaped = text.replace('\\', "\\\\").replace('\'', "\\'");
        assert_eq!(escaped, "path\\\\to\\\\thing");
    }

    #[test]
    fn inject_text_clean_string_unchanged() {
        let text = "hello world";
        let escaped = text.replace('\\', "\\\\").replace('\'', "\\'").replace('\n', "\\n").replace('\r', "\\r");
        assert_eq!(escaped, "hello world");
    }

    #[test]
    fn inject_text_escapes_newlines() {
        let text = "line one\nline two\r\nline three";
        let escaped = text.replace('\\', "\\\\").replace('\'', "\\'").replace('\n', "\\n").replace('\r', "\\r");
        assert_eq!(escaped, "line one\\nline two\\r\\nline three");
    }
}
