use crate::shared::llm_access;
use crate::shared::state::AppState;

#[tauri::command]
pub async fn generate_response(
    message: String,
    agent_name: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    match llm_access::generate(&state, &message, &agent_name, None, None).await {
        Ok(res) => Ok(res),
        Err(e) => {
            println!("LLM Generation Error: {e}");
            Err(e)
        }
    }
}
