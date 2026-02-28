//! Text enhancement using Qwen2.5-0.5B LLM

use crate::error::{I18nError, Result};
use llama_cpp_2::{
    context::params::LlamaContextParams,
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::{AddBos, LlamaModel, params::LlamaModelParams},
    token::data_array::LlamaTokenDataArray,
};
use std::path::Path;

const SYSTEM_PROMPT: &str = "You are a text enhancement assistant. Remove filler words (um, uh, like, you know), fix grammar, add punctuation, and make text clear and professional. Output only the enhanced text.";

/// Text enhancer using Qwen2.5-0.5B
pub struct TextEnhancer {
    backend: LlamaBackend,
    model: LlamaModel,
}

impl TextEnhancer {
    /// Create a new text enhancer
    pub fn new(model_path: impl AsRef<Path>) -> Result<Self> {
        let backend = LlamaBackend::init()
            .map_err(|e| I18nError::Other(format!("Failed to init llama backend: {}", e)))?;

        let model_params = LlamaModelParams::default();
        let model = LlamaModel::load_from_file(&backend, model_path.as_ref(), &model_params)
            .map_err(|e| I18nError::Other(format!("Failed to load model: {}", e)))?;

        Ok(Self { backend, model })
    }

    /// Enhance text with grammar correction and formatting
    pub fn enhance(&self, text: &str) -> Result<String> {
        // Build simpler prompt
        let prompt = format!(
            "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
            SYSTEM_PROMPT, text
        );

        // Create context
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(std::num::NonZeroU32::new(2048))
            .with_n_batch(512);

        let mut ctx = self
            .model
            .new_context(&self.backend, ctx_params)
            .map_err(|e| I18nError::Other(format!("Failed to create context: {}", e)))?;

        // Tokenize
        let tokens = self
            .model
            .str_to_token(&prompt, AddBos::Always)
            .map_err(|e| I18nError::Other(format!("Failed to tokenize: {}", e)))?;

        // Create batch and add all prompt tokens
        let mut batch = LlamaBatch::new(512, 1);

        let last_index = tokens.len() - 1;
        for (i, token) in tokens.iter().enumerate() {
            let is_last = i == last_index;
            batch
                .add(*token, i as i32, &[0], is_last)
                .map_err(|e| I18nError::Other(format!("Failed to add token: {}", e)))?;
        }

        // Decode prompt
        ctx.decode(&mut batch)
            .map_err(|e| I18nError::Other(format!("Failed to decode: {}", e)))?;

        // Generate response with greedy sampling
        let mut result = String::new();
        let max_tokens = 256;
        let mut n_cur = tokens.len();

        for _ in 0..max_tokens {
            let candidates = ctx.candidates();
            let candidates_p = LlamaTokenDataArray::from_iter(candidates, false);

            // Greedy sampling - take most likely token
            let new_token = candidates_p.data[0].id();

            // Check for EOS
            if self.model.is_eog_token(new_token) {
                break;
            }

            // Convert token to string
            let piece = self
                .model
                .token_to_str(new_token, llama_cpp_2::model::Special::Tokenize)
                .map_err(|e| I18nError::Other(format!("Failed to convert token: {}", e)))?;

            result.push_str(&piece);

            // Prepare next batch with single token
            batch.clear();
            batch
                .add(new_token, n_cur as i32, &[0], true)
                .map_err(|e| I18nError::Other(format!("Failed to add token: {}", e)))?;

            n_cur += 1;

            // Decode next token
            ctx.decode(&mut batch)
                .map_err(|e| I18nError::Other(format!("Failed to decode: {}", e)))?;
        }

        Ok(result.trim().to_string())
    }
}
