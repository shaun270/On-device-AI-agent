use serde_json::{json, Value};

use crate::shared::Tool;

/// Writes text to a file. **No approval gating happens here** — the `Tool`
/// trait is synchronous and approval is an async UI round-trip, so callers
/// MUST get user approval before ever calling `capabilities::dispatch`
/// with this tool's name. See `commands/route.rs`'s files handler and
/// `commands/chat.rs`'s `write_file` branch, which are the only two
/// approved call sites.
pub struct WriteFileTool;

impl Tool for WriteFileTool {
    fn name(&self) -> &'static str {
        "write_file"
    }

    fn description(&self) -> &'static str {
        "Write text to a file at an absolute path, creating parent directories if needed. Requires prior user approval — never call this without it."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute destination file path."
                },
                "content": {
                    "type": "string",
                    "description": "Text to write."
                }
            },
            "required": ["path", "content"]
        })
    }

    fn execute(&self, input: Value) -> Result<String, String> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing required field: path".to_string())?;
        let content = input
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing required field: content".to_string())?;

        crate::tools::write_file(path, content)
    }
}
