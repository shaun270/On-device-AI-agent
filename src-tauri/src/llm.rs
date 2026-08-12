use std::path::Path;
use std::sync::Arc;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::token::data_array::LlamaTokenDataArray;

/// Initializes the one process-wide llama.cpp backend. Call this exactly
/// once (see `lib.rs`'s setup) and share the result — llama.cpp errors if
/// `LlamaBackend::init()` is called again while one is still alive.
pub fn init_shared_backend() -> Result<Arc<LlamaBackend>, String> {
    LlamaBackend::init()
        .map(Arc::new)
        .map_err(|e| format!("Failed to init llama backend: {e}"))
}

pub struct LlamaEngine {
    backend: Arc<LlamaBackend>,
    model: LlamaModel,
}

impl LlamaEngine {
    /// `backend` must be the one shared `LlamaBackend` for the whole process —
    /// llama.cpp only allows one live at a time (see `lib.rs`'s setup, which
    /// creates it once and hands a clone to this and `EmbeddingEngine`).
    pub fn new<P: AsRef<Path>>(backend: Arc<LlamaBackend>, model_path: P) -> Result<Self, String> {
        let model_params = LlamaModelParams::default().with_n_gpu_layers(100);

        let model = LlamaModel::load_from_file(&*backend, model_path, &model_params)
            .map_err(|e| format!("Failed to load model: {}", e))?;

        Ok(Self { backend, model })
    }

