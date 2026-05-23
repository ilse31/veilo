use crate::config::{read_config, write_config, Config};
use crate::stt::{start_stt, SttHandle};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

pub struct SttState(pub Mutex<Option<SttHandle>>);

#[tauri::command]
pub fn start_stt_cmd(
    app: AppHandle,
    state: tauri::State<SttState>,
) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    if guard.is_some() {
        return Ok(());
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
pub fn toggle_visibility(app: AppHandle) {
    for label in ["ai-view", "control-bar"] {
        if let Some(w) = app.get_webview_window(label) {
            if w.is_visible().unwrap_or(false) {
                let _ = w.hide();
            } else {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }
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
pub fn save_config(app: AppHandle, config: Config) -> Result<(), String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    write_config(&dir, &config)
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
