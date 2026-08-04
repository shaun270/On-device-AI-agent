use serde_json::{json, Value};

use crate::shared::Tool;

/// Searches the local filesystem by filename (Spotlight/mdfind on macOS).
pub struct SearchFilesTool;

impl Tool for SearchFilesTool {
    fn name(&self) -> &'static str {
        "search_files"
    }

    fn description(&self) -> &'static str {
        "Search the local filesystem for a file by name. Use this to locate a file before reading it — you almost never already know the absolute path."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Filename or search term, e.g. \"resume\" or \"budget.xlsx\"."
                },
                "directory": {
                    "type": "string",
                    "description": "Optional directory to scope the search to (supports ~)."
                }
            },
            "required": ["query"]
        })
    }

    fn execute(&self, input: Value) -> Result<String, String> {
        let query = input
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing required field: query".to_string())?;

        let directory = input
            .get("directory")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());

        crate::tools::search_files(query, directory)
    }
}
