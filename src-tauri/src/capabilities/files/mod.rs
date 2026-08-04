//! Files capability — search/read/write on the local filesystem.
//!
//! Layout mirrors `reminders/` (see that module's doc comment for the
//! full pattern). No `commands.rs` here: unlike reminders, files has no
//! deterministic/slash-command fast path from the frontend — the only
//! caller is `commands/route.rs`'s files handler, which talks to
//! `capabilities::dispatch` directly.

mod read_file;
mod search_files;
mod write_file;
pub mod exemplars;
pub mod prompts;

use crate::shared::Tool;
use read_file::ReadFileTool;
use search_files::SearchFilesTool;
use write_file::WriteFileTool;

/// Tools this capability contributes to the agent.
pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(SearchFilesTool) as Box<dyn Tool>,
        Box::new(ReadFileTool),
        Box::new(WriteFileTool),
    ]
}
