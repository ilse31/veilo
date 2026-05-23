use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::{AppHandle, Emitter};

pub struct SttHandle {
    stop_flag: Arc<AtomicBool>,
}

impl SttHandle {
    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}

/// Start Windows Speech Recognition on a background thread.
/// Uses whatever is set as the default recording device in Windows
/// (e.g. Stereo Mix), NOT the WebView2 audio device.
/// Results are emitted as "stt-result" Tauri events.
pub fn start_stt(app: AppHandle) -> Result<SttHandle, String> {
    let stop_flag = Arc::new(AtomicBool::new(false));
    let flag_clone = stop_flag.clone();

    std::thread::spawn(move || {
        if let Err(e) = run_stt(&app, &flag_clone) {
            eprintln!("[stt] {e}");
            let _ = app.emit("stt-error", e);
        }
        let _ = app.emit("stt-stopped", ());
    });

    Ok(SttHandle { stop_flag })
}

// ── Windows implementation ────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn run_stt(app: &AppHandle, stop: &AtomicBool) -> Result<(), String> {
    use windows::Foundation::TypedEventHandler;
    use windows::Media::SpeechRecognition::*;
    use windows::Win32::System::Com::*;

    // WinRT requires COM/MTA initialisation on the calling thread
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    // SpeechRecognizer() uses the Windows default recording device —
    // Stereo Mix if the user set it as default in Sound Settings.
    let recognizer = SpeechRecognizer::new()
        .map_err(|e| format!("SpeechRecognizer::new: {e}"))?;

    // Dictation grammar — recognises free-form speech
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

    let session = recognizer
        .ContinuousRecognitionSession()
        .map_err(|e| format!("ContinuousRecognitionSession: {e}"))?;

    let app_clone = app.clone();
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
                            if !t.trim().is_empty() {
                                let _ = app_clone
                                    .emit("stt-result", t.trim().to_string());
                            }
                        }
                    }
                }
                Ok(())
            },
        ))
        .map_err(|e| format!("ResultGenerated: {e}"))?;

    session
        .StartAsync()
        .map_err(|e| format!("StartAsync: {e}"))?
        .get()
        .map_err(|e| {
            // 0x80045509 = SPERR_SPEECH_PRIVACY_POLICY_NOT_ACCEPTED
            if e.code().0 as u32 == 0x80045509 {
                "Windows Online Speech Recognition is disabled. \
                 Go to Settings → Privacy & Security → Speech \
                 and turn ON \"Online speech recognition\".".to_string()
            } else {
                format!("Start session: {e}")
            }
        })?;

    // Keep thread alive until stop is requested
    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    let _ = session.StopAsync().and_then(|a| a.get());
    let _ = recognizer.Close();

    unsafe { CoUninitialize() };

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn run_stt(_app: &AppHandle, _stop: &AtomicBool) -> Result<(), String> {
    Err("Windows Speech Recognition is only available on Windows".into())
}
