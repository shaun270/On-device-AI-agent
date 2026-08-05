//! Martha — Rust backend shell.
//!
//! Wiring only: modules, AppState setup, command registration.
//! Feature logic lives under `capabilities/<name>/`.
//! Shared LLM engine: `llm.rs` + `shared/llm_access.rs`.

mod capabilities;
mod commands;
mod embedding;
mod llm;
mod model_download;
mod router;
mod shared;
pub mod tools;

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

            // llama.cpp's backend is a process-wide singleton (init() fails if one
            // is already alive) — both the chat model and the embedding model must
            // share this one instance, not each create their own.
            let backend = match llm::init_shared_backend() {
                Ok(backend) => backend,
                Err(e) => {
                    println!("Failed to init llama backend: {e}");
                    return Ok(());
                }
            };

            let app_handle = app.handle().clone();
            let llm_backend = std::sync::Arc::clone(&backend);
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
                match llm::LlamaEngine::new(llm_backend, &model_path) {
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

            let embed_app_handle = app.handle().clone();
            let embed_backend = std::sync::Arc::clone(&backend);
            std::thread::spawn(move || {
                let embed_model_path = model_download::default_embedding_model_path();

                if let Err(e) = model_download::ensure_embedding_model(&embed_model_path) {
                    println!("Failed to prepare embedding model: {e}");
                    return;
                }

                if !embed_model_path.is_file() {
                    println!("Embedding model file missing at {}", embed_model_path.display());
                    return;
                }

                println!("Loading embedding model from {}", embed_model_path.display());
                let engine = match embedding::EmbeddingEngine::new(embed_backend, &embed_model_path) {
                    Ok(engine) => {
                        println!("Embedding model loaded successfully!");
                        engine
                    }
                    Err(e) => {
                        println!("Failed to load embedding model: {e}");
                        return;
                    }
                };

                println!("Building intent router from capability exemplars…");
                let domains = match capabilities::all_domain_exemplars_personalized() {
                    Ok(domains) => domains,
                    Err(e) => {
                        println!("Failed to load router exemplars: {e}");
                        return;
                    }
                };

                match router::build::build_router(&engine, domains) {
                    Ok(router) => {
                        println!("Router built successfully!");
                        if let Ok(mut lock) = embed_app_handle.state::<AppState>().router.lock() {
                            *lock = Some(router);
                        }
                    }
                    Err(e) => {
                        println!("Failed to build router: {e}");
                    }
                }

                if let Ok(mut lock) = embed_app_handle.state::<AppState>().embedder.lock() {
                    *lock = Some(engine);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // reminders capability
            capabilities::reminders::commands::set_reminder,
            capabilities::reminders::commands::list_reminders,
            capabilities::reminders::commands::complete_reminder,
            capabilities::reminders::commands::list_reminders_structured,
            capabilities::reminders::commands::complete_reminder_by_id,
            capabilities::reminders::commands::open_reminders_settings,
            capabilities::reminders::commands::classify_intent,
            // shared shell
            commands::chat::generate_response,
            commands::chat::approve_write,
            commands::quit::quit_app,
            commands::hotkey::update_hotkey,
            commands::route::route_intent,
            commands::route::submit_router_correction,
            commands::route::execute_forced_action,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
