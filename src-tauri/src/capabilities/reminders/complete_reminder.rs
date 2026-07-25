use serde_json::{json, Value};

use super::system;
use crate::shared::Tool;

/// Marks an incomplete reminder as completed.
pub struct CompleteReminderTool;

impl Tool for CompleteReminderTool {
    fn name(&self) -> &'static str {
        "complete_reminder"
    }

    fn description(&self) -> &'static str {
        "Check off / mark complete an existing incomplete reminder in macOS Reminders. Use when the user finished a task. Prefer exact title when known; use match=contains for partial phrases like \"the milk one\". Does not create or list reminders."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "Title (or phrase) of the incomplete reminder to complete."
                },
                "match": {
                    "type": "string",
                    "enum": ["exact", "contains"],
                    "description": "exact = full title must match; contains = title substring. Default exact."
                },
                "list_name": {
                    "type": "string",
                    "description": "Optional Reminders list to search within."
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
        let match_mode = input
            .get("match")
            .and_then(|v| v.as_str())
            .unwrap_or("exact");
        let list_name = input
            .get("list_name")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());
        system::complete_reminder(title, match_mode, list_name)
    }
}
