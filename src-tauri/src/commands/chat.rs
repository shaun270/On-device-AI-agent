use crate::shared::llm_access;
use crate::shared::state::AppState;
use tauri::Emitter;

#[derive(serde::Deserialize, Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[tauri::command]
pub async fn generate_response(
    mut history: Vec<ChatMessage>,
    agent_name: String,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    // Wait for model
    llm_access::wait_until_ready(&state).await?;

    for _ in 0..5 { // Max 5 tool iterations per prompt
        // Call the LLM
        let response = match llm_access::generate(&state, crate::shared::PromptOrHistory::History(&history), &agent_name, None, None).await {
            Ok(res) => res,
            Err(e) => {
                println!("LLM Generation Error: {}", e);
                return Err(e);
            }
        };

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
pub async fn approve_write(approved: bool, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut tx_lock = state.write_approval_tx.lock().await;
    if let Some(tx) = tx_lock.take() {
        let _ = tx.send(approved);
    }
    Ok(())
}
