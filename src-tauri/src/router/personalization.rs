//! User-specific router corrections, layered on top of the shipped
//! `capabilities/*/exemplars.toml` files. Captured via the feedback UI
//! (see `commands/route.rs`'s `submit_router_correction`), stored
//! separately from source-controlled exemplars so the curated baseline
//! stays reviewable and per-user learning stays per-user.
//!
//! Same directory convention as `tools.rs`'s `memory.json`: `~/.martha/`.

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::router::build::DomainExemplars;
use crate::router::types::Exemplar;

/// Cap per (domain, action) bucket so a noisy run of corrections can't
/// drown out the curated baseline exemplars. Keeps the most recent ones.
const MAX_PER_BUCKET: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Correction {
    pub domain: String,
    pub action: String,
    pub text: String,
}

fn corrections_file_path() -> Result<PathBuf, String> {
    let home_dir = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| "Could not find home directory".to_string())?;

    let martha_dir = std::path::Path::new(&home_dir).join(".martha");
    if !martha_dir.exists() {
        fs::create_dir_all(&martha_dir)
            .map_err(|e| format!("Failed to create .martha dir: {e}"))?;
    }

    Ok(martha_dir.join("router_exemplars.jsonl"))
}

/// Load every stored correction, oldest first (append order).
pub fn load_corrections() -> Result<Vec<Correction>, String> {
    let path = corrections_file_path()?;
    if !path.is_file() {
        return Ok(Vec::new());
    }

    let file = File::open(&path).map_err(|e| format!("Failed to open corrections file: {e}"))?;
    let mut out = Vec::new();
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|e| format!("Failed to read corrections file: {e}"))?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<Correction>(line) {
            Ok(c) => out.push(c),
            Err(e) => println!("Skipping malformed router_exemplars.jsonl line: {e}"),
        }
    }
    Ok(out)
}

/// Append one correction. No-ops on an exact (domain, action, text) repeat
/// so the same feedback click twice doesn't inflate a bucket's weight.
pub fn append_correction(correction: &Correction) -> Result<(), String> {
    let existing = load_corrections()?;
    let duplicate = existing.iter().any(|c| {
        c.domain == correction.domain
            && c.action == correction.action
            && c.text.trim().eq_ignore_ascii_case(correction.text.trim())
    });
    if duplicate {
        return Ok(());
    }

    let path = corrections_file_path()?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("Failed to open corrections file: {e}"))?;

    let line = serde_json::to_string(correction)
        .map_err(|e| format!("Failed to serialize correction: {e}"))?;
    writeln!(file, "{line}").map_err(|e| format!("Failed to write correction: {e}"))?;
    Ok(())
}

/// Merge stored corrections into the shipped domain exemplars in place —
/// called right before `build_router()` so a learned exemplar and a
/// shipped one are indistinguishable to the router itself.
pub fn merge_corrections_into(domains: &mut [DomainExemplars]) {
    let corrections = match load_corrections() {
        Ok(c) => c,
        Err(e) => {
            println!("Failed to load personalized router exemplars: {e}");
            return;
        }
    };
    merge_corrections(domains, corrections);
}

/// Pure merge step, split out from `merge_corrections_into` so it's
/// testable without touching the real `~/.martha` directory.
fn merge_corrections(domains: &mut [DomainExemplars], corrections: Vec<Correction>) {
    if corrections.is_empty() {
        return;
    }

    let mut by_bucket: std::collections::HashMap<(String, String), Vec<String>> =
        std::collections::HashMap::new();
    for c in corrections {
        by_bucket.entry((c.domain, c.action)).or_default().push(c.text);
    }

    for d in domains.iter_mut() {
        for ((domain, action), texts) in &by_bucket {
            if domain != &d.domain {
                continue;
            }
            for text in texts.iter().rev().take(MAX_PER_BUCKET) {
                d.action_exemplars.push(Exemplar { text: text.clone(), label: action.clone() });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_adds_learned_exemplars_to_matching_domain_only() {
        let mut domains = vec![
            DomainExemplars {
                domain: "reminders".to_string(),
                domain_exemplars: vec![],
                action_exemplars: vec![Exemplar { text: "existing".to_string(), label: "set".to_string() }],
            },
            DomainExemplars {
                domain: "files".to_string(),
                domain_exemplars: vec![],
                action_exemplars: vec![],
            },
        ];

        let corrections = vec![Correction {
            domain: "reminders".to_string(),
            action: "complete".to_string(),
            text: "tick off the sleep reminder".to_string(),
        }];
        merge_corrections(&mut domains, corrections);

        assert_eq!(domains[0].action_exemplars.len(), 2);
        assert!(domains[0]
            .action_exemplars
            .iter()
            .any(|e| e.label == "complete" && e.text == "tick off the sleep reminder"));
        assert!(domains[1].action_exemplars.is_empty(), "correction should not leak into an unrelated domain");
    }

    #[test]
    fn merge_caps_each_bucket_to_the_most_recent_entries() {
        let mut domains = vec![DomainExemplars {
            domain: "reminders".to_string(),
            domain_exemplars: vec![],
            action_exemplars: vec![],
        }];

        let corrections: Vec<Correction> = (0..(MAX_PER_BUCKET + 10))
            .map(|i| Correction {
                domain: "reminders".to_string(),
                action: "complete".to_string(),
                text: format!("phrase {i}"),
            })
            .collect();
        merge_corrections(&mut domains, corrections);

        assert_eq!(domains[0].action_exemplars.len(), MAX_PER_BUCKET);
        // Most recently appended (highest index) survive the cap.
        let texts: Vec<&str> = domains[0].action_exemplars.iter().map(|e| e.text.as_str()).collect();
        assert!(texts.contains(&"phrase 59"));
        assert!(!texts.contains(&"phrase 0"));
    }
}
