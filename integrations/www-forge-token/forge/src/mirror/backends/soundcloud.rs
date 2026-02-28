//! SoundCloud backend — multipart upload as a private track.
use crate::mirror::{auth::AuthStore, MirrorBackend, MirrorError, MirrorMetadata, MirrorTarget, MediaType};
use async_trait::async_trait;
use std::sync::Arc;

pub struct SoundCloudBackend {
    auth: Arc<AuthStore>,
}

impl SoundCloudBackend {
    pub fn new(auth: Arc<AuthStore>) -> Self { Self { auth } }
}

#[async_trait]
impl MirrorBackend for SoundCloudBackend {
    fn name(&self) -> &'static str { "soundcloud" }

    fn can_handle(&self, media_type: &MediaType) -> bool {
        matches!(media_type, MediaType::Audio)
    }

    async fn upload(&self, data: Vec<u8>, meta: &MirrorMetadata) -> Result<MirrorTarget, MirrorError> {
        let bundle = self
            .auth
            .load("soundcloud")
            .map_err(|e| MirrorError::Upload(e.to_string()))?
            .ok_or(MirrorError::AuthMissing("soundcloud"))?;

        let client = reqwest::Client::new();

        let ext = std::path::Path::new(&meta.filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp3")
            .to_lowercase();
        let mime = match ext.as_str() {
            "wav"  => "audio/wav",
            "flac" => "audio/flac",
            "ogg"  => "audio/ogg",
            _      => "audio/mpeg",
        };

        let part = reqwest::multipart::Part::bytes(data)
            .file_name(meta.filename.clone())
            .mime_str(mime)
            .map_err(|e| MirrorError::Upload(e.to_string()))?;

        let form = reqwest::multipart::Form::new()
            .part("track[asset_data]", part)
            .text("track[title]", meta.filename.clone())
            .text("track[sharing]", "private");

        let resp = client
            .post("https://api.soundcloud.com/tracks")
            .bearer_auth(&bundle.access_token)
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            let msg = resp.text().await.unwrap_or_default();
            return Err(MirrorError::Upload(format!("SoundCloud upload failed: {msg}")));
        }

        let json: serde_json::Value = resp.json().await?;
        let track_id = json["id"].to_string();

        tracing::info!("SoundCloud ✓  track id {track_id}  (private)");
        Ok(MirrorTarget::SoundCloud { track_id })
    }
}
