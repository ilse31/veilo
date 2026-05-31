//! WASAPI loopback audio capture → 16kHz mono f32 chunks (~100ms).
//!
//! On Windows, uses cpal's default *output* device in loopback mode so we
//! capture exactly what the user hears — no Stereo Mix setup required.
//!
//! TODO(v2): switch eprintln → tracing; smarter resampler (phase-tracked
//! across callbacks); avoid unnecessary tx.clone in single-consumer path.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::mpsc;

const TARGET_SAMPLE_RATE: u32 = 16_000;
const CHUNK_SAMPLES: usize = 1_600; // 100ms at 16kHz mono

pub struct AudioCapture {
    pub stop_flag: Arc<AtomicBool>,
    rx: Option<mpsc::Receiver<Vec<f32>>>,
}

impl AudioCapture {
    /// Take the audio receiver once. Returns `None` if already taken.
    pub fn take_rx(&mut self) -> Option<mpsc::Receiver<Vec<f32>>> {
        self.rx.take()
    }

    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}

#[cfg(target_os = "windows")]
pub fn start_loopback() -> Result<AudioCapture, String> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    // Discover & validate device BEFORE spawning thread so we can return errors synchronously.
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "no default output device".to_string())?;
    let device_name = device.name().unwrap_or_else(|_| "unknown".into());
    let supported_config = device
        .default_output_config()
        .map_err(|e| format!("default_output_config: {e}"))?;

    if supported_config.sample_format() != cpal::SampleFormat::F32 {
        return Err(format!(
            "unsupported sample format: {:?} (only F32 supported in v1)",
            supported_config.sample_format()
        ));
    }
    let input_rate = supported_config.sample_rate().0;
    let channels = supported_config.channels() as usize;
    let stream_config = supported_config.config();

    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_for_thread = stop_flag.clone();
    let (tx, rx) = mpsc::channel::<Vec<f32>>(64);

    std::thread::spawn(move || {
        eprintln!("[audio] loopback start: device={device_name:?} sr={input_rate} ch={channels}");

        let mut buf16k: Vec<f32> = Vec::with_capacity(CHUNK_SAMPLES);
        let step = input_rate as f32 / TARGET_SAMPLE_RATE as f32;
        let tx_inner = tx;
        let err_fn = |e| eprintln!("[audio] stream error: {e}");

        let stream = device.build_input_stream(
            &stream_config,
            move |data: &[f32], _| {
                // Downmix to mono + naive linear resample to 16kHz
                let mut i = 0.0_f32;
                while (i as usize) * channels < data.len() {
                    let idx = (i as usize) * channels;
                    let mut sum = 0.0_f32;
                    for c in 0..channels {
                        sum += data[idx + c];
                    }
                    let mono = sum / channels as f32;
                    buf16k.push(mono);
                    if buf16k.len() >= CHUNK_SAMPLES {
                        let chunk = std::mem::replace(&mut buf16k, Vec::with_capacity(CHUNK_SAMPLES));
                        // Back-pressure: silently drop chunks if consumer is > 6.4s behind (buffer cap = 64).
                        let _ = tx_inner.try_send(chunk);
                    }
                    i += step;
                }
            },
            err_fn,
            None,
        );

        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[audio] build_input_stream: {e}");
                stop_for_thread.store(true, Ordering::Relaxed);
                return;
            }
        };
        if let Err(e) = stream.play() {
            eprintln!("[audio] stream.play: {e}");
            stop_for_thread.store(true, Ordering::Relaxed);
            return;
        }

        while !stop_for_thread.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        eprintln!("[audio] loopback stop");
        // stream dropped here, releasing WASAPI resources
    });

    Ok(AudioCapture { stop_flag, rx: Some(rx) })
}

#[cfg(not(target_os = "windows"))]
pub fn start_loopback() -> Result<AudioCapture, String> {
    Err("Loopback capture is Windows-only in v1".into())
}

// ── non-Windows stub for tests ────────────────────────────────────────────────
#[cfg(all(not(target_os = "windows"), test))]
fn _make_stub_capture() -> AudioCapture {
    let stop_flag = Arc::new(AtomicBool::new(false));
    let (_tx, rx) = tokio::sync::mpsc::channel::<Vec<f32>>(1);
    AudioCapture { stop_flag, rx: Some(rx) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_compiles() {
        // No hardware test; just ensure the type system holds.
        let _: fn() -> Result<AudioCapture, String> = start_loopback;
    }
}
