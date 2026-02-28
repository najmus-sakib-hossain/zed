//! Sketchfab backend — multipart upload as a private model.
use crate::mirror::{auth::AuthStore, MirrorBackend, MirrorError, MirrorMetadata, MirrorTarget, MediaType};
use async_trait::async_trait;
use std::sync::Arc;

pub struct SketchfabBackend {
    auth: Arc<AuthStore>,
}

impl SketchfabBackend {
    pub fn new(auth: Arc<AuthStore>) -> Self { Self { auth } }
}

#[async_trait]
impl MirrorBackend for SketchfabBackend {
    fn name(&self) -> &'static str { "sketchfab" }

    fn can_handle(&self, media_type: &MediaType) -> bool {
        matches!(media_type, MediaType::Model3D)
    }

    async fn upload(&self, data: Vec<u8>, meta: &MirrorMetadata) -> Result<MirrorTarget, MirrorError> {
        let bundle = self
            .auth
            .load("sketchfab")
            .map_err(|e| MirrorError::Upload(e.to_string()))?
            .ok_or(MirrorError::AuthMissing("sketchfab"))?;

        let client = reqwest::Client::new();

        let file_part = reqwest::multipart::Part::bytes(data)
            .file_name(meta.filename.clone());

        let form = reqwest::multipart::Form::new()
            .text("name", meta.filename.clone())
            .text("isPublished", "false")
            .text("private", "true")
            .part("modelFile", file_part);

        let resp = client
            .post("https://api.sketchfab.com/v3/models")
            .bearer_auth(&bundle.access_token)
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            let msg = resp.text().await.unwrap_or_default();
            return Err(MirrorError::Upload(format!("Sketchfab upload failed: {msg}")));
        }

        let json: serde_json::Value = resp.json().await?;
        let model_id = json["uid"]
            .as_str()
            .ok_or_else(|| MirrorError::Upload("no uid in Sketchfab response".into()))?
            .to_string();

        tracing::info!("Sketchfab ✓  https://sketchfab.com/models/{model_id}  (private)");
        Ok(MirrorTarget::Sketchfab { model_id })
    }
}
