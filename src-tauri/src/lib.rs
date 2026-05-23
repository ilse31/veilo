mod commands;
mod config;
mod protection;
mod stt;
mod tray;

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

const INJECT_BOOTSTRAP: &str = r#"
(function() {
  // --- Anti-bot fingerprint patch (runs before page scripts) ---

  // Remove webdriver flag that Cloudflare checks
  try {
    Object.defineProperty(navigator, 'webdriver', { get: () => undefined, configurable: true });
  } catch(e) {}

  // Inject window.chrome so sites think this is a real Chrome browser
  if (!window.chrome) {
    try {
      Object.defineProperty(window, 'chrome', {
        value: { runtime: {}, loadTimes: function(){}, csi: function(){}, app: {} },
        writable: true, configurable: true
      });
    } catch(e) {}
  }

  // Ensure navigator.languages looks real
  try {
    Object.defineProperty(navigator, 'languages', { get: () => ['en-US', 'en'], configurable: true });
  } catch(e) {}

  // Ensure navigator.plugins is non-empty (empty = headless red flag)
  try {
    if (!navigator.plugins || navigator.plugins.length === 0) {
      Object.defineProperty(navigator, 'plugins', { get: () => [1,2,3,4,5], configurable: true });
    }
  } catch(e) {}

  // --- noscreen text injection ---
  window.__noscreen_inject = function(text) {
    // Try AI chat selectors first, then fall back to any textarea
    var el = document.querySelector('div[contenteditable="true"]')
           || document.querySelector('#prompt-textarea')
           || document.querySelector('textarea');
    if (!el) { console.warn('[noscreen] input element not found'); return; }

    el.focus();
    el.click();

    // execCommand works for both contenteditable and textarea in WebView2/Chrome
    var inserted = document.execCommand('insertText', false, text);

    // Fallback for textarea if execCommand didn't work
    if (!inserted && (el.tagName === 'TEXTAREA' || el.tagName === 'INPUT')) {
      var nativeSetter = Object.getOwnPropertyDescriptor(window.HTMLTextAreaElement.prototype, 'value').set;
      nativeSetter.call(el, el.value + text);
      el.dispatchEvent(new Event('input', { bubbles: true }));
      el.dispatchEvent(new Event('change', { bubbles: true }));
    }

    // Press Enter only for AI chat sites (not Google Translate which auto-translates)
    var isAiChat = location.hostname.includes('claude.ai') || location.hostname.includes('chatgpt.com');
    if (isAiChat) {
      el.dispatchEvent(new KeyboardEvent('keydown', {
        key: 'Enter', code: 'Enter', bubbles: true, cancelable: true
      }));
    }
  };
})();
"#;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .manage(commands::SttState(std::sync::Mutex::new(None)))
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let cfg = config::read_config(&app_data_dir);
            let hotkey = cfg.hotkey.clone();

            // Window 1: AI WebView
            let ai_url: tauri::Url = cfg.site.parse()?;
            let ai_view = WebviewWindowBuilder::new(
                app,
                "ai-view",
                WebviewUrl::External(ai_url),
            )
            .title("noscreen")
            .decorations(false)
            .always_on_top(true)
            .content_protected(true)
            .skip_taskbar(true)
            .inner_size(420.0, 700.0)
            .initialization_script(INJECT_BOOTSTRAP)
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .build()?;
            protection::apply(&ai_view);

            // Window 2: Control Bar
            let control_url = if cfg!(debug_assertions) {
                WebviewUrl::External("http://localhost:1420/control-bar".parse()?)
            } else {
                WebviewUrl::App("control-bar/index.html".into())
            };
            // NOTE: .transparent(true) requires the `macos-private-api` Cargo feature on macOS.
            // Omitted until that feature is enabled; add it back along with the feature flag.
            let control_bar = WebviewWindowBuilder::new(app, "control-bar", control_url)
                .decorations(false)
                .always_on_top(true)
                .content_protected(true)
                .skip_taskbar(true)
                .inner_size(360.0, 108.0)
                .resizable(false)
                .build()?;
            protection::apply(&control_bar);

            // Window 3: Settings
            let settings_url = if cfg!(debug_assertions) {
                WebviewUrl::External("http://localhost:1420/settings".parse()?)
            } else {
                WebviewUrl::App("settings/index.html".into())
            };
            let settings_win = WebviewWindowBuilder::new(app, "settings", settings_url)
                .title("noscreen — Settings")
                .content_protected(true)
                .inner_size(380.0, 480.0)
                .resizable(false)
                .center()
                .visible(false)
                .build()?;
            protection::apply(&settings_win);
            // System Tray
            tray::setup_tray(app.handle())?;

            // Global Hotkey
            use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
            let handle = app.handle().clone();
            app.handle()
                .global_shortcut()
                .on_shortcut(hotkey.as_str(), move |_app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        commands::toggle_visibility(handle.clone());
                    }
                })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::inject_text,
            commands::get_config,
            commands::save_config,
            commands::set_site,
            commands::toggle_visibility_cmd,
            commands::open_settings_cmd,
            commands::start_stt_cmd,
            commands::stop_stt_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
