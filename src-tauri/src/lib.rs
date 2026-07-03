mod commands;
mod config;
pub mod copilot;
mod db;
mod ghost_typing;
mod protection;
mod stt;
mod tray;

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

pub const INJECT_BOOTSTRAP: &str = r#"
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
            let profile_db = db::open(&app_data_dir).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
            })?;
            app.manage(profile_db);
            app.manage(crate::copilot::session::CopilotState::new());
            let cfg = config::read_config(&app_data_dir);
            let hotkey = cfg.hotkey.clone();

            // Sync autostart on startup
            {
                use tauri_plugin_autostart::ManagerExt;
                let autostart_manager = app.autolaunch();
                if cfg.autostart {
                    let _ = autostart_manager.enable();
                } else {
                    let _ = autostart_manager.disable();
                }
            }

            // Window 1: Hub (main app UI)
            let hub_url = if cfg!(debug_assertions) {
                WebviewUrl::External("http://localhost:1420".parse()?)
            } else {
                WebviewUrl::App("index.html".into())
            };
            let mut ai_view_builder = WebviewWindowBuilder::new(
                app,
                "ai-view",
                hub_url,
            )
            .title("no‑screen")
            .decorations(false)
            .always_on_top(true)
            .content_protected(true)
            .skip_taskbar(true)
            .transparent(true)
            .inner_size(1200.0, 820.0)
            .min_inner_size(900.0, 600.0)
            .resizable(true);

            if let Some((x, y)) = cfg.position {
                ai_view_builder = ai_view_builder.position(x as f64, y as f64);
            } else {
                ai_view_builder = ai_view_builder.center();
            }

            let ai_view = ai_view_builder.build()?;
            protection::apply(&ai_view);

            // Listen to Moved event and save position with 500ms debounce
            {
                let ai_view_clone = ai_view.clone();
                let app_handle = app.handle().clone();
                let move_counter = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));

                ai_view.on_window_event(move |event| {
                    if let tauri::WindowEvent::Moved(pos) = event {
                        let is_normal = ai_view_clone.is_maximized().unwrap_or(false) == false 
                                     && ai_view_clone.is_minimized().unwrap_or(false) == false;
                        if is_normal {
                            if let Ok(scale) = ai_view_clone.scale_factor() {
                                let logical_x = (pos.x as f64 / scale) as i32;
                                let logical_y = (pos.y as f64 / scale) as i32;

                                let version = move_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                                let move_counter_clone = move_counter.clone();
                                let app_handle_clone = app_handle.clone();

                                tauri::async_runtime::spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                    if move_counter_clone.load(std::sync::atomic::Ordering::SeqCst) == version {
                                        if let Ok(dir) = app_handle_clone.path().app_data_dir() {
                                            let mut cfg = config::read_config(&dir);
                                            cfg.position = Some((logical_x, logical_y));
                                            let _ = config::write_config(&dir, &cfg);
                                        }
                                    }
                                });
                            }
                        }
                    }
                });
            }

            // Window 2: Settings
            let settings_url = if cfg!(debug_assertions) {
                WebviewUrl::External("http://localhost:1420/settings".parse()?)
            } else {
                WebviewUrl::App("index.html".into())
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

            // Prevent Settings window from destroying itself on close
            {
                let settings_win_clone = settings_win.clone();
                settings_win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = settings_win_clone.hide();
                    }
                });
            }

            // Window 3: Copilot floating answer card (created hidden, shown when session starts)
            let card_url = if cfg!(debug_assertions) {
                WebviewUrl::External("http://localhost:1420/copilot-card".parse()?)
            } else {
                WebviewUrl::App("index.html".into())
            };
            let copilot_card = WebviewWindowBuilder::new(app, "copilot-card", card_url)
                .title("noscreen — Copilot")
                .decorations(false)
                .always_on_top(true)
                .content_protected(true)
                .skip_taskbar(true)
                .inner_size(380.0, 340.0)
                .min_inner_size(320.0, 200.0)
                .resizable(true)
                .transparent(true)
                .visible(false)
                .build()?;
            protection::apply(&copilot_card);

            // Prevent Copilot Card window from destroying itself on close and stop the session
            {
                let copilot_card_clone = copilot_card.clone();
                let app_handle = app.handle().clone();
                copilot_card.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = copilot_card_clone.hide();

                        // Stop the active copilot session when window is closed
                        let app_clone = app_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            let state = app_clone.try_state::<crate::copilot::session::CopilotState>();
                            let db = app_clone.try_state::<crate::db::Db>();
                            if let (Some(state), Some(db)) = (state, db) {
                                let _ = commands::copilot_stop_session(app_clone.clone(), state, db).await;
                            }
                        });
                    }
                });
            }

            // Service windows (pre-created hidden; shown/repositioned on demand)
            for (service, url_str) in [
                ("gpt",       "https://chatgpt.com"),
                ("claude",    "https://claude.ai"),
                ("translate", "https://translate.google.com"),
            ] {
                let label = format!("svc-{service}");
                let parsed: url::Url = url_str.parse().expect("valid service URL");
                let svc = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(parsed))
                    .decorations(false)
                    .always_on_top(true)
                    .skip_taskbar(true)
                    .content_protected(true)
                    .inner_size(800.0, 600.0)
                    .position(0.0, 0.0)
                    .visible(false)
                    .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
                    .initialization_script(INJECT_BOOTSTRAP)
                    .build()?;
                protection::apply(&svc);
            }

            // System Tray
            tray::setup_tray(app.handle())?;

            // Global Hotkeys
            use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

            // Toggle visibility
            let handle = app.handle().clone();
            app.handle()
                .global_shortcut()
                .on_shortcut(hotkey.as_str(), move |_app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        commands::toggle_visibility(handle.clone());
                    }
                })?;

            // Toggle ghost typing (Ctrl+Alt+G)
            // This fires without stealing focus from the browser.
            let handle2 = app.handle().clone();
            app.handle()
                .global_shortcut()
                .on_shortcut("Ctrl+Alt+G", move |_app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        commands::toggle_ghost_typing(handle2.clone());
                    }
                })?;

            // Copilot: bring hub forward (Ctrl+Alt+L)
            // Idle → user opens session modal from hub. Active → user can stop via panel.
            let handle3 = app.handle().clone();
            app.handle()
                .global_shortcut()
                .on_shortcut("Ctrl+Alt+L", move |_app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        if let Some(w) = handle3.get_webview_window("ai-view") {
                            crate::protection::show_no_activate(&w);
                            let _ = w.set_focus();
                        }
                    }
                })?;

            // Copilot: hide/show floating card (Ctrl+Alt+H)
            let handle4 = app.handle().clone();
            app.handle()
                .global_shortcut()
                .on_shortcut("Ctrl+Alt+H", move |_app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        if let Some(w) = handle4.get_webview_window("copilot-card") {
                            let visible = crate::protection::is_os_visible(&w);
                            if visible {
                                crate::protection::hide_window(&w);
                            } else {
                                crate::protection::show_no_activate(&w);
                            }
                        }
                    }
                })?;

            // Copilot: force regenerate suggestion (Ctrl+Alt+R)
            let handle5 = app.handle().clone();
            app.handle()
                .global_shortcut()
                .on_shortcut("Ctrl+Alt+R", move |_app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        if let Some(state) = handle5.try_state::<crate::copilot::session::CopilotState>() {
                            if let Ok(guard) = state.0.lock() {
                                if let Some(session) = guard.as_ref() {
                                    session.orchestrator.force_regenerate();
                                }
                            }
                        }
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
            commands::open_service_webview,
            commands::hide_service_webview,
            commands::resize_service_webview,
            commands::set_all_content_protected,
            commands::inject_to_service,
            commands::get_profile_name,
            commands::set_profile_name,
            commands::list_conversations,
            commands::create_conversation,
            commands::update_conversation_title,
            commands::delete_conversation,
            commands::get_messages,
            commands::append_message,
            commands::set_click_through,
            commands::start_ghost_typing_cmd,
            commands::stop_ghost_typing_cmd,
            commands::copilot_get_presets,
            commands::copilot_session_status,
            commands::copilot_start_session,
            commands::copilot_stop_session,
            commands::copilot_force_regenerate,
            commands::copilot_list_sessions,
            commands::get_profile_value,
            commands::set_profile_value,
            commands::copilot_set_custom_instruction,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
