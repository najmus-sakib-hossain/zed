//! Cloud storage integrations

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageFile {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub mime_type: String,
    pub modified: i64,
}

pub trait StorageProvider {
    async fn upload(&self, path: &str, data: &[u8]) -> Result<String>;
    async fn download(&self, path: &str) -> Result<Vec<u8>>;
    async fn list(&self, path: &str) -> Result<Vec<StorageFile>>;
    async fn delete(&self, path: &str) -> Result<()>;
    async fn create_folder(&self, path: &str) -> Result<()>;
}

pub struct S3Storage {
    bucket: String,
    region: String,
    access_key: String,
    secret_key: String,
}

impl S3Storage {
    pub fn new(bucket: String, region: String, access_key: String, secret_key: String) -> Self {
        Self {
            bucket,
            region,
            access_key,
            secret_key,
        }
    }
}

pub struct GoogleDrive {
    api_key: String,
}

impl GoogleDrive {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

pub struct Dropbox {
    access_token: String,
}

impl Dropbox {
    pub fn new(access_token: String) -> Self {
        Self { access_token }
    }
}
