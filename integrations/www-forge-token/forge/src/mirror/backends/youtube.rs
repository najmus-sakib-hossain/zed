//! YouTube backend — uploads as private draft using resumable upload API.
use crate::mirror::{auth::AuthStore, MirrorBackend, MirrorError, MirrorMetadata, MirrorTarget, MediaType};
use async_trait::async_trait;
use std::sync::Arc;

pub struct YouTubeBackend {
    auth: Arc<AuthStore>,
}

impl YouTubeBackend {
    pub fn new(auth: Arc<AuthStore>) -> Self {
        Self { auth }
    }
}

#[async_trait]
impl MirrorBackend for YouTubeBackend {
    fn name(&self) -> &'static str { "youtube" }

    fn can_handle(&self, media_type: &MediaType) -> bool {
        matches!(media_type, MediaType::Video)
    }

    async fn upload(&self, data: Vec<u8>, meta: &MirrorMetadata) -> Result<MirrorTarget, MirrorError> {
        let bundle = self
            .auth
            .load("youtube")
            .map_err(|e| MirrorError::Upload(e.to_string()))?
            .ok_or(MirrorError::AuthMissing("youtube"))?;

        let client = reqwest::Client::new();

        // Step 1: initiate resumable upload
        let init_url =
            "https://www.googleapis.com/upload/youtube/v3/videos\
             ?uploadType=resumable&part=snippet,status";

        let body = serde_json::json!({
            "snippet": {
                "title": meta.filename,
                "description": meta.description.as_deref().unwrap_or("Uploaded by Forge"),
                "categoryId": "22"
            },
            "status": { "privacyStatus": "private" }
        });

        let init = client
            .post(init_url)
            .bearer_auth(&bundle.access_token)
            .header("X-Upload-Content-Type", "video/*")
            .header("X-Upload-Content-Length", data.len().to_string())
            .json(&body)
            .send()
            .await?;

        let upload_url = init
            .headers()
            .get("Location")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| MirrorError::Upload("no Location header from YouTube".into()))?
            .to_string();

        // Step 2: upload bytes
        let resp = client
            .put(&upload_url)
            .bearer_auth(&bundle.access_token)
            .header("Content-Type", "video/*")
            .body(data)
            .send()
            .await?;

        if !resp.status().is_success() {
            let msg = resp.text().await.unwrap_or_default();
            return Err(MirrorError::Upload(format!("YouTube upload failed: {msg}")));
        }

        let json: serde_json::Value = resp.json().await?;
        let video_id = json["id"]
            .as_str()
            .ok_or_else(|| MirrorError::Upload("no id in YouTube response".into()))?
            .to_string();

        tracing::info!("YouTube ✓  https://youtu.be/{video_id}  (private)");
        Ok(MirrorTarget::YouTube { video_id })
    }
}
