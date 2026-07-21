//! Shared types used by every capability (Tool trait, AppState, LLM helpers).

pub mod llm_access;
pub mod state;
pub mod tool;

pub use state::AppState;
pub use tool::Tool;
