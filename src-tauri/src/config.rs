use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub site: String,
    pub hotkey: String,
    pub language: String,
    pub opacity: f32,
    pub autostart: bool,
    pub position: Option<(i32, i32)>,
    // Copilot additions
    #[serde(default = "default_stt_backend")]
    pub stt_backend:            String,
    #[serde(default = "default_whisper_model")]
    pub whisper_model:          String,
    #[serde(default = "default_copilot_context_s")]
    pub copilot_context_s:      u64,
    #[serde(default = "default_copilot_auto_dismiss_s")]
    pub copilot_auto_dismiss_s: u64,
    #[serde(default = "default_copilot_min_interval_s")]
    pub copilot_min_interval_s: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            site: "https://claude.ai".into(),
            hotkey: "Ctrl+Shift+Space".into(),
            language: "id-ID".into(),
            opacity: 0.9,
            autostart: false,
            position: None,
            stt_backend:            "whisper-cloud".into(),
            whisper_model:          "whisper-1".into(),
            copilot_context_s:      90,
            copilot_auto_dismiss_s: 25,
            copilot_min_interval_s: 4,
        }
    }
}

fn default_stt_backend()            -> String { "whisper-cloud".into() }
fn default_whisper_model()          -> String { "whisper-1".into() }
fn default_copilot_context_s()      -> u64    { 90 }
fn default_copilot_auto_dismiss_s() -> u64    { 25 }
fn default_copilot_min_interval_s() -> u64    { 4 }

pub fn config_path(app_data_dir: &PathBuf) -> PathBuf {
    app_data_dir.join("config.json")
}

pub fn read_config(app_data_dir: &PathBuf) -> Config {
    let path = config_path(app_data_dir);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn write_config(app_data_dir: &PathBuf, config: &Config) -> Result<(), String> {
    let path = config_path(app_data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_config_values() {
        let c = Config::default();
        assert_eq!(c.site, "https://claude.ai");
        assert_eq!(c.hotkey, "Ctrl+Shift+Space");
        assert_eq!(c.language, "id-ID");
        assert!((c.opacity - 0.9).abs() < 0.001);
        assert!(!c.autostart);
        assert!(c.position.is_none());
    }

    #[test]
    fn roundtrip_write_and_read() {
        let dir = TempDir::new().unwrap();
        let config = Config {
            site: "https://chatgpt.com".into(),
            hotkey: "Ctrl+Shift+A".into(),
            language: "en-US".into(),
            opacity: 0.8,
            autostart: true,
            position: Some((100, 200)),
            stt_backend: "whisper-cloud".into(),
            whisper_model: "whisper-1".into(),
            copilot_context_s: 90,
            copilot_auto_dismiss_s: 25,
            copilot_min_interval_s: 4,
        };
        write_config(&dir.path().to_path_buf(), &config).unwrap();
        let loaded = read_config(&dir.path().to_path_buf());
        assert_eq!(loaded, config);
    }

    #[test]
    fn read_returns_default_when_file_missing() {
        let dir = TempDir::new().unwrap();
        let config = read_config(&dir.path().to_path_buf());
        assert_eq!(config, Config::default());
    }

    #[test]
    fn read_returns_default_when_file_invalid() {
        let dir = TempDir::new().unwrap();
        let path = config_path(&dir.path().to_path_buf());
        std::fs::write(&path, "not valid json").unwrap();
        let config = read_config(&dir.path().to_path_buf());
        assert_eq!(config, Config::default());
    }

    #[test]
    fn default_includes_copilot_fields() {
        let c = Config::default();
        assert_eq!(c.stt_backend, "whisper-cloud");
        assert_eq!(c.whisper_model, "whisper-1");
        assert_eq!(c.copilot_context_s, 90);
        assert_eq!(c.copilot_auto_dismiss_s, 25);
        assert_eq!(c.copilot_min_interval_s, 4);
    }

    #[test]
    fn copilot_fields_roundtrip() {
        let dir = TempDir::new().unwrap();
        let mut c = Config::default();
        c.stt_backend = "whisper-local".into();
        c.copilot_context_s = 120;
        c.copilot_min_interval_s = 10;
        write_config(&dir.path().to_path_buf(), &c).unwrap();
        let loaded = read_config(&dir.path().to_path_buf());
        assert_eq!(loaded, c);
    }

    #[test]
    fn old_config_without_copilot_fields_uses_defaults() {
        let dir = TempDir::new().unwrap();
        let path = config_path(&dir.path().to_path_buf());
        let old_json = r#"{"site":"https://claude.ai","hotkey":"Ctrl+Shift+Space","language":"id-ID","opacity":0.9,"autostart":false,"position":null}"#;
        std::fs::write(&path, old_json).unwrap();
        let loaded = read_config(&dir.path().to_path_buf());
        assert_eq!(loaded.stt_backend, "whisper-cloud");
        assert_eq!(loaded.copilot_context_s, 90);
    }
}
