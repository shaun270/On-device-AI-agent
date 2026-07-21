//! Shared app state — owned by the shell (`lib.rs`), used by every command.

use std::sync::Mutex;

use crate::llm::LlamaEngine;

pub struct AppState {
    pub llm: Mutex<Option<LlamaEngine>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            llm: Mutex::new(None),
        }
    }
}
