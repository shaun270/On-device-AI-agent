//! Per-action slot-filling prompts for the files domain. Each assumes the
//! router already decided which action this is — the LLM's only job is
//! extracting arguments, not deciding intent.

pub mod read;
pub mod search;
pub mod write;

pub fn extract_json_object(raw: &str) -> Option<String> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(raw[start..=end].trim().to_string())
}
