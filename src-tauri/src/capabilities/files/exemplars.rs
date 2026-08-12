//! Loads this capability's embedding-router exemplars (domain + action
//! phrases) from the bundled `exemplars.toml` alongside this file. Same
//! shape as `capabilities/reminders/exemplars.rs`.

use serde::Deserialize;

use crate::router::build::DomainExemplars;
use crate::router::types::Exemplar;

const RAW: &str = include_str!("exemplars.toml");

#[derive(Deserialize)]
struct ExemplarFile {
    #[serde(default)]
    domain_exemplar: Vec<DomainRow>,
    #[serde(default)]
    action_exemplar: Vec<ActionRow>,
}

#[derive(Deserialize)]
struct DomainRow {
    text: String,
}

#[derive(Deserialize)]
struct ActionRow {
    action: String,
    text: String,
}

pub fn load() -> Result<DomainExemplars, String> {
    let parsed: ExemplarFile =
        toml::from_str(RAW).map_err(|e| format!("failed to parse files exemplars.toml: {e}"))?;

    let domain_exemplars = parsed
        .domain_exemplar
        .into_iter()
        .map(|row| Exemplar { text: row.text, label: "files".to_string() })
        .collect();

    let action_exemplars = parsed
        .action_exemplar
        .into_iter()
        .map(|row| Exemplar { text: row.text, label: row.action })
        .collect();

    Ok(DomainExemplars { domain: "files".to_string(), domain_exemplars, action_exemplars })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bundled_toml_with_expected_shape() {
        let d = load().expect("exemplars.toml should parse");
        assert_eq!(d.domain, "files");
        assert!(d.domain_exemplars.len() >= 8);
        assert!(d.action_exemplars.iter().any(|e| e.label == "search"));
        assert!(d.action_exemplars.iter().any(|e| e.label == "read"));
        assert!(d.action_exemplars.iter().any(|e| e.label == "write"));
    }
}
