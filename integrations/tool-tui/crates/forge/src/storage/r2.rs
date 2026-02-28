/// Cloudflare R2 Storage Backend
///
/// This module provides integration with Cloudflare R2 for blob storage.
/// Zero egress fees make it perfect for code hosting platforms.
use anyhow::{Context, Result};
use reqwest::{Client, StatusCode, header};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::time::Duration;

use super::blob::Blob;

/// R2 configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct R2Config {
    /// R2 account ID
    pub account_id: String,

    /// R2 bucket name
    pub bucket_name: String,

    /// R2 access key ID
    pub access_key_id: String,

    /// R2 secret access key
    pub secret_access_key: String,

    /// Custom domain (optional)
    pub custom_domain: Option<String>,
}

impl R2Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let account_id = std::env::var("R2_ACCOUNT_ID").context("R2_ACCOUNT_ID not set in .env")?;
        let bucket_name =
            std::env::var("R2_BUCKET_NAME").context("R2_BUCKET_NAME not set in .env")?;
        let access_key_id =
            std::env::var("R2_ACCESS_KEY_ID").context("R2_ACCESS_KEY_ID not set in .env")?;
        let secret_access_key = std::env::var("R2_SECRET_ACCESS_KEY")
            .context("R2_SECRET_ACCESS_KEY not set in .env")?;
        let custom_domain = std::env::var("R2_CUSTOM_DOMAIN").ok().filter(|s| !s.is_empty());

        Ok(Self {
            account_id,
            bucket_name,
            access_key_id,
            secret_access_key,
            custom_domain,
        })
    }

    /// Get R2 endpoint URL (account-based)
    pub fn endpoint_url(&self) -> String {
        if let Some(domain) = &self.custom_domain {
            format!("https://{}", domain)
        } else {
            format!("https://{}.r2.cloudflarestorage.com", self.account_id)
        }
    }

    /// Get full URL for a key (path-style: bucket in path, not hostname)
    fn get_url(&self, key: &str) -> String {
        format!("{}/{}/{}", self.endpoint_url(), self.bucket_name, key)
    }
}

/// R2 storage client
pub struct R2Storage {
    config: R2Config,
    client: Client,
}

impl R2Storage {
    /// Create new R2 storage client
    pub fn new(config: R2Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client for R2 storage")?;

        Ok(Self { config, client })
    }

    /// Upload blob to R2
    pub async fn upload_blob(&self, blob: &Blob) -> Result<String> {
        let hash = blob.hash();
        let key = format!("blobs/{}/{}", &hash[..2], &hash[2..]);

        let binary = blob.to_binary().context("Failed to serialize blob for upload")?;
        let content_hash = compute_sha256_hex(&binary);
        let date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

        let url = self.config.get_url(&key);

        // Create AWS Signature V4 (simplified - in production use aws-sigv4 crate)
        let authorization = self
            .create_auth_header("PUT", &key, &binary)
            .context("Failed to create authorization header for blob upload")?;

        let response = self
            .client
            .put(&url)
            .header(header::AUTHORIZATION, authorization)
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .header("x-amz-content-sha256", content_hash)
            .header("x-amz-date", date)
            .body(binary)
            .send()
            .await
            .context("Failed to send blob upload request to R2")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("R2 upload failed: {} - {}", status, body);
        }

