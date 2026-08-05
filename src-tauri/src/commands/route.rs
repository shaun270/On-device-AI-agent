//! Cross-capability entry point: hierarchical embedding router → per-action
//! slot-filling LLM call → (for domains that execute server-side) actually
//! running the tool. Runs alongside the older, reminders-only
//! `classify_intent` command during migration (see the build plan) —
//! nothing here touches that path.
//!
//! Two different contracts come back as JSON, both under a `kind` field:
//! - reminders: `{"kind":"set"|"set_many"|"list"|"complete"|"clarify",...}` —
//!   the frontend still normalizes/executes these itself (`runReminderAction`).
//! - anything already executed here (files, and any future domain that
//!   doesn't need frontend-side formatting): `{"kind":"reply","message":"..."}`.
//! - `{"kind":"unhandled"}` — nothing confidently matched, or a domain's
//!   slot-filler couldn't safely proceed alone (e.g. it needs chat history
//!   this single message doesn't have). The frontend falls back to
//!   `generate_response`, which has that history.

use serde_json::{json, Value};
use tauri::Emitter;

use crate::capabilities;
use crate::capabilities::files::prompts as file_prompts;
use crate::capabilities::reminders::prompts as reminder_prompts;
use crate::router::RouteResult;
use crate::shared::llm_access;
use crate::shared::state::AppState;

