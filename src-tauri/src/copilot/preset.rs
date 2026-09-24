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
        Preset {
            id:   "interview-frontend".into(),
            name: "Interview – Frontend".into(),
            system_prompt: "You are coaching a frontend engineer in a live technical interview. \
                The transcript shows what the interviewer is asking. Provide: \
                (1) a 1-line direct answer, \
                (2) a concise code/markup example if technical, \
                (3) key talking points (performance, accessibility, browser quirks). \
                Use Indonesian or English to match the interviewer.".into(),
            response_format:   ResponseFormat::Code,
            default_context_s: 120,
        },
        Preset {
            id:   "interview-behavioral".into(),
            name: "Interview – Behavioral".into(),
            system_prompt: "You are coaching a candidate in a behavioral/HR interview. \
                The transcript shows the interviewer's question. Suggest a brief STAR-format \
                (Situation, Task, Action, Result) outline the candidate can adapt on the spot. \
                Keep it to 3-4 short bullets, not a full script.".into(),
            response_format:   ResponseFormat::Bullets,
            default_context_s: 90,
        },
        Preset {
            id:   "sales-call".into(),
            name: "Sales Call".into(),
            system_prompt: "You are a live sales coach listening to a sales call. \
                From the transcript, suggest: \
                (1) how to respond to the prospect's last point or objection, \
                (2) one relevant discovery question to ask next. \
                Keep it to 2-3 crisp bullets.".into(),
            response_format:   ResponseFormat::Bullets,
            default_context_s: 90,
        },
        Preset {
            id:   "meeting-notes".into(),
            name: "Meeting Notes".into(),
            system_prompt: "You are taking live notes in a general meeting. \
                From the transcript, extract: decisions made, action items (with owner if \
                mentioned), and open questions. Output as short bullets grouped under those \
                three headings when applicable.".into(),
            response_format:   ResponseFormat::Bullets,
            default_context_s: 120,
        },
        Preset {
            id:   "customer-support".into(),
            name: "Customer Support".into(),
            system_prompt: "You are assisting a customer support agent during a live call. \
                From the transcript, suggest: \
                (1) a brief empathetic acknowledgment of the customer's issue, \
                (2) the next troubleshooting step or resolution to offer. \
                Keep it to 2-3 bullets, practical and ready to say out loud.".into(),
            response_format:   ResponseFormat::Bullets,
            default_context_s: 90,
        },
    ]
}

pub fn find_preset(id: &str) -> Option<Preset> {
    builtin_presets().into_iter().find(|p| p.id == id)
}

impl From<crate::db::CustomPresetRow> for Preset {
    fn from(row: crate::db::CustomPresetRow) -> Self {
        let response_format = match row.response_format.as_str() {
            "Headline" => ResponseFormat::Headline,
            "Code"     => ResponseFormat::Code,
            _          => ResponseFormat::Bullets,
        };
        Preset {
            id:                 row.id,
            name:               row.name,
            system_prompt:      row.system_prompt,
            response_format,
            default_context_s: row.default_context_s.max(0) as u64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_builtin_presets() {
        let presets = builtin_presets();
        assert_eq!(presets.len(), 8);
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

    #[test]
    fn custom_preset_row_converts_known_response_formats() {
        let row = crate::db::CustomPresetRow {
            id: "custom-1".into(), name: "Mine".into(),
            system_prompt: "Be helpful.".into(),
            response_format: "Code".into(), default_context_s: 90,
        };
        let preset: Preset = row.into();
        assert_eq!(preset.response_format, ResponseFormat::Code);
        assert_eq!(preset.id, "custom-1");
    }

    #[test]
    fn custom_preset_row_falls_back_to_bullets_for_unknown_format() {
        let row = crate::db::CustomPresetRow {
            id: "custom-2".into(), name: "Mine".into(),
            system_prompt: "Be helpful.".into(),
            response_format: "Whatever".into(), default_context_s: 90,
        };
        let preset: Preset = row.into();
        assert_eq!(preset.response_format, ResponseFormat::Bullets);
    }
}
