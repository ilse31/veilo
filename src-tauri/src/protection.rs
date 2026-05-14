use tauri::WebviewWindow;

/// Apply content capture exclusion to a window.
/// On Windows: calls WDA_EXCLUDEFROMCAPTURE (on top of Tauri's content_protected).
/// On macOS: content_protected: true in the builder handles sharingType = .none.
pub fn apply(window: &WebviewWindow) {
    #[cfg(windows)]
    apply_windows(window);

    #[cfg(not(windows))]
    let _ = window;
}

#[cfg(windows)]
fn apply_windows(window: &WebviewWindow) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE,
    };

    if let Ok(handle) = window.window_handle() {
        if let RawWindowHandle::Win32(h) = handle.as_raw() {
            unsafe {
                let _ = SetWindowDisplayAffinity(
                    HWND(h.hwnd.get() as isize),
                    WDA_EXCLUDEFROMCAPTURE,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn module_compiles() {
        assert!(true);
    }
}