        Ok(key)
    }

    /// Download blob from R2
    pub async fn download_blob(&self, hash: &str) -> Result<Blob> {
        let key = format!("blobs/{}/{}", &hash[..2], &hash[2..]);
        let date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

        let url = self.config.get_url(&key);

        // For GET requests, use empty body for signing
        let authorization = self
            .create_auth_header("GET", &key, b"")
            .context("Failed to create authorization header for blob download")?;

        let response = self
            .client
            .get(&url)
            .header(header::AUTHORIZATION, authorization)
            .header("x-amz-date", date)
            .header("x-amz-content-sha256", compute_sha256_hex(b""))
            .send()
            .await
            .context("Failed to send blob download request to R2")?;

        if response.status() == StatusCode::NOT_FOUND {
            anyhow::bail!("Blob not found: {}", hash);
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("R2 download failed: {} - {}", status, body);
        }

        let binary =
            response.bytes().await.context("Failed to read blob content from R2 response")?;
        Blob::from_binary(binary.as_ref())
    }

    /// Check if blob exists in R2
    pub async fn blob_exists(&self, hash: &str) -> Result<bool> {
        let key = format!("blobs/{}/{}", &hash[..2], &hash[2..]);
        let date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

        let url = self.config.get_url(&key);

        let authorization = self
            .create_auth_header("HEAD", &key, b"")
            .context("Failed to create authorization header for blob existence check")?;

        let response = self
            .client
            .head(&url)
            .header(header::AUTHORIZATION, authorization)
            .header("x-amz-date", date)
            .header("x-amz-content-sha256", compute_sha256_hex(b""))
            .send()
            .await
            .context("Failed to send blob existence check request to R2")?;

        Ok(response.status().is_success())
    }

    /// Delete blob from R2
    pub async fn delete_blob(&self, hash: &str) -> Result<()> {
        let key = format!("blobs/{}/{}", &hash[..2], &hash[2..]);
        let date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

        let url = self.config.get_url(&key);

        let authorization = self
            .create_auth_header("DELETE", &key, b"")
            .context("Failed to create authorization header for blob deletion")?;

        let response = self
            .client
            .delete(&url)
            .header(header::AUTHORIZATION, authorization)
            .header("x-amz-date", date)
            .header("x-amz-content-sha256", compute_sha256_hex(b""))
            .send()
            .await
            .context("Failed to send blob deletion request to R2")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("R2 delete failed: {} - {}", status, body);
        }

        Ok(())
    }

    /// Download component from R2
    pub async fn download_component(
        &self,
        tool: &str,
        component: &str,
        version: Option<&str>,
    ) -> Result<String> {
        let version = version.unwrap_or("latest");
        let key = format!("components/{}/{}/{}.tsx", tool, version, component);
        let date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

        let url = self.config.get_url(&key);

        let authorization = self
            .create_auth_header("GET", &key, b"")
            .context("Failed to create authorization header for component download")?;

        let response = self
            .client
            .get(&url)
            .header(header::AUTHORIZATION, authorization)
            .header("x-amz-date", date)
            .header("x-amz-content-sha256", compute_sha256_hex(b""))
            .send()
            .await
            .context("Failed to send component download request to R2")?;

        if response.status() == StatusCode::NOT_FOUND {
            anyhow::bail!("Component not found: {}/{} v{}", tool, component, version);
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("R2 component download failed: {} - {}", status, body);
        }

        let content = response
            .text()
            .await
            .context("Failed to read component content from R2 response")?;
        Ok(content)
    }

    /// Upload component to R2
    pub async fn upload_component(
        &self,
        tool: &str,
        component: &str,
        version: &str,
        content: &str,
    ) -> Result<String> {
        let key = format!("components/{}/{}/{}.tsx", tool, version, component);
        let binary = content.as_bytes();
        let content_hash = compute_sha256_hex(binary);
        let date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

        let url = self.config.get_url(&key);

        let authorization = self
            .create_auth_header("PUT", &key, binary)
            .context("Failed to create authorization header for component upload")?;

        let response = self
            .client
            .put(&url)
            .header(header::AUTHORIZATION, authorization)
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .header("x-amz-content-sha256", content_hash)
            .header("x-amz-date", date)
            .body(content.to_string())
            .send()
            .await
            .context("Failed to send component upload request to R2")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("R2 component upload failed: {} - {}", status, body);
        }

        Ok(key)
    }

    /// Check if component exists in R2
    pub async fn component_exists(
        &self,
        tool: &str,
        component: &str,
        version: Option<&str>,
    ) -> Result<bool> {
        let version = version.unwrap_or("latest");
        let key = format!("components/{}/{}/{}.tsx", tool, version, component);
        let date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

        let url = self.config.get_url(&key);

        let authorization = self
            .create_auth_header("HEAD", &key, b"")
            .context("Failed to create authorization header for component existence check")?;

        let response = self
            .client
            .head(&url)
            .header(header::AUTHORIZATION, authorization)
            .header("x-amz-date", date)
            .header("x-amz-content-sha256", compute_sha256_hex(b""))
            .send()
            .await
            .context("Failed to send component existence check request to R2")?;

        Ok(response.status().is_success())
    }

    /// List all components in R2
    pub async fn list_components(&self, tool: &str) -> Result<Vec<String>> {
        let prefix = format!("components/{}/", tool);
        let url = format!("{}/?list-type=2&prefix={}", self.config.endpoint_url(), prefix);

        let date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let authorization = self
            .create_auth_header("GET", &format!("?list-type=2&prefix={}", prefix), b"")
            .context("Failed to create authorization header for component listing")?;

        let response = self
            .client
            .get(&url)
            .header(header::AUTHORIZATION, authorization)
            .header("x-amz-date", date)
            .header("x-amz-content-sha256", compute_sha256_hex(b""))
            .send()
            .await
            .context("Failed to send component listing request to R2")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("R2 list failed: {} - {}", status, body);
        }

        // Parse XML response (simplified - in production use proper XML parser)
        let body = response
            .text()
            .await
            .context("Failed to read component listing response from R2")?;
        let mut components = Vec::new();

        for line in body.lines() {
            if line.contains("<Key>") {
                let key = line.replace("<Key>", "").replace("</Key>", "").trim().to_string();
                if let Some(name) = key.split('/').next_back() {
                    if let Some(component_name) = name.strip_suffix(".tsx") {
                        components.push(component_name.to_string());
                    }
                }
            }
        }

        Ok(components)
    }

    /// Sync components (bidirectional)
    pub async fn sync_components(
        &self,
        tool: &str,
        local_components: &[String],
        on_download: impl Fn(&str),
        on_upload: impl Fn(&str),
    ) -> Result<()> {
        // 1. List remote components
        let remote_components = self
            .list_components(tool)
            .await
            .with_context(|| format!("Failed to list remote components for tool: {}", tool))?;

        // 2. Calculate sync actions
        let (to_download, to_upload) =
            self.calculate_sync_actions(&remote_components, local_components);

        // 3. Execute actions
        for remote in to_download {
            on_download(&remote);
        }

        for local in to_upload {
            on_upload(&local);
        }

        Ok(())
    }

    /// Calculate what needs to be downloaded and uploaded
    /// Returns (to_download, to_upload)
    #[cfg_attr(test, allow(dead_code))]
    pub(crate) fn calculate_sync_actions(
        &self,
        remote_components: &[String],
        local_components: &[String],
    ) -> (Vec<String>, Vec<String>) {
        let mut to_download = Vec::new();
        let mut to_upload = Vec::new();

        for remote in remote_components {
            if !local_components.contains(remote) {
                to_download.push(remote.clone());
            }
        }

        for local in local_components {
            if !remote_components.contains(local) {
                to_upload.push(local.clone());
            }
        }

        (to_download, to_upload)
    }

    /// Create AWS Signature V4 authorization header (manual implementation)
    fn create_auth_header(&self, method: &str, key: &str, body: &[u8]) -> Result<String> {
        let now = chrono::Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date_stamp = now.format("%Y%m%d").to_string();

        // Parse URL
        let url = self.config.get_url(key);
        let parsed_url = url::Url::parse(&url).context("Failed to parse URL")?;
        let host = parsed_url.host_str().context("No host in URL")?;
        let canonical_uri = parsed_url.path();

        // Compute payload hash
        let payload_hash = compute_sha256_hex(body);

        // Step 1: Create canonical request
        let canonical_headers = format!(
            "host:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n",
            host, payload_hash, amz_date
        );
        let signed_headers = "host;x-amz-content-sha256;x-amz-date";

        let canonical_request = format!(
            "{}\n{}\n\n{}\n{}\n{}",
            method, canonical_uri, canonical_headers, signed_headers, payload_hash
        );

        // Step 2: Create string to sign
        let credential_scope = format!("{}/auto/s3/aws4_request", date_stamp);
        let canonical_request_hash = compute_sha256_hex(canonical_request.as_bytes());
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            amz_date, credential_scope, canonical_request_hash
        );

        // Step 3: Calculate signature
        let k_date = hmac_sha256(
            format!("AWS4{}", self.config.secret_access_key).as_bytes(),
            date_stamp.as_bytes(),
        );
        let k_region = hmac_sha256(&k_date, b"auto");
        let k_service = hmac_sha256(&k_region, b"s3");
        let k_signing = hmac_sha256(&k_service, b"aws4_request");
        let signature = hex::encode(hmac_sha256(&k_signing, string_to_sign.as_bytes()));

        // Step 4: Create authorization header
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            self.config.access_key_id, credential_scope, signed_headers, signature
        );

        Ok(authorization)
    }

    /// Sync local blobs up to R2 (upload missing blobs)
    pub async fn sync_up(
        &self,
        local_blobs: Vec<Blob>,
        progress_callback: Option<impl Fn(usize, usize) + Send + Sync>,
    ) -> Result<SyncResult> {
        use futures::stream::{self, StreamExt};

        tracing::info!("ðŸ”„ Starting R2 sync up: {} local blobs", local_blobs.len());

        let mut uploaded = 0;
        let mut skipped = 0;
        let mut errors = Vec::new();
        let total = local_blobs.len();

        // Check which blobs already exist in R2
        let mut to_upload = Vec::new();
        for blob in local_blobs {
            match self.blob_exists(blob.hash()).await {
                Ok(exists) => {
                    if exists {
                        skipped += 1;
                    } else {
                        to_upload.push(blob);
                    }
                }
                Err(e) => {
                    errors.push(format!("Failed to check blob {}: {}", blob.hash(), e));
                    to_upload.push(blob); // Try to upload anyway
                }
            }
        }

        // Upload missing blobs in parallel (max 10 concurrent)
        let mut stream = stream::iter(to_upload.into_iter().enumerate())
            .map(|(idx, blob)| async move {
                let hash = blob.hash();
                match self.upload_blob(&blob).await {
                    Ok(_) => Ok::<(usize, String), String>((idx, hash.to_string())),
                    Err(e) => Err(format!("Failed to upload {}: {}", hash, e)),
                }
            })
            .buffer_unordered(10);

        while let Some(result) = stream.next().await {
            match result {
                Ok((_idx, _hash)) => {
                    uploaded += 1;
                    if let Some(cb) = &progress_callback {
                        cb(uploaded + skipped, total);
                    }
                }
                Err(e) => {
                    errors.push(e);
                }
            }
        }

        tracing::info!(
            "âœ… Sync up complete: {} uploaded, {} skipped, {} errors",
            uploaded,
            skipped,
            errors.len()
        );

        Ok(SyncResult {
            uploaded,
            downloaded: 0,
            skipped,
            errors,
        })
    }

    /// Sync remote blobs down from R2 (download missing blobs)
    pub async fn sync_down(
        &self,
        remote_hashes: Vec<String>,
        progress_callback: Option<impl Fn(usize, usize) + Send + Sync>,
    ) -> Result<Vec<Blob>> {
        use futures::stream::{self, StreamExt};

        tracing::info!("ðŸ”„ Starting R2 sync down: {} remote blobs", remote_hashes.len());

        let total = remote_hashes.len();
        let mut downloaded_blobs = Vec::new();

        // Download blobs in parallel (max 10 concurrent)
        let mut stream = stream::iter(remote_hashes.into_iter().enumerate())
            .map(|(idx, hash)| async move {
                match self.download_blob(&hash).await {
                    Ok(blob) => Ok::<(usize, Blob), String>((idx, blob)),
                    Err(e) => Err(format!("Failed to download {}: {}", hash, e)),
                }
            })
            .buffer_unordered(10);

        let mut errors = Vec::new();
        while let Some(result) = stream.next().await {
            match result {
                Ok((idx, blob)) => {
                    downloaded_blobs.push(blob);
                    if let Some(cb) = &progress_callback {
                        cb(idx + 1, total);
                    }
                }
                Err(e) => {
                    tracing::warn!("âš ï¸ {}", e);
                    errors.push(e);
                }
            }
        }

        tracing::info!(
            "âœ… Sync down complete: {} downloaded, {} errors",
            downloaded_blobs.len(),
            errors.len()
        );

        Ok(downloaded_blobs)
    }

    /// List all blob hashes in R2 bucket (simplified - in production use pagination)
    pub async fn list_blobs(&self, _prefix: Option<&str>) -> Result<Vec<String>> {
        // This is a simplified version. In production, use S3 ListObjects API
        // For now, return empty list as listing requires more complex S3 API integration
        tracing::warn!("R2 list_blobs not fully implemented - requires S3 ListObjects API");
        Ok(Vec::new())
    }
}

