//! Plain data types for the router — no Tauri, no LLM prompt knowledge.
//! Kept dependency-free on purpose so this module stays portable if the
//! core engine is ever split out of the Tauri app.

/// One labeled example sentence used to seed a nearest-neighbor pool.
/// A domain or action pool is many of these, not one vector per category.
#[derive(Debug, Clone)]
pub struct Exemplar {
    pub text: String,
    pub label: String,
}
