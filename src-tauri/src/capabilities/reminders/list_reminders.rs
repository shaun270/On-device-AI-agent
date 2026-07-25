use serde_json::{json, Value};

use super::system;
use crate::shared::Tool;

/// Lists reminders with a flexible filter the LLM fills from natural language.
pub struct ListRemindersTool;

impl Tool for ListRemindersTool {
    fn name(&self) -> &'static str {
        "list_reminders"
    }

    fn description(&self) -> &'static str {
        "List reminders from macOS Reminders. Fill structured fields from what the user meant — e.g. \"next 3 days\" → days_ahead=3; \"today\" → range=today; \"between Monday and Friday\" → start/end ISO dates; \"anything about dentist\" → search=dentist; \"including stuff with no due date\" → include_undated=true. You resolve relative language into these fields; do not invent unsupported keys."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "range": {
                    "type": "string",
                    "enum": ["all", "today", "week"],
                    "description": "Convenience preset when the user says today / this week / everything. Ignored if days_ahead or start/end are set."
                },
                "days_ahead": {
                    "type": "integer",
                    "description": "Include reminders due from local today through the next N days (e.g. 3 for \"next 3 days\"). Inclusive of today; end is exclusive after N days."
                },
                "start": {
                    "type": "string",
                    "description": "Inclusive window start as YYYY-MM-DD (local)."
                },
                "end": {
                    "type": "string",
                    "description": "Inclusive window end as YYYY-MM-DD (local)."
                },
                "include_undated": {
                    "type": "boolean",
                    "description": "Include reminders with no due date. Defaults to true for range=all, false when a date window is set."
                },
                "include_overdue": {
                    "type": "boolean",
                    "description": "When a date window is set, also include past-due incomplete items. Defaults to true for windowed queries."
                },
                "include_completed": {
                    "type": "boolean",
                    "description": "Include already-completed reminders. Default false."
                },
                "search": {
                    "type": "string",
                    "description": "Case-insensitive substring filter on the reminder title."
                },
                "list_name": {
                    "type": "string",
                    "description": "Only reminders in this Reminders list (e.g. \"Work\")."
                },
                "limit": {
                    "type": "integer",
                    "description": "Max items to return (default 25, max 200)."
                }
            }
        })
    }

    fn execute(&self, input: Value) -> Result<String, String> {
        let query = system::list_query_from_json(&input)?;
        system::list_reminders(&query)
    }
}
