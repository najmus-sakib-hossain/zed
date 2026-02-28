use crate::error::{RLMError, Result};
use rhai::{Engine, Scope, AST};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct REPLExecutor {
    engine: Engine,
    max_output_chars: usize,
    ast_cache: Arc<Mutex<HashMap<String, AST>>>,
    cache_hits: Arc<Mutex<usize>>,
    cache_misses: Arc<Mutex<usize>>,
}

impl REPLExecutor {
    pub fn new() -> Self {
        let mut engine = Engine::new();
        
        // Configure engine for safety
        engine.set_max_expr_depths(50, 50);
        engine.set_max_operations(100_000);
        engine.set_max_string_size(10_000_000); // 10MB max string
        
        // Register SIMD-accelerated search functions
        Self::register_fast_search(&mut engine);
        
        Self {
            engine,
            max_output_chars: 2000,
            ast_cache: Arc::new(Mutex::new(HashMap::new())),
            cache_hits: Arc::new(Mutex::new(0)),
            cache_misses: Arc::new(Mutex::new(0)),
        }
    }

    pub fn cache_stats(&self) -> (usize, usize) {
        let hits = *self.cache_hits.lock().unwrap();
        let misses = *self.cache_misses.lock().unwrap();
        (hits, misses)
    }

    pub fn clear_cache(&self) {
        self.ast_cache.lock().unwrap().clear();
        *self.cache_hits.lock().unwrap() = 0;
        *self.cache_misses.lock().unwrap() = 0;
    }

    fn register_fast_search(engine: &mut Engine) {
        // Fast substring search using SIMD (memchr)
        engine.register_fn("fast_find", |text: &str, pattern: &str| -> i64 {
            memchr::memmem::find(text.as_bytes(), pattern.as_bytes())
                .map(|i| i as i64)
                .unwrap_or(-1)
        });

        // Fast contains check
        engine.register_fn("fast_contains", |text: &str, pattern: &str| -> bool {
            memchr::memmem::find(text.as_bytes(), pattern.as_bytes()).is_some()
        });

        // Find all occurrences (returns array of indices)
        engine.register_fn("fast_find_all", |text: &str, pattern: &str| -> Vec<i64> {
            memchr::memmem::find_iter(text.as_bytes(), pattern.as_bytes())
                .map(|i| i as i64)
                .collect()
        });
    }

    pub fn execute(&self, code: &str, scope: &mut Scope) -> Result<String> {
        // Extract code from markdown blocks if present
        let code = self.extract_code(code);

        if code.trim().is_empty() {
            return Ok("No code to execute".to_string());
        }

        // Check cache first (30-50% speedup on repeated patterns)
        let ast = {
            let mut cache = self.ast_cache.lock().unwrap();
            
            if let Some(cached_ast) = cache.get(&code) {
                // Cache hit!
                *self.cache_hits.lock().unwrap() += 1;
                cached_ast.clone()
            } else {
                // Cache miss - compile and store
                *self.cache_misses.lock().unwrap() += 1;
                
                let ast = self.engine
                    .compile(&code)
                    .map_err(|e| RLMError::REPLError(format!("Compilation error: {}", e)))?;
                
                // Store in cache (limit cache size to prevent memory bloat)
                if cache.len() < 1000 {
                    cache.insert(code.clone(), ast.clone());
                }
                
                ast
            }
        };

        // Execute with scope
        let result: rhai::Dynamic = self.engine
            .eval_ast_with_scope(scope, &ast)
            .map_err(|e| RLMError::REPLError(format!("Execution error: {}", e)))?;

        // Convert result to string
        let output = result.to_string();

        // Truncate if too long
        if output.len() > self.max_output_chars {
            Ok(format!(
                "{}\n\n[Output truncated: {} chars total, showing first {}]",
                &output[..self.max_output_chars],
                output.len(),
                self.max_output_chars
            ))
        } else if output.is_empty() {
            Ok("Code executed successfully (no output)".to_string())
        } else {
            Ok(output)
        }
    }

    fn extract_code(&self, text: &str) -> String {
        // Check for markdown code blocks
        if text.contains("```") {
            if let Some(start) = text.find("```rhai") {
                let start = start + 7;
                if let Some(end) = text[start..].find("```") {
                    return text[start..start + end].trim().to_string();
                }
            }
            
            if let Some(start) = text.find("```") {
                let start = start + 3;
                if let Some(end) = text[start..].find("```") {
                    return text[start..start + end].trim().to_string();
                }
            }
        }

        text.to_string()
    }
}

impl Default for REPLExecutor {
    fn default() -> Self {
        Self::new()
    }
}
