//! Product capabilities — one folder per feature.
//!
//! To add a new feature (alarms, music, files, …):
//! 1. Create `capabilities/<name>/` with the same shape as `reminders/`
//!    (tools + `commands.rs` + `exemplars.toml`/`exemplars.rs` +
//!    `prompts/` + optional `router.rs` + `system.rs`)
//! 2. Add `pub mod <name>;` here
//! 3. Append that module's tools in `all_tools()`
//! 4. Append that module's exemplars in `all_domain_exemplars()`
//! 5. Register that module's Tauri commands in `lib.rs` (one line each)
//!
//! Do not put feature prompts or OS logic in `lib.rs` or `llm.rs`.

pub mod files;
pub mod reminders;

use crate::router::build::DomainExemplars;
use crate::shared::Tool;

/// Every tool across all capabilities (agent registers this list with Claude).
pub fn all_tools() -> Vec<Box<dyn Tool>> {
    let mut tools = Vec::new();
    tools.extend(reminders::tools());
    tools.extend(files::tools());
    // tools.extend(alarms::tools());
    // tools.extend(music::tools());
    tools
}

/// Every capability's router exemplars, fed into `router::build::build_router`.
pub fn all_domain_exemplars() -> Result<Vec<DomainExemplars>, String> {
    Ok(vec![
        reminders::exemplars::load()?,
        files::exemplars::load()?,
        // alarms::exemplars::load()?,
        // music::exemplars::load()?,
    ])
}

/// Anthropic-style tool definitions for the agent core.
#[allow(dead_code)]
pub fn tool_definitions() -> Vec<serde_json::Value> {
    all_tools()
        .into_iter()
        .map(|tool| {
            serde_json::json!({
                "name": tool.name(),
                "description": tool.description(),
                "input_schema": tool.input_schema(),
            })
        })
        .collect()
}

/// Look up a tool by name and run it.
pub fn dispatch(name: &str, input: serde_json::Value) -> Result<String, String> {
    for tool in all_tools() {
        if tool.name() == name {
            return tool.execute(input);
        }
    }
    Err(format!("unknown tool: {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_reminder_tools() {
        let defs = tool_definitions();
        let names: Vec<&str> = defs.iter().filter_map(|d| d["name"].as_str()).collect();
        assert!(names.contains(&"set_reminder"));
        assert!(names.contains(&"list_reminders"));
        assert!(names.contains(&"complete_reminder"));
    }

    #[test]
    fn dispatch_rejects_empty_title() {
        let err = dispatch("set_reminder", serde_json::json!({ "title": "  " })).unwrap_err();
        assert!(err.contains("empty"));
    }
}
