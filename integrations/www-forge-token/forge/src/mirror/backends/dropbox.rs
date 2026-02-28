//! Dropbox backend — simple upload API (≤150 MB). Large-file session TBD.
use crate::mirror::{auth::AuthStore, MirrorBackend, MirrorError, MirrorMetadata, MirrorTarget, MediaType};
use async_trait::async_trait;
use std::sync::Arc;

const SIMPLE_LIMIT: usize = 150 * 1024 * 1024;

pub struct DropboxBackend {
    auth: Arc<AuthStore>,
}

impl DropboxBackend {
    pub fn new(auth: Arc<AuthStore>) -> Self { Self { auth } }
}

#[async_trait]
impl MirrorBackend for DropboxBackend {
    fn name(&self) -> &'static str { "dropbox" }

    fn can_handle(&self, _: &MediaType) -> bool { true }

    async fn upload(&self, data: Vec<u8>, meta: &MirrorMetadata) -> Result<MirrorTarget, MirrorError> {
        let bundle = self
            .auth
            .load("dropbox")
            .map_err(|e| MirrorError::Upload(e.to_string()))?
            .ok_or(MirrorError::AuthMissing("dropbox"))?;

        if data.len() > SIMPLE_LIMIT {
            return Err(MirrorError::Upload(
                "file > 150 MB: Dropbox upload_session not yet implemented".into(),
            ));
        }

        let client = reqwest::Client::new();
        let path = format!("/forge-mirror/{}", meta.filename);

        let arg = serde_json::json!({
            "path": path,
            "mode": "overwrite",
            "autorename": true,
            "mute": true
        });

        let resp = client
            .post("https://content.dropboxapi.com/2/files/upload")
            .bearer_auth(&bundle.access_token)
            .header(
                "Dropbox-API-Arg",
                serde_json::to_string(&arg).unwrap(),
            )
            .header("Content-Type", "application/octet-stream")
            .body(data)
            .send()
            .await?;

        if !resp.status().is_success() {
            let msg = resp.text().await.unwrap_or_default();
            return Err(MirrorError::Upload(format!("Dropbox upload failed: {msg}")));
        }

        tracing::info!("Dropbox ✓  {path}");
        Ok(MirrorTarget::Dropbox { path })
    }
}
