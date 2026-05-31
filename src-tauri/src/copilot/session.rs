//! Session lifecycle: start audio + stt + orchestrator; stop in reverse;
//! optionally purge ephemeral data on stop.

use std::sync::{Arc, Mutex};

use crate::copilot::audio::AudioCapture;
use crate::copilot::orchestrator::{Orchestrator, OrchestratorConfig};
use crate::copilot::preset::Preset;
use crate::copilot::stt::SttBackend;

pub struct ActiveSession {
    pub id:           i64,
    pub preset_id:    String,
    pub started_at:   i64,
    pub save:         bool,
    pub audio:        AudioCapture,
    pub stt:          Box<dyn SttBackend>,
    pub orchestrator: Arc<Orchestrator>,
}

pub struct CopilotState(pub Mutex<Option<ActiveSession>>);

impl CopilotState {
    pub fn new() -> Self {
        Self(Mutex::new(None))
    }
}

#[derive(serde::Serialize, Clone)]
pub struct SessionStatus {
    pub id:         i64,
    pub preset_id:  String,
    pub started_at: i64,
}

pub fn current_status(state: &CopilotState) -> Option<SessionStatus> {
    let guard = state.0.lock().unwrap();
    guard.as_ref().map(|s| SessionStatus {
        id:         s.id,
        preset_id:  s.preset_id.clone(),
        started_at: s.started_at,
    })
}

/// Build the OrchestratorConfig from a preset + user-chosen overrides + global config.
pub fn build_orchestrator_config(
    session_id: i64,
    preset: Preset,
    context_window_s: u64,
    save_transcript: bool,
    app_cfg: &crate::config::Config,
) -> OrchestratorConfig {
    OrchestratorConfig {
        session_id,
        preset,
        context_window_s,
        min_interval_s:  app_cfg.copilot_min_interval_s,
        debounce_ms:     1200,
        api_url:         "https://api.openai.com".into(),
        api_key:         String::new(),
        model:           app_cfg.whisper_model.clone(),
        save_transcript,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::copilot::preset::builtin_presets;

    #[test]
    fn build_config_uses_min_interval_from_settings() {
        let mut cfg = Config::default();
        cfg.copilot_min_interval_s = 7;
        let preset = builtin_presets()[0].clone();
        let o = build_orchestrator_config(42, preset, 60, true, &cfg);
        assert_eq!(o.session_id, 42);
        assert_eq!(o.min_interval_s, 7);
        assert_eq!(o.context_window_s, 60);
        assert!(o.save_transcript);
    }

    #[test]
    fn current_status_is_none_when_no_session() {
        let st = CopilotState::new();
        assert!(current_status(&st).is_none());
    }
}
