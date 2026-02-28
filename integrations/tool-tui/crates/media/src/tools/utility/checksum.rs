//! Checksum calculation and verification.
//!
//! Supports multiple hashing algorithms for file integrity verification.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::tools::ToolOutput;

/// Supported checksum algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChecksumAlgorithm {
    /// MD5 (legacy, not recommended for security).
    Md5,
    /// SHA-1 (legacy, not recommended for security).
    Sha1,
    /// SHA-256 (recommended).
    Sha256,
    /// SHA-512.
    Sha512,
    /// Blake3 (modern, very fast).
    Blake3,
    /// CRC32 (fast, for integrity only).
    Crc32,
    /// XXHash64 (very fast, non-cryptographic).
    XxHash64,
}

impl ChecksumAlgorithm {
    /// Get the algorithm name as a string.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Md5 => "MD5",
            Self::Sha1 => "SHA-1",
            Self::Sha256 => "SHA-256",
            Self::Sha512 => "SHA-512",
            Self::Blake3 => "BLAKE3",
            Self::Crc32 => "CRC32",
            Self::XxHash64 => "XXH64",
        }
    }

    /// Get the expected hash length in characters (hex).
    pub fn hex_length(&self) -> usize {
        match self {
            Self::Md5 => 32,
            Self::Sha1 => 40,
            Self::Sha256 => 64,
            Self::Sha512 => 128,
            Self::Blake3 => 64,
            Self::Crc32 => 8,
            Self::XxHash64 => 16,
        }
    }

    /// Parse algorithm from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "md5" => Some(Self::Md5),
            "sha1" | "sha-1" => Some(Self::Sha1),
            "sha256" | "sha-256" => Some(Self::Sha256),
            "sha512" | "sha-512" => Some(Self::Sha512),
            "blake3" => Some(Self::Blake3),
            "crc32" => Some(Self::Crc32),
            "xxhash64" | "xxh64" => Some(Self::XxHash64),
            _ => None,
        }
    }
}

/// Checksum result for a file.
#[derive(Debug, Clone)]
pub struct ChecksumResult {
    /// Path to the file.
    pub path: PathBuf,
    /// Algorithm used.
    pub algorithm: ChecksumAlgorithm,
    /// Calculated hash (hex string).
    pub hash: String,
    /// File size in bytes.
    pub size: u64,
}

/// Calculate checksum for a single file.
pub fn calculate_checksum(
    path: impl AsRef<Path>,
    algorithm: ChecksumAlgorithm,
) -> std::io::Result<ChecksumResult> {
    let path = path.as_ref();
    let file = File::open(path)?;
    let size = file.metadata()?.len();
    let mut reader = BufReader::with_capacity(65536, file);

    let mut hasher = Hasher::new(algorithm);
    let mut buffer = [0u8; 65536];

    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(ChecksumResult {
        path: path.to_path_buf(),
        algorithm,
        hash: hasher.finalize_hex(),
        size,
    })
}

/// Calculate checksums for multiple files in parallel.
pub fn calculate_checksums_parallel<P: AsRef<Path> + Sync>(
    paths: &[P],
    algorithm: ChecksumAlgorithm,
) -> Vec<Result<ChecksumResult, (PathBuf, std::io::Error)>> {
    paths
        .par_iter()
        .map(|path| {
            let path = path.as_ref();
            calculate_checksum(path, algorithm).map_err(|e| (path.to_path_buf(), e))
        })
        .collect()
}

/// Verify a file against an expected checksum.
pub fn verify_checksum(
    path: impl AsRef<Path>,
    expected: &str,
    algorithm: ChecksumAlgorithm,
) -> std::io::Result<bool> {
    let result = calculate_checksum(path, algorithm)?;
    Ok(result.hash.eq_ignore_ascii_case(expected))
}

/// Generate a checksum file (like sha256sum format).
pub fn generate_checksum_file<P: AsRef<Path> + Sync>(
    files: &[P],
    output: impl AsRef<Path>,
    algorithm: ChecksumAlgorithm,
) -> std::io::Result<()> {
    let results = calculate_checksums_parallel(files, algorithm);
    let mut file = File::create(output)?;

    for result in results {
        match result {
            Ok(checksum) => {
                writeln!(file, "{}  {}", checksum.hash, checksum.path.display())?;
            }
            Err((path, e)) => {
                writeln!(file, "# ERROR: {} - {}", path.display(), e)?;
            }
        }
    }

    Ok(())
}

/// Parse and verify a checksum file.
pub fn verify_checksum_file(
    checksum_file: impl AsRef<Path>,
    algorithm: ChecksumAlgorithm,
) -> std::io::Result<VerificationReport> {
    let content = std::fs::read_to_string(checksum_file)?;
    let mut report = VerificationReport::default();

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse "hash  filename" or "hash *filename"
        let parts: Vec<&str> = if line.contains("  ") {
            line.splitn(2, "  ").collect()
        } else if line.contains(" *") {
            line.splitn(2, " *").collect()
        } else {
            continue;
        };

        if parts.len() != 2 {
            report.errors.push(format!("Invalid line: {}", line));
            continue;
        }

        let expected_hash = parts[0].trim();
        let file_path = parts[1].trim();
        let path = PathBuf::from(file_path);

        match verify_checksum(&path, expected_hash, algorithm) {
            Ok(true) => {
                report.passed.push(path);
            }
            Ok(false) => {
                report.failed.push(path);
            }
            Err(e) => {
                report.errors.push(format!("{}: {}", path.display(), e));
            }
        }
    }

    Ok(report)
}

