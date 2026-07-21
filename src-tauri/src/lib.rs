// Martha — Rust backend

use tauri::Manager;
use std::sync::Mutex;
use std::str::FromStr;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

mod llm;

struct AppState {
    llm: Mutex<Option<llm::LlamaEngine>>,
}

#[tauri::command]
async fn generate_response(message: String, agent_name: String, state: tauri::State<'_, AppState>) -> Result<String, String> {
    // Wait for the model to finish loading if it hasn't yet (up to 15 seconds)
    for _ in 0..30 {
        let is_loaded = {
            let engine = state.llm.lock().map_err(|_| "Failed to lock engine".to_string())?;
            engine.is_some()
        };
        
        if is_loaded {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    let engine = state.llm.lock().map_err(|_| "Failed to lock engine".to_string())?;
    
    if let Some(llm) = &*engine {
        match llm.generate(&message, &agent_name) {
            Ok(res) => Ok(res),
            Err(e) => {
                println!("LLM Generation Error: {}", e);
                Err(e)
            }
        }
    } else {
        Err("LLM failed to initialize. Please check the terminal logs.".to_string())
    }
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn update_hotkey(app: tauri::AppHandle, new_shortcut: String, old_shortcut: Option<String>) -> Result<(), String> {
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let state = AppState {
                llm: Mutex::new(None),
            };
            app.manage(state);

            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                let home = std::env::var("HOME").unwrap_or_default();
                let model_path = format!("{}/Library/Application Support/com.hey-martha.dev/models/qwen2.5-3b-coder-q4_k_m.gguf", home);
                
                println!("Loading local model from {}", model_path);
                match llm::LlamaEngine::new(&model_path) {
                    Ok(engine) => {
                        println!("Model loaded successfully!");
                        if let Ok(mut lock) = app_handle.state::<AppState>().llm.lock() {
                            *lock = Some(engine);
                        }
                    }
                    Err(e) => {
                        println!("Failed to load model: {}. Does it exist at the path?", e);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![generate_response, quit_app, update_hotkey])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
