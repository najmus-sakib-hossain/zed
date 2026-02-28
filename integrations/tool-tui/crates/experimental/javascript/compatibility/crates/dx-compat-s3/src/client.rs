//! S3 client implementation.
//!
//! Provides S3-compatible object storage with support for:
//! - AWS S3, Cloudflare R2, MinIO, and other S3-compatible services
//! - Presigned URL generation
//! - Multipart uploads for large files

use crate::error::{S3Error, S3Result};
use crate::file::S3File;
use bytes::Bytes;
use s3::creds::Credentials;
use s3::{Bucket, Region};
use std::time::Duration;

/// S3 client configuration.
#[derive(Debug, Clone)]
pub struct S3Config {
    /// AWS access key ID
    pub access_key_id: String,
    /// AWS secret access key
    pub secret_access_key: String,
    /// Custom endpoint (for R2, MinIO, etc.)
    pub endpoint: Option<String>,
    /// AWS region
    pub region: Option<String>,
    /// Bucket name
    pub bucket: String,
    /// Session token (optional)
    pub session_token: Option<String>,
}

impl S3Config {
    /// Create a new S3 configuration.
    pub fn new(bucket: impl Into<String>) -> Self {
        Self {
            access_key_id: String::new(),
            secret_access_key: String::new(),
            endpoint: None,
            region: None,
            bucket: bucket.into(),
            session_token: None,
        }
    }

    /// Set credentials.
    pub fn with_credentials(
        mut self,
        access_key_id: impl Into<String>,
        secret_access_key: impl Into<String>,
    ) -> Self {
        self.access_key_id = access_key_id.into();
        self.secret_access_key = secret_access_key.into();
        self
    }

    /// Set custom endpoint (for R2, MinIO, etc.).
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Set region.
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }
}

/// S3 client for object storage operations.
pub struct S3Client {
    bucket: Box<Bucket>,
}

impl S3Client {
    /// Create a new S3 client.
    pub async fn new(config: S3Config) -> S3Result<Self> {
        let credentials = Credentials::new(
            Some(&config.access_key_id),
            Some(&config.secret_access_key),
            config.session_token.as_deref(),
            None,
            None,
        )
        .map_err(|e| S3Error::Config(e.to_string()))?;

        let region = if let Some(endpoint) = &config.endpoint {
            Region::Custom {
                region: config.region.clone().unwrap_or_else(|| "us-east-1".to_string()),
                endpoint: endpoint.clone(),
            }
        } else {
            Region::from_str(&config.region.clone().unwrap_or_else(|| "us-east-1".to_string()))
        };

        let bucket = Bucket::new(&config.bucket, region, credentials)
            .map_err(|e| S3Error::Config(e.to_string()))?
            .with_path_style();

        Ok(Self { bucket })
    }

    /// Get a file handle for the given key.
    pub fn file(&self, key: &str) -> S3File {
        S3File::new(self.bucket.clone(), key.to_string())
    }

    /// Check if an object exists.
    pub async fn exists(&self, key: &str) -> S3Result<bool> {
        match self.bucket.head_object(key).await {
            Ok((_, code)) => Ok(code == 200),
            Err(e) => {
                // 404 means not found
                if e.to_string().contains("404") {
                    Ok(false)
                } else {
                    Err(S3Error::Network(e.to_string()))
                }
            }
        }
    }

    /// Delete an object.
    pub async fn delete(&self, key: &str) -> S3Result<()> {
        self.bucket
            .delete_object(key)
            .await
            .map_err(|e| S3Error::Network(e.to_string()))?;
        Ok(())
    }

    /// List objects with optional prefix.
    pub async fn list(&self, prefix: Option<&str>) -> S3Result<Vec<S3ObjectInfo>> {
        let results = self
            .bucket
            .list(prefix.unwrap_or("").to_string(), None)
            .await
            .map_err(|e| S3Error::Network(e.to_string()))?;

        let mut objects = Vec::new();
        for result in results {
            for obj in result.contents {
                objects.push(S3ObjectInfo {
                    key: obj.key,
                    size: obj.size,
                    last_modified: 0, // rust-s3 returns string, would need parsing
                    etag: obj.e_tag,
                });
            }
        }

        Ok(objects)
    }

    /// Write data to an object.
    pub async fn write(&self, key: &str, data: impl Into<Bytes>) -> S3Result<()> {
        let bytes: Bytes = data.into();
        self.bucket
            .put_object(key, &bytes)
            .await
            .map_err(|e| S3Error::Network(e.to_string()))?;
        Ok(())
    }

