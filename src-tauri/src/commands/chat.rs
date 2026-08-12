use crate::shared::llm_access;
use crate::shared::state::AppState;
use tauri::Emitter;

#[derive(serde::Deserialize, Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Index (inclusive) of the closing `}` of the FIRST balanced JSON object in
/// `s`, ignoring braces inside string literals. The model sometimes emits
/// two tool calls back-to-back in one response (`{...}{...}`) instead of
/// one at a time as the loop below expects — `s.rfind('}')` would grab both
/// concatenated into one invalid blob and fail to parse at all, silently
/// executing nothing. Only ever parsing the first object here means we
/// still make progress on iteration N and pick up the rest — if there is a
/// second tool call — on iteration N+1, once its result is in history.
fn find_first_json_object_end(s: &str) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    let mut started = false;
    for (i, b) in s.bytes().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'{' => {
                depth += 1;
                started = true;
            }
            b'}' => {
                depth -= 1;
                if started && depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

#[tauri::command]
pub async fn generate_response(
    mut history: Vec<ChatMessage>,
    agent_name: String,
    current_date: String,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    // Wait for model
    llm_access::wait_until_ready(&state).await?;

    for _ in 0..5 { // Max 5 tool iterations per prompt
        // Call the LLM
        let response = match llm_access::generate(&state, crate::shared::PromptOrHistory::History(&history), &agent_name, None, None, &current_date).await {
            Ok(res) => res,
            Err(e) => {
                println!("LLM Generation Error: {}", e);
                return Err(e);
            }
        };

        // We prefill '{' in the prompt to force JSON mode, so we must prepend it to the response
        let full_response = format!("{{{}", response);

        let json_str = match find_first_json_object_end(&full_response) {
            Some(end) => &full_response[..=end],
            None => &full_response,
        };

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
                        "list_reminders" => {
                            let args = tool_call.get("arguments").cloned().unwrap_or_else(|| serde_json::json!({}));
                            crate::capabilities::dispatch("list_reminders", args)
                        },
                        "complete_reminder" => {
                            if let Some(title) = tool_call.get("arguments").and_then(|a| a.get("title")).and_then(|v| v.as_str()) {
                                let mut args = serde_json::json!({ "title": title });
                                if let Some(m) = tool_call.get("arguments").and_then(|a| a.get("match")).and_then(|v| v.as_str()) {
                                    args["match"] = serde_json::json!(m);
                                }
                                crate::capabilities::dispatch("complete_reminder", args)
                            } else {
                                Err("Missing title argument".to_string())
                            }
                        },
                        "set_reminder" => {
                            if let Some(title) = tool_call.get("arguments").and_then(|a| a.get("title")).and_then(|v| v.as_str()) {
                                let mut args = serde_json::json!({ "title": title });
                                if let Some(due) = tool_call.get("arguments").and_then(|a| a.get("due")).and_then(|v| v.as_str()) {
                                    args["due"] = serde_json::json!(due);
                                }
                                crate::capabilities::dispatch("set_reminder", args)
                            } else {
                                Err("Missing title argument".to_string())
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

#[cfg(test)]
mod tests {
    use super::{find_first_json_object_end, ChatMessage};

    #[test]
    fn finds_end_of_single_object() {
        let s = r#"{"name":"reply","arguments":{"message":"hi"}}"#;
        assert_eq!(find_first_json_object_end(s), Some(s.len() - 1));
    }

    #[test]
    fn stops_at_first_object_when_two_are_concatenated() {
        // The exact failure mode found live: two tool calls back to back.
        let first = r#"{"name":"set_reminder","arguments":{"title":"a"}}"#;
        let second = r#"{"name":"set_reminder","arguments":{"title":"b"}}"#;
        let s = format!("{first}{second}");
        let end = find_first_json_object_end(&s).expect("should find the first object");
        assert_eq!(&s[..=end], first);
    }

    #[test]
    fn braces_inside_string_values_dont_confuse_depth() {
        let s = r#"{"name":"reply","arguments":{"message":"say { or } if you like"}}"#;
        let end = find_first_json_object_end(s).expect("should find the object");
        assert_eq!(end, s.len() - 1);
        assert!(serde_json::from_str::<serde_json::Value>(&s[..=end]).is_ok());
    }

    #[test]
    fn escaped_quotes_dont_end_string_early() {
        let s = r#"{"name":"reply","arguments":{"message":"she said \"hi {there}\""}}"#;
        let end = find_first_json_object_end(s).expect("should find the object");
        assert_eq!(end, s.len() - 1);
        assert!(serde_json::from_str::<serde_json::Value>(&s[..=end]).is_ok());
    }

    #[test]
    fn no_object_returns_none() {
        assert_eq!(find_first_json_object_end("just plain text"), None);
    }

    /// Real Qwen model, real inference — downloads the ~1.9GB GGUF on first
    /// run. Reproduces the exact "check that off" scenario: the general
    /// chat fallback must use conversation history to resolve "that" to a
    /// real reminder title and call `complete_reminder` for real — not
    /// fabricate a success message (see llm.rs's system prompt, which now
    /// includes real reminders tools). Not run by default:
    /// `cargo test --lib -- --ignored fallback_resolves_pronoun_reference`.
    #[test]
    #[ignore]
    fn fallback_resolves_pronoun_reference_to_real_complete_call() {
        let model_path = crate::model_download::default_model_path();
        crate::model_download::ensure_model(&model_path).expect("failed to download model");

        let backend = crate::llm::init_shared_backend().expect("failed to init llama backend");
        let engine = crate::llm::LlamaEngine::new(backend, &model_path).expect("failed to load model");

        let history = vec![
            ChatMessage { role: "user".to_string(), content: "show me all reminders".to_string() },
            ChatMessage {
                role: "assistant".to_string(),
                content: "- call my mom (due: 2026-08-04 14:00)".to_string(),
            },
            ChatMessage { role: "user".to_string(), content: "check that off".to_string() },
        ];

        let current_date = "now=2026-08-03T21:20; tomorrow=2026-08-04; year=2026";
        let res = engine
            .generate(crate::shared::PromptOrHistory::History(&history), "Martha", None, None, current_date)
            .expect("generation failed");

        println!("raw response: {res}");

        // generate() prefills '{' to force JSON mode; callers re-add it (see the
        // real loop in generate_response above).
        let full_response = format!("{{{res}");
        let json_str = match full_response.rfind('}') {
            Some(end) => &full_response[..=end],
            None => &full_response,
        };
        let parsed: serde_json::Value =
            serde_json::from_str(json_str).unwrap_or_else(|e| panic!("invalid JSON: {e}\nraw: {res}"));

        assert_eq!(
            parsed["name"], "complete_reminder",
            "expected a real complete_reminder tool call using context, got {parsed}"
        );
        let title = parsed["arguments"]["title"].as_str().unwrap_or("").to_lowercase();
        assert!(title.contains("mom"), "expected title to mention 'mom' (from history), got {title:?}");
    }

    /// Real Qwen model, real inference. Reproduces the exact bug found live:
    /// asking the fallback agent to create a reminder for "tomorrow" produced
    /// a due date 6 days out, because the chat-mode system prompt had no
    /// notion of what day "today" actually is. Not run by default:
    /// `cargo test --lib -- --ignored fallback_computes_correct_due_date`.
    #[test]
    #[ignore]
    fn fallback_computes_correct_due_date_from_current_date() {
        let model_path = crate::model_download::default_model_path();
        crate::model_download::ensure_model(&model_path).expect("failed to download model");

        let backend = crate::llm::init_shared_backend().expect("failed to init llama backend");
        let engine = crate::llm::LlamaEngine::new(backend, &model_path).expect("failed to load model");

        let history = vec![ChatMessage {
            role: "user".to_string(),
            content: "make a reminder to play football tomorrow at 9pm".to_string(),
        }];

        // Today is 2026-08-03, so "tomorrow" must be 2026-08-04.
        let current_date = "now=2026-08-03T21:20; tomorrow=2026-08-04; year=2026";

        let res = engine
            .generate(crate::shared::PromptOrHistory::History(&history), "Martha", None, None, current_date)
            .expect("generation failed");

        println!("raw response: {res}");

        let full_response = format!("{{{res}");
        let json_str = match full_response.rfind('}') {
            Some(end) => &full_response[..=end],
            None => &full_response,
        };
        let parsed: serde_json::Value =
            serde_json::from_str(json_str).unwrap_or_else(|e| panic!("invalid JSON: {e}\nraw: {res}"));

        assert_eq!(parsed["name"], "set_reminder", "expected a set_reminder call, got {parsed}");
        let due = parsed["arguments"]["due"].as_str().unwrap_or("");
        assert!(
            due.starts_with("2026-08-04"),
            "expected due date on 2026-08-04 (tomorrow), got {due:?} — raw: {res}"
        );
    }
}

#[tauri::command]
pub async fn approve_write(approved: bool, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut tx_lock = state.write_approval_tx.lock().await;
    if let Some(tx) = tx_lock.take() {
        let _ = tx.send(approved);
    }
    Ok(())
}
