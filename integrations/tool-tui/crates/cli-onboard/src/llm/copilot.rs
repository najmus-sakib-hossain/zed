use crate::llm::error::ProviderError;
use url::Url;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CopilotTier {
    Free,
    Paid,
    Enterprise,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CopilotApiSurface {
    ChatCompletions,
    Responses,
}

pub fn detect_copilot_tier_from_env() -> CopilotTier {
    if std::env::var("GITHUB_COPILOT_ENTERPRISE").is_ok() {
        return CopilotTier::Enterprise;
    }
    if std::env::var("GITHUB_COPILOT_PAID").is_ok() {
        return CopilotTier::Paid;
    }
    CopilotTier::Free
}

pub fn model_api_surface(model: &str) -> CopilotApiSurface {
    let normalized = model.to_ascii_lowercase();
    if normalized.starts_with("gpt-5") {
        CopilotApiSurface::Responses
    } else {
        CopilotApiSurface::ChatCompletions
    }
}

pub fn endpoint_for_model(base_url: &str, model: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    match model_api_surface(model) {
        CopilotApiSurface::Responses => format!("{trimmed}/v1/responses"),
        CopilotApiSurface::ChatCompletions => format!("{trimmed}/v1/chat/completions"),
    }
}

pub fn retrieve_copilot_token() -> Option<String> {
    // DX expects the *Copilot service token* (from GitHub's `/copilot_internal/v2/token`),
    // which is short-lived and should be refreshed per run via bootstrap.
    //
    // Important: we intentionally DO NOT read VS Code / extension cached tokens here,
    // and we DO NOT fall back to `GITHUB_TOKEN`/PAT, because those frequently lead to
    // “works once, then access denied” when reused.
    std::env::var("GITHUB_COPILOT_TOKEN").ok().and_then(|token| {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

#[derive(Debug, Clone)]
pub struct CopilotOAuthConfig {
    pub client_id: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

impl CopilotOAuthConfig {
    pub fn auth_url(&self, state: &str) -> Result<String, ProviderError> {
        let mut url = Url::parse("https://github.com/login/oauth/authorize").map_err(|err| {
            ProviderError::InvalidConfig {
                provider: "github-copilot".to_string(),
                detail: format!("invalid oauth url: {err}"),
            }
        })?;

        let scope = if self.scopes.is_empty() {
            "read:user user:email".to_string()
        } else {
            self.scopes.join(" ")
        };

        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &self.redirect_uri)
            .append_pair("scope", &scope)
            .append_pair("state", state);

        Ok(url.into())
    }
}

#[derive(Debug, Clone)]
pub struct CopilotOAuthFlow {
    pub config: CopilotOAuthConfig,
}

impl CopilotOAuthFlow {
    pub fn new(config: CopilotOAuthConfig) -> Self {
        Self { config }
    }

    pub fn begin(&self, state: &str) -> Result<String, ProviderError> {
        self.config.auth_url(state)
    }

    pub fn parse_callback_code(
        &self,
        callback_url: &str,
        expected_state: &str,
    ) -> Result<String, ProviderError> {
        let parsed = Url::parse(callback_url).map_err(|err| ProviderError::InvalidConfig {
            provider: "github-copilot".to_string(),
            detail: format!("invalid callback url: {err}"),
        })?;

        let mut code: Option<String> = None;
        let mut state: Option<String> = None;

        for (key, value) in parsed.query_pairs() {
            match key.as_ref() {
                "code" => code = Some(value.to_string()),
                "state" => state = Some(value.to_string()),
                _ => {}
            }
        }

        let state = state.ok_or_else(|| ProviderError::InvalidConfig {
            provider: "github-copilot".to_string(),
            detail: "missing oauth state".to_string(),
        })?;
        if state != expected_state {
            return Err(ProviderError::InvalidConfig {
                provider: "github-copilot".to_string(),
                detail: "oauth state mismatch".to_string(),
            });
        }

        code.ok_or_else(|| ProviderError::InvalidConfig {
            provider: "github-copilot".to_string(),
            detail: "missing oauth code".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn token_retrieval_from_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::remove_var("GITHUB_TOKEN");
            std::env::set_var("GITHUB_COPILOT_TOKEN", "test-token-123");
        }
        let token = retrieve_copilot_token();
        assert_eq!(token, Some("test-token-123".to_string()));
        unsafe {
            std::env::remove_var("GITHUB_COPILOT_TOKEN");
        }
    }

    #[test]
    fn token_fallback_to_github_token() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::remove_var("GITHUB_COPILOT_TOKEN");
            std::env::set_var("GITHUB_TOKEN", "gh-token-456");
        }
        let token = retrieve_copilot_token();
        assert_eq!(token, None);
        unsafe {
            std::env::remove_var("GITHUB_TOKEN");
        }
    }

    #[test]
    fn auth_url_contains_required_parameters() {
        let flow = CopilotOAuthFlow::new(CopilotOAuthConfig {
            client_id: "client123".to_string(),
            redirect_uri: "http://localhost:8787/callback".to_string(),
            scopes: vec!["read:user".to_string(), "user:email".to_string()],
        });

        let url = flow.begin("state-1").expect("auth url");
        assert!(url.contains("client_id=client123"));
        assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A8787%2Fcallback"));
        assert!(url.contains("scope=read%3Auser+user%3Aemail"));
        assert!(url.contains("state=state-1"));
    }

    #[test]
    fn callback_parser_validates_state() {
        let flow = CopilotOAuthFlow::new(CopilotOAuthConfig {
            client_id: "client123".to_string(),
            redirect_uri: "http://localhost:8787/callback".to_string(),
            scopes: vec![],
        });

        let callback = "http://localhost:8787/callback?code=abc123&state=state-1";
        let code = flow.parse_callback_code(callback, "state-1").expect("valid callback");
        assert_eq!(code, "abc123");

        let mismatch = flow.parse_callback_code(callback, "other-state");
        assert!(mismatch.is_err());
    }

    #[test]
    fn model_surface_routes_gpt5_to_responses() {
        assert_eq!(model_api_surface("gpt-5"), CopilotApiSurface::Responses);
        assert_eq!(
            endpoint_for_model("https://api.githubcopilot.com", "gpt-5"),
            "https://api.githubcopilot.com/v1/responses"
        );

        assert_eq!(model_api_surface("gpt-4o-mini"), CopilotApiSurface::ChatCompletions);
    }
}
