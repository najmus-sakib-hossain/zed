//! R2 / S3-compatible backend via object_store.
use crate::mirror::{auth::AuthStore, MirrorBackend, MirrorError, MirrorMetadata, MirrorTarget, MediaType};
use async_trait::async_trait;
use bytes::Bytes;
use object_store::{aws::AmazonS3Builder, path::Path as ObjPath, ObjectStore};
use std::sync::Arc;

pub struct R2Backend {
    auth: Arc<AuthStore>,
    bucket: String,
    endpoint: String,
}

impl R2Backend {
    pub fn new(auth: Arc<AuthStore>, bucket: String, endpoint: String) -> Self {
        Self { auth, bucket, endpoint }
    }
}

#[async_trait]
impl MirrorBackend for R2Backend {
    fn name(&self) -> &'static str { "r2" }

    fn can_handle(&self, _: &MediaType) -> bool { true }

    async fn upload(&self, data: Vec<u8>, meta: &MirrorMetadata) -> Result<MirrorTarget, MirrorError> {
        let bundle = self
            .auth
            .load("r2")
            .map_err(|e| MirrorError::Upload(e.to_string()))?
            .ok_or(MirrorError::AuthMissing("r2"))?;

        let access_key = bundle
            .extra["access_key_id"]
            .as_str()
            .ok_or_else(|| MirrorError::Upload("r2: missing access_key_id".into()))?
            .to_string();
        let secret_key = bundle
            .extra["secret_access_key"]
            .as_str()
            .ok_or_else(|| MirrorError::Upload("r2: missing secret_access_key".into()))?
            .to_string();

        let store = AmazonS3Builder::new()
            .with_bucket_name(&self.bucket)
            .with_endpoint(&self.endpoint)
            .with_access_key_id(&access_key)
            .with_secret_access_key(&secret_key)
            .build()
            .map_err(|e| MirrorError::Upload(e.to_string()))?;

        let key = format!("forge-mirror/{}", meta.filename);
        let path = ObjPath::from(key.as_str());

        store
            .put(&path, Bytes::from(data).into())
            .await
            .map_err(|e| MirrorError::Upload(e.to_string()))?;

        tracing::info!("R2 ✓  r2://{}/{}", self.bucket, key);
        Ok(MirrorTarget::R2 {
            bucket: self.bucket.clone(),
            key,
        })
    }
}
