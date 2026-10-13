//! One-shot screen capture for the "read my screen" shortcut.

use base64::Engine;
use image::{codecs::jpeg::JpegEncoder, imageops::FilterType, DynamicImage, RgbaImage};

pub const MAX_SIDE: u32 = 1600;
pub const JPEG_QUALITY: u8 = 80;
const THUMB_SIDE: u32 = 320;
const THUMB_QUALITY: u8 = 60;

pub enum MonitorChoice {
    /// Monitor containing this physical point; falls back to the primary monitor.
    Cursor(Option<(i32, i32)>),
    /// Index into the left-to-right sorted monitor list.
    Index(usize),
}

pub struct Shot {
    pub data_uri: String,
    pub thumbnail: String,
    pub monitor_name: String,
}

pub fn next_selection(current: Option<usize>, count: usize) -> usize {
    match current {
        Some(i) if i + 1 < count => i + 1,
        _ => 0,
    }
}

pub fn fit_within(w: u32, h: u32, max_side: u32) -> (u32, u32) {
    let longest = w.max(h);
    if longest <= max_side || longest == 0 {
        return (w, h);
    }
    let scale = max_side as f64 / longest as f64;
    (
        ((w as f64 * scale).round() as u32).max(1),
        ((h as f64 * scale).round() as u32).max(1),
    )
}

fn encode_data_uri(img: &RgbaImage, max_side: u32, quality: u8) -> Result<String, String> {
    let (w, h) = fit_within(img.width(), img.height(), max_side);
    let rgb = if (w, h) == (img.width(), img.height()) {
        DynamicImage::ImageRgba8(img.clone()).to_rgb8()
    } else {
        let small = image::imageops::resize(img, w, h, FilterType::Triangle);
        DynamicImage::ImageRgba8(small).to_rgb8()
    };
    let mut buf = Vec::new();
    JpegEncoder::new_with_quality(&mut buf, quality)
        .encode_image(&rgb)
        .map_err(|e| e.to_string())?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&buf);
    Ok(format!("data:image/jpeg;base64,{b64}"))
}

fn sorted_monitors() -> Result<Vec<xcap::Monitor>, String> {
    let mut all = xcap::Monitor::all().map_err(|e| e.to_string())?;
    all.sort_by_key(|m| (m.x().unwrap_or(0), m.y().unwrap_or(0)));
    Ok(all)
}

pub fn monitor_count() -> usize {
    sorted_monitors().map(|m| m.len()).unwrap_or(0)
}

pub fn monitor_name(index: usize) -> String {
    sorted_monitors()
        .ok()
        .and_then(|m| m.get(index).and_then(|m| m.friendly_name().or_else(|_| m.name()).ok()))
        .unwrap_or_else(|| format!("Layar {}", index + 1))
}

pub fn capture(choice: MonitorChoice) -> Result<Shot, String> {
    let monitors = sorted_monitors()?;
    if monitors.is_empty() {
        return Err("Tidak ada layar yang terdeteksi.".into());
    }
    let (idx, monitor) = match choice {
        MonitorChoice::Index(i) => {
            let i = i.min(monitors.len() - 1);
            (i, monitors.into_iter().nth(i).unwrap())
        }
        MonitorChoice::Cursor(point) => {
            let hit = point.and_then(|(x, y)| xcap::Monitor::from_point(x, y).ok());
            let id = hit.as_ref().and_then(|m| m.id().ok());
            let pos = id
                .and_then(|id| monitors.iter().position(|m| m.id().ok() == Some(id)))
                .or_else(|| monitors.iter().position(|m| m.is_primary().unwrap_or(false)))
                .unwrap_or(0);
            (pos, monitors.into_iter().nth(pos).unwrap())
        }
    };
    let name = monitor
        .friendly_name()
        .or_else(|_| monitor.name())
        .unwrap_or_else(|_| format!("Layar {}", idx + 1));
    let img = monitor.capture_image().map_err(|e| format!("Gagal menangkap layar: {e}"))?;
    Ok(Shot {
        data_uri: encode_data_uri(&img, MAX_SIDE, JPEG_QUALITY)?,
        thumbnail: encode_data_uri(&img, THUMB_SIDE, THUMB_QUALITY)?,
        monitor_name: name,
    })
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

/// macOS: true once Screen Recording is granted; otherwise triggers the system
/// prompt and returns false. Other platforms need no permission.
pub fn screen_access_granted() -> bool {
    #[cfg(target_os = "macos")]
    unsafe {
        if CGPreflightScreenCaptureAccess() {
            return true;
        }
        let _ = CGRequestScreenCaptureAccess();
        false
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_starts_at_first_then_wraps() {
        assert_eq!(next_selection(None, 3), 0);
        assert_eq!(next_selection(Some(0), 3), 1);
        assert_eq!(next_selection(Some(2), 3), 0);
        assert_eq!(next_selection(Some(7), 3), 0); // stale index after a monitor was unplugged
        assert_eq!(next_selection(Some(0), 1), 0);
    }

    #[test]
    fn fit_within_keeps_aspect_and_never_upscales() {
        assert_eq!(fit_within(3200, 1800, 1600), (1600, 900));
        assert_eq!(fit_within(1800, 3200, 1600), (900, 1600));
        assert_eq!(fit_within(800, 600, 1600), (800, 600));
        assert_eq!(fit_within(1600, 1600, 1600), (1600, 1600));
        assert_eq!(fit_within(10000, 1, 1600), (1600, 1)); // never collapses to 0
    }

    #[test]
    fn encodes_data_uri_jpeg_within_bounds() {
        let img = RgbaImage::from_pixel(2000, 1000, image::Rgba([10, 120, 200, 255]));
        let uri = encode_data_uri(&img, 1600, 80).unwrap();
        assert!(uri.starts_with("data:image/jpeg;base64,"));
        let b64 = uri.trim_start_matches("data:image/jpeg;base64,");
        let bytes = base64::engine::general_purpose::STANDARD.decode(b64).unwrap();
        let decoded = image::load_from_memory(&bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (1600, 800));
    }
}
