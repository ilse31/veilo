//! Heuristic "does this model accept image input?" flag for the model picker.
//! `/v1/models` carries no capability data on most providers, so this is a
//! best-effort name match; OpenRouter-style `input_modalities` wins when present.

use serde::Serialize;

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct ModelInfo {
    pub id: String,
    pub vision: bool,
}

/// Never vision, even if the name otherwise matches (audio/embedding/etc.).
const NEVER: &[&str] = &[
    "whisper", "embed", "tts", "transcribe", "moderation", "dall-e", "rerank", "guard",
    "audio", "realtime",
];
const CONTAINS: &[&str] = &[
    "vision", "-vl", "llava", "pixtral", "minicpm-v", "moondream", "gemma3", "gemma-3",
    "llama-4", "llama4",
];
const PREFIXES: &[&str] = &[
    "gpt-4o", "chatgpt-4o", "gpt-4.1", "gpt-4.5", "gpt-4-turbo", "gpt-5",
    "o1-20", "o3-20", "o4-mini",
    "claude-3", "claude-sonnet", "claude-opus", "claude-haiku", "claude-fable",
    "gemini-1.5", "gemini-2", "gemini-3", "gemini-flash", "gemini-exp",
    "grok-4",
];

pub fn is_vision_model(id: &str) -> bool {
    let id = id.rsplit('/').next().unwrap_or(id).to_lowercase();
    if id.is_empty() || NEVER.iter().any(|n| id.contains(n)) {
        return false;
    }
    if CONTAINS.iter().any(|n| id.contains(n)) {
        return true;
    }
    matches!(id.as_str(), "o1" | "o3" | "o3-pro") || PREFIXES.iter().any(|p| id.starts_with(p))
}

pub fn model_info(id: String, input_modalities: &[String]) -> ModelInfo {
    let vision = if input_modalities.is_empty() {
        is_vision_model(&id)
    } else {
        input_modalities.iter().any(|m| m.eq_ignore_ascii_case("image"))
    };
    ModelInfo { id, vision }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_known_models() {
        let yes = [
            "gpt-4o", "gpt-4o-mini", "gpt-4.1", "gpt-5", "chatgpt-4o-latest", "o4-mini", "o1", "o3",
            "claude-3-5-sonnet-20241022", "claude-sonnet-4", "claude-opus-4", "claude-haiku-4-5",
            "gemini-1.5-pro", "gemini-2.0-flash", "gemini-flash-latest", "gemini-pro-vision",
            "llava:13b", "pixtral-12b", "qwen2.5-vl-7b", "llama-3.2-11b-vision-instruct",
            "meta-llama/llama-4-scout", "openai/gpt-4o", "gemma3:12b", "grok-4",
        ];
        for id in yes {
            assert!(is_vision_model(id), "{id} should be vision");
        }
        let no = [
            "gpt-3.5-turbo", "gpt-4", "o3-mini", "o1-mini", "whisper-1", "text-embedding-3-small",
            "gpt-4o-mini-transcribe", "gpt-4o-audio-preview", "gpt-4o-realtime-preview",
            "claude-2.1", "claude-instant-1.2", "gemini-1.0-pro", "llama-3.1-70b-versatile",
            "mistral-7b-instruct", "deepseek-chat", "tts-1", "dall-e-3", "",
        ];
        for id in no {
            assert!(!is_vision_model(id), "{id} should not be vision");
        }
    }

    #[test]
    fn provider_metadata_wins_over_name() {
        let img = vec!["text".to_string(), "image".to_string()];
        let txt = vec!["text".to_string()];
        assert!(model_info("some-custom-model".into(), &img).vision);
        assert!(!model_info("gpt-4o".into(), &txt).vision);
        assert!(model_info("gpt-4o".into(), &[]).vision); // no metadata → heuristic
    }
}
