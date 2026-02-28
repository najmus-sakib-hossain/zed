pub mod auth;
pub mod dispatcher;
pub mod media_type;

pub mod backends {
    pub mod dropbox;
    pub mod gdrive;
    pub mod github;
    pub mod mega;
    pub mod pinterest;
    pub mod r2;
    pub mod sketchfab;
    pub mod soundcloud;
    pub mod youtube;
}

pub use dispatcher::{MirrorDispatcher, MirrorResult};
pub use media_type::MediaType;

use async_trait::async_trait;

#[derive(Debug, Clone)]
pub enum MirrorTarget {
    YouTube { video_id: String },
    Pinterest { pin_id: String },
    SoundCloud { track_id: String },
    Sketchfab { model_id: String },
    GitHub { repo: String, path: String },
    GoogleDrive { file_id: String },
    Dropbox { path: String },
    Mega { handle: String },
    R2 { bucket: String, key: String },
}

impl MirrorTarget {
    pub fn public_url(&self) -> String {
        match self {
            MirrorTarget::YouTube { video_id } =>
                format!("https://www.youtube.com/watch?v={video_id}"),
            MirrorTarget::Pinterest { pin_id } =>
                format!("https://www.pinterest.com/pin/{pin_id}/"),
            MirrorTarget::SoundCloud { track_id } =>
                format!("https://soundcloud.com/track/{track_id}"),
            MirrorTarget::Sketchfab { model_id } =>
                format!("https://sketchfab.com/models/{model_id}"),
            MirrorTarget::GitHub { repo, path } =>
                format!("https://github.com/{repo}/blob/main/{path}"),
            MirrorTarget::GoogleDrive { file_id } =>
                format!("https://drive.google.com/file/d/{file_id}/view"),
            MirrorTarget::Dropbox { path } =>
                format!("https://www.dropbox.com/home{path}"),
            MirrorTarget::Mega { handle } =>
                format!("https://mega.nz/file/{handle}"),
            MirrorTarget::R2 { bucket, key } =>
                format!("r2://{bucket}/{key}"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MirrorError {
    #[error("upload failed: {0}")]
    Upload(String),
    #[error("auth missing for backend: {0}")]
    AuthMissing(&'static str),
    #[error("unsupported media type")]
    UnsupportedMediaType,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
}

pub struct MirrorMetadata {
    pub filename: String,
    pub media_type: MediaType,
    pub description: Option<String>,
}

#[async_trait]
pub trait MirrorBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn can_handle(&self, media_type: &MediaType) -> bool;
    async fn upload(
        &self,
        data: Vec<u8>,
        meta: &MirrorMetadata,
    ) -> Result<MirrorTarget, MirrorError>;
}
