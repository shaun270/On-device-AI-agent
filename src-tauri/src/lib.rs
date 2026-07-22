// Martha — Rust backend

use tauri::{Manager, Emitter};
use std::sync::Mutex;
use std::str::FromStr;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

mod llm;
pub mod tools;

struct AppState {
    llm: Mutex<Option<llm::LlamaEngine>>,
    write_approval_tx: tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<bool>>>,
}

#[derive(serde::Deserialize, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[tauri::command]
async fn generate_response(mut history: Vec<ChatMessage>, agent_name: String, app_handle: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<String, String> {
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

    for _ in 0..5 { // Max 5 iterations per prompt
        let response = {
            let engine = state.llm.lock().map_err(|_| "Failed to lock engine".to_string())?;
            let llm = engine.as_ref().ok_or("LLM failed to initialize. Please check the terminal logs.")?;
            llm.generate(&history, &agent_name).map_err(|e| {
                println!("LLM Generation Error: {}", e);
                e
            })?
        };
        
        // Better JSON extraction to handle Qwen XML format AND markdown fallback
        // We prefill '{' in the prompt to force JSON mode, so we must prepend it to the response
        let full_response = format!("{{{}", response);
        
        let mut json_str = "";
        if let Some(end) = full_response.rfind("}") {
            json_str = &full_response[..=end];
        } else {
            json_str = &full_response;
        }

        println!("Trying to parse JSON: {}", json_str);
        match serde_json::from_str::<serde_json::Value>(json_str) {
                    Ok(tool_call) => {
                        println!("Successfully parsed JSON tool call.");
                        if let Some(tool_name) = tool_call.get("name").and_then(|v| v.as_str()) {
                            let _ = app_handle.emit("tool-status", format!("Using tool: {}", tool_name));
                        
                        let tool_result = match tool_name {
                            "search_files" => {
                                if let Some(query) = tool_call.get("arguments").and_then(|a| a.get("query")).and_then(|v| v.as_str()) {
                                    let directory = tool_call.get("arguments").and_then(|a| a.get("directory")).and_then(|v| v.as_str());
                                    crate::tools::search_files(query, directory)
                                } else {
                                    Err("Missing query argument".to_string())
                                }
                            },
                            "read_file" => {
                                if let Some(path) = tool_call.get("arguments").and_then(|a| a.get("path")).and_then(|v| v.as_str()) {
                                    crate::tools::read_file(path)
                                } else {
                                    Err("Missing path argument".to_string())
                                }
                            },
                            "write_file" => {
                                if let Some(path) = tool_call.get("arguments").and_then(|a| a.get("path")).and_then(|v| v.as_str()) {
                                    if let Some(content) = tool_call.get("arguments").and_then(|a| a.get("content")).and_then(|v| v.as_str()) {
                                        // Request human approval
                                        let (tx, rx) = tokio::sync::oneshot::channel();
                                        {
                                            let mut tx_lock = state.write_approval_tx.lock().await;
                                            *tx_lock = Some(tx);
                                        }
                                        let _ = app_handle.emit("write-approval-request", path);
                                        
                                        // Wait for UI to approve/deny
                                        let approved = rx.await.unwrap_or(false);
                                        if approved {
                                            crate::tools::write_file(path, content)
                                        } else {
                                            Err("User denied write permission".to_string())
                                        }
                                    } else {
                                        Err("Missing content argument".to_string())
                                    }
                                } else {
                                    Err("Missing path argument".to_string())
                                }
                            },
                            "save_memory" => {
                                if let Some(fact) = tool_call.get("arguments").and_then(|a| a.get("fact")).and_then(|v| v.as_str()) {
                                    crate::tools::save_memory(fact)
                                } else {
                                    Err("Missing fact argument".to_string())
                                }
                            },
                            "reply" => {
                                if let Some(msg) = tool_call.get("arguments").and_then(|a| a.get("message")).and_then(|v| v.as_str()) {
                                    let _ = app_handle.emit("tool-status", "Done.");
                                    return Ok(msg.to_string());
                                } else {
                                    Err("Missing message argument".to_string())
                                }
                            },
                            _ => Err(format!("Unknown tool: {}", tool_name))
                        };
                        
                        let result_str = match tool_result {
                            Ok(res) => res,
                            Err(e) => format!("Error: {}", e)
                        };
                        
                        history.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: json_str.to_string(),
                        });
                        history.push(ChatMessage {
                            role: "user".to_string(),
                            content: format!("Tool {} result:\n{}", tool_name, result_str),
                        });
                        
                        continue;
                        }
                    },
                Err(e) => {
                    println!("Failed to parse JSON: {}", e);
                    // Fallback manual extraction for malformed `reply` JSON (e.g. unescaped quotes)
                    if response.contains("\"reply\"") && response.contains("\"message\"") {
                        if let Some(msg_start) = response.find("\"message\": \"") {
                            let extracted = &response[msg_start + 12..];
                            // Remove trailing quotation marks and braces
                            let cleaned = extracted.trim_end_matches(&['}', ' ', '\n', '\r', '"'][..]);
                            let _ = app_handle.emit("tool-status", "Done.");
                            return Ok(cleaned.to_string());
                        }
                    }
                }
            } // Closes match
        
        let _ = app_handle.emit("tool-status", "Done.");
        return Ok(response);
    }
    
    Err("Max tool iterations reached".to_string())
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle, state: tauri::State<'_, AppState>) {
    if let Ok(mut engine) = state.llm.lock() {
        *engine = None; // Explicitly drop LlamaModel and Backend to prevent Metal crash
    }
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

#[tauri::command]
async fn approve_write(approved: bool, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut tx_lock = state.write_approval_tx.lock().await;
    if let Some(tx) = tx_lock.take() {
        let _ = tx.send(approved);
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState {
            llm: Mutex::new(None),
            write_approval_tx: tokio::sync::Mutex::new(None),
        })
        .setup(|app| {
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
        .invoke_handler(tauri::generate_handler![generate_response, quit_app, update_hotkey, approve_write])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
