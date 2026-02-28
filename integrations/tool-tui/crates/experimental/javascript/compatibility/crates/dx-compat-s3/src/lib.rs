//! # dx-compat-s3
//!
//! S3-compatible object storage compatibility layer.
//!
//! Supports AWS S3, Cloudflare R2, MinIO, and other S3-compatible services.
//!
//! Features:
//! - Presigned URL generation
//! - Multipart uploads for large files
//! - Streaming reads and writes
//! - JSON serialization support

#![warn(missing_docs)]

mod client;
mod error;
mod file;

pub use client::{MultipartUpload, S3Client, S3Config, S3ObjectInfo};
pub use error::{S3Error, S3Result};
pub use file::S3File;
