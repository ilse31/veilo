//! Windows Speech Recognition STT backend.
//! Uses native Windows WinRT SpeechRecognition API.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use crate::copilot::stt::SttBackend;
use crate::copilot::transcript::TranscriptChunk;

pub struct WindowsSrStt {
    stop_flag: Arc<AtomicBool>,
    join_handle: Arc<std::sync::Mutex<Option<std::thread::JoinHandle<()>>>>,
}

impl WindowsSrStt {
    pub fn new() -> Self {
        Self {
            stop_flag: Arc::new(AtomicBool::new(false)),
            join_handle: Arc::new(std::sync::Mutex::new(None)),
        }
    }
}

impl SttBackend for WindowsSrStt {
    fn start(
        &mut self,
        session_id: i64,
        _audio_rx: mpsc::Receiver<Vec<f32>>, // Ignored, Windows SR captures from default input device directly
        app: AppHandle,
    ) -> Result<(), String> {
        let stop_flag = self.stop_flag.clone();
        stop_flag.store(false, Ordering::Relaxed);

        let handle = std::thread::spawn(move || {
            if let Err(e) = run_win_sr(&app, &stop_flag, session_id) {
                eprintln!("[windows-sr] error: {e}");
            }
        });
        if let Ok(mut lock) = self.join_handle.lock() {
            *lock = Some(handle);
        }
        Ok(())
    }

    fn stop(self: Box<Self>) {
        self.stop_flag.store(true, Ordering::Relaxed);
        // We don't block on join_handle since stop() is called synchronously and shouldn't block the caller.
    }
}

#[cfg(target_os = "windows")]
fn run_win_sr(app: &AppHandle, stop: &AtomicBool, session_id: i64) -> Result<(), String> {
    use windows::Foundation::TypedEventHandler;
    use windows::Media::SpeechRecognition::*;
    use windows::Win32::System::Com::*;

    // MTA Initialisation
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    let recognizer = SpeechRecognizer::new()
        .map_err(|e| format!("SpeechRecognizer::new: {e}"))?;
    eprintln!("[windows-sr] SpeechRecognizer initialized");

    let constraint = SpeechRecognitionTopicConstraint::Create(
        SpeechRecognitionScenario::Dictation,
        &windows::core::HSTRING::from("dictation"),
    )
    .map_err(|e| format!("Create constraint: {e}"))?;

    recognizer
        .Constraints()
        .and_then(|c| c.Append(&constraint))
        .map_err(|e| format!("Append constraint: {e}"))?;

    recognizer
        .CompileConstraintsAsync()
        .map_err(|e| format!("CompileConstraintsAsync: {e}"))?
        .get()
        .map_err(|e| format!("CompileConstraints: {e}"))?;
    eprintln!("[windows-sr] constraints compiled");

    let session = recognizer
        .ContinuousRecognitionSession()
        .map_err(|e| format!("ContinuousRecognitionSession: {e}"))?;
    eprintln!("[windows-sr] continuous session created");

    let app_clone = app.clone();
    let start_time = Instant::now();

    session
        .ResultGenerated(&TypedEventHandler::new(
            move |_,
                  args: &Option<
                SpeechContinuousRecognitionResultGeneratedEventArgs,
            >| {
                if let Some(args) = args {
                    if let Ok(result) = args.Result() {
                        if let Ok(text) = result.Text() {
                            let t = text.to_string();
                            let trimmed = t.trim();
                            eprintln!("[windows-sr] ResultGenerated triggered. text={:?}", trimmed);
                            if !trimmed.is_empty() {
                                let end_ms = start_time.elapsed().as_millis() as u64;
                                let start_ms = end_ms.saturating_sub(1500);

                                let _ = app_clone.emit(
                                    "copilot-transcript-final",
                                    TranscriptChunk {
                                        session_id,
                                        start_ms,
                                        end_ms,
                                        text: trimmed.to_string(),
                                        is_final: true,
                                    },
                                );
                            }
                        }
                    }
                }
                Ok(())
            },
        ))
        .map_err(|e| format!("ResultGenerated: {e}"))?;

    // Detect when the continuous session stops so we can restart it.
    // Without this handler the session dies silently and recognition stops
    // while the UI still shows "listening".
    let (completed_tx, completed_rx) = std::sync::mpsc::channel::<i32>();
    let completed_tx_clone = completed_tx.clone();
    session
        .Completed(&TypedEventHandler::new(
            move |_, args: &Option<SpeechContinuousRecognitionCompletedEventArgs>| {
                let status = args
                    .as_ref()
                    .and_then(|a| a.Status().ok())
                    .map(|s| s.0)
                    .unwrap_or(-1);
                eprintln!("[windows-sr] session completed: status={status}");
                // status values: 0=Success, 5=Paused, 6=TimeLimit, 7=PauseLimit
                let _ = completed_tx_clone.send(status);
                Ok(())
            },
        ))
        .map_err(|e| format!("Completed: {e}"))?;
    // Keep completed_tx alive so the channel stays open for the duration of the loop.
    let _completed_tx = completed_tx;

    let start_session = |s: &SpeechContinuousRecognitionSession| -> Result<(), String> {
        s.StartAsync()
            .map_err(|e| format!("StartAsync: {e}"))?
            .get()
            .map_err(|e| {
                if e.code().0 as u32 == 0x80045509 {
                    "Windows Online Speech Recognition is disabled. Go to Settings → Privacy & Security → Speech and turn ON \"Online speech recognition\".".to_string()
                } else {
                    format!("Start session: {e}")
                }
            })
    };

    start_session(&session)?;
    eprintln!("[windows-sr] speech recognizer started successfully!");

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        match completed_rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(status) => {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                // Resume() is only valid after explicit PauseAsync() — it fails
                // (InvalidOperationException) when called after Completed fires.
                // For all statuses, do StopAsync + StartAsync.
                // status=7 (PauseLimit/silence) gets a longer delay to avoid
                // tight restart loops when no one is speaking.
                let delay_ms = if status == 7 { 1500 } else { 300 };
                eprintln!("[windows-sr] session ended (status={status}), restarting in {delay_ms}ms");
                let _ = session.StopAsync().and_then(|a| a.get());
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                if let Err(e) = start_session(&session) {
                    eprintln!("[windows-sr] restart failed: {e}");
                    break;
                }
                eprintln!("[windows-sr] restarted");
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = session.StopAsync().and_then(|a| a.get());
    let _ = recognizer.Close();

    unsafe { CoUninitialize() };
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn run_win_sr(_app: &AppHandle, _stop: &AtomicBool, _session_id: i64) -> Result<(), String> {
    Err("Windows Speech Recognition is only available on Windows".into())
}
