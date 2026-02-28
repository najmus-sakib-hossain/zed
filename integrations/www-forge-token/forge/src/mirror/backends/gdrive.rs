//! Google Drive backend — resumable upload to user's own Drive.
use crate::mirror::{auth::AuthStore, MirrorBackend, MirrorError, MirrorMetadata, MirrorTarget, MediaType};
use async_trait::async_trait;
use std::sync::Arc;

pub struct GoogleDriveBackend {
    auth: Arc<AuthStore>,
}

impl GoogleDriveBackend {
    pub fn new(auth: Arc<AuthStore>) -> Self { Self { auth } }
}

fn mime_for(filename: &str) -> &'static str {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "mp4" | "mkv" | "mov" | "avi" | "webm" => "video/mp4",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
}

#[async_trait]
impl MirrorBackend for GoogleDriveBackend {
    fn name(&self) -> &'static str { "gdrive" }

    fn can_handle(&self, _: &MediaType) -> bool { true } // catch-all

    async fn upload(&self, data: Vec<u8>, meta: &MirrorMetadata) -> Result<MirrorTarget, MirrorError> {
        let bundle = self
            .auth
            .load("gdrive")
            .map_err(|e| MirrorError::Upload(e.to_string()))?
            .ok_or(MirrorError::AuthMissing("gdrive"))?;

        let client = reqwest::Client::new();
        let mime = mime_for(&meta.filename);

        // Initiate resumable upload
        let metadata = serde_json::json!({ "name": meta.filename });
        let init = client
            .post(
                "https://www.googleapis.com/upload/drive/v3/files\
                 ?uploadType=resumable",
            )
            .bearer_auth(&bundle.access_token)
            .header("X-Upload-Content-Type", mime)
            .header("X-Upload-Content-Length", data.len().to_string())
            .json(&metadata)
            .send()
            .await?;

        let upload_url = init
            .headers()
            .get("Location")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| MirrorError::Upload("no Location header from Drive".into()))?
            .to_string();

        let resp = client
            .put(&upload_url)
            .bearer_auth(&bundle.access_token)
            .header("Content-Type", mime)
            .body(data)
            .send()
            .await?;

        if !resp.status().is_success() {
            let msg = resp.text().await.unwrap_or_default();
            return Err(MirrorError::Upload(format!("Drive upload failed: {msg}")));
        }

        let json: serde_json::Value = resp.json().await?;
        let file_id = json["id"]
            .as_str()
            .ok_or_else(|| MirrorError::Upload("no id in Drive response".into()))?
            .to_string();

        tracing::info!(
            "Google Drive ✓  https://drive.google.com/file/d/{file_id}/view"
        );
        Ok(MirrorTarget::GoogleDrive { file_id })
    }
}