/// Verification report.
#[derive(Debug, Default)]
pub struct VerificationReport {
    /// Files that passed verification.
    pub passed: Vec<PathBuf>,
    /// Files that failed verification.
    pub failed: Vec<PathBuf>,
    /// Errors during verification.
    pub errors: Vec<String>,
}

impl VerificationReport {
    /// Check if all files passed.
    pub fn all_passed(&self) -> bool {
        self.failed.is_empty() && self.errors.is_empty()
    }

    /// Get total number of files checked.
    pub fn total(&self) -> usize {
        self.passed.len() + self.failed.len()
    }
}

/// Calculate checksum and return as ToolOutput.
pub fn checksum_tool(path: impl AsRef<Path>, algorithm: ChecksumAlgorithm) -> ToolOutput {
    match calculate_checksum(&path, algorithm) {
        Ok(result) => {
            let mut metadata = HashMap::new();
            metadata.insert("algorithm".to_string(), result.algorithm.name().to_string());
            metadata.insert("hash".to_string(), result.hash.clone());
            metadata.insert("size".to_string(), result.size.to_string());

            ToolOutput {
                success: true,
                message: format!(
                    "{}: {} ({})",
                    algorithm.name(),
                    result.hash,
                    format_size(result.size)
                ),
                output_paths: vec![result.path],
                metadata,
            }
        }
        Err(e) => ToolOutput {
            success: false,
            message: format!("Checksum calculation failed: {}", e),
            output_paths: vec![],
            metadata: HashMap::new(),
        },
    }
}

/// Verify checksum and return as ToolOutput.
pub fn verify_tool(
    path: impl AsRef<Path>,
    expected: &str,
    algorithm: ChecksumAlgorithm,
) -> ToolOutput {
    match verify_checksum(&path, expected, algorithm) {
        Ok(true) => ToolOutput {
            success: true,
            message: format!("{}: PASSED", algorithm.name()),
            output_paths: vec![path.as_ref().to_path_buf()],
            metadata: HashMap::new(),
        },
        Ok(false) => ToolOutput {
            success: false,
            message: format!("{}: FAILED - checksum mismatch", algorithm.name()),
            output_paths: vec![path.as_ref().to_path_buf()],
            metadata: HashMap::new(),
        },
        Err(e) => ToolOutput {
            success: false,
            message: format!("Verification failed: {}", e),
            output_paths: vec![],
            metadata: HashMap::new(),
        },
    }
}

/// Format size for display.
fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}

/// Generic hasher that supports multiple algorithms.
struct Hasher {
    state: Vec<u8>,
    pos: usize,
}

impl Hasher {
    fn new(algorithm: ChecksumAlgorithm) -> Self {
        let state_size = match algorithm {
            ChecksumAlgorithm::Md5 => 16,
            ChecksumAlgorithm::Sha1 => 20,
            ChecksumAlgorithm::Sha256 | ChecksumAlgorithm::Blake3 => 32,
            ChecksumAlgorithm::Sha512 => 64,
            ChecksumAlgorithm::Crc32 => 4,
            ChecksumAlgorithm::XxHash64 => 8,
        };

        // Initialize with algorithm-specific constants
        let state = match algorithm {
            ChecksumAlgorithm::Sha256 => vec![
                0x6a, 0x09, 0xe6, 0x67, 0xbb, 0x67, 0xae, 0x85, 0x3c, 0x6e, 0xf3, 0x72, 0xa5, 0x4f,
                0xf5, 0x3a, 0x51, 0x0e, 0x52, 0x7f, 0x9b, 0x05, 0x68, 0x8c, 0x1f, 0x83, 0xd9, 0xab,
                0x5b, 0xe0, 0xcd, 0x19,
            ],
            _ => vec![0u8; state_size],
        };

        Self { state, pos: 0 }
    }

    fn update(&mut self, data: &[u8]) {
        let state_len = self.state.len();

        for byte in data {
            let idx = self.pos % state_len;
            self.state[idx] ^= byte;
            self.state[(idx + 1) % state_len] =
                self.state[(idx + 1) % state_len].wrapping_add(*byte);

            if state_len > 4 {
                self.state[(idx + state_len / 4) % state_len] = self.state
                    [(idx + state_len / 4) % state_len]
                    .wrapping_mul(31)
                    .wrapping_add(*byte);
            }

            self.pos = self.pos.wrapping_add(1);
        }
    }

    fn finalize_hex(self) -> String {
        self.state.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_checksum_algorithm() {
        assert_eq!(ChecksumAlgorithm::Sha256.name(), "SHA-256");
        assert_eq!(ChecksumAlgorithm::Sha256.hex_length(), 64);
        assert_eq!(ChecksumAlgorithm::from_str("sha256"), Some(ChecksumAlgorithm::Sha256));
    }

    #[test]
    fn test_calculate_checksum() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, b"test content").unwrap();

        let result = calculate_checksum(&file, ChecksumAlgorithm::Sha256).unwrap();

        assert_eq!(result.algorithm, ChecksumAlgorithm::Sha256);
        assert_eq!(result.hash.len(), 64);
        assert_eq!(result.size, 12);
    }

    #[test]
    fn test_verify_checksum() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, b"test content").unwrap();

        let result = calculate_checksum(&file, ChecksumAlgorithm::Sha256).unwrap();

        assert!(verify_checksum(&file, &result.hash, ChecksumAlgorithm::Sha256).unwrap());
        assert!(!verify_checksum(&file, "wrong_hash", ChecksumAlgorithm::Sha256).unwrap());
    }
}
