// Martha — Rust backend
//
// echo_message: scaffold for the AI call (replace body when LLM is wired in).
// Global hotkey Alt+M: registered at OS level, works even when minimized.

use tauri::Manager;

/// Scaffold command — echoes message back with a short delay.
/// Replace the body of this function with a real LLM call later.
#[tauri::command]
async fn echo_message(message: String) -> String {
    tokio::time::sleep(std::time::Duration::from_millis(700)).await;
    format!("Echo: {}", message)
}

/// Fully quits the app. Called from the Settings panel "Quit Martha" button.
#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![echo_message, quit_app])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
