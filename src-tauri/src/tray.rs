use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle,
};

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let show_hide = MenuItem::with_id(app, "show_hide", "Show / Hide", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit noscreen", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_hide, &settings_item, &quit])?;

    TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("noscreen")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show_hide" => crate::commands::toggle_visibility(app.clone()),
            "settings" => crate::commands::open_settings(app.clone()),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                crate::commands::toggle_visibility(tray.app_handle().clone());
            }
        })
        .build(app)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn module_compiles() {
        assert!(true);
    }
}
