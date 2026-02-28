//! R2 Client - S3-compatible API for Cloudflare R2
//!
//! Uses AWS SDK compatible signing for authentication

use anyhow::Result;
use std::path::PathBuf;
use std::time::SystemTime;

use super::R2Config;

/// R2 storage client
pub struct R2Client {
    config: R2Config,
}

/// S3 object metadata
#[derive(Debug, Clone)]
pub struct S3Object {
    pub key: String,
    pub size: u64,
    pub last_modified: SystemTime,
    pub etag: String,
    pub storage_class: String,
}

impl R2Client {
    /// Create new R2 client
    pub fn new(config: R2Config) -> Result<Self> {
        Ok(Self { config })
    }

    /// Upload a file to R2
    pub async fn upload_file(&self, local: &PathBuf, key: &str, compress: bool) -> Result<()> {
        let content = std::fs::read(local)?;

        let body = if compress {
            compress_data(&content)?
        } else {
            content
        };

        self.put_object(key, &body).await
    }

    /// Download a file from R2
    pub async fn download_file(&self, key: &str, local: &PathBuf) -> Result<()> {
        let body = self.get_object(key).await?;
        std::fs::write(local, body)?;
        Ok(())
    }

    /// Put object to R2
    pub async fn put_object(&self, key: &str, body: &[u8]) -> Result<()> {
        let url = format!("{}/{}/{}", self.config.endpoint, self.config.bucket, key);

        let client = reqwest::Client::new();
        let request = client
            .put(&url)
            .header("Content-Type", "application/octet-stream")
            .header("Content-Length", body.len())
            .body(body.to_vec());

        let request = self.sign_request(request, "PUT", key)?;

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("PUT failed: {} - {}", status, body));
        }

        Ok(())
    }

    /// Get object from R2
    pub async fn get_object(&self, key: &str) -> Result<Vec<u8>> {
        let url = format!("{}/{}/{}", self.config.endpoint, self.config.bucket, key);

        let client = reqwest::Client::new();
        let request = client.get(&url);
        let request = self.sign_request(request, "GET", key)?;

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("GET failed: {} - {}", status, body));
        }

        Ok(response.bytes().await?.to_vec())
    }

    /// Delete object from R2
    pub async fn delete_object(&self, key: &str) -> Result<()> {
        let url = format!("{}/{}/{}", self.config.endpoint, self.config.bucket, key);

        let client = reqwest::Client::new();
        let request = client.delete(&url);
        let request = self.sign_request(request, "DELETE", key)?;

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("DELETE failed: {} - {}", status, body));
        }

        Ok(())
    }

    /// List objects in bucket
    pub async fn list_objects(&self, prefix: &str, recursive: bool) -> Result<Vec<S3Object>> {
        let delimiter = if recursive { "" } else { "/" };
        let url = format!(
            "{}/{}?list-type=2&prefix={}&delimiter={}",
            self.config.endpoint,
            self.config.bucket,
            urlencoding::encode(prefix),
            delimiter
        );

        let client = reqwest::Client::new();
        let request = client.get(&url);
        let request = self.sign_request(request, "GET", "")?;

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("LIST failed: {} - {}", status, body));
        }

        let body = response.text().await?;
        parse_list_response(&body)
    }

    /// Head bucket (test connection)
    pub async fn head_bucket(&self) -> Result<()> {
        let url = format!("{}/{}", self.config.endpoint, self.config.bucket);

        let client = reqwest::Client::new();
        let request = client.head(&url);
        let request = self.sign_request(request, "HEAD", "")?;

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(anyhow::anyhow!("HEAD failed: {}", status));
        }

        Ok(())
    }

    /// Get object metadata
    pub async fn head_object(&self, key: &str) -> Result<S3Object> {
        let url = format!("{}/{}/{}", self.config.endpoint, self.config.bucket, key);

        let client = reqwest::Client::new();
        let request = client.head(&url);
        let request = self.sign_request(request, "HEAD", key)?;

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(anyhow::anyhow!("HEAD failed: {}", status));
        }

        let headers = response.headers();

        Ok(S3Object {
            key: key.to_string(),
            size: headers
                .get("content-length")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            last_modified: SystemTime::now(), // TODO: Parse from header
            etag: headers
                .get("etag")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .trim_matches('"')
                .to_string(),
            storage_class: "STANDARD".to_string(),
        })
    }

    /// Sign request with AWS Signature Version 4
    fn sign_request(
        &self,
        request: reqwest::RequestBuilder,
        method: &str,
        key: &str,
    ) -> Result<reqwest::RequestBuilder> {
        use chrono::Utc;
        use hmac::{Hmac, Mac};
        use sha2::{Digest, Sha256};

        let now = Utc::now();
        let date_stamp = now.format("%Y%m%d").to_string();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();

        let host = self
            .config
            .endpoint
            .trim_start_matches("https://")
            .trim_start_matches("http://");

        // Create canonical request
        let canonical_uri = format!("/{}/{}", self.config.bucket, key);
        let canonical_querystring = "";
        let canonical_headers = format!("host:{}\nx-amz-date:{}\n", host, amz_date);
        let signed_headers = "host;x-amz-date";
        let payload_hash = "UNSIGNED-PAYLOAD";

        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            method,
            canonical_uri,
            canonical_querystring,
            canonical_headers,
            signed_headers,
            payload_hash
        );

        // Create string to sign
        let algorithm = "AWS4-HMAC-SHA256";
        let credential_scope = format!("{}/{}/s3/aws4_request", date_stamp, self.config.region);

        let mut hasher = Sha256::new();
        hasher.update(canonical_request.as_bytes());
        let canonical_request_hash = hex::encode(hasher.finalize());

        let string_to_sign = format!(
            "{}\n{}\n{}\n{}",
            algorithm, amz_date, credential_scope, canonical_request_hash
        );

        // Calculate signature
        type HmacSha256 = Hmac<Sha256>;

        let k_date =
            HmacSha256::new_from_slice(format!("AWS4{}", self.config.secret_key).as_bytes())?
                .chain_update(date_stamp.as_bytes())
                .finalize()
                .into_bytes();

        let k_region = HmacSha256::new_from_slice(&k_date)?
            .chain_update(self.config.region.as_bytes())
            .finalize()
            .into_bytes();

        let k_service = HmacSha256::new_from_slice(&k_region)?
            .chain_update(b"s3")
            .finalize()
            .into_bytes();

        let k_signing = HmacSha256::new_from_slice(&k_service)?
            .chain_update(b"aws4_request")
            .finalize()
            .into_bytes();

        let signature = HmacSha256::new_from_slice(&k_signing)?
            .chain_update(string_to_sign.as_bytes())
            .finalize();

        let signature_hex = hex::encode(signature.into_bytes());

        // Create authorization header
        let authorization = format!(
            "{} Credential={}/{}, SignedHeaders={}, Signature={}",
            algorithm, self.config.access_key, credential_scope, signed_headers, signature_hex
        );

        Ok(request
            .header("Authorization", authorization)
            .header("x-amz-date", amz_date)
            .header("x-amz-content-sha256", payload_hash))
    }
}

