//! LLM message handling for chat app

use super::app_state::ChatApp;

impl ChatApp {
    /// Check if LLM is available
    pub fn has_llm(&self) -> bool {
        self.llm.is_some()
    }

    /// Check if LLM is initialized
    pub fn is_llm_ready(&self) -> bool {
        self.llm_initialized && self.llm.as_ref().map_or(false, |llm| llm.is_initialized())
    }

    /// Initialize LLM asynchronously
    pub fn initialize_llm(&mut self) {
        if let Some(llm) = self.llm.clone() {
            // Load cached Google models immediately for fast startup
            if llm.get_google_api_key().is_some() {
                self.google_models = llm.get_cached_google_models();
            }

            let tx = self.llm_tx.clone();
            tokio::spawn(async move {
                // Initialize LLM engine (required for generation)
                if let Err(_e) = llm.initialize().await {
                    // Silently fail - user can configure backend via model modal
                    return;
                }

                // Fetch Google models in background (non-blocking)
                if llm.get_google_api_key().is_some() {
                    tokio::spawn(async move {
                        if let Ok(models) = llm.fetch_google_models().await {
                            let models_json = serde_json::to_string(&models).unwrap_or_default();
                            let _ = tx.send(format!("__GOOGLE_MODELS__:{}", models_json));
                        }
                    });
                }
            });
            self.llm_initialized = true;
        }
    }
}
