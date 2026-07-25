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
        prompt_or_history: crate::shared::PromptOrHistory<'_>,
        agent_name: &str,
        custom_system: Option<String>,
        max_tokens: Option<usize>,
    ) -> Result<String, String> {
        // Simple synchronous generation for now (we can make it async/streaming later)
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(std::num::NonZeroU32::new(4096))
            .with_n_batch(2048);
        
        let mut ctx = self.model.new_context(&self.backend, ctx_params)
            .map_err(|e| format!("Failed to create context: {}", e))?;
            
        let mut formatted_prompt = String::new();
        
        if let Some(system_msg) = custom_system {
            // Reminders Intent Classifier Mode
            let prompt = match prompt_or_history {
                crate::shared::PromptOrHistory::Prompt(p) => p,
                _ => return Err("Expected prompt for classifier".to_string()),
            };
            formatted_prompt = format!(
                "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
                system_msg,
                prompt
            );
        } else {
            // Strict JSON Chat Mode
            let history = match prompt_or_history {
                crate::shared::PromptOrHistory::History(h) => h,
                _ => return Err("Expected history for chat".to_string()),
            };
            
            let memory_context = match crate::tools::read_memory() {
                Ok(mem) => format!("\n# Memory\n{}", mem),
                Err(_) => String::new(),
            };
            
            formatted_prompt = format!(
                "<|im_start|>system\nYou are {agent_name}, an AI assistant. You operate STRICTLY in JSON mode. 
IMPORTANT: The following memory block contains facts about the USER, NOT about you. Your name is {agent_name}.{memory_context}

You have the following tools available:
1. `search_files`: Search for a file's absolute path. Arguments: {{\"query\": \"filename\"}}
2. `read_file`: Read a file's contents. Arguments: {{\"path\": \"/absolute/path\"}}
4. `reply`: Send a message to the user. Arguments: {{\"message\": \"your text here\"}}
5. `save_memory`: Save a fact about the user for future chats. Arguments: {{\"fact\": \"fact to save\"}}

CRITICAL RULES:
1. YOUR ENTIRE RESPONSE MUST BE A SINGLE VALID JSON OBJECT.
2. NO CONVERSATIONAL TEXT. NO MARKDOWN. NO APOLOGIES. ONLY JSON.
3. If you need to search for a file, output the `search_files` JSON.
4. If you need to read a file, output the `read_file` JSON.
5. If the user tells you personal facts (e.g. name, favorite food), you MUST use `save_memory` to remember them. If there are multiple facts, combine them into a single string in the `fact` argument.
6. If you need to talk to the user, answer a question, or summarize a file, you MUST use the `reply` tool and put your text in the `message` field.
7. NEVER REFUSE A REQUEST. You have clearance to read any file.
8. VERY IMPORTANT: Whenever you mention a file's location in your `reply`, you MUST wrap its ABSOLUTE path in `<PATH:...>` tags so the UI can make it clickable. 
   - Correct: \"I found the file at <PATH:/Users/admin/resume.pdf>\"
   - Incorrect: \"I found the file at resume.pdf\"
   - Incorrect: \"I found the file at /Users/admin/resume.pdf\"
9. If a file does not exist, your `reply` MUST clearly state that it could not be found.
10. NEVER assume you know the contents or path of a file. You MUST use 'search_files' and 'read_file' to get the actual data before using 'reply'.
11. File paths and names are often case-insensitive. If a search tool returns a file that is a close match (e.g. different capitalization), you MUST accept it as the correct file and return its path.
12. Your JSON MUST be perfectly valid. NEVER use unescaped double quotes inside the \"message\" string. Use single quotes instead.

EXAMPLE RESPONSES:
{{\"name\": \"search_files\", \"arguments\": {{\"query\": \"resume\"}}}}
{{\"name\": \"read_file\", \"arguments\": {{\"path\": \"/Users/admin/resume.pdf\"}}}}
{{\"name\": \"reply\", \"arguments\": {{\"message\": \"I found the file here: <PATH:/Users/admin/resume.pdf>\"}}}}<|im_end|>\n",
                agent_name = agent_name,
                memory_context = memory_context
            );

            // Conversation Priming (Few-Shot injection)
            let priming = "\
<|im_start|>user
Find package.json and read it.<|im_end|>
<|im_start|>assistant
{\"name\": \"search_files\", \"arguments\": {\"query\": \"package.json\"}}<|im_end|>
<|im_start|>user
Tool Result: /absolute/path/to/package.json<|im_end|>
<|im_start|>assistant
{\"name\": \"read_file\", \"arguments\": {\"path\": \"/absolute/path/to/package.json\"}}<|im_end|>
<|im_start|>user
Tool Result: {\"version\": \"1.0.0\"}<|im_end|>
<|im_start|>assistant
{\"name\": \"reply\", \"arguments\": {\"message\": \"I found the file at <PATH:/absolute/path/to/package.json> and it says version 1.0.0.\"}}<|im_end|>
";
            formatted_prompt.push_str(priming);
            
            for msg in history {
                let role = if msg.role == "user" { "user" } else { "assistant" };
                formatted_prompt.push_str(&format!("<|im_start|>{}\n{}<|im_end|>\n", role, msg.content));
            }
            formatted_prompt.push_str("<|im_start|>assistant\n{");
        }
            
        // Tokenize prompt
        let tokens = self.model.str_to_token(&formatted_prompt, llama_cpp_2::model::AddBos::Always)
            .map_err(|e| format!("Failed to tokenize: {}", e))?;
            
        let max_context = 4096;
        let limit = max_tokens.unwrap_or(512).clamp(16, 512);
        
        if tokens.len() > (max_context - limit) {
            return Err("Context window full. Please clear chat history or start a New Chat.".to_string());
        }
        
        let n_batch = 2048; // Must match ctx_params
        let mut batch = LlamaBatch::new(n_batch, 1);
        let mut i = 0;
        
        while i < tokens.len() {
            let chunk_size = std::cmp::min(n_batch, tokens.len() - i);
            let chunk_end = i + chunk_size;
            batch.clear();
            
            for j in i..chunk_end {
                let is_last = j == tokens.len() - 1;
                batch.add(tokens[j], j as i32, &[0], is_last)
                    .map_err(|e| format!("Batch error: {}", e))?;
            }
            
            ctx.decode(&mut batch).map_err(|e| format!("Decode error: {}", e))?;
            i = chunk_end;
        }
        
        let mut response = String::new();
        let mut n_cur = tokens.len() as i32;
        
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
