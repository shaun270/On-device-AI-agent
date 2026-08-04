//! Tauri commands for the reminders capability.
//!
//! Register these from `lib.rs` — do not put reminder logic back in the shell.

use serde_json::json;

use crate::capabilities;
use crate::shared::llm_access;
use crate::shared::state::AppState;

use super::router;

/// Create a Reminders.app item.
#[tauri::command]
pub async fn set_reminder(
    title: String,
    time: Option<String>,
    due: Option<String>,
    notes: Option<String>,
    list_name: Option<String>,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let mut input = json!({ "title": title });
        if let Some(t) = due.or(time) {
            if !t.trim().is_empty() {
                input["due"] = json!(t);
            }
        }
        if let Some(n) = notes {
            if !n.trim().is_empty() {
                input["notes"] = json!(n);
            }
        }
        if let Some(l) = list_name {
            if !l.trim().is_empty() {
                input["list_name"] = json!(l);
            }
        }
        capabilities::dispatch("set_reminder", input)
    })
    .await
    .map_err(|e| format!("task failed: {e}"))?
}

/// List reminders. Pass any subset of the tool fields as JSON-friendly args.
#[tauri::command]
pub async fn list_reminders(
    range: Option<String>,
    days_ahead: Option<u32>,
    start: Option<String>,
    end: Option<String>,
    include_undated: Option<bool>,
    include_overdue: Option<bool>,
    include_completed: Option<bool>,
    search: Option<String>,
    list_name: Option<String>,
    limit: Option<u32>,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let mut input = json!({});
        if let Some(r) = range {
            input["range"] = json!(r);
        }
        if let Some(n) = days_ahead {
            input["days_ahead"] = json!(n);
        }
        if let Some(s) = start {
            input["start"] = json!(s);
        }
        if let Some(e) = end {
            input["end"] = json!(e);
        }
        if let Some(v) = include_undated {
            input["include_undated"] = json!(v);
        }
        if let Some(v) = include_overdue {
            input["include_overdue"] = json!(v);
        }
        if let Some(v) = include_completed {
            input["include_completed"] = json!(v);
        }
        if let Some(s) = search {
            input["search"] = json!(s);
        }
        if let Some(l) = list_name {
            input["list_name"] = json!(l);
        }
        if let Some(n) = limit {
            input["limit"] = json!(n);
        }
        capabilities::dispatch("list_reminders", input)
    })
    .await
    .map_err(|e| format!("task failed: {e}"))?
}

/// Check off a reminder.
#[tauri::command]
pub async fn complete_reminder(
    title: String,
    match_mode: Option<String>,
    list_name: Option<String>,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let mut input = json!({ "title": title });
        if let Some(m) = match_mode {
            input["match"] = json!(m);
        }
        if let Some(l) = list_name {
            input["list_name"] = json!(l);
        }
        capabilities::dispatch("complete_reminder", input)
    })
    .await
    .map_err(|e| format!("task failed: {e}"))?
}

/// Open System Settings → Privacy → Reminders (EventKit grant).
#[tauri::command]
pub fn open_reminders_settings() -> Result<String, String> {
    super::open_privacy_settings()
}

/// LLM JSON router for natural-language reminder intents.
#[tauri::command]
pub async fn classify_intent(
    text: String,
    current_date: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let system_prompt = router::system_prompt(&current_date);
    let res = llm_access::generate(&state, crate::shared::PromptOrHistory::Prompt(&text), "Router", Some(system_prompt), Some(128), &current_date).await?;
    let cleaned = router::extract_json_object(&res).unwrap_or_else(|| {
        res.replace("```json", "").replace("```", "").trim().to_string()
    });
    Ok(cleaned)
}
