use std::path::Path;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::token::data_array::LlamaTokenDataArray;

pub struct LlamaEngine {
    backend: LlamaBackend,
    model: LlamaModel,
}

impl LlamaEngine {
    pub fn new<P: AsRef<Path>>(model_path: P) -> Result<Self, String> {
        let backend = LlamaBackend::init().map_err(|e| format!("Failed to init llama backend: {}", e))?;
        
        let model_params = LlamaModelParams::default().with_n_gpu_layers(100);
        
        let model = LlamaModel::load_from_file(&backend, model_path, &model_params)
            .map_err(|e| format!("Failed to load model: {}", e))?;
            
        Ok(Self { backend, model })
    }

    pub fn generate(
        &self,
        prompt: &str,
        agent_name: &str,
        custom_system: Option<String>,
        max_tokens: Option<usize>,
    ) -> Result<String, String> {
        // Simple synchronous generation for now (we can make it async/streaming later)
        let ctx_params = LlamaContextParams::default().with_n_ctx(std::num::NonZeroU32::new(4096));
        
        let mut ctx = self.model.new_context(&self.backend, ctx_params)
            .map_err(|e| format!("Failed to create context: {}", e))?;
            
        let system_msg = custom_system.unwrap_or_else(|| {
            format!("Your name is {agent_name}. You are a fast on-device macOS assistant. Answer concisely. Reminder create/list/complete requests are handled by the app outside this chat model — if the user asks about reminders in a vague way, suggest concrete wording like \"check my reminders\", \"reminders for tomorrow\", or \"check off Buy milk\". Never claim you lack access to their reminders or that they must open another app.")
        });

        // Wrap the prompt in Qwen's ChatML format
        let formatted_prompt = format!(
            "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
            system_msg,
            prompt
        );
            
        // Tokenize prompt
        let tokens = self.model.str_to_token(&formatted_prompt, llama_cpp_2::model::AddBos::Always)
            .map_err(|e| format!("Failed to tokenize: {}", e))?;
            
        let mut batch = LlamaBatch::new(1024, 1);
        let last_index = tokens.len() - 1;
        
        for (i, &token) in tokens.iter().enumerate() {
            let is_last = i == last_index;
            batch.add(token, i as i32, &[0], is_last)
                .map_err(|e| format!("Batch error: {}", e))?;
        }
        
        ctx.decode(&mut batch).map_err(|e| format!("Decode error: {}", e))?;
        
        let mut response = String::new();
        let mut n_cur = batch.n_tokens();
        let limit = max_tokens.unwrap_or(512).clamp(16, 512);
        
        for _ in 0..limit {
            let mut candidates = ctx.candidates_ith(batch.n_tokens() - 1);
            let mut candidates_p = LlamaTokenDataArray::from_iter(candidates, false);
            
            // Greedily pick the best token
            let new_token_id = candidates_p.sample_token_greedy();
            
            // Check for EOS
            if new_token_id == self.model.token_eos() {
                break;
            }
            
            let mut decoder = encoding_rs::UTF_8.new_decoder();
            
            // Pass `true` to allow decoding special tokens like <|im_end|>
            if let Ok(token_str) = self.model.token_to_piece(new_token_id, &mut decoder, true, None) {
                // Qwen uses <|im_end|> as its stop token for chat.
                if token_str.contains("<|im_end|>") || token_str.contains("<|endoftext|>") {
                    break;
                }
                response.push_str(&token_str);
            }
            
            batch.clear();
            batch.add(new_token_id, n_cur, &[0], true)
                .map_err(|e| format!("Batch error: {}", e))?;
            n_cur += 1;
            
            ctx.decode(&mut batch).map_err(|e| format!("Decode error: {}", e))?;
        }
        
        Ok(response)
    }
}
