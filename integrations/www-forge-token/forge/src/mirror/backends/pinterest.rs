//! Pinterest backend — creates a secret pin with base64 image data.
use crate::mirror::{auth::AuthStore, MirrorBackend, MirrorError, MirrorMetadata, MirrorTarget, MediaType};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use std::sync::Arc;

pub struct PinterestBackend {
    auth: Arc<AuthStore>,
}

impl PinterestBackend {
    pub fn new(auth: Arc<AuthStore>) -> Self { Self { auth } }
}

#[async_trait]
impl MirrorBackend for PinterestBackend {
    fn name(&self) -> &'static str { "pinterest" }

    fn can_handle(&self, media_type: &MediaType) -> bool {
        matches!(media_type, MediaType::Image)
    }

    async fn upload(&self, data: Vec<u8>, meta: &MirrorMetadata) -> Result<MirrorTarget, MirrorError> {
        let bundle = self
            .auth
            .load("pinterest")
            .map_err(|e| MirrorError::Upload(e.to_string()))?
            .ok_or(MirrorError::AuthMissing("pinterest"))?;

        let client = reqwest::Client::new();
        let b64 = STANDARD.encode(&data);

        let ext = std::path::Path::new(&meta.filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpeg")
            .to_lowercase();
        let content_type = if ext == "png" { "image/png" } else { "image/jpeg" };

        let body = serde_json::json!({
            "title": meta.filename,
            "description": meta.description.as_deref().unwrap_or("Uploaded by Forge"),
            "media_source": {
                "source_type": "image_base64",
                "content_type": content_type,
                "data": b64
            }
        });

        let resp = client
            .post("https://api.pinterest.com/v5/pins")
            .bearer_auth(&bundle.access_token)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let msg = resp.text().await.unwrap_or_default();
            return Err(MirrorError::Upload(format!("Pinterest upload failed: {msg}")));
        }

        let json: serde_json::Value = resp.json().await?;
        let pin_id = json["id"]
            .as_str()
            .ok_or_else(|| MirrorError::Upload("no id in Pinterest response".into()))?
            .to_string();

        tracing::info!("Pinterest ✓  https://www.pinterest.com/pin/{pin_id}/");
        Ok(MirrorTarget::Pinterest { pin_id })
    }
}
