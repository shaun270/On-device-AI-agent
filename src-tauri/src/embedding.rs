//! Sentence embeddings for the intent router — a separate, tiny model from
//! the chat/coder model in `llm.rs`. Kept isolated on purpose: the router
//! needs a fast embed() call, not a chat context, and this file has zero
//! knowledge of prompts, tools, or capabilities.

use std::path::Path;
use std::sync::Arc;

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};

pub struct EmbeddingEngine {
    backend: Arc<LlamaBackend>,
    model: LlamaModel,
}

impl EmbeddingEngine {
    /// `backend` must be the one shared `LlamaBackend` for the whole process —
    /// see `LlamaEngine::new`'s doc comment for why.
    pub fn new<P: AsRef<Path>>(backend: Arc<LlamaBackend>, model_path: P) -> Result<Self, String> {
        let model_params = LlamaModelParams::default().with_n_gpu_layers(100);
        let model = LlamaModel::load_from_file(&*backend, model_path, &model_params)
            .map_err(|e| format!("Failed to load embedding model: {e}"))?;

        Ok(Self { backend, model })
    }

    /// Embed one line of text, L2-normalized so cosine similarity is a plain dot product.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(std::num::NonZeroU32::new(512))
            .with_embeddings(true);

        let mut ctx = self
            .model
            .new_context(&self.backend, ctx_params)
            .map_err(|e| format!("Failed to create embedding context: {e}"))?;

        let tokens = self
            .model
            .str_to_token(text, AddBos::Always)
            .map_err(|e| format!("Failed to tokenize for embedding: {e}"))?;

        if tokens.is_empty() {
            return Err("cannot embed empty text".to_string());
        }

        let mut batch = LlamaBatch::new(tokens.len().max(512), 1);
        batch
            .add_sequence(&tokens, 0, false)
            .map_err(|e| format!("Failed to build embedding batch: {e}"))?;

        ctx.clear_kv_cache();
        ctx.decode(&mut batch)
            .map_err(|e| format!("Embedding decode failed: {e}"))?;

        let raw = ctx
            .embeddings_seq_ith(0)
            .map_err(|e| format!("Failed to read embedding: {e}"))?;

        Ok(normalize(raw))
    }
}

fn normalize(input: &[f32]) -> Vec<f32> {
    let magnitude = input.iter().fold(0.0_f32, |acc, &v| v.mul_add(v, acc)).sqrt();
    if magnitude == 0.0 {
        return input.to_vec();
    }
    input.iter().map(|&v| v / magnitude).collect()
}

#[cfg(test)]
mod tests {
    use super::{normalize, EmbeddingEngine};

    #[test]
    fn normalize_produces_unit_vector() {
        let v = normalize(&[3.0, 4.0]);
        let mag = (v[0] * v[0] + v[1] * v[1]).sqrt();
        assert!((mag - 1.0).abs() < 1e-6);
    }

    /// Real model, real inference — downloads the ~25MB GGUF on first run.
    /// Not run by default: `cargo test --lib -- --ignored embedding_end_to_end`.
    #[test]
    #[ignore]
    fn embedding_end_to_end_similarity_makes_sense() {
        let model_path = crate::model_download::default_embedding_model_path();
        crate::model_download::ensure_embedding_model(&model_path)
            .expect("failed to download embedding model");

        let backend = std::sync::Arc::new(
            llama_cpp_2::llama_backend::LlamaBackend::init().expect("failed to init llama backend"),
        );
        let engine = EmbeddingEngine::new(backend, &model_path).expect("failed to load embedding model");

        let a = engine.embed("remind me to call the dentist tomorrow").unwrap();
        let b = engine.embed("set a reminder to buy milk").unwrap();
        let c = engine.embed("what's the weather like in paris").unwrap();

        let dot = |x: &[f32], y: &[f32]| x.iter().zip(y).map(|(p, q)| p * q).sum::<f32>();

        let reminders_vs_reminders = dot(&a, &b);
        let reminders_vs_unrelated = dot(&a, &c);

        println!("reminders-vs-reminders: {reminders_vs_reminders}, reminders-vs-unrelated: {reminders_vs_unrelated}");

        assert!(
            reminders_vs_reminders > reminders_vs_unrelated,
            "expected two reminders phrases to be closer than a reminders phrase and an unrelated one"
        );
        assert!((a.iter().map(|v| v * v).sum::<f32>() - 1.0).abs() < 1e-3, "embedding should be L2-normalized");
    }
}
