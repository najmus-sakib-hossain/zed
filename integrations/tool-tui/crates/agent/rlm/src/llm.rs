use crate::error::{RLMError, Result};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use futures::stream::StreamExt;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
struct GroqRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct GroqResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: Delta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    content: Option<String>,
}

pub struct LLMClient {
    client: Client,
    api_key: String,
    model: String,
    fast_model: Option<String>, // For search/exploration tasks
    response_cache: Arc<Mutex<HashMap<u64, String>>>,
    cache_hits: Arc<Mutex<usize>>,
    cache_misses: Arc<Mutex<usize>>,
    fast_model_calls: Arc<Mutex<usize>>,
    smart_model_calls: Arc<Mutex<usize>>,
}

impl Clone for LLMClient {
    fn clone(&self) -> Self {
        Self {
            client: Client::new(),
            api_key: self.api_key.clone(),
            model: self.model.clone(),
            fast_model: self.fast_model.clone(),
            response_cache: self.response_cache.clone(),
            cache_hits: self.cache_hits.clone(),
            cache_misses: self.cache_misses.clone(),
            fast_model_calls: self.fast_model_calls.clone(),
            smart_model_calls: self.smart_model_calls.clone(),
        }
    }
}

impl LLMClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            fast_model: None,
            response_cache: Arc::new(Mutex::new(HashMap::new())),
            cache_hits: Arc::new(Mutex::new(0)),
            cache_misses: Arc::new(Mutex::new(0)),
            fast_model_calls: Arc::new(Mutex::new(0)),
            smart_model_calls: Arc::new(Mutex::new(0)),
        }
    }

    pub fn with_fast_model(mut self, fast_model: String) -> Self {
        self.fast_model = Some(fast_model);
        self
    }

    pub fn model_stats(&self) -> (usize, usize) {
        let fast = *self.fast_model_calls.lock().unwrap();
        let smart = *self.smart_model_calls.lock().unwrap();
        (fast, smart)
    }

    /// Determine which model to use based on task type
    fn select_model(&self, messages: &[Message]) -> String {
        // If no fast model configured, always use smart model
        let fast_model = match &self.fast_model {
            Some(m) => m,
            None => return self.model.clone(),
        };

        // Check last user message for task indicators
        if let Some(last_msg) = messages.iter().rev().find(|m| m.role == "user") {
            let content = last_msg.content.to_lowercase();
            
            // Use fast model for search/exploration tasks
            if content.contains("fast_find") 
                || content.contains("fast_contains")
                || content.contains("fast_find_all")
                || content.contains("search")
                || content.contains("find")
                || content.contains("extract")
                || content.contains("locate")
                || content.contains("index_of")
                || content.contains("sub_string") {
                return fast_model.clone();
            }
            
            // Use smart model for synthesis/reasoning tasks
            if content.contains("final(")
                || content.contains("summarize")
                || content.contains("analyze")
                || content.contains("explain")
                || content.contains("compare")
                || content.contains("conclude") {
                return self.model.clone();
            }
        }

        // Default to fast model for REPL iterations
        fast_model.clone()
    }

    fn hash_messages(messages: &[Message]) -> u64 {
        let mut hasher = DefaultHasher::new();
        for msg in messages {
            msg.role.hash(&mut hasher);
            msg.content.hash(&mut hasher);
        }
        hasher.finish()
    }

    pub fn cache_stats(&self) -> (usize, usize) {
        let hits = *self.cache_hits.lock().unwrap();
        let misses = *self.cache_misses.lock().unwrap();
        (hits, misses)
    }

    pub fn clear_cache(&self) {
        self.response_cache.lock().unwrap().clear();
        *self.cache_hits.lock().unwrap() = 0;
        *self.cache_misses.lock().unwrap() = 0;
    }

    pub async fn complete(&self, messages: Vec<Message>) -> Result<String> {
        // Check cache first
        let cache_key = Self::hash_messages(&messages);
        
        {
            let cache = self.response_cache.lock().unwrap();
            if let Some(cached_response) = cache.get(&cache_key) {
                // Cache hit! Save API call
                *self.cache_hits.lock().unwrap() += 1;
                return Ok(cached_response.clone());
            }
        }

        // Cache miss - make API call
        *self.cache_misses.lock().unwrap() += 1;
        
        // Select appropriate model based on task
        let selected_model = self.select_model(&messages);
        
        // Track model usage
        if self.fast_model.as_ref() == Some(&selected_model) {
            *self.fast_model_calls.lock().unwrap() += 1;
        } else {
            *self.smart_model_calls.lock().unwrap() += 1;
        }
        
        let request = GroqRequest {
            model: selected_model,
            messages,
            temperature: 1.0,
            max_tokens: 1024,
            stream: None,
        };

        let response = self.client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RLMError::LLMError(format!(
                "API error {}: {}",
                status, error_text
            )));
        }

        let groq_response: GroqResponse = response.json().await?;

        let result = groq_response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .ok_or_else(|| RLMError::LLMError("No response from LLM".to_string()))?;

        // Store in cache (limit size to prevent memory bloat)
        {
            let mut cache = self.response_cache.lock().unwrap();
            if cache.len() < 500 {
                cache.insert(cache_key, result.clone());
            }
        }

        Ok(result)
    }
}

    /// Stream LLM response token by token (Phase 2 optimization)
    /// Returns a channel receiver that yields tokens as they arrive
    pub async fn stream(&self, messages: Vec<Message>) -> Result<mpsc::Receiver<String>> {
        let (tx, rx) = mpsc::channel(100);

        // Select appropriate model
        let selected_model = self.select_model(&messages);
        
        // Track model usage
        if self.fast_model.as_ref() == Some(&selected_model) {
            *self.fast_model_calls.lock().unwrap() += 1;
        } else {
            *self.smart_model_calls.lock().unwrap() += 1;
        }

        let request = GroqRequest {
            model: selected_model,
            messages,
            temperature: 1.0,
            max_tokens: 1024,
            stream: Some(true),
        };

        let response = self.client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RLMError::LLMError(format!(
                "API error {}: {}",
                status, error_text
            )));
        }

        // Spawn task to process stream
        tokio::spawn(async move {
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk) = stream.next().await {
                if let Ok(bytes) = chunk {
                    let text = String::from_utf8_lossy(&bytes);
                    
                    // Parse SSE format: "data: {...}\n\n"
                    for line in text.lines() {
                        if line.starts_with("data: ") {
                            let json_str = &line[6..];
                            
                            if json_str == "[DONE]" {
                                break;
                            }
                            
                            if let Ok(chunk) = serde_json::from_str::<StreamChunk>(json_str) {
                                if let Some(choice) = chunk.choices.first() {
                                    if let Some(content) = &choice.delta.content {
                                        let _ = tx.send(content.clone()).await;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(rx)
    }
