//! Martha — Rust backend shell.
//!
//! Wiring only: modules, AppState setup, command registration.
//! Feature logic lives under `capabilities/<name>/`.
//! Shared LLM engine: `llm.rs` + `shared/llm_access.rs`.

mod capabilities;
mod commands;
mod llm;
mod model_download;
mod shared;

use tauri::Manager;

use shared::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            capabilities::reminders::prewarm();

            app.manage(AppState::new());

            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                let model_path = model_download::default_model_path();

                if let Err(e) = model_download::ensure_model(&model_path) {
                    println!("Failed to prepare model: {e}");
                    return;
                }

                if !model_path.is_file() {
                    println!("Model file missing at {}", model_path.display());
                    return;
                }

                println!("Loading local model from {}", model_path.display());
                match llm::LlamaEngine::new(&model_path) {
                    Ok(engine) => {
                        println!("Model loaded successfully!");
                        if let Ok(mut lock) = app_handle.state::<AppState>().llm.lock() {
                            *lock = Some(engine);
                        }
                    }
                    Err(e) => {
                        println!("Failed to load model: {e}");
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // reminders capability
            capabilities::reminders::commands::set_reminder,
            capabilities::reminders::commands::list_reminders,
            capabilities::reminders::commands::complete_reminder,
            capabilities::reminders::commands::open_reminders_settings,
            capabilities::reminders::commands::classify_intent,
            // shared shell
            commands::chat::generate_response,
            commands::quit::quit_app,
            commands::hotkey::update_hotkey,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