/// Sync operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub uploaded: usize,
    pub downloaded: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

/// Compute SHA-256 hex string
fn compute_sha256_hex(data: &[u8]) -> String {
    use sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// HMAC-SHA256 helper
fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// Batch upload blobs with progress tracking
pub async fn batch_upload_blobs(
    storage: &R2Storage,
    blobs: Vec<Blob>,
    progress_callback: impl Fn(usize, usize),
) -> Result<Vec<String>> {
    use futures::stream::{self, StreamExt};

    let total = blobs.len();
    let mut keys = Vec::with_capacity(total);

    // Upload in parallel (max 10 concurrent)
    let mut stream = stream::iter(blobs.into_iter().enumerate())
        .map(|(idx, blob)| async move {
            let key = storage.upload_blob(&blob).await?;
            Ok::<(usize, String), anyhow::Error>((idx, key))
        })
        .buffer_unordered(10);

    while let Some(result) = stream.next().await {
        let (idx, key) = result?;
        keys.push(key);
        progress_callback(idx + 1, total);
    }

    Ok(keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_r2_config() {
        let config = R2Config {
            account_id: "test-account".to_string(),
            bucket_name: "forge-blobs".to_string(),
            access_key_id: "test-key".to_string(),
            secret_access_key: "test-secret".to_string(),
            custom_domain: None,
        };

        assert!(config.endpoint_url().contains("test-account"));
        assert!(config.endpoint_url().contains("r2.cloudflarestorage.com"));
    }

    #[test]
    fn test_sync_calculation() {
        let config = R2Config::default();
        let storage = R2Storage::new(config).unwrap();

        let remote = vec!["comp1.tsx".to_string(), "comp2.tsx".to_string()];
        let local = vec!["comp2.tsx".to_string(), "comp3.tsx".to_string()];

        let (download, upload) = storage.calculate_sync_actions(&remote, &local);

        assert_eq!(download, vec!["comp1.tsx".to_string()]);
        assert_eq!(upload, vec!["comp3.tsx".to_string()]);
    }

    #[test]
    fn test_sync_empty() {
        let config = R2Config::default();
        let storage = R2Storage::new(config).unwrap();

        let remote = vec![];
        let local = vec![];

        let (download, upload) = storage.calculate_sync_actions(&remote, &local);

        assert!(download.is_empty());
        assert!(upload.is_empty());
    }
}
