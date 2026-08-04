use serde_json::{json, Value};

use crate::shared::Tool;

/// Reads a file's text contents (txt/pdf/docx/xlsx and other plain text).
pub struct ReadFileTool;

impl Tool for ReadFileTool {
    fn name(&self) -> &'static str {
        "read_file"
    }

    fn description(&self) -> &'static str {
        "Read the contents of a file at an absolute path. Use after search_files finds the path."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute file path."
                }
            },
            "required": ["path"]
        })
    }

    fn execute(&self, input: Value) -> Result<String, String> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing required field: path".to_string())?;

        crate::tools::read_file(path)
    }
}
