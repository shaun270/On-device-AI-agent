//! One-time model fetch — kept separate so `llm.rs` stays the working inference engine.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use futures::StreamExt;

/// Same local path your friend's build already uses.
const LOCAL_FILENAME: &str = "qwen2.5-3b-coder-q4_k_m.gguf";

/// Correct HuggingFace object name (word order differs from local filename).
const REMOTE_URL: &str =
    "https://huggingface.co/Qwen/Qwen2.5-Coder-3B-Instruct-GGUF/resolve/main/qwen2.5-coder-3b-instruct-q4_k_m.gguf";

pub fn default_model_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home)
        .join("Library/Application Support/com.hey-martha.dev/models")
        .join(LOCAL_FILENAME)
}

/// Download the GGUF if missing. No-op when the file already exists and looks complete.
pub fn ensure_model(model_path: &Path) -> Result<(), String> {
    if model_path.is_file() {
        let size = fs::metadata(model_path).map(|m| m.len()).unwrap_or(0);
        if size > 500_000_000 {
            return Ok(());
        }
        println!(
            "Model at {} looks incomplete ({} bytes); re-downloading…",
            model_path.display(),
            size
        );
        let _ = fs::remove_file(model_path);
    }

    if let Some(parent) = model_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create models directory: {e}"))?;
    }

    let tmp_path = model_path.with_extension("gguf.partial");
    println!("Downloading model from HuggingFace (~1.9GB)…");
    println!("  → {}", model_path.display());

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Failed to start download runtime: {e}"))?;

    rt.block_on(async {
        let client = reqwest::Client::builder()
            .user_agent("hey-martha/0.1")
            .build()
            .map_err(|e| format!("HTTP client error: {e}"))?;

        let response = client
            .get(REMOTE_URL)
            .send()
            .await
            .map_err(|e| format!("Download request failed: {e}"))?;

        if !response.status().is_success() {
            return Err(format!(
                "Download failed with HTTP {}: {}",
                response.status(),
                REMOTE_URL
            ));
        }

        let total = response.content_length();
        let mut stream = response.bytes_stream();
        let mut file = File::create(&tmp_path)
            .map_err(|e| format!("Failed to create temp model file: {e}"))?;
        let mut downloaded: u64 = 0;
        let mut last_pct: u64 = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Download stream error: {e}"))?;
            file.write_all(&chunk)
                .map_err(|e| format!("Failed writing model: {e}"))?;
            downloaded += chunk.len() as u64;

            if let Some(total) = total {
                let pct = downloaded * 100 / total;
                if pct >= last_pct + 5 {
                    println!("  Download progress: {pct}% ({downloaded}/{total} bytes)");
                    last_pct = pct;
                }
            }
        }

        file.sync_all()
            .map_err(|e| format!("Failed to flush model file: {e}"))?;
        drop(file);

        fs::rename(&tmp_path, model_path).map_err(|e| {
            let _ = fs::remove_file(&tmp_path);
            format!("Failed to finalize model file: {e}")
        })?;

        println!("Model download complete.");
        Ok(())
    })
}
