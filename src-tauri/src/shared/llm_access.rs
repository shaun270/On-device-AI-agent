//! Shared helpers for calling the local LLM from any capability/command.
//!
//! Features should use this instead of locking `AppState` themselves.

use crate::shared::state::AppState;

/// Wait up to ~15s for the model thread to finish loading.
pub async fn wait_until_ready(state: &tauri::State<'_, AppState>) -> Result<(), String> {
    for _ in 0..30 {
        let is_loaded = {
            let engine = state.llm.lock().map_err(|_| "Failed to lock engine".to_string())?;
            engine.is_some()
        };
        if is_loaded {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    Ok(())
}

/// Generate with the loaded engine. `custom_system` / `max_tokens` are optional.
pub async fn generate(
    state: &tauri::State<'_, AppState>,
    prompt: &str,
    agent_name: &str,
    custom_system: Option<String>,
    max_tokens: Option<usize>,
) -> Result<String, String> {
    wait_until_ready(state).await?;

    let engine = state.llm.lock().map_err(|_| "Failed to lock engine".to_string())?;
    if let Some(llm) = &*engine {
        llm.generate(prompt, agent_name, custom_system, max_tokens)
    } else {
        Err("LLM failed to initialize. Please check the terminal logs.".to_string())
    }
}
