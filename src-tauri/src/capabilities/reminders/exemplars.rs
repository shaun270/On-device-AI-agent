//! Loads this capability's embedding-router exemplars (domain + action
//! phrases) from the bundled `exemplars.toml` alongside this file.

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
        toml::from_str(RAW).map_err(|e| format!("failed to parse reminders exemplars.toml: {e}"))?;

    let domain_exemplars = parsed
        .domain_exemplar
        .into_iter()
        .map(|row| Exemplar { text: row.text, label: "reminders".to_string() })
        .collect();

    let action_exemplars = parsed
        .action_exemplar
        .into_iter()
        .map(|row| Exemplar { text: row.text, label: row.action })
        .collect();

    Ok(DomainExemplars { domain: "reminders".to_string(), domain_exemplars, action_exemplars })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bundled_toml_with_expected_shape() {
        let d = load().expect("exemplars.toml should parse");
        assert_eq!(d.domain, "reminders");
        assert!(d.domain_exemplars.len() >= 10);
        assert!(d.action_exemplars.iter().any(|e| e.label == "set"));
        assert!(d.action_exemplars.iter().any(|e| e.label == "set_many"));
        assert!(d.action_exemplars.iter().any(|e| e.label == "list"));
        assert!(d.action_exemplars.iter().any(|e| e.label == "complete"));
        assert!(d.action_exemplars.iter().any(|e| e.label == "clarify"));
    }
}