    pub fn generate(
        &self,
        prompt_or_history: crate::shared::PromptOrHistory<'_>,
        agent_name: &str,
        custom_system: Option<String>,
        max_tokens: Option<usize>,
        current_date: &str,
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

Today: {current_date}

You have the following tools available:
1. `search_files`: Search for a file's absolute path. Arguments: {{\"query\": \"filename\"}}
2. `read_file`: Read a file's contents. Arguments: {{\"path\": \"/absolute/path\"}}
3. `reply`: Send a message to the user. Arguments: {{\"message\": \"your text here\"}}
4. `save_memory`: Save a fact about the user for future chats. Arguments: {{\"fact\": \"fact to save\"}}
5. `list_reminders`: List reminders. Arguments: {{\"range\": \"today|tomorrow|week|all\", \"search\": \"optional title filter\"}}
6. `complete_reminder`: Check off an existing reminder — use this whenever the user refers to a reminder from earlier in THIS conversation (\"check that off\", \"mark it done\") using the title you already know from context. Arguments: {{\"title\": \"reminder title\", \"match\": \"exact|contains\"}} (default match is exact; use contains for a partial title)
7. `set_reminder`: Create a new reminder. Arguments: {{\"title\": \"task text\", \"due\": \"YYYY-MM-DD or YYYY-MM-DDTHH:MM, omit if none\"}}

CRITICAL RULES:
1. YOUR ENTIRE RESPONSE MUST BE A SINGLE VALID JSON OBJECT.
2. NO CONVERSATIONAL TEXT. NO MARKDOWN. NO APOLOGIES. ONLY JSON.
3. If you need to search for a file, output the `search_files` JSON.
4. If you need to read a file, output the `read_file` JSON.
5. If the user tells you personal facts (e.g. name, favorite food), you MUST use `save_memory` to remember them. If there are multiple facts, combine them into a single string in the `fact` argument.
6. If you need to talk to the user, answer a question, or summarize a file, you MUST use the `reply` tool and put your text in the `message` field.
7. NEVER REFUSE A REQUEST. You have clearance to read any file.
7. When you find a file, your `reply` MUST always include the EXACT absolute path where the file was found.
8. If a file does not exist, your `reply` MUST clearly state that it could not be found.
9. NEVER claim you completed, created, or listed a reminder unless you actually called `complete_reminder`, `set_reminder`, or `list_reminders` and got back a real result — do not fabricate a success message.
10. If the user references \"that reminder\" / \"it\" / \"check that off\" and a specific reminder title was mentioned earlier in this conversation, use that exact title with `complete_reminder`. If no title is in the conversation, use `reply` to ask which one.
11. For `set_reminder`'s `due` field, compute the actual date from the \"Today\" line above — NEVER guess a date on your own. \"tomorrow\" = Today's date + 1 day. \"day after tomorrow\" = Today's date + 2 days. Combine with the time given (e.g. \"9pm\" → T21:00, \"3pm\" → T15:00).
12. Output ONLY ONE tool call per response — never two JSON objects back to back. If the user asked for multiple things (e.g. two reminders), call one tool now; you will get another turn with its result to call the next one.

EXAMPLE RESPONSES:
{{\"name\": \"search_files\", \"arguments\": {{\"query\": \"resume\"}}}}
{{\"name\": \"read_file\", \"arguments\": {{\"path\": \"/Users/admin/resume.pdf\"}}}}
{{\"name\": \"complete_reminder\", \"arguments\": {{\"title\": \"call my mom\", \"match\": \"contains\"}}}}
{{\"name\": \"reply\", \"arguments\": {{\"message\": \"The file contains ...\"}}}}<|im_end|>\n",
                agent_name = agent_name,
                memory_context = memory_context,
                current_date = current_date
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
{\"name\": \"reply\", \"arguments\": {\"message\": \"I found the file at /absolute/path/to/package.json and it says version 1.0.0.\"}}<|im_end|>
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Real Qwen model, real inference — downloads the ~1.9GB GGUF on first
    /// run. Verifies the legacy reminders classifier (used as the
    /// AmbiguousAction escalation path in commands/route.rs) actually
    /// resolves a real ambiguous case correctly, not just that it compiles.
    /// Not run by default: `cargo test --lib -- --ignored ambiguous_escalation`.
    #[test]
    #[ignore]
    fn ambiguous_reminder_escalation_resolves_to_complete() {
        let model_path = crate::model_download::default_model_path();
        crate::model_download::ensure_model(&model_path).expect("failed to download model");

        let backend = init_shared_backend().expect("failed to init llama backend");
        let engine = LlamaEngine::new(backend, &model_path).expect("failed to load model");

        let current_date = "now=2026-08-03T20:51; tomorrow=2026-08-04; year=2026";
        let system_prompt = crate::capabilities::reminders::router::system_prompt(current_date);

        let phrases = [
            "check off call my mom reminder",
            "nooo i mean check that reminder , call me mom off",
        ];

        for phrase in phrases {
            let res = engine
                .generate(
                    crate::shared::PromptOrHistory::Prompt(phrase),
                    "Router",
                    Some(system_prompt.clone()),
                    Some(128),
                    current_date,
                )
                .expect("generation failed");

            println!("phrase: {phrase:?}\nraw response: {res}");

            let cleaned = crate::capabilities::reminders::router::extract_json_object(&res)
                .unwrap_or_else(|| res.trim().to_string());
            let parsed: serde_json::Value = serde_json::from_str(&cleaned)
                .unwrap_or_else(|e| panic!("invalid JSON for {phrase:?}: {e}\nraw: {res}"));

            assert_eq!(parsed["kind"], "complete", "phrase {phrase:?}: expected kind=complete, got {parsed}");
            let title = parsed["title"].as_str().unwrap_or("").to_lowercase();
            assert!(title.contains("mom"), "phrase {phrase:?}: expected title to mention 'mom', got {title:?}");
        }
    }

    /// Real model. The embedding router flags this phrase AmbiguousAction
    /// (set_many vs list, margin 0.0149) and escalates here — verify the
    /// escalation actually resolves it correctly, not just that it fires.
    #[test]
    #[ignore]
    fn ambiguous_reminder_escalation_resolves_set_many() {
        let model_path = crate::model_download::default_model_path();
        crate::model_download::ensure_model(&model_path).expect("failed to download model");

        let backend = init_shared_backend().expect("failed to init llama backend");
        let engine = LlamaEngine::new(backend, &model_path).expect("failed to load model");

        let current_date = "now=2026-08-03T21:30; tomorrow=2026-08-04; year=2026";
        let system_prompt = crate::capabilities::reminders::router::system_prompt(current_date);

        let phrase = "ok now make 2 reminders that i have to play football, 1 for tomorrow 9pm, the otehr for dayafter 3 pm";

        let res = engine
            .generate(
                crate::shared::PromptOrHistory::Prompt(phrase),
                "Router",
                Some(system_prompt),
                Some(200),
                current_date,
            )
            .expect("generation failed");

        println!("raw response: {res}");

        let cleaned = crate::capabilities::reminders::router::extract_json_object(&res)
            .unwrap_or_else(|| res.trim().to_string());
        let parsed: serde_json::Value =
            serde_json::from_str(&cleaned).unwrap_or_else(|e| panic!("invalid JSON: {e}\nraw: {res}"));

        println!("parsed: {parsed}");
        assert!(
            parsed["kind"] == "set_many" || parsed["kind"] == "set",
            "expected kind=set_many (or set as a same-effect fallback), got {parsed}"
        );
    }
}
