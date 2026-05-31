//! Manual smoke test: run for 5 seconds, print RMS of each chunk.
//! Usage: cargo run --example test_audio_capture
//! Expected: nonzero RMS while audio is playing on the system.

use tauri_app_lib::copilot::audio;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut cap = audio::start_loopback().expect("start_loopback failed");
    println!("Capturing for 5 seconds — play any audio on your system...");

    let start = std::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_secs(5) {
        match tokio::time::timeout(std::time::Duration::from_millis(200), cap.rx.recv()).await {
            Ok(Some(chunk)) => {
                let rms = (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32).sqrt();
                println!("chunk len={} rms={:.4}", chunk.len(), rms);
            }
            Ok(None) => break,
            Err(_) => println!("(timeout — no chunk in 200ms)"),
        }
    }
    cap.stop();
    println!("Done.");
}
