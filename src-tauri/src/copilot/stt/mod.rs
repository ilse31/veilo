pub mod whisper_cloud;

use tauri::AppHandle;
use tokio::sync::mpsc;

use crate::copilot::transcript::TranscriptChunk;

/// Backend-agnostic STT interface. Implementations consume f32 audio chunks
/// (16kHz mono) and emit `TranscriptChunk`s via Tauri events
/// (`copilot-transcript-final` + optional `copilot-transcript-partial`).
pub trait SttBackend: Send {
    /// Start the backend. Spawns its own task; returns immediately.
    /// `session_id` is stamped onto every emitted chunk.
    fn start(
        &mut self,
        session_id: i64,
        audio_rx: mpsc::Receiver<Vec<f32>>,
        app: AppHandle,
    ) -> Result<(), String>;

    /// Stop the backend and free resources.
    fn stop(self: Box<Self>);
}

/// Event payload emitted as `copilot-transcript-final` / `copilot-transcript-partial`.
pub type TranscriptEvent = TranscriptChunk;
