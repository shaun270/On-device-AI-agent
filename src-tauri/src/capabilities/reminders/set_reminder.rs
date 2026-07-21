use serde_json::{json, Value};

use super::system;
use crate::shared::Tool;

/// Creates an item in macOS Reminders.app.
pub struct SetReminderTool;

impl Tool for SetReminderTool {
    fn name(&self) -> &'static str {
        "set_reminder"
    }

    fn description(&self) -> &'static str {
        "Create a to-do item in the macOS Reminders app. Use for things the user wants to remember later (errands, tasks, follow-ups). Do not use for alarms/clock timers, calendar events, or opening apps. Convert relative times the user says (tomorrow, Friday 3pm) into ISO `due` yourself — this tool only accepts structured fields."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "Short reminder text, e.g. \"Call dentist\" or \"Buy milk\"."
                },
                "due": {
                    "type": "string",
                    "description": "Optional due date/time as YYYY-MM-DD or YYYY-MM-DDTHH:MM (local). Prefer this over `time`."
                },
                "time": {
                    "type": "string",
                    "description": "Alias for `due` (same ISO formats)."
                },
                "notes": {
                    "type": "string",
                    "description": "Optional longer body/notes on the reminder."
                },
                "list_name": {
                    "type": "string",
                    "description": "Optional Reminders list name (e.g. \"Work\", \"Personal\"). Defaults to the default list."
                }
            },
            "required": ["title"]
        })
    }

    fn execute(&self, input: Value) -> Result<String, String> {
        let title = input
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing required field: title".to_string())?;

        let due = input
            .get("due")
            .or_else(|| input.get("time"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());

        let notes = input
            .get("notes")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());

        let list_name = input
            .get("list_name")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());

        system::create_reminder(title, due, notes, list_name)
    }
}
