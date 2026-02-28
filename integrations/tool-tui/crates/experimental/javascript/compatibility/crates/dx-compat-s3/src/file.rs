//! S3 file handle implementation.
//!
//! Provides lazy-loading file operations similar to Bun.file().

use crate::error::{S3Error, S3Result};
use bytes::Bytes;
use s3::Bucket;

/// S3 file handle for lazy-loading object operations.
///
/// Similar to Bun.file() but for S3 objects.
pub struct S3File {
    bucket: Box<Bucket>,
    key: String,
}

impl S3File {
    /// Create a new S3 file handle.
    pub(crate) fn new(bucket: Box<Bucket>, key: String) -> Self {
        Self { bucket, key }
    }

    /// Get the object key.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Get the bucket name.
    pub fn bucket_name(&self) -> String {
        self.bucket.name()
    }

    /// Check if the object exists.
    pub async fn exists(&self) -> S3Result<bool> {
        match self.bucket.head_object(&self.key).await {
            Ok((_, code)) => Ok(code == 200),
            Err(e) => {
                if e.to_string().contains("404") {
                    Ok(false)
                } else {
                    Err(S3Error::Network(e.to_string()))
                }
            }
        }
    }

    /// Get object size in bytes.
    pub async fn size(&self) -> S3Result<u64> {
        let (head, code) = self
            .bucket
            .head_object(&self.key)
            .await
            .map_err(|e| S3Error::Network(e.to_string()))?;

        if code == 404 {
            return Err(S3Error::NoSuchKey(self.key.clone()));
        }

        Ok(head.content_length.unwrap_or(0) as u64)
    }

    /// Get content type.
    pub async fn content_type(&self) -> S3Result<Option<String>> {
        let (head, _) = self
            .bucket
            .head_object(&self.key)
            .await
            .map_err(|e| S3Error::Network(e.to_string()))?;

        Ok(head.content_type)
    }

    /// Read file as text (UTF-8).
    pub async fn text(&self) -> S3Result<String> {
        let bytes = self.array_buffer().await?;
        String::from_utf8(bytes).map_err(|e| S3Error::Network(format!("Invalid UTF-8: {}", e)))
    }

    /// Read file as bytes.
    pub async fn array_buffer(&self) -> S3Result<Vec<u8>> {
        let response = self.bucket.get_object(&self.key).await.map_err(|e| {
            if e.to_string().contains("NoSuchKey") || e.to_string().contains("404") {
                S3Error::NoSuchKey(self.key.clone())
            } else {
                S3Error::Network(e.to_string())
            }
        })?;

        Ok(response.to_vec())
    }

    /// Read file as JSON.
    pub async fn json<T: serde::de::DeserializeOwned>(&self) -> S3Result<T> {
        let text = self.text().await?;
        serde_json::from_str(&text).map_err(|e| S3Error::Network(format!("Invalid JSON: {}", e)))
    }

    /// Read a range of bytes.
    pub async fn slice(&self, start: u64, end: u64) -> S3Result<Vec<u8>> {
        let response = self
            .bucket
            .get_object_range(&self.key, start, Some(end))
            .await
            .map_err(|e| S3Error::Network(e.to_string()))?;

        Ok(response.to_vec())
    }

    /// Write data to the object.
    pub async fn write(&self, data: impl Into<Bytes>) -> S3Result<()> {
        let bytes: Bytes = data.into();
        self.bucket
            .put_object(&self.key, &bytes)
            .await
            .map_err(|e| S3Error::Network(e.to_string()))?;
        Ok(())
    }

    /// Write text to the object.
    pub async fn write_text(&self, text: &str) -> S3Result<()> {
        self.write(Bytes::from(text.to_string())).await
    }

    /// Write JSON to the object.
    pub async fn write_json<T: serde::Serialize>(&self, value: &T) -> S3Result<()> {
        let json = serde_json::to_string(value)
            .map_err(|e| S3Error::Network(format!("JSON serialization error: {}", e)))?;
        self.write_text(&json).await
    }

    /// Delete the object.
    pub async fn delete(&self) -> S3Result<()> {
        self.bucket
            .delete_object(&self.key)
            .await
            .map_err(|e| S3Error::Network(e.to_string()))?;
        Ok(())
    }

    /// Copy to another key.
    pub async fn copy_to(&self, dest_key: &str) -> S3Result<S3File> {
        let source = format!("{}/{}", self.bucket.name(), self.key);

        self.bucket
            .copy_object_internal(&source, dest_key)
            .await
            .map_err(|e| S3Error::Network(e.to_string()))?;

        Ok(S3File::new(self.bucket.clone(), dest_key.to_string()))
    }
}

#[cfg(test)]
mod tests {
    // Note: These tests require a running S3-compatible service
    // They are marked as ignored by default

    #[test]
    fn test_file_key() {
        // Create a mock client for testing
        // In real tests, we'd use a mock or localstack
    }
}
