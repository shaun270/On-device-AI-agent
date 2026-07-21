// Martha — Rust backend
//
// Reminder commands: temporary Tauri entries so tools can be tested before an LLM agent is wired.

mod capabilities;
mod shared;

use serde_json::json;

/// Create a Reminders.app item.
#[tauri::command]
async fn set_reminder(
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
async fn list_reminders(
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
async fn complete_reminder(
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

/// Fully quits the app. Called from the Settings panel "Quit Martha" button.
#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

/// Open System Settings → Privacy → Reminders (EventKit grant).
#[tauri::command]
fn open_reminders_settings() -> Result<String, String> {
    capabilities::reminders::open_privacy_settings()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|_app| {
            #[cfg(target_os = "macos")]
            capabilities::reminders::prewarm();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            set_reminder,
            list_reminders,
            complete_reminder,
            open_reminders_settings,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