/// Parse ListObjectsV2 response
fn parse_list_response(xml: &str) -> Result<Vec<S3Object>> {
    let mut objects = vec![];

    // Simple XML parsing (should use proper XML parser)
    for content in xml.split("<Contents>").skip(1) {
        if let Some(end) = content.find("</Contents>") {
            let content = &content[..end];

            let key = extract_xml_value(content, "Key").unwrap_or_default();
            let size: u64 =
                extract_xml_value(content, "Size").and_then(|s| s.parse().ok()).unwrap_or(0);
            let etag = extract_xml_value(content, "ETag")
                .unwrap_or_default()
                .trim_matches('"')
                .to_string();
            let storage_class = extract_xml_value(content, "StorageClass")
                .unwrap_or_else(|| "STANDARD".to_string());

            objects.push(S3Object {
                key,
                size,
                last_modified: SystemTime::now(), // TODO: Parse from XML
                etag,
                storage_class,
            });
        }
    }

    Ok(objects)
}

fn extract_xml_value(xml: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{}>", tag);
    let end_tag = format!("</{}>", tag);

    let start = xml.find(&start_tag)? + start_tag.len();
    let end = xml.find(&end_tag)?;

    Some(xml[start..end].to_string())
}

/// Compress data using gzip
fn compress_data(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}

/// Decompress gzip data
pub fn decompress_data(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::read::GzDecoder;
    use std::io::Read;

    let mut decoder = GzDecoder::new(data);
    let mut result = Vec::new();
    decoder.read_to_end(&mut result)?;
    Ok(result)
}
