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

/// Show a window WITHOUT activating it (browser keeps foreground focus).
/// Tauri's w.show() uses SW_SHOW which activates the window → browser blur fires.
pub fn show_no_activate(window: &WebviewWindow) {
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE};
        if let Some(hwnd) = get_hwnd(window) {
            unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE); }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = window.show();
    }
}

/// Hide a window via Win32 directly.
/// Tauri's w.hide() checks its own internal visible flag — after show_no_activate
/// Tauri still thinks the window is hidden, so hide() becomes a no-op.
pub fn hide_window(window: &WebviewWindow) {
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
        if let Some(hwnd) = get_hwnd(window) {
            unsafe { ShowWindow(hwnd, SW_HIDE); }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = window.hide();
    }
}

/// Check OS-level visibility via IsWindowVisible — not Tauri's internal flag.
pub fn is_os_visible(window: &WebviewWindow) -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::IsWindowVisible;
        if let Some(hwnd) = get_hwnd(window) {
            return unsafe { IsWindowVisible(hwnd).as_bool() };
        }
    }
    window.is_visible().unwrap_or(false)
}

/// Add WS_EX_NOACTIVATE to a window so clicking it never steals keyboard focus.
/// Used when ghost typing mode is active.
pub fn apply_noactivate(window: &WebviewWindow) {
    #[cfg(windows)]
    set_noactivate_windows(window, true);
    #[cfg(not(windows))]
    let _ = window;
}

/// Remove WS_EX_NOACTIVATE — restores normal focus behaviour.
pub fn remove_noactivate(window: &WebviewWindow) {
    #[cfg(windows)]
    set_noactivate_windows(window, false);
    #[cfg(not(windows))]
    let _ = window;
}

#[cfg(windows)]
fn get_hwnd(window: &WebviewWindow) -> Option<windows::Win32::Foundation::HWND> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    if let Ok(handle) = window.window_handle() {
        if let RawWindowHandle::Win32(h) = handle.as_raw() {
            return Some(windows::Win32::Foundation::HWND(
                h.hwnd.get() as *mut core::ffi::c_void,
            ));
        }
    }
    None
}

#[cfg(windows)]
fn apply_windows(window: &WebviewWindow) {
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE,
    };
    if let Some(hwnd) = get_hwnd(window) {
        unsafe {
            let _ = SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);
        }
    }
}

#[cfg(windows)]
fn set_noactivate_windows(window: &WebviewWindow, enable: bool) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE,
    };
    if let Some(hwnd) = get_hwnd(window) {
        unsafe {
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let new_ex = if enable {
                ex | WS_EX_NOACTIVATE.0 as isize
            } else {
                ex & !(WS_EX_NOACTIVATE.0 as isize)
            };
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex);
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