/// Poll up to ~15s for the router (embedder + exemplars) to finish building.
async fn wait_until_router_ready(state: &tauri::State<'_, AppState>) -> Result<(), String> {
    for _ in 0..30 {
        let ready = {
            let router = state.router.lock().map_err(|_| "Failed to lock router".to_string())?;
            router.is_some()
        };
        if ready {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    Ok(())
}

fn unhandled() -> String {
    json!({ "kind": "unhandled" }).to_string()
}

fn reply(message: impl Into<String>) -> String {
    json!({ "kind": "reply", "message": message.into() }).to_string()
}

fn extract_json_object(raw: &str) -> Option<String> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(raw[start..=end].trim().to_string())
}

/// Runs a single-shot slot-filling LLM call and parses the result as JSON.
/// `current_date` is unused here (these prompts are always `custom_system`,
/// never chat mode) — just threaded through to match `LlamaEngine::generate`'s
/// signature; pass `""` when the specific prompt doesn't need a date.
async fn call_llm_for_json(
    state: &tauri::State<'_, AppState>,
    text: &str,
    system_prompt: String,
    current_date: &str,
) -> Result<Value, String> {
    let res = llm_access::generate(
        state,
        crate::shared::PromptOrHistory::Prompt(text),
        "Router",
        Some(system_prompt),
        Some(256),
        current_date,
    )
    .await?;

    let cleaned = extract_json_object(&res)
        .unwrap_or_else(|| res.replace("```json", "").replace("```", "").trim().to_string());

    serde_json::from_str(&cleaned).map_err(|e| format!("slot-filler returned invalid JSON: {e}"))
}

/// Routes `text` through the embedding router, then fills slots for the
/// matched action. Returns `{"kind":"unhandled"}` when nothing confidently
/// matched — the frontend falls back to `generate_response` in that case.
#[tauri::command]
pub async fn route_intent(
    text: String,
    current_date: String,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    wait_until_router_ready(&state).await?;
    llm_access::wait_until_ready(&state).await?;

    let result = {
        let router = state.router.lock().map_err(|_| "Failed to lock router".to_string())?;
        let embedder = state.embedder.lock().map_err(|_| "Failed to lock embedder".to_string())?;

        let (Some(router), Some(embedder)) = (router.as_ref(), embedder.as_ref()) else {
            return Ok(unhandled());
        };

        router.route(&text, embedder)?
    };

    match result {
        RouteResult::Matched { domain, action } if domain == "reminders" => {
            fill_reminder_slots(&action, &text, &current_date, &state).await
        }
        RouteResult::Matched { domain, action } if domain == "files" => {
            fill_file_slots(&action, &text, &state, &app_handle).await
        }
        // The domain-blind general chat agent (generate_response) has zero
        // reminders tools — if we dumped this there it would just fabricate a
        // plausible-sounding "done!" without doing anything. We already know
        // it's reminders, just not which action, so escalate to the older
        // full reminders classifier (still domain-scoped) instead. Files
        // doesn't need this: generate_response already has real
        // search_files/read_file/write_file tools, so its Ambiguous case is
        // safe to fall through.
        RouteResult::AmbiguousAction { domain } if domain == "reminders" => {
            escalate_ambiguous_reminders(&text, &current_date, &state).await
        }
        _ => Ok(unhandled()),
    }
}

async fn escalate_ambiguous_reminders(
    text: &str,
    current_date: &str,
    state: &tauri::State<'_, AppState>,
) -> Result<String, String> {
    let system_prompt = crate::capabilities::reminders::router::system_prompt(current_date);
    let res = llm_access::generate(
        state,
        crate::shared::PromptOrHistory::Prompt(text),
        "Router",
        Some(system_prompt),
        Some(128),
        current_date,
    )
    .await?;

    let cleaned = crate::capabilities::reminders::router::extract_json_object(&res)
        .unwrap_or_else(|| res.replace("```json", "").replace("```", "").trim().to_string());

    // Already includes "kind" (set/set_many/list/complete/clarify/chat) — same
    // contract fill_reminder_slots produces, so the frontend handles it identically.
    serde_json::from_str::<Value>(&cleaned)
        .map(|v| v.to_string())
        .map_err(|e| format!("escalation classifier returned invalid JSON: {e}"))
}

async fn fill_reminder_slots(
    action: &str,
    text: &str,
    current_date: &str,
    state: &tauri::State<'_, AppState>,
) -> Result<String, String> {
    if action == "clarify" {
        return Ok(json!({
            "kind": "clarify",
            "message": "I can't delete reminders yet — remove them in the Reminders app or check them off."
        })
        .to_string());
    }

    let system_prompt = match action {
        "set" => reminder_prompts::set::system_prompt(current_date),
        "set_many" => reminder_prompts::set_many::system_prompt(current_date),
        "list" => reminder_prompts::list::system_prompt(current_date),
        "complete" => reminder_prompts::complete::system_prompt(current_date),
        other => return Err(format!("unknown reminders action: {other}")),
    };

    let mut value = call_llm_for_json(state, text, system_prompt, current_date).await?;
    value["kind"] = json!(action);
    Ok(value.to_string())
}

async fn fill_file_slots(
    action: &str,
    text: &str,
    state: &tauri::State<'_, AppState>,
    app_handle: &tauri::AppHandle,
) -> Result<String, String> {
    match action {
        "search" => {
            let value = call_llm_for_json(state, text, file_prompts::search::system_prompt(""), "").await?;
            let query = value.get("query").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if query.is_empty() {
                return Ok(unhandled());
            }
            let directory = value.get("directory").and_then(|v| v.as_str()).map(str::trim).filter(|s| !s.is_empty());
            let mut input = json!({ "query": query });
            if let Some(dir) = directory {
                input["directory"] = json!(dir);
            }
            let result = capabilities::dispatch("search_files", input)?;
            Ok(reply(result))
        }
        "read" => {
            let value = call_llm_for_json(state, text, file_prompts::read::system_prompt(""), "").await?;
            let path_or_query = value.get("path_or_query").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if path_or_query.is_empty() {
                return Ok(unhandled());
            }

            let resolved_path = resolve_read_path(&path_or_query)?;
            let Some(resolved_path) = resolved_path else {
                return Ok(reply(format!("I couldn't find a file matching \"{path_or_query}\".")));
            };

            let content = capabilities::dispatch("read_file", json!({ "path": resolved_path }))?;
            Ok(reply(content))
        }
        "write" => {
            let value = call_llm_for_json(state, text, file_prompts::write::system_prompt(""), "").await?;
            let path = value.get("path").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            let content = value.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if path.is_empty() || content.is_empty() {
                // Most likely depends on chat history ("save what you just wrote") —
                // this single message doesn't have that context, but generate_response
                // does, so defer to it instead of guessing or writing something wrong.
                return Ok(unhandled());
            }

            // Same human-approval gate as commands/chat.rs's write_file branch —
            // never skip this. See capabilities/files/write_file.rs's doc comment.
            let (tx, rx) = tokio::sync::oneshot::channel();
            {
                let mut tx_lock = state.write_approval_tx.lock().await;
                *tx_lock = Some(tx);
            }
            let _ = app_handle.emit("write-approval-request", &path);
            let approved = rx.await.unwrap_or(false);
            if !approved {
                return Ok(reply("Okay, I didn't write the file."));
            }

            let result = capabilities::dispatch("write_file", json!({ "path": path, "content": content }))?;
            Ok(reply(result))
        }
        other => Err(format!("unknown files action: {other}")),
    }
}

/// `path_or_query` is either a real path the user gave, or a description
/// the LLM couldn't turn into a path — search for it deterministically
/// rather than looping the LLM (no multi-turn agent needed for this).
fn resolve_read_path(path_or_query: &str) -> Result<Option<String>, String> {
    let expanded = expand_tilde(path_or_query);
    if (path_or_query.starts_with('/') || path_or_query.starts_with('~'))
        && std::path::Path::new(&expanded).is_file()
    {
        return Ok(Some(expanded));
    }

    let search_result = capabilities::dispatch("search_files", json!({ "query": path_or_query }))?;
    let first_hit = search_result.lines().next().unwrap_or("").trim().to_string();
    if first_hit.is_empty() || first_hit == "No files found." {
        return Ok(None);
    }
    Ok(Some(first_hit))
}

fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_default();
        return format!("{home}/{rest}");
    }
    if path == "~" {
        return std::env::var("HOME").unwrap_or_default();
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real Qwen model, real inference — downloads the ~1.9GB GGUF on first
    /// run. Reproduces a real live bug: the user had 5 reminders all titled
    /// "sleep" (one per day), and "check off the sleep reminder on 8th
    /// august" needs the "due" field (added to prompts/complete.rs) so
    /// system::complete_reminder can disambiguate by date instead of
    /// checking off an arbitrary one. Not run by default:
    /// `cargo test --lib -- --ignored complete_slot_filler_extracts_disambiguating_due_date`.
    #[test]
    #[ignore]
    fn complete_slot_filler_extracts_disambiguating_due_date() {
        let model_path = crate::model_download::default_model_path();
        crate::model_download::ensure_model(&model_path).expect("failed to download model");

        let backend = crate::llm::init_shared_backend().expect("failed to init llama backend");
        let engine = crate::llm::LlamaEngine::new(backend, &model_path).expect("failed to load model");

        let current_date = "now=2026-08-04T19:50; tomorrow=2026-08-05; year=2026";
        let system_prompt = reminder_prompts::complete::system_prompt(current_date);

        let res = engine
            .generate(
                crate::shared::PromptOrHistory::Prompt("check off the sleep reminder on 8th august"),
                "Router",
                Some(system_prompt),
                Some(128),
                current_date,
            )
            .expect("generation failed");

        println!("raw response: {res}");

        let cleaned = extract_json_object(&res).unwrap_or_else(|| res.trim().to_string());
        let parsed: Value =
            serde_json::from_str(&cleaned).unwrap_or_else(|e| panic!("invalid JSON: {e}\nraw: {res}"));

        assert_eq!(
            parsed["title"].as_str().unwrap_or("").to_lowercase(),
            "sleep",
            "expected title=sleep, got {parsed}"
        );
        let due = parsed["due"].as_str().unwrap_or("");
        assert!(
            due.starts_with("2026-08-08"),
            "expected due starting 2026-08-08, got {due:?} — raw: {res}"
        );
    }
}
