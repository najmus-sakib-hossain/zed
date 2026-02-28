//! Tokenizer utilities

use anyhow::Result;
use tokenizers::Tokenizer;

pub struct TokenizerWrapper {
    tokenizer: Tokenizer,
}

impl TokenizerWrapper {
    pub fn from_file(path: &str) -> Result<Self> {
        let tokenizer = Tokenizer::from_file(path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;
        Ok(Self { tokenizer })
    }

    pub fn encode(&self, text: &str) -> Result<Vec<u32>> {
        let encoding = self
            .tokenizer
            .encode(text, false)
            .map_err(|e| anyhow::anyhow!("Failed to encode text: {}", e))?;
        Ok(encoding.get_ids().to_vec())
    }

    pub fn decode(&self, ids: &[u32]) -> Result<String> {
        self.tokenizer
            .decode(ids, false)
            .map_err(|e| anyhow::anyhow!("Failed to decode tokens: {}", e))
    }

    pub fn count_tokens(&self, text: &str) -> Result<usize> {
        let encoding = self
            .tokenizer
            .encode(text, false)
            .map_err(|e| anyhow::anyhow!("Failed to count tokens: {}", e))?;
        Ok(encoding.get_ids().len())
    }
}
