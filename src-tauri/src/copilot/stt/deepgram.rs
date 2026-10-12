//! Deepgram streaming STT backend.
//!
//! Opens a WebSocket to `wss://api.deepgram.com/v1/listen`, streams 16kHz mono
//! PCM16 audio as it arrives and emits `copilot-transcript-partial` for interim
//! results and `copilot-transcript-final` for finalized segments.

use futures_util::{SinkExt, StreamExt};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message};

use crate::copilot::stt::SttBackend;
use crate::copilot::transcript::TranscriptChunk;

const DEFAULT_MODEL: &str = "nova-3";
const DEFAULT_LANGUAGE: &str = "multi";
const SAMPLE_RATE: u32 = 16_000;
const KEEPALIVE_SECS: u64 = 5;
/// Silence (ms) before Deepgram marks `speech_final`; also the fallback
/// `UtteranceEnd` window. ~1s matches the Whisper backend's silence cut, so
/// the orchestrator sees one chunk per utterance instead of one per phrase.
const ENDPOINTING_MS: u32 = 1_000;
const UTTERANCE_END_MS: u32 = 1_500;
/// Flush a monologue that never pauses, mirroring Whisper's 20s cap.
const MAX_UTTERANCE_MS: u64 = 20_000;

pub struct DeepgramStt {
    pub api_key:  String,
    pub model:    String,
    pub language: Option<String>,
    /// Transcript source label stamped on emitted chunks.
    pub source:   String,
    task:         Option<tauri::async_runtime::JoinHandle<()>>,
}

impl DeepgramStt {
    /// `model` falls back to nova-3 unless it names a Deepgram model; `language`
    /// falls back to `multi` (code-switching) when unset.
    pub fn new(api_key: String, model: String, language: Option<String>) -> Self {
        let model = if model.starts_with("nova") || model.starts_with("enhanced") || model.starts_with("base") {
            model
        } else {
            DEFAULT_MODEL.to_string()
        };
        Self {
            api_key: api_key.trim().to_string(),
            model,
            language,
            source: "main".into(),
            task: None,
        }
    }
}

pub fn listen_url(model: &str, language: &str) -> String {
    format!(
        "wss://api.deepgram.com/v1/listen?model={model}&language={language}\
         &encoding=linear16&sample_rate={SAMPLE_RATE}&channels=1\
         &interim_results=true&smart_format=true\
         &endpointing={ENDPOINTING_MS}&utterance_end_ms={UTTERANCE_END_MS}"
    )
}

/// Convert f32 samples in [-1, 1] to little-endian PCM16 bytes.
pub fn samples_to_pcm16(samples: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(samples.len() * 2);
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// One parsed Deepgram `Results` message.
#[derive(Debug, PartialEq)]
pub struct DgResult {
    pub start_ms:     u64,
    pub end_ms:       u64,
    pub text:         String,
    pub is_final:     bool,
    pub speech_final: bool,
}

/// Parse a Deepgram `Results` message. Returns `None` for other message types.
/// `text` may be empty (e.g. a `speech_final` marker with no words).
pub fn parse_results(text: &str) -> Option<DgResult> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    if v.get("type").and_then(|t| t.as_str()) != Some("Results") {
        return None;
    }
    let transcript = v
        .pointer("/channel/alternatives/0/transcript")?
        .as_str()?
        .trim()
        .to_string();
    let start = v.get("start").and_then(|s| s.as_f64()).unwrap_or(0.0);
    let dur   = v.get("duration").and_then(|d| d.as_f64()).unwrap_or(0.0);
    let flag = |k: &str| v.get(k).and_then(|f| f.as_bool()).unwrap_or(false);
    Some(DgResult {
        start_ms: (start * 1000.0) as u64,
        end_ms: ((start + dur) * 1000.0) as u64,
        text: transcript,
        is_final: flag("is_final"),
        speech_final: flag("speech_final"),
    })
}

/// Joins Deepgram's per-phrase finals into whole utterances.
#[derive(Default)]
pub struct UtteranceBuffer {
    text:     String,
    start_ms: Option<u64>,
    end_ms:   u64,
}

impl UtteranceBuffer {
    pub fn add_final(&mut self, r: &DgResult) {
        if r.text.is_empty() {
            return;
        }
        if !self.text.is_empty() {
            self.text.push(' ');
        }
        self.text.push_str(&r.text);
        self.start_ms.get_or_insert(r.start_ms);
        self.end_ms = r.end_ms;
    }

