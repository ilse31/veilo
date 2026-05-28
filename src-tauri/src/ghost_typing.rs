//! Ghost typing: low-level keyboard hook that intercepts keystrokes globally
//! so the user can "type" into noscreen without the browser ever losing focus.
//!
//! Activation flow:
//!   1. Caller sets WS_EX_NOACTIVATE on the hub window (clicking it won't steal focus)
//!   2. `start(app)` installs WH_KEYBOARD_LL and pumps messages on a worker thread
//!   3. Every keystroke is consumed (not forwarded to the browser) and emitted as
//!      a `ghost-key` Tauri event to the frontend
//!   4. `stop()` tears the hook down and signals the worker thread to exit

#[cfg(windows)]
pub use imp::*;

#[cfg(not(windows))]
pub use stub::*;

// ── Windows implementation ─────────────────────────────────────────────────

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;
    use tauri::{AppHandle, Emitter};

    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, GetKeyboardLayout, MapVirtualKeyW, ToUnicodeEx,
        MAPVK_VK_TO_VSC, VIRTUAL_KEY, VK_BACK, VK_CAPITAL, VK_CONTROL,
        VK_DELETE, VK_END, VK_ESCAPE, VK_HOME, VK_LEFT, VK_MENU, VK_RETURN,
        VK_RIGHT, VK_SHIFT, VK_SPACE, VK_TAB,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW,
        SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG,
        WH_KEYBOARD_LL, WM_KEYDOWN, WM_QUIT, WM_SYSKEYDOWN,
    };

    static ACTIVE: AtomicBool = AtomicBool::new(false);
    static THREAD_ID: Mutex<u32> = Mutex::new(0);
    static APP: Mutex<Option<AppHandle>> = Mutex::new(None);

    /// Key events emitted to the frontend as `ghost-key`.
    #[derive(serde::Serialize, Clone)]
    #[serde(tag = "type", content = "value")]
    pub enum GhostKey {
        Char(char),
        Backspace,
        Delete,
        Enter,
        Tab,
        Escape,
        Left,
        Right,
        Home,
        End,
    }

    pub fn is_active() -> bool {
        ACTIVE.load(Ordering::Relaxed)
    }

    /// Install the hook and start the worker thread.
    pub fn start(app: AppHandle) -> Result<(), String> {
        if ACTIVE.load(Ordering::Relaxed) {
            return Ok(());
        }
        *APP.lock().unwrap() = Some(app);
        ACTIVE.store(true, Ordering::Relaxed);

        std::thread::spawn(|| unsafe {
            *THREAD_ID.lock().unwrap() = GetCurrentThreadId();

            let hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), None, 0) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("[ghost] hook install failed: {e}");
                    ACTIVE.store(false, Ordering::Relaxed);
                    return;
                }
            };

            // Pump messages — hook callbacks are delivered here.
            let mut msg = MSG::default();
            while ACTIVE.load(Ordering::Relaxed)
                && GetMessageW(&mut msg, None, 0, 0).as_bool()
            {
                DispatchMessageW(&msg);
            }

            let _ = UnhookWindowsHookEx(hook);
            *THREAD_ID.lock().unwrap() = 0;
        });

        Ok(())
    }

    /// Uninstall the hook and tear down the worker thread.
    pub fn stop() {
        if !ACTIVE.load(Ordering::Relaxed) {
            return;
        }
        ACTIVE.store(false, Ordering::Relaxed);
        let tid = *THREAD_ID.lock().unwrap();
        if tid != 0 {
            unsafe {
                let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
        *APP.lock().unwrap() = None;
    }

    /// Low-level keyboard hook callback.
    /// Returns LRESULT(1) to consume the keystroke (browser never sees it).
    unsafe extern "system" fn keyboard_proc(
        code: i32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if code < 0 || !ACTIVE.load(Ordering::Relaxed) {
            return CallNextHookEx(HHOOK(std::ptr::null_mut()), code, wparam, lparam);
        }

        let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let is_down = wparam.0 as u32 == WM_KEYDOWN || wparam.0 as u32 == WM_SYSKEYDOWN;

        // Consume key-up silently; only process key-down.
        if !is_down {
            return LRESULT(1);
        }

        let vk = VIRTUAL_KEY(kb.vkCode as u16);

        // Pass modifier keys through so the browser can still use Ctrl+Tab, etc.
        // We intercept everything else.
        if matches!(vk, VK_SHIFT | VK_CONTROL | VK_MENU | VK_CAPITAL) {
            return CallNextHookEx(HHOOK(std::ptr::null_mut()), code, wparam, lparam);
        }

        let key = match vk {
            VK_BACK => Some(GhostKey::Backspace),
            VK_DELETE => Some(GhostKey::Delete),
            VK_RETURN => Some(GhostKey::Enter),
            VK_TAB => Some(GhostKey::Tab),
            VK_ESCAPE => Some(GhostKey::Escape),
            VK_LEFT => Some(GhostKey::Left),
            VK_RIGHT => Some(GhostKey::Right),
            VK_HOME => Some(GhostKey::Home),
            VK_END => Some(GhostKey::End),
            VK_SPACE => Some(GhostKey::Char(' ')),
            _ => vk_to_char(vk, kb.scanCode).map(GhostKey::Char),
        };

        if let Some(k) = key {
            if let Ok(guard) = APP.lock() {
                if let Some(app) = guard.as_ref() {
                    let _ = app.emit("ghost-key", &k);
                }
            }
        }

        LRESULT(1) // consume — browser never receives this keystroke
    }

    /// Convert a virtual key to a Unicode character, respecting shift / caps lock.
    unsafe fn vk_to_char(vk: VIRTUAL_KEY, scan: u32) -> Option<char> {
        let mut buf = [0u16; 8];
        let mut kbd_state = [0u8; 256];

        // GetAsyncKeyState is more reliable than GetKeyState inside LL hooks.
        if (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16) & 0x8000 != 0 {
            kbd_state[VK_SHIFT.0 as usize] = 0x80;
        }
        if (GetAsyncKeyState(VK_CAPITAL.0 as i32) as u16) & 0x0001 != 0 {
            kbd_state[VK_CAPITAL.0 as usize] = 0x01;
        }

        let scan_code = if scan == 0 {
            MapVirtualKeyW(vk.0 as u32, MAPVK_VK_TO_VSC)
        } else {
            scan
        };
        let hkl = GetKeyboardLayout(0);
        let n = ToUnicodeEx(vk.0 as u32, scan_code, &kbd_state, &mut buf, 0, hkl);

        if n > 0 {
            char::from_u32(buf[0] as u32).filter(|c| !c.is_control())
        } else {
            None
        }
    }
}

// ── Non-Windows stub ───────────────────────────────────────────────────────

#[cfg(not(windows))]
mod stub {
    use tauri::AppHandle;

    pub fn is_active() -> bool {
        false
    }
    pub fn start(_app: AppHandle) -> Result<(), String> {
        Err("ghost typing is Windows-only".into())
    }
    pub fn stop() {}
}
