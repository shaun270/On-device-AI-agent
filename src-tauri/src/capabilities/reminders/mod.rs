//! Reminders capability — tools + EventKit + Tauri commands + intent router.
//!
//! Layout (copy this for new features):
//!   mod.rs              — exports tools() + re-exports commands
//!   commands.rs         — Tauri commands (register from lib.rs)
//!   router.rs           — feature-owned LLM prompt for intent JSON
//!   system.rs           — OS/EventKit only
//!   set_reminder.rs     — one tool file
//!   list_reminders.rs
//!   complete_reminder.rs

mod complete_reminder;
mod list_reminders;
mod set_reminder;
pub mod commands;
pub mod exemplars;
pub mod prompts;
pub mod router;
pub mod system;

use crate::shared::Tool;
use complete_reminder::CompleteReminderTool;
use list_reminders::ListRemindersTool;
use set_reminder::SetReminderTool;

/// Tools this capability contributes to the agent.
pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(SetReminderTool) as Box<dyn Tool>,
        Box::new(ListRemindersTool),
        Box::new(CompleteReminderTool),
    ]
}

/// Warm EventKit permission on the main thread (do not background this).
pub fn prewarm() {
    system::prewarm_eventkit();
}

/// Open System Settings → Privacy → Reminders.
pub fn open_privacy_settings() -> Result<String, String> {
    system::open_reminders_privacy_settings()
}
