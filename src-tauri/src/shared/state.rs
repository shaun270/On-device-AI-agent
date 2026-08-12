//! Shared app state — owned by the shell (`lib.rs`), used by every command.

use std::sync::Mutex;

use crate::embedding::EmbeddingEngine;
use crate::llm::LlamaEngine;
use crate::router::Router;

pub struct AppState {
    pub llm: Mutex<Option<LlamaEngine>>,
    pub embedder: Mutex<Option<EmbeddingEngine>>,
    pub router: Mutex<Option<Router>>,
    pub write_approval_tx: tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<bool>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            llm: Mutex::new(None),
            embedder: Mutex::new(None),
            router: Mutex::new(None),
            write_approval_tx: tokio::sync::Mutex::new(None),
        }
    }
}
