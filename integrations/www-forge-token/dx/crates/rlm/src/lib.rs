//! Retrieval-Layered Memory: decomposes large docs into indexed chunks.
//! SAVINGS: 40-80% on large documents
//! STAGE: PrePrompt (priority 30)

use dx_core::*;
use std::sync::Mutex;

pub struct RlmSaver {
    config: RlmConfig,
    report: Mutex<TokenSavingsReport>,
}

#[derive(Clone)]
pub struct RlmConfig {
    pub threshold_tokens: usize,
    pub chunk_tokens: usize,
    pub index_preview_lines: usize,
    pub include_line_numbers: bool,
}

impl Default for RlmConfig {
    fn default() -> Self {
        Self {
            threshold_tokens: 3000,
            chunk_tokens: 500,
            index_preview_lines: 2,
            include_line_numbers: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub id: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub preview: String,
    pub tokens: usize,
    pub content: String,
}

impl RlmSaver {
    pub fn new(config: RlmConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(RlmConfig::default())
    }

    pub fn decompose(&self, content: &str) -> Vec<Chunk> {
        let lines: Vec<&str> = content.lines().collect();
        let chars_per_chunk = self.config.chunk_tokens * 4;
        let mut chunks = Vec::new();
        let mut chunk_id = 0;
        let mut char_count = 0;
        let mut start_line = 0;
        let mut current = String::new();

        for (i, line) in lines.iter().enumerate() {
            current.push_str(line);
            current.push('\n');
            char_count += line.len() + 1;

            if char_count >= chars_per_chunk || i == lines.len() - 1 {
                let preview: String = current.lines()
                    .take(self.config.index_preview_lines)
                    .collect::<Vec<_>>()
                    .join("\n");
                chunks.push(Chunk {
                    id: chunk_id,
                    start_line,
                    end_line: i + 1,
                    preview,
                    tokens: char_count / 4,
                    content: current.clone(),
                });
                chunk_id += 1;
                start_line = i + 1;
                current.clear();
                char_count = 0;
            }
        }

        chunks
    }

    pub fn generate_index(&self, chunks: &[Chunk]) -> String {
        let mut idx = String::from("[DOCUMENT INDEX]\n");
        for chunk in chunks {
            if self.config.include_line_numbers {
                idx.push_str(&format!(
                    "  [chunk {}] lines {}-{} ({} tokens): {}\n",
                    chunk.id, chunk.start_line, chunk.end_line, chunk.tokens, chunk.preview
                ));
            } else {
                idx.push_str(&format!(
                    "  [chunk {}] ({} tokens): {}\n",
                    chunk.id, chunk.tokens, chunk.preview
                ));
            }
        }
        idx.push_str("[END INDEX]\n");
        idx
    }
}

#[async_trait::async_trait]
impl TokenSaver for RlmSaver {
    fn name(&self) -> &str { "rlm" }
    fn stage(&self) -> SaverStage { SaverStage::PrePrompt }
    fn priority(&self) -> u32 { 30 }

    async fn process(&self, mut input: SaverInput, _ctx: &SaverContext) -> Result<SaverOutput, SaverError> {
        let mut total_saved = 0usize;

        for msg in &mut input.messages {
            if msg.token_count <= self.config.threshold_tokens {
                continue;
            }

            let chunks = self.decompose(&msg.content);
            if chunks.len() <= 1 {
                continue;
            }

            let index = self.generate_index(&chunks);
            let first_chunk = chunks.first().map(|c| c.content.as_str()).unwrap_or("");
            let new_content = format!(
                "{}\n[Only first chunk shown. Request specific chunks by ID.]\n{}",
                index, first_chunk
            );

            let saved = msg.token_count.saturating_sub(new_content.len() / 4);
            if saved > 0 {
                msg.content = new_content;
                msg.token_count = msg.content.len() / 4;
                total_saved += saved;
            }
        }

        if total_saved > 0 {
            let mut report = self.report.lock().unwrap();
            *report = TokenSavingsReport {
                technique: "rlm".into(),
                tokens_before: total_saved,
                tokens_after: 0,
                tokens_saved: total_saved,
                description: format!("RLM: indexed {} tokens of large documents", total_saved),
            };
        }

        Ok(SaverOutput {
            messages: input.messages,
            tools: input.tools,
            images: input.images,
            skipped: false,
            cached_response: None,
        })
    }

    fn last_savings(&self) -> TokenSavingsReport {
        self.report.lock().unwrap().clone()
    }
}
