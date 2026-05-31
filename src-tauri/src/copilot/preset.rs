use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ResponseFormat {
    Bullets,
    Headline,
    Code,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preset {
    pub id:                String,
    pub name:              String,
    pub system_prompt:     String,
    pub response_format:   ResponseFormat,
    pub default_context_s: u64,
}

pub fn builtin_presets() -> Vec<Preset> {
    vec![
        Preset {
            id:   "generic".into(),
            name: "Generic Assistant".into(),
            system_prompt: "You are a real-time assistant. Listen to the conversation \
                transcript and provide a brief, helpful suggestion or answer in 2-4 bullets. \
                Respond in the same language as the transcript.".into(),
            response_format:   ResponseFormat::Bullets,
            default_context_s: 90,
        },
        Preset {
            id:   "interview-backend".into(),
            name: "Interview – Backend".into(),
            system_prompt: "You are coaching a backend engineer in a live technical interview. \
                The transcript shows what the interviewer is asking. Provide: \
                (1) a 1-line direct answer, \
                (2) a concise code example if technical, \
                (3) key talking points. \
                Use Indonesian or English to match the interviewer.".into(),
            response_format:   ResponseFormat::Code,
            default_context_s: 120,
        },
        Preset {
            id:   "daily-standup".into(),
            name: "Daily Standup".into(),
            system_prompt: "You are taking notes in a daily standup. \
                Extract: yesterday/today/blockers per speaker if detectable. \
                Output: 1 headline (status) + brief detail.".into(),
            response_format:   ResponseFormat::Headline,
            default_context_s: 60,
        },
    ]
}

pub fn find_preset(id: &str) -> Option<Preset> {
    builtin_presets().into_iter().find(|p| p.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_builtin_presets() {
        let presets = builtin_presets();
        assert_eq!(presets.len(), 3);
    }

    #[test]
    fn lookup_by_id_finds_generic() {
        assert!(find_preset("generic").is_some());
    }

    #[test]
    fn lookup_unknown_returns_none() {
        assert!(find_preset("nonexistent").is_none());
    }

    #[test]
    fn all_presets_have_unique_ids() {
        let presets = builtin_presets();
        let mut ids: Vec<_> = presets.iter().map(|p| p.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), presets.len());
    }

    #[test]
    fn response_format_serializes_as_string() {
        let bullets = serde_json::to_string(&ResponseFormat::Bullets).unwrap();
        assert_eq!(bullets, r#""Bullets""#);
    }
}
