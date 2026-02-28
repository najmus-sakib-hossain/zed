//! GitHub backend — pushes files into a private repo via the Contents REST API.
//! Uses reqwest directly to avoid fighting octocrab builder version differences.
use crate::mirror::{auth::AuthStore, MirrorBackend, MirrorError, MirrorMetadata, MirrorTarget, MediaType};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use std::sync::Arc;

pub struct GitHubBackend {
    auth: Arc<AuthStore>,
    /// "owner/repo"
    repo: String,
}

impl GitHubBackend {
    pub fn new(auth: Arc<AuthStore>, repo: String) -> Self {
        Self { auth, repo }
    }
}

#[async_trait]
impl MirrorBackend for GitHubBackend {
    fn name(&self) -> &'static str { "github" }

    fn can_handle(&self, media_type: &MediaType) -> bool {
        matches!(
            media_type,
            MediaType::Code | MediaType::Document | MediaType::Archive | MediaType::Unknown
        )
    }

    async fn upload(&self, data: Vec<u8>, meta: &MirrorMetadata) -> Result<MirrorTarget, MirrorError> {
        let bundle = self
            .auth
            .load("github")
            .map_err(|e| MirrorError::Upload(e.to_string()))?
            .ok_or(MirrorError::AuthMissing("github"))?;

        let (owner, repo_name) = self
            .repo
            .split_once('/')
            .ok_or_else(|| MirrorError::Upload(format!("invalid repo: {}", self.repo)))?;

        let file_path = format!("forge-mirror/{}", meta.filename);
        let api_url = format!(
            "https://api.github.com/repos/{}/{}/contents/{}",
            owner, repo_name, file_path
        );

        let client = reqwest::Client::new();

        // Check if file already exists (to get SHA for update)
        let existing_sha: Option<String> = {
            let r = client
                .get(&api_url)
                .bearer_auth(&bundle.access_token)
                .header("User-Agent", "forge/0.1")
                .send()
                .await;
            match r {
                Ok(resp) if resp.status().is_success() => {
                    resp.json::<serde_json::Value>()
                        .await
                        .ok()
                        .and_then(|j| j["sha"].as_str().map(|s| s.to_string()))
                }
                _ => None,
            }
        };

        let content = STANDARD.encode(&data);
        let mut body = serde_json::json!({
            "message": format!("forge mirror: {}", meta.filename),
            "content": content,
        });

        if let Some(sha) = existing_sha {
            body["sha"] = serde_json::Value::String(sha);
        }

        let resp = client
            .put(&api_url)
            .bearer_auth(&bundle.access_token)
            .header("User-Agent", "forge/0.1")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let msg = resp.text().await.unwrap_or_default();
            return Err(MirrorError::Upload(format!("GitHub upload failed: {msg}")));
        }

        tracing::info!(
            "GitHub ✓  https://github.com/{}/{}/blob/main/{}",
            owner, repo_name, file_path
        );
        Ok(MirrorTarget::GitHub {
            repo: self.repo.clone(),
            path: file_path,
        })
    }
}
