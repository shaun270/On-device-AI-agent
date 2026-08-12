//! Per-action slot-filling prompts. Each one assumes the router already
//! decided *which* action this is — the LLM's only job here is extracting
//! arguments, not guessing intent (that's what `router.rs`'s older,
//! single-big-prompt classifier still does, kept around during migration).

pub mod complete;
pub mod list;
pub mod set;
pub mod set_many;

pub fn extract_json_object(raw: &str) -> Option<String> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(raw[start..=end].trim().to_string())
}
