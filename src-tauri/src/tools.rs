use std::process::Command;
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// Search for files matching a query.
pub fn search_files(query: &str, directory: Option<&str>) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        // Spotlight search (mdfind)
        let mut cmd = Command::new("mdfind");
        cmd.arg("-name").arg(query);
        
        if let Some(dir) = directory {
            // Expand ~ to HOME if present
            let expanded_dir = if dir.starts_with("~/") {
                let home = std::env::var("HOME").unwrap_or_default();
                dir.replacen("~", &home, 1)
            } else if dir == "~" {
                std::env::var("HOME").unwrap_or_default()
            } else {
                dir.to_string()
            };
            
            // Only use -onlyin if the path actually exists to prevent hallucinated directory constraints
            if Path::new(&expanded_dir).exists() {
                cmd.arg("-onlyin").arg(expanded_dir);
            }
        }
        
        let output = cmd.output()
            .map_err(|e| format!("Failed to execute mdfind: {}", e))?;
            
        let result_str = String::from_utf8_lossy(&output.stdout).to_string();
        
        let lines: Vec<&str> = result_str.lines().take(10).collect(); // Limit to top 10 results
        if lines.is_empty() {
            Ok("No files found.".to_string())
        } else {
            Ok(lines.join("\n"))
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        // PowerShell search in user profile
        let script = format!("Get-ChildItem -Path $env:USERPROFILE -Recurse -Filter '*{}*' -ErrorAction SilentlyContinue | Select-Object -First 10 -ExpandProperty FullName", query);
        let output = Command::new("powershell")
            .args(&["-Command", &script])
            .output()
            .map_err(|e| format!("Failed to execute powershell: {}", e))?;
            
        let result_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if result_str.is_empty() {
            Ok("No files found.".to_string())
        } else {
            Ok(result_str)
        }
    }
    
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("Search not supported on this OS".to_string())
    }
}

/// Read the contents of a text or document file.
pub fn read_file(path: &str) -> Result<String, String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(format!("File does not exist: {}", path));
    }
    
    let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    
    let content = match ext.as_str() {
        "pdf" => {
            pdf_extract::extract_text(p).map_err(|e| format!("Failed to parse PDF: {}", e))?
        },
        "docx" => {
            use std::io::Read;
            use dotext::MsDoc;
            let mut file = dotext::Docx::open(p).map_err(|e| format!("Failed to open DOCX: {:?}", e))?;
            let mut text = String::new();
            file.read_to_string(&mut text).map_err(|e| format!("Failed to read DOCX: {}", e))?;
            text
        },
        "xlsx" => {
            use calamine::{Reader, open_workbook, Xlsx, Data};
            let mut excel: Xlsx<_> = open_workbook(p).map_err(|e| format!("Failed to open XLSX: {:?}", e))?;
            let mut text = String::new();
            for sheet_name in excel.sheet_names().to_owned() {
                if let Ok(range) = excel.worksheet_range(&sheet_name) {
                    for row in range.rows() {
                        let row_str: Vec<String> = row.iter().map(|c| match c {
                            Data::String(s) => s.to_string(),
                            Data::Float(f) => f.to_string(),
                            Data::Int(i) => i.to_string(),
                            Data::Bool(b) => b.to_string(),
                            Data::DateTime(d) => d.to_string(),
                            _ => String::new(),
                        }).collect();
                        text.push_str(&row_str.join("\t"));
                        text.push('\n');
                    }
                }
            }
            text
        },
        _ => {
            fs::read_to_string(p).map_err(|e| format!("Failed to read file (is it binary?): {}", e))?
        }
    };
    
    // Truncate to 2000 characters to avoid blowing up context window
    if content.chars().count() > 2000 {
        let truncated: String = content.chars().take(2000).collect();
        Ok(format!("{}...\n[File truncated due to length]", truncated))
    } else {
        Ok(content)
    }
}

/// Write contents to a file.
pub fn write_file(path: &str, content: &str) -> Result<String, String> {
    let p = Path::new(path);
    
    if let Some(parent) = p.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create parent directories: {}", e))?;
        }
    }
    
    fs::write(p, content).map_err(|e| format!("Failed to write file: {}", e))?;
    
    Ok(format!("Successfully wrote to {}", path))
}

#[derive(Serialize, Deserialize, Default)]
struct MemoryStore {
    facts: Vec<String>,
}

fn get_memory_file_path() -> Result<PathBuf, String> {
    let home_dir = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| "Could not find home directory".to_string())?;
    
    let martha_dir = Path::new(&home_dir).join(".martha");
    if !martha_dir.exists() {
        fs::create_dir_all(&martha_dir).map_err(|e| format!("Failed to create .martha dir: {}", e))?;
    }
    
    Ok(martha_dir.join("memory.json"))
}

/// Read all saved memories.
pub fn read_memory() -> Result<String, String> {
    let path = get_memory_file_path()?;
    if !path.exists() {
        return Ok("No memories saved yet.".to_string());
    }
    
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read memory: {}", e))?;
    let store: MemoryStore = serde_json::from_str(&content).unwrap_or_default();
    
    if store.facts.is_empty() {
        return Ok("No memories saved yet.".to_string());
    }
    
    let mut result = String::from("Here are the facts you must remember about the user:\n");
    for (i, fact) in store.facts.iter().enumerate() {
        result.push_str(&format!("- {}\n", fact));
    }
    Ok(result)
}

/// Save a new fact to memory.
pub fn save_memory(fact: &str) -> Result<String, String> {
    let path = get_memory_file_path()?;
    
    let mut store = if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        MemoryStore::default()
    };
    
    if store.facts.contains(&fact.to_string()) {
        return Ok("I already know this fact.".to_string());
    }
    
    store.facts.push(fact.to_string());
    
    let json = serde_json::to_string_pretty(&store).map_err(|e| format!("Failed to serialize memory: {}", e))?;
    fs::write(path, json).map_err(|e| format!("Failed to write memory: {}", e))?;
    
    Ok(format!("Successfully saved fact to long-term memory: {}", fact))
}
