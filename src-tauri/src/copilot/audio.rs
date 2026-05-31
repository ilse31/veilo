//! WASAPI loopback audio capture → 16kHz mono f32 chunks (~100ms).
//!
//! On Windows, uses cpal's default *output* device in loopback mode so we
//! capture exactly what the user hears — no Stereo Mix setup required.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::mpsc;

const TARGET_SAMPLE_RATE: u32 = 16_000;
const CHUNK_SAMPLES: usize = 1_600; // 100ms at 16kHz mono

pub struct AudioCapture {
    pub stop_flag: Arc<AtomicBool>,
    pub rx: mpsc::Receiver<Vec<f32>>,
}

impl AudioCapture {
    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}

#[cfg(target_os = "windows")]
pub fn start_loopback() -> Result<AudioCapture, String> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_for_thread = stop_flag.clone();
    let (tx, rx) = mpsc::channel::<Vec<f32>>(64);

    std::thread::spawn(move || {
        let host = cpal::default_host();
        let device = match host.default_output_device() {
            Some(d) => d,
            None => {
                eprintln!("[audio] no default output device");
                return;
            }
        };
        let config = match device.default_output_config() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[audio] default_output_config: {e}");
                return;
            }
        };
        let input_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        eprintln!("[audio] loopback start: device={:?} sr={input_rate} ch={channels}",
                  device.name().ok());

        let mut buf16k: Vec<f32> = Vec::with_capacity(CHUNK_SAMPLES);
        let step = input_rate as f32 / TARGET_SAMPLE_RATE as f32;

        let stream_config = config.config();
        let stop_inner = stop_for_thread.clone();
        let tx_inner = tx.clone();

        let err_fn = |e| eprintln!("[audio] stream error: {e}");

        // Build a loopback input stream on the OUTPUT device (cpal supports this on Windows/WASAPI).
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
                return;
            }
        };
        if let Err(e) = stream.play() {
            eprintln!("[audio] stream.play: {e}");
            return;
        }

        while !stop_inner.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        eprintln!("[audio] loopback stop");
    });

    Ok(AudioCapture { stop_flag, rx })
}

#[cfg(not(target_os = "windows"))]
pub fn start_loopback() -> Result<AudioCapture, String> {
    Err("Loopback capture is Windows-only in v1".into())
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
