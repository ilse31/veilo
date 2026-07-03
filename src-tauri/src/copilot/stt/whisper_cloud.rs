//! OpenAI Whisper Cloud STT backend.
//!
//! Buffers audio until silence is detected, then encodes to WAV and POSTs
//! to `{api_url}/v1/audio/transcriptions`. Emits one final chunk per utterance.

use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, Mutex as AsyncMutex};

use crate::copilot::stt::SttBackend;
use crate::copilot::transcript::TranscriptChunk;

const SILENCE_RMS_THRESHOLD: f32 = 0.005;
const SILENCE_DURATION_MS: u64 = 1_000;
// KNOWN LIMITATION: long monologues are sliced at 20s boundaries with no overlap.
// Whisper may mis-transcribe words straddling the boundary. v2 should add a small
// overlap buffer (~300ms) when flushing on MAX rather than silence.
const MAX_UTTERANCE_MS: u64 = 20_000;
const MIN_UTTERANCE_MS: u64 = 1_000;
const CHUNK_MS: u64 = 100;
const SAMPLE_RATE: u32 = 16_000;

/// Returns true if the chunk's RMS is below the silence threshold.
pub fn is_silent_chunk(samples: &[f32]) -> bool {
    if samples.is_empty() {
        return true;
    }
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    rms < SILENCE_RMS_THRESHOLD
}

/// Encode 16kHz mono f32 samples to in-memory WAV bytes.
pub fn samples_to_wav(samples: &[f32]) -> Vec<u8> {
    let spec = hound::WavSpec {
        channels:        1,
        sample_rate:     SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format:   hound::SampleFormat::Int,
    };
    let mut buf = std::io::Cursor::new(Vec::<u8>::new());
    {
        let mut writer = hound::WavWriter::new(&mut buf, spec).expect("wav writer");
        for s in samples {
            let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer.write_sample(v).unwrap();
        }
        writer.finalize().unwrap();
    }
    buf.into_inner()
}

pub struct WhisperCloudStt {
    pub api_url:   String,
    pub api_key:   String,
    pub model:     String,
    pub language:  Option<String>,
    join_handle:   Arc<AsyncMutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl WhisperCloudStt {
    pub fn new(api_url: String, api_key: String, model: String) -> Self {
        Self {
            api_url, api_key, model, language: None,
            join_handle: Arc::new(AsyncMutex::new(None)),
        }
    }
}

impl SttBackend for WhisperCloudStt {
    fn start(
        &mut self,
        session_id: i64,
        mut audio_rx: mpsc::Receiver<Vec<f32>>,
        app: AppHandle,
    ) -> Result<(), String> {
        let api_url = self.api_url.trim_end_matches('/').to_string();
        let api_key = self.api_key.clone();
        let model   = self.model.clone();
        let language = self.language.clone();
        let handle_slot = self.join_handle.clone();

        let task = tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("reqwest client");

            let mut utterance: Vec<f32> = Vec::new();
            let mut silent_ms: u64 = 0;
            let mut utt_start_ms: u64 = 0;
            let mut clock_ms: u64 = 0;

            while let Some(chunk) = audio_rx.recv().await {
                clock_ms += CHUNK_MS;
                let silent = is_silent_chunk(&chunk);

                if utterance.is_empty() && !silent {
                    utt_start_ms = clock_ms - CHUNK_MS;
                }
                utterance.extend_from_slice(&chunk);
                if silent { silent_ms += CHUNK_MS; } else { silent_ms = 0; }

                let utt_len_ms = clock_ms - utt_start_ms;
                let should_flush = !utterance.is_empty() &&
                    ((silent_ms >= SILENCE_DURATION_MS && utt_len_ms >= MIN_UTTERANCE_MS)
                     || utt_len_ms >= MAX_UTTERANCE_MS);

                if should_flush {
                    let wav = samples_to_wav(&utterance);
                    let start = utt_start_ms;
                    let end   = clock_ms;
                    utterance.clear();
                    silent_ms = 0;
                    utt_start_ms = 0;

                    let mut form = reqwest::multipart::Form::new()
                        .text("model", model.clone())
                        .part("file",
                            reqwest::multipart::Part::bytes(wav)
                                .file_name("audio.wav")
                                .mime_str("audio/wav").unwrap()
                        );
                    if let Some(ref lang) = language {
                        form = form.text("language", lang.clone());
                    }
                    let req = client.post(format!("{api_url}/v1/audio/transcriptions"))
                        .bearer_auth(&api_key)
                        .multipart(form);

                    match req.send().await {
                        Ok(resp) if resp.status().is_success() => {
                            if let Ok(json) = resp.json::<serde_json::Value>().await {
                                if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                                    let trimmed = text.trim();
                                    if !trimmed.is_empty() {
                                        let _ = app.emit(
                                            "copilot-transcript-final",
                                            TranscriptChunk {
                                                session_id,
                                                start_ms: start,
                                                end_ms:   end,
                                                text:     trimmed.into(),
                                                is_final: true,
                                            },
                                        );
                                    }
                                }
                            }
                        }
                        Ok(resp) => {
                            eprintln!("[whisper] HTTP {}", resp.status());
                        }
                        Err(e) => eprintln!("[whisper] request error: {e}"),
                    }
                }
            }
            eprintln!("[whisper] audio_rx closed; stopping");
        });

        // TODO(v1.1): the second spawn here is racy — if stop() is called before this
        // completes, the worker task is not aborted. Acceptable for v1 because audio_rx
        // is owned by the worker and the channel closes on session teardown.
        // Stash the handle so stop() can abort
        tokio::spawn(async move {
            *handle_slot.lock().await = Some(task);
        });

        Ok(())
    }

    fn stop(self: Box<Self>) {
        let handle = self.join_handle.clone();
        tauri::async_runtime::spawn(async move {
            if let Some(h) = handle.lock().await.take() { h.abort(); }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silent_buffer_is_silent() {
        let samples = vec![0.0_f32; 1600];
        assert!(is_silent_chunk(&samples));
    }

    #[test]
    fn loud_buffer_is_not_silent() {
        let samples = vec![0.5_f32; 1600];
        assert!(!is_silent_chunk(&samples));
    }

    #[test]
    fn empty_buffer_is_silent() {
        assert!(is_silent_chunk(&[]));
    }

    #[test]
    fn wav_encoding_produces_riff_header() {
        let samples = vec![0.1_f32; 100];
        let wav = samples_to_wav(&samples);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
    }
}