    /// Generate a presigned URL for downloading.
    pub async fn presign_get(&self, key: &str, expires_in: Duration) -> S3Result<String> {
        let url = self
            .bucket
            .presign_get(key, expires_in.as_secs() as u32, None)
            .await
            .map_err(|e| S3Error::Network(e.to_string()))?;
        Ok(url)
    }

    /// Generate a presigned URL for uploading.
    pub async fn presign_put(&self, key: &str, expires_in: Duration) -> S3Result<String> {
        let url = self
            .bucket
            .presign_put(key, expires_in.as_secs() as u32, None, None)
            .await
            .map_err(|e| S3Error::Network(e.to_string()))?;
        Ok(url)
    }

    /// Start a multipart upload for large files.
    pub async fn create_multipart_upload(&self, key: &str) -> S3Result<MultipartUpload> {
        let response = self
            .bucket
            .initiate_multipart_upload(key, "application/octet-stream")
            .await
            .map_err(|e| S3Error::Network(e.to_string()))?;

        Ok(MultipartUpload {
            bucket: self.bucket.clone(),
            key: key.to_string(),
            upload_id: response.upload_id,
            parts: Vec::new(),
        })
    }
}

/// Helper to convert region string to Region enum.
trait RegionExt {
    fn from_str(s: &str) -> Region;
}

impl RegionExt for Region {
    fn from_str(s: &str) -> Region {
        match s {
            "us-east-1" => Region::UsEast1,
            "us-east-2" => Region::UsEast2,
            "us-west-1" => Region::UsWest1,
            "us-west-2" => Region::UsWest2,
            "eu-west-1" => Region::EuWest1,
            "eu-west-2" => Region::EuWest2,
            "eu-west-3" => Region::EuWest3,
            "eu-central-1" => Region::EuCentral1,
            "ap-northeast-1" => Region::ApNortheast1,
            "ap-northeast-2" => Region::ApNortheast2,
            "ap-southeast-1" => Region::ApSoutheast1,
            "ap-southeast-2" => Region::ApSoutheast2,
            "ap-south-1" => Region::ApSouth1,
            "sa-east-1" => Region::SaEast1,
            _ => Region::Custom {
                region: s.to_string(),
                endpoint: format!("https://s3.{}.amazonaws.com", s),
            },
        }
    }
}

/// Information about an S3 object.
#[derive(Debug, Clone)]
pub struct S3ObjectInfo {
    /// Object key
    pub key: String,
    /// Object size in bytes
    pub size: u64,
    /// Last modified timestamp (Unix seconds)
    pub last_modified: i64,
    /// ETag (optional)
    pub etag: Option<String>,
}

/// Multipart upload handle.
pub struct MultipartUpload {
    bucket: Box<Bucket>,
    key: String,
    upload_id: String,
    parts: Vec<s3::serde_types::Part>,
}

impl MultipartUpload {
    /// Upload a part.
    ///
    /// Part numbers must be between 1 and 10,000.
    /// Each part (except the last) must be at least 5MB.
    pub async fn upload_part(&mut self, part_number: i32, data: impl Into<Bytes>) -> S3Result<()> {
        let bytes: Bytes = data.into();

        let part = self
            .bucket
            .put_multipart_chunk(
                bytes.to_vec(),
                &self.key,
                part_number as u32,
                &self.upload_id,
                "application/octet-stream",
            )
            .await
            .map_err(|e| S3Error::Network(e.to_string()))?;

        self.parts.push(part);
        Ok(())
    }

    /// Complete the multipart upload.
    pub async fn complete(self) -> S3Result<()> {
        self.bucket
            .complete_multipart_upload(&self.key, &self.upload_id, self.parts)
            .await
            .map_err(|e| S3Error::Network(e.to_string()))?;
        Ok(())
    }

    /// Abort the multipart upload.
    pub async fn abort(self) -> S3Result<()> {
        self.bucket
            .abort_upload(&self.key, &self.upload_id)
            .await
            .map_err(|e| S3Error::Network(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = S3Config::new("my-bucket")
            .with_credentials("access_key", "secret_key")
            .with_endpoint("http://localhost:9000")
            .with_region("us-west-2");

        assert_eq!(config.bucket, "my-bucket");
        assert_eq!(config.access_key_id, "access_key");
        assert_eq!(config.secret_access_key, "secret_key");
        assert_eq!(config.endpoint, Some("http://localhost:9000".to_string()));
        assert_eq!(config.region, Some("us-west-2".to_string()));
    }
}
