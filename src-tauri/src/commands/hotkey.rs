use std::str::FromStr;

use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[tauri::command]
pub fn update_hotkey(
    app: tauri::AppHandle,
    new_shortcut: String,
    old_shortcut: Option<String>,
) -> Result<(), String> {
    if let Some(old) = old_shortcut {
        if let Ok(shortcut) = Shortcut::from_str(&old) {
            let _ = app.global_shortcut().unregister(shortcut);
        }
    }

    if let Ok(shortcut) = Shortcut::from_str(&new_shortcut) {
        let handle = app.clone();
        app.global_shortcut()
            .on_shortcut(shortcut, move |_app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    if let Some(win) = handle.get_webview_window("main") {
                        let is_minimized = win.is_minimized().unwrap_or(false);
                        let is_visible = win.is_visible().unwrap_or(false);
                        let is_focused = win.is_focused().unwrap_or(false);

                        if is_visible && is_focused && !is_minimized {
                            let _ = win.minimize();
                        } else {
                            if is_minimized {
                                let _ = win.unminimize();
                            }
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                }
            })
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
