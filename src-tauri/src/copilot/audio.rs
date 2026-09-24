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

/// macOS loopback via the Core Audio Process Tap API (macOS 14.2+ / Sonoma).
/// Creates a private, per-process aggregate device combining the system's
/// default output device with a system-wide tap, then reads audio from it
/// via an IOProc — no Screen Recording permission and no virtual audio
/// driver (e.g. BlackHole) required.
#[cfg(target_os = "macos")]
pub fn start_loopback() -> Result<AudioCapture, String> {
    use cidre::{cat, cf, core_audio as ca, ns, os};

    struct TapCtx {
        tx: mpsc::Sender<Vec<f32>>,
        buf16k: Vec<f32>,
        channels: usize,
        step: f32,
    }

    extern "C" fn tap_io_proc(
        _device: ca::Device,
        _now: &cat::AudioTimeStamp,
        input_data: &cat::AudioBufList<1>,
        _input_time: &cat::AudioTimeStamp,
        _output_data: &mut cat::AudioBufList<1>,
        _output_time: &cat::AudioTimeStamp,
        ctx: Option<&mut TapCtx>,
    ) -> os::Status {
        let Some(ctx) = ctx else { return Default::default() };
        if ctx.channels == 0 {
            return Default::default();
        }
        let buf = &input_data.buffers[0];
        if buf.data.is_null() {
            return Default::default();
        }
        let frame_count = (buf.data_bytes_size as usize) / 4 / ctx.channels;
        // SAFETY: `data` points to `frame_count * channels` valid f32 samples
        // for the duration of this callback (owned by CoreAudio's IO cycle).
        let data = unsafe {
            std::slice::from_raw_parts(buf.data as *const f32, frame_count * ctx.channels)
        };

        let mut i = 0.0_f32;
        while (i as usize) < frame_count {
            let idx = (i as usize) * ctx.channels;
            let mut sum = 0.0_f32;
            for c in 0..ctx.channels {
                sum += data[idx + c];
            }
            let mono = sum / ctx.channels as f32;
            ctx.buf16k.push(mono);
            if ctx.buf16k.len() >= CHUNK_SAMPLES {
                let chunk = std::mem::replace(&mut ctx.buf16k, Vec::with_capacity(CHUNK_SAMPLES));
                let _ = ctx.tx.try_send(chunk);
            }
            i += ctx.step;
        }
        Default::default()
    }

    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_for_thread = stop_flag.clone();
    let (tx, rx) = mpsc::channel::<Vec<f32>>(64);
    let (setup_tx, setup_rx) = std::sync::mpsc::channel::<Result<(), String>>();

    // All Core Audio / Objective-C objects below are created, used, and torn
    // down on this single thread — none of it is shared across threads, so
    // we sidestep any question of whether cidre's types are `Send`.
    std::thread::spawn(move || {
        let setup = (|| -> Result<_, String> {
            let output_device = ca::System::default_output_device()
                .map_err(|e| format!("default_output_device: {e:?}"))?;
            let output_uid = output_device
                .uid()
                .map_err(|e| format!("output device uid: {e:?}"))?;

            let sub_device = cf::DictionaryOf::with_keys_values(
                &[ca::sub_device_keys::uid()],
                &[output_uid.as_type_ref()],
            );

            // NOTE: not `TapDesc::with_stereo_global_tap_excluding_processes` —
            // that convenience wrapper unconditionally unwraps `alloc()`, which
            // is `Option` because `TapDesc` is a weakly-linked, version-gated
            // class (macOS 14.2+ Process Tap API); on an older OS it would
            // panic instead of returning our error below.
            let tap_desc = ca::TapDesc::alloc()
                .ok_or_else(|| "TapDesc class unavailable (butuh macOS 14.2+ / Sonoma untuk Process Tap API)".to_string())?
                .init_stereo_global_tap_but_exclude_processes(&ns::Array::new());
            let tap = tap_desc.create_process_tap().map_err(|e| {
                format!("create_process_tap: {e:?} (butuh macOS 14.2+ / Sonoma untuk Process Tap API)")
            })?;
            let tap_uid = tap.uid().map_err(|e| format!("tap uid: {e:?}"))?;

            let sub_tap = cf::DictionaryOf::with_keys_values(
                &[ca::hardware::sub_tap_keys::uid()],
                &[tap_uid.as_type_ref()],
            );

            let desc = cf::DictionaryOf::with_keys_values(
                &[
                    ca::aggregate_device_keys::is_private(),
                    ca::aggregate_device_keys::is_stacked(),
                    ca::aggregate_device_keys::tap_auto_start(),
                    ca::aggregate_device_keys::name(),
                    ca::aggregate_device_keys::main_sub_device(),
                    ca::aggregate_device_keys::uid(),
                    ca::aggregate_device_keys::sub_device_list(),
                    ca::aggregate_device_keys::tap_list(),
                ],
                &[
                    cf::Boolean::value_true().as_type_ref(),
                    cf::Boolean::value_false(),
                    cf::Boolean::value_true(),
                    cf::str!(c"noscreen-copilot-tap"),
                    &output_uid,
                    &cf::Uuid::new().to_cf_string(),
                    &cf::ArrayOf::from_slice(&[sub_device.as_ref()]),
                    &cf::ArrayOf::from_slice(&[sub_tap.as_ref()]),
                ],
            );
            let agg_device = ca::AggregateDevice::with_desc(&desc)
                .map_err(|e| format!("create aggregate device: {e:?}"))?;

            let asbd = tap.asbd().map_err(|e| format!("tap asbd: {e:?}"))?;
            let input_rate = asbd.sample_rate as u32;
            let channels = asbd.channels_per_frame as usize;
            if channels == 0 || input_rate == 0 {
                return Err(format!("unexpected tap format: {asbd:?}"));
            }
            eprintln!("[audio] mac process-tap start: sr={input_rate} ch={channels}");

            Ok((tap, agg_device, input_rate, channels))
        })();

        let (tap, agg_device, input_rate, channels) = match setup {
            Ok(v) => v,
            Err(e) => {
                let _ = setup_tx.send(Err(e));
                return;
            }
        };

        let mut ctx = Box::new(TapCtx {
            tx,
            buf16k: Vec::with_capacity(CHUNK_SAMPLES),
            channels,
            step: input_rate as f32 / TARGET_SAMPLE_RATE as f32,
        });

        let proc_id = match agg_device.create_io_proc_id(tap_io_proc, Some(ctx.as_mut())) {
            Ok(id) => id,
            Err(e) => {
                let _ = setup_tx.send(Err(format!("create_io_proc_id: {e:?}")));
                return;
            }
        };

        let started = match ca::device_start(agg_device, Some(proc_id)) {
            Ok(s) => s,
            Err(e) => {
                let _ = setup_tx.send(Err(format!("device_start: {e:?}")));
                return;
            }
        };

        let _ = setup_tx.send(Ok(()));

        while !stop_for_thread.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        eprintln!("[audio] mac process-tap stop");

        // Ordered teardown: dropping `started` stops IO and destroys the
        // aggregate device (its Drop impl does both, in that order); the
        // tap and IOProc context are then safe to release.
        drop(started);
        drop(tap);
        drop(ctx);
    });

    match setup_rx.recv() {
        Ok(Ok(())) => Ok(AudioCapture { stop_flag, rx: Some(rx) }),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("audio setup thread panicked before reporting a result".into()),
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn start_loopback() -> Result<AudioCapture, String> {
    Err("Loopback capture is only supported on Windows and macOS 14.2+ in v1".into())
}

// ── stub for tests on platforms without a loopback implementation ─────────────
#[cfg(all(not(target_os = "windows"), not(target_os = "macos"), test))]
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
