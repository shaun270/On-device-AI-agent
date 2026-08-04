//! Hierarchical embedding router — root-level domain match, then a
//! leaf-level action match scoped to that domain. Plain Rust, no `tauri::`
//! imports anywhere in this module tree; the Tauri command boundary lives
//! in `capabilities/*/commands.rs`, which calls into this.
//!
//! This is the cheap first stage of a cascade: the LLM only runs after
//! `route()` returns, either to fill slots for a `Matched` action or to
//! handle `Fallback`/`AmbiguousAction` (low confidence, or a correction
//! that needs recent chat history to resolve).

pub mod build;
pub mod decide;
pub mod index;
pub mod types;

use std::collections::HashMap;

use crate::embedding::EmbeddingEngine;
use decide::{decide, Decision, Thresholds};
use index::NearestNeighborIndex;

pub struct Router {
    pub domain_index: NearestNeighborIndex,
    /// Keyed by domain label (e.g. "reminders") — each domain owns its own action pool.
    pub action_indexes: HashMap<String, NearestNeighborIndex>,
    pub thresholds: Thresholds,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RouteResult {
    /// Confidently resolved to domain + action — proceed to that action's slot-filling step.
    Matched { domain: String, action: String },
    /// Domain matched, but which action within it was unclear — ask, or escalate to the LLM with domain context.
    AmbiguousAction { domain: String },
    /// Didn't confidently match any domain — hand off to the general chat/tool agent.
    Fallback,
}

impl Router {
    pub fn route(&self, text: &str, embedder: &EmbeddingEngine) -> Result<RouteResult, String> {
        let query = embedder.embed(text)?;

        let domain = match decide(self.domain_index.top_match(&query), &self.thresholds) {
            Decision::Matched(label) => label,
            Decision::Ambiguous | Decision::NoMatch => return Ok(RouteResult::Fallback),
        };

        let Some(action_index) = self.action_indexes.get(&domain) else {
            return Ok(RouteResult::Fallback);
        };

        match decide(action_index.top_match(&query), &self.thresholds) {
            Decision::Matched(action) => Ok(RouteResult::Matched { domain, action }),
            Decision::Ambiguous | Decision::NoMatch => Ok(RouteResult::AmbiguousAction { domain }),
        }
    }
}
