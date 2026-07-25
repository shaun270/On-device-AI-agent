use std::path::Path;

#[tauri::command]
pub fn open_file(path: String) -> Result<(), String> {
    if !Path::new(&path).exists() {
        return Err(format!("File does not exist: {}", path));
    }
    
    if let Err(e) = open::that(&path) {
        println!("Failed to open file {}: {}", path, e);
        return Err(e.to_string());
    }
    
    Ok(())
}
