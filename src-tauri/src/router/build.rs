//! Assembles a `Router` from every capability's exemplar phrases by
//! embedding them once at startup (see `lib.rs`'s embed-model load thread).
//! Not cached to disk yet — a few dozen short exemplars embed in well
//! under a second; revisit if the exemplar count grows large enough for
//! that cost to matter.

use std::collections::HashMap;

use crate::embedding::EmbeddingEngine;
use crate::router::decide::Thresholds;
use crate::router::index::NearestNeighborIndex;
use crate::router::types::Exemplar;
use crate::router::Router;

/// One capability's contribution to the router: its domain-level phrases
/// (label = the domain name, e.g. "reminders") and its action-level
/// phrases (label = action name, e.g. "set"/"list"/"complete").
pub struct DomainExemplars {
    pub domain: String,
    pub domain_exemplars: Vec<Exemplar>,
    pub action_exemplars: Vec<Exemplar>,
}

pub fn build_router(
    embedder: &EmbeddingEngine,
    domains: Vec<DomainExemplars>,
) -> Result<Router, String> {
    let mut all_domain_exemplars = Vec::new();
    let mut action_indexes = HashMap::new();

    for d in domains {
        for ex in &d.domain_exemplars {
            all_domain_exemplars.push(Exemplar { text: ex.text.clone(), label: d.domain.clone() });
        }

        let mut vectors = Vec::with_capacity(d.action_exemplars.len());
        for ex in &d.action_exemplars {
            vectors.push(embedder.embed(&ex.text)?);
        }
        action_indexes.insert(
            d.domain.clone(),
            NearestNeighborIndex::new(d.action_exemplars, vectors),
        );
    }

    let mut domain_vectors = Vec::with_capacity(all_domain_exemplars.len());
    for ex in &all_domain_exemplars {
        domain_vectors.push(embedder.embed(&ex.text)?);
    }
    let domain_index = NearestNeighborIndex::new(all_domain_exemplars, domain_vectors);

    Ok(Router { domain_index, action_indexes, thresholds: Thresholds::default() })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real embedding model, real exemplars.toml — downloads the model on
    /// first run. Prints domain/action scores for a labeled probe set (held
    /// out from exemplars.toml, paraphrased rather than copied) so
    /// `Thresholds::default()` can be calibrated against real numbers
    /// instead of a guess. Not run by default:
    /// `cargo test --lib -- --ignored calibration_probe`.
    #[test]
    #[ignore]
    fn calibration_probe_prints_real_scores() {
        let model_path = crate::model_download::default_embedding_model_path();
        crate::model_download::ensure_embedding_model(&model_path)
            .expect("failed to download embedding model");
        let backend = std::sync::Arc::new(
            llama_cpp_2::llama_backend::LlamaBackend::init().expect("failed to init llama backend"),
        );
        let embedder = crate::embedding::EmbeddingEngine::new(backend, &model_path)
            .expect("failed to load embedding model");

        let domains = crate::capabilities::all_domain_exemplars().expect("failed to load exemplars");
        let router = build_router(&embedder, domains).expect("failed to build router");

        // (utterance, expected_domain, expected_action_or_none)
        let probes: &[(&str, &str, Option<&str>)] = &[
            ("hey can you remind me to pick up my prescription this evening", "reminders", Some("set")),
            ("I need you to set a reminder about texting sarah back", "reminders", Some("set")),
            ("please remind me daily for the next four days to floss", "reminders", Some("set_many")),
            ("add a recurring reminder for the next week to journal every morning", "reminders", Some("set_many")),
            ("what's on my to-do list for tomorrow", "reminders", Some("list")),
            ("anything due this week", "reminders", Some("list")),
            ("I finished calling the plumber, mark it done", "reminders", Some("complete")),
            ("tick off the grocery run reminder", "reminders", Some("complete")),
            ("get rid of my old reminders please", "reminders", Some("clarify")),
            ("wipe all my reminders", "reminders", Some("clarify")),
            ("can you open my resume file", "files", Some("read")),
            ("where's that budget spreadsheet from last month", "files", Some("search")),
            ("what does my cover letter say", "files", Some("read")),
            ("save this paragraph to a file called draft.txt", "files", Some("write")),
            ("dump this text into a new file on my desktop", "files", Some("write")),
            ("what's the capital of france", "general", None),
            ("tell me a joke", "general", None),
            ("play some music", "general", None),
            ("how's the weather today", "general", None),
            // Known gap: there is no "list every file in a directory" tool —
            // search_files only does name-matching. See what actually happens.
            ("tell me all the files present in my downloads", "files?", None),
            // Real user session, investigating whether "checking off" ever
            // actually reached the reminders complete action.
            ("check off all the reminders i have", "reminders?", None),
            ("check off call my mom reminder", "reminders", Some("complete")),
            ("didnt you check off that?", "general?", None),
            ("check that off", "general?", None),
            ("check that reminder, call my mom off", "reminders?", None),
            ("nooo i mean check that reminder , call me mom off", "reminders?", None),
            (
                "ok now make 2 reminders that i have to play football, 1 for tomorrow 9pm, the otehr for dayafter 3 pm",
                "reminders",
                Some("set_many"),
            ),
            // Real user session: "check off the sleep reminder on 8th august"
            // got routed to list, and "check off the sleep reminder bro" got
            // routed to clarify (the delete-not-supported message) — both
            // should be "complete".
            ("check off the sleep reminder on 8th august", "reminders", Some("complete")),
            ("check off the sleep reminder bro", "reminders", Some("complete")),
            ("check off the sleep reminder", "reminders", Some("complete")),
        ];

        println!("\n{:<55} {:<10} {:>8} {:>8}  {}", "utterance", "expected", "score", "margin", "top_label");
        for (text, expected_domain, expected_action) in probes {
            let query = embedder.embed(text).unwrap();
            let domain_top = router.domain_index.top_match(&query).unwrap();
            println!(
                "{:<55} {:<10} {:>8.4} {:>8.4}  {}",
                text, expected_domain, domain_top.score, domain_top.margin, domain_top.label
            );

            if let Some(expected_action) = expected_action {
                if let Some(action_index) = router.action_indexes.get(&domain_top.label) {
                    let action_top = action_index.top_match(&query).unwrap();
                    println!(
                        "  └─ action: expected={:<10} got={:<10} score={:.4} margin={:.4}",
                        expected_action, action_top.label, action_top.score, action_top.margin
                    );
                }
            }

            let result = router.route(text, &embedder).unwrap();
            println!("  └─ route(): {result:?}");
        }
    }
}