    /// Pending finals plus an in-flight interim phrase, for live display.
    pub fn preview(&self, interim: &str) -> String {
        match (self.text.is_empty(), interim.is_empty()) {
            (true, _) => interim.to_string(),
            (false, true) => self.text.clone(),
            (false, false) => format!("{} {}", self.text, interim),
        }
    }

    pub fn start_ms(&self) -> Option<u64> {
        self.start_ms
    }

    pub fn too_long(&self) -> bool {
        self.start_ms.map_or(false, |s| self.end_ms.saturating_sub(s) >= MAX_UTTERANCE_MS)
    }

    /// Drain the buffered utterance as `(start_ms, end_ms, text)`.
    pub fn take(&mut self) -> Option<(u64, u64, String)> {
        if self.text.is_empty() {
            return None;
        }
        let out = (self.start_ms.unwrap_or(0), self.end_ms, std::mem::take(&mut self.text));
        self.start_ms = None;
        self.end_ms = 0;
        Some(out)
    }
}

impl SttBackend for DeepgramStt {
    fn start(
        &mut self,
        session_id: i64,
        mut audio_rx: mpsc::Receiver<Vec<f32>>,
        app: AppHandle,
    ) -> Result<(), String> {
        let language = self.language.clone().unwrap_or_else(|| DEFAULT_LANGUAGE.to_string());
        let url = listen_url(&self.model, &language);
        let source = self.source.clone();
        let model = self.model.clone();
        let mut request = url.into_client_request().map_err(|e| format!("deepgram url: {e}"))?;
        let auth = format!("Token {}", self.api_key)
            .parse()
            .map_err(|_| "deepgram api key contains invalid characters".to_string())?;
        request.headers_mut().insert("Authorization", auth);

        let task = tauri::async_runtime::spawn(async move {
            let (ws, _) = match tokio_tungstenite::connect_async(request).await {
                Ok(ok) => ok,
                Err(e) => {
                    eprintln!("[deepgram] connect error: {e}");
                    if e.to_string().contains("401") {
                        eprintln!("[deepgram] 401 = API key ditolak: pastikan key Deepgram valid (field 'Deepgram API Key') dan sudah disimpan");
                    }
                    return;
                }
            };
            eprintln!("[deepgram] connected (url language={language})");
            let (mut tx, mut rx) = ws.split();
            // Diagnostics: loudest sample + result counts, logged every 5s.
            let (mut peak, mut sent, mut results, mut empty) = (0.0_f32, 0u32, 0u32, 0u32);
            let mut utterance = UtteranceBuffer::default();
            let mut keepalive = tokio::time::interval(std::time::Duration::from_secs(KEEPALIVE_SECS));

            loop {
                tokio::select! {
                    chunk = audio_rx.recv() => {
                        let Some(chunk) = chunk else { break };
                        peak = chunk.iter().fold(peak, |m, s| m.max(s.abs()));
                        sent += 1;
                        if let Err(e) = tx.send(Message::Binary(samples_to_pcm16(&chunk))).await {
                            eprintln!("[deepgram] send error: {e}");
                            break;
                        }
                    }
                    _ = keepalive.tick() => {
                        eprintln!(
                            "[deepgram:{source}] 5s lang={language} model={model}: audio chunks={sent} peak={peak:.3} results={results} empty={empty}"
                        );
                        (peak, sent, results, empty) = (0.0, 0, 0, 0);
                        let _ = tx.send(Message::Text(r#"{"type":"KeepAlive"}"#.into())).await;
                    }
                    msg = rx.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                let emit = |event: &str, start_ms, end_ms, text: String, is_final| {
                                    let _ = app.emit(event, TranscriptChunk {
                                        session_id, start_ms, end_ms, text, is_final,
                                        source: source.clone(),
                                    });
                                };
                                if let Some(r) = parse_results(&text) {
                                    results += 1;
                                    if r.is_final {
                                        utterance.add_final(&r);
                                        if r.speech_final || utterance.too_long() {
                                            if let Some((s, e, t)) = utterance.take() {
                                                emit("copilot-transcript-final", s, e, t, true);
                                            }
                                        }
                                    } else if !r.text.is_empty() {
                                        let start = utterance.start_ms().unwrap_or(r.start_ms);
                                        emit("copilot-transcript-partial", start, r.end_ms,
                                             utterance.preview(&r.text), false);
                                    }
                                    if r.text.is_empty() { empty += 1; }
                                } else if text.contains(r#""type":"UtteranceEnd""#) {
                                    if let Some((s, e, t)) = utterance.take() {
                                        emit("copilot-transcript-final", s, e, t, true);
                                    }
                                } else if !text.contains(r#""type":"Metadata""#) {
                                    eprintln!("[deepgram] msg: {text}");
                                }
                            }
                            Some(Ok(Message::Close(_))) | None => break,
                            Some(Err(e)) => {
                                eprintln!("[deepgram] ws error: {e}");
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
            let _ = tx.send(Message::Text(r#"{"type":"CloseStream"}"#.into())).await;
            eprintln!("[deepgram] stream closed");
        });

        self.task = Some(task);
        Ok(())
    }

    fn stop(self: Box<Self>) {
        if let Some(h) = self.task {
            h.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_carries_model_language_and_audio_params() {
        let u = listen_url("nova-3", "multi");
        assert!(u.contains("model=nova-3"));
        assert!(u.contains("language=multi"));
        assert!(u.contains("encoding=linear16"));
        assert!(u.contains("sample_rate=16000"));
    }

    #[test]
    fn pcm16_is_little_endian_and_clamped() {
        let b = samples_to_pcm16(&[0.0, 1.0, -2.0]);
        assert_eq!(b.len(), 6);
        assert_eq!(i16::from_le_bytes([b[0], b[1]]), 0);
        assert_eq!(i16::from_le_bytes([b[2], b[3]]), i16::MAX);
        assert_eq!(i16::from_le_bytes([b[4], b[5]]), -i16::MAX);
    }

    #[test]
    fn parses_result_with_timestamps_and_flags() {
        let j = r#"{"type":"Results","start":1.5,"duration":2.0,"is_final":true,"speech_final":true,
                    "channel":{"alternatives":[{"transcript":" halo dunia "}]}}"#;
        assert_eq!(
            parse_results(j),
            Some(DgResult { start_ms: 1500, end_ms: 3500, text: "halo dunia".into(), is_final: true, speech_final: true })
        );
    }

    #[test]
    fn non_result_messages_are_ignored_and_empty_text_is_kept() {
        assert_eq!(parse_results(r#"{"type":"Metadata"}"#), None);
        let empty = r#"{"type":"Results","speech_final":true,"is_final":true,
                        "channel":{"alternatives":[{"transcript":""}]}}"#;
        assert!(parse_results(empty).unwrap().text.is_empty());
    }

    fn fin(start: u64, end: u64, text: &str) -> DgResult {
        DgResult { start_ms: start, end_ms: end, text: text.into(), is_final: true, speech_final: false }
    }

    #[test]
    fn utterance_buffer_joins_phrases_into_one_chunk() {
        let mut b = UtteranceBuffer::default();
        b.add_final(&fin(1000, 2000, "jelaskan perbedaan"));
        b.add_final(&fin(2200, 3000, "REST dan GraphQL"));
        assert_eq!(b.preview("apa"), "jelaskan perbedaan REST dan GraphQL apa");
        assert_eq!(
            b.take(),
            Some((1000, 3000, "jelaskan perbedaan REST dan GraphQL".to_string()))
        );
        assert_eq!(b.take(), None);
    }

    #[test]
    fn utterance_buffer_flags_overlong_monologue() {
        let mut b = UtteranceBuffer::default();
        b.add_final(&fin(0, 10_000, "a"));
        assert!(!b.too_long());
        b.add_final(&fin(10_000, 20_000, "b"));
        assert!(b.too_long());
    }

    #[test]
    fn api_key_is_trimmed() {
        assert_eq!(DeepgramStt::new("  abc\n".into(), "nova-3".into(), None).api_key, "abc");
    }

    #[test]
    fn unknown_model_falls_back_to_nova3() {
        assert_eq!(DeepgramStt::new("k".into(), "whisper-1".into(), None).model, "nova-3");
        assert_eq!(DeepgramStt::new("k".into(), "nova-2".into(), None).model, "nova-2");
    }
}
