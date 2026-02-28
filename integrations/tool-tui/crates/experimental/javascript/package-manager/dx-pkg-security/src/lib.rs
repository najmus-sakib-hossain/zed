//! Security & Sandboxing for Dx Package Manager
//!
//! Provides:
//! - Capability-based permission system
//! - Path sandboxing
//! - Integrity verification (SHA-256, SHA-512, xxhash)
//! - Attack vector protection
//! - Checksum verification for package integrity

use anyhow::{bail, Result};
use dx_pkg_core::hash::ContentHash;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Security capabilities for package operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityCapabilities {
    /// Allowed file system paths (read)
    pub read_paths: HashSet<PathBuf>,
    /// Allowed file system paths (write)
    pub write_paths: HashSet<PathBuf>,
    /// Allowed network hosts
    pub network_hosts: HashSet<String>,
    /// Allow script execution
    pub allow_scripts: bool,
    /// Maximum package size (bytes)
    pub max_package_size: u64,
}

impl Default for SecurityCapabilities {
    fn default() -> Self {
        Self {
            read_paths: HashSet::new(),
            write_paths: HashSet::new(),
            network_hosts: HashSet::from_iter(vec!["registry.dx.dev".to_string()]),
            allow_scripts: false,
            max_package_size: 100 * 1024 * 1024, // 100MB
        }
    }
}

impl SecurityCapabilities {
    /// Create capabilities for installation
    pub fn for_install(install_dir: impl AsRef<Path>) -> Self {
        let mut caps = Self::default();
        caps.write_paths.insert(install_dir.as_ref().to_path_buf());
        caps.read_paths.insert(install_dir.as_ref().to_path_buf());
        caps
    }

    /// Check if path is allowed for reading
    pub fn can_read(&self, path: impl AsRef<Path>) -> bool {
        let path = path.as_ref();
        self.read_paths.iter().any(|allowed| path.starts_with(allowed))
    }

    /// Check if path is allowed for writing
    pub fn can_write(&self, path: impl AsRef<Path>) -> bool {
        let path = path.as_ref();
        self.write_paths.iter().any(|allowed| path.starts_with(allowed))
    }

    /// Check if network host is allowed
    pub fn can_access_network(&self, host: &str) -> bool {
        self.network_hosts.contains(host)
    }
}

/// Supported integrity hash algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    Sha256,
    Sha512,
    XxHash64,
    XxHash128,
}

impl HashAlgorithm {
    /// Parse algorithm from npm integrity string prefix
    pub fn from_integrity_prefix(s: &str) -> Option<Self> {
        if s.starts_with("sha512-") {
            Some(HashAlgorithm::Sha512)
        } else if s.starts_with("sha256-") {
            Some(HashAlgorithm::Sha256)
        } else if s.starts_with("xxhash64-") {
            Some(HashAlgorithm::XxHash64)
        } else if s.starts_with("xxhash128-") {
            Some(HashAlgorithm::XxHash128)
        } else {
            None
        }
    }
}

/// Package integrity information
#[derive(Debug, Clone)]
pub struct IntegrityInfo {
    /// Hash algorithm used
    pub algorithm: HashAlgorithm,
    /// Expected hash value (base64 encoded for SHA, hex for xxhash)
    pub hash: String,
}

impl IntegrityInfo {
    /// Parse npm-style integrity string (e.g., "sha512-abc123...")
    pub fn parse(integrity: &str) -> Option<Self> {
        let algorithm = HashAlgorithm::from_integrity_prefix(integrity)?;
        let hash = match algorithm {
            HashAlgorithm::Sha512 => integrity.strip_prefix("sha512-")?.to_string(),
            HashAlgorithm::Sha256 => integrity.strip_prefix("sha256-")?.to_string(),
            HashAlgorithm::XxHash64 => integrity.strip_prefix("xxhash64-")?.to_string(),
            HashAlgorithm::XxHash128 => integrity.strip_prefix("xxhash128-")?.to_string(),
        };
        Some(Self { algorithm, hash })
    }

    /// Create from SHA-512 hash
    pub fn sha512(hash: &str) -> Self {
        Self {
            algorithm: HashAlgorithm::Sha512,
            hash: hash.to_string(),
        }
    }

    /// Create from SHA-256 hash
    pub fn sha256(hash: &str) -> Self {
        Self {
            algorithm: HashAlgorithm::Sha256,
            hash: hash.to_string(),
        }
    }
}

/// Checksum verifier for package integrity
pub struct ChecksumVerifier;

impl ChecksumVerifier {
    /// Verify data against an integrity string
    pub fn verify(data: &[u8], integrity: &str) -> Result<bool> {
        let info = IntegrityInfo::parse(integrity)
            .ok_or_else(|| anyhow::anyhow!("Invalid integrity string format: {}", integrity))?;

        Self::verify_with_info(data, &info)
    }

    /// Verify data against integrity info
    pub fn verify_with_info(data: &[u8], info: &IntegrityInfo) -> Result<bool> {
        match info.algorithm {
            HashAlgorithm::Sha512 => Self::verify_sha512(data, &info.hash),
            HashAlgorithm::Sha256 => Self::verify_sha256(data, &info.hash),
            HashAlgorithm::XxHash64 => Self::verify_xxhash64(data, &info.hash),
            HashAlgorithm::XxHash128 => Self::verify_xxhash128(data, &info.hash),
        }
    }

    /// Verify SHA-512 hash (npm standard)
    pub fn verify_sha512(data: &[u8], expected_base64: &str) -> Result<bool> {
        let mut hasher = Sha512::new();
        hasher.update(data);
        let actual = hasher.finalize();

        // Decode expected hash from base64
        let expected = base64_decode(expected_base64)?;

        Ok(actual.as_slice() == expected.as_slice())
    }

    /// Verify SHA-256 hash
    pub fn verify_sha256(data: &[u8], expected_base64: &str) -> Result<bool> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let actual = hasher.finalize();

        // Decode expected hash from base64
        let expected = base64_decode(expected_base64)?;

        Ok(actual.as_slice() == expected.as_slice())
    }

    /// Verify xxhash64 (dx native format)
    pub fn verify_xxhash64(data: &[u8], expected_hex: &str) -> Result<bool> {
        let actual = dx_pkg_core::hash::xxhash64(data);
        let expected = u64::from_str_radix(expected_hex, 16)
            .map_err(|_| anyhow::anyhow!("Invalid hex hash: {}", expected_hex))?;

        Ok(actual == expected)
    }

    /// Verify xxhash128 (dx native format)
    pub fn verify_xxhash128(data: &[u8], expected_hex: &str) -> Result<bool> {
        let actual = dx_pkg_core::hash::xxhash128(data);
        let expected = u128::from_str_radix(expected_hex, 16)
            .map_err(|_| anyhow::anyhow!("Invalid hex hash: {}", expected_hex))?;

        Ok(actual == expected)
    }

    /// Compute SHA-512 hash and return as base64
    pub fn compute_sha512(data: &[u8]) -> String {
        let mut hasher = Sha512::new();
        hasher.update(data);
        let hash = hasher.finalize();
        base64_encode(&hash)
    }

    /// Compute SHA-256 hash and return as base64
    pub fn compute_sha256(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();
        base64_encode(&hash)
    }

    /// Compute integrity string (npm format)
    pub fn compute_integrity(data: &[u8]) -> String {
        format!("sha512-{}", Self::compute_sha512(data))
    }
}

/// Audit result for a package operation
#[derive(Debug, Clone)]
pub struct AuditResult {
    pub passed: bool,
    pub issues: Vec<SecurityIssue>,
    pub risk_score: u32, // 0-100
}

/// Security issue detected during audit
#[derive(Debug, Clone)]
pub struct SecurityIssue {
    pub severity: Severity,
    pub category: IssueCategory,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueCategory {
    PathTraversal,
    IntegrityViolation,
    ExcessiveSize,
    SuspiciousScript,
    UnauthorizedNetwork,
    ChecksumMismatch,
}

/// Security auditor for package operations
pub struct SecurityAuditor {
    capabilities: SecurityCapabilities,
}

impl SecurityAuditor {
    /// Create new auditor with capabilities
    pub fn new(capabilities: SecurityCapabilities) -> Self {
        Self { capabilities }
    }

    /// Audit package before installation
    pub fn audit_package(
        &self,
        path: &Path,
        _expected_hash: ContentHash,
        size: u64,
    ) -> Result<AuditResult> {
        let mut issues = Vec::new();
        let mut risk_score = 0;

        // Check 1: Path traversal protection
        if !self.is_safe_path(path) {
            issues.push(SecurityIssue {
                severity: Severity::Critical,
                category: IssueCategory::PathTraversal,
                description: format!("Path traversal attempt detected: {}", path.display()),
            });
            risk_score += 40;
        }

        // Check 2: Size limit
        if size > self.capabilities.max_package_size {
            issues.push(SecurityIssue {
                severity: Severity::High,
                category: IssueCategory::ExcessiveSize,
                description: format!(
                    "Package size {} exceeds limit {}",
                    size, self.capabilities.max_package_size
                ),
            });
            risk_score += 30;
        }

        // Check 3: Write permission
        if !self.capabilities.can_write(path) {
            issues.push(SecurityIssue {
                severity: Severity::High,
                category: IssueCategory::UnauthorizedNetwork,
                description: format!("No write permission for: {}", path.display()),
            });
            risk_score += 25;
        }

        let passed = risk_score < 50; // Threshold for blocking

        Ok(AuditResult {
            passed,
            issues,
            risk_score,
        })
    }

    /// Audit package with integrity verification
    pub fn audit_package_with_integrity(
        &self,
        path: &Path,
        data: &[u8],
        integrity: &str,
        size: u64,
    ) -> Result<AuditResult> {
        let mut issues = Vec::new();
        let mut risk_score = 0;

        // Check 1: Path traversal protection
        if !self.is_safe_path(path) {
            issues.push(SecurityIssue {
                severity: Severity::Critical,
                category: IssueCategory::PathTraversal,
                description: format!("Path traversal attempt detected: {}", path.display()),
            });
            risk_score += 40;
        }

        // Check 2: Size limit
        if size > self.capabilities.max_package_size {
            issues.push(SecurityIssue {
                severity: Severity::High,
                category: IssueCategory::ExcessiveSize,
                description: format!(
                    "Package size {} exceeds limit {}",
                    size, self.capabilities.max_package_size
                ),
            });
            risk_score += 30;
        }

        // Check 3: Integrity verification
        if !integrity.is_empty() {
            match ChecksumVerifier::verify(data, integrity) {
                Ok(true) => {
                    // Integrity check passed
                }
                Ok(false) => {
                    issues.push(SecurityIssue {
                        severity: Severity::Critical,
                        category: IssueCategory::ChecksumMismatch,
                        description: format!(
                            "Package integrity check failed. Expected: {}",
                            integrity
                        ),
                    });
                    risk_score += 50;
                }
                Err(e) => {
                    issues.push(SecurityIssue {
                        severity: Severity::High,
                        category: IssueCategory::IntegrityViolation,
                        description: format!("Could not verify integrity: {}", e),
                    });
                    risk_score += 35;
                }
            }
        }

        // Check 4: Write permission
        if !self.capabilities.can_write(path) {
            issues.push(SecurityIssue {
                severity: Severity::High,
                category: IssueCategory::UnauthorizedNetwork,
                description: format!("No write permission for: {}", path.display()),
            });
            risk_score += 25;
        }

        let passed = risk_score < 50;

        Ok(AuditResult {
            passed,
            issues,
            risk_score,
        })
    }

    /// Check if path is safe (no traversal attacks)
    fn is_safe_path(&self, path: &Path) -> bool {
        // Check for path traversal patterns
        let path_str = path.to_string_lossy();

        // Block obvious attacks
        if path_str.contains("..") || path_str.contains("~") {
            return false;
        }

        // Ensure path is within allowed directories
        self.capabilities.can_write(path)
    }

    /// Verify package integrity
    pub fn verify_integrity(&self, data: &[u8], expected: ContentHash) -> Result<()> {
        let actual = dx_pkg_core::hash::xxhash64(data);

        if u128::from(actual) != expected {
            bail!("Integrity check failed: expected {:016x}, got {:016x}", expected, actual);
        }

        Ok(())
    }

    /// Check network access permission
    pub fn check_network_access(&self, host: &str) -> Result<()> {
        if !self.capabilities.can_access_network(host) {
            bail!("Network access denied for host: {}", host);
        }
        Ok(())
    }
}

// Base64 encoding/decoding helpers
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::new();
    let mut i = 0;

    while i < data.len() {
        let b0 = data[i] as usize;
        let b1 = if i + 1 < data.len() {
            data[i + 1] as usize
        } else {
            0
        };
        let b2 = if i + 2 < data.len() {
            data[i + 2] as usize
        } else {
            0
        };

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if i + 1 < data.len() {
            result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if i + 2 < data.len() {
            result.push(ALPHABET[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }

        i += 3;
    }

    result
}

fn base64_decode(s: &str) -> Result<Vec<u8>> {
    const DECODE_TABLE: [i8; 128] = [
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 62, -1, -1,
        -1, 63, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, -1, -1, -1, -1, -1, -1, -1, 0, 1, 2, 3, 4,
        5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, -1, -1, -1,
        -1, -1, -1, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        46, 47, 48, 49, 50, 51, -1, -1, -1, -1, -1,
    ];

    let s = s.trim_end_matches('=');
    let mut result = Vec::with_capacity(s.len() * 3 / 4);
    let bytes: Vec<u8> = s.bytes().collect();

    let mut i = 0;
    while i + 3 < bytes.len() {
        let b0 = DECODE_TABLE[bytes[i] as usize] as u8;
        let b1 = DECODE_TABLE[bytes[i + 1] as usize] as u8;
        let b2 = DECODE_TABLE[bytes[i + 2] as usize] as u8;
        let b3 = DECODE_TABLE[bytes[i + 3] as usize] as u8;

        result.push((b0 << 2) | (b1 >> 4));
        result.push((b1 << 4) | (b2 >> 2));
        result.push((b2 << 6) | b3);

        i += 4;
    }

    // Handle remaining bytes
    if i + 1 < bytes.len() {
        let b0 = DECODE_TABLE[bytes[i] as usize] as u8;
        let b1 = DECODE_TABLE[bytes[i + 1] as usize] as u8;
        result.push((b0 << 2) | (b1 >> 4));

        if i + 2 < bytes.len() {
            let b2 = DECODE_TABLE[bytes[i + 2] as usize] as u8;
            result.push((b1 << 4) | (b2 >> 2));
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_capabilities() {
        let caps = SecurityCapabilities::default();
        assert!(!caps.allow_scripts);
        assert_eq!(caps.max_package_size, 100 * 1024 * 1024);
        assert!(caps.can_access_network("registry.dx.dev"));
        assert!(!caps.can_access_network("evil.com"));
    }

    #[test]
    fn test_install_capabilities() {
        let caps = SecurityCapabilities::for_install("./node_modules");
        assert!(caps.can_write("./node_modules"));
        assert!(caps.can_write("./node_modules/react"));
        assert!(!caps.can_write("../etc/passwd"));
    }

    #[test]
    fn test_path_traversal_detection() {
        let caps = SecurityCapabilities::for_install("./node_modules");
        let auditor = SecurityAuditor::new(caps);

        assert!(!auditor.is_safe_path(Path::new("../etc/passwd")));
        assert!(!auditor.is_safe_path(Path::new("~/secret")));
        assert!(auditor.is_safe_path(Path::new("./node_modules/react")));
    }

    #[test]
    fn test_size_limit() {
        let caps = SecurityCapabilities::default();
        let auditor = SecurityAuditor::new(caps);

        let result = auditor
            .audit_package(
                Path::new("./node_modules/huge"),
                0x1234567890abcdef,
                200 * 1024 * 1024, // 200MB (exceeds limit)
            )
            .unwrap();

        assert!(!result.passed);
        assert!(result.risk_score > 0);
        assert!(!result.issues.is_empty());
    }

    #[test]
    fn test_integrity_verification() {
        let caps = SecurityCapabilities::default();
        let auditor = SecurityAuditor::new(caps);

        let data = b"hello world";
        let hash = dx_pkg_core::hash::xxhash64(data);

        // Valid hash
        assert!(auditor.verify_integrity(data, hash.into()).is_ok());

        // Invalid hash
        assert!(auditor.verify_integrity(data, 0xdeadbeef).is_err());
    }

    #[test]
    fn test_checksum_sha256() {
        let data = b"test data for hashing";
        let hash = ChecksumVerifier::compute_sha256(data);

        // Verify the computed hash
        assert!(ChecksumVerifier::verify_sha256(data, &hash).unwrap());

        // Verify wrong data fails
        assert!(!ChecksumVerifier::verify_sha256(b"wrong data", &hash).unwrap());
    }

    #[test]
    fn test_checksum_sha512() {
        let data = b"test data for hashing";
        let hash = ChecksumVerifier::compute_sha512(data);

        // Verify the computed hash
        assert!(ChecksumVerifier::verify_sha512(data, &hash).unwrap());

        // Verify wrong data fails
        assert!(!ChecksumVerifier::verify_sha512(b"wrong data", &hash).unwrap());
    }

    #[test]
    fn test_integrity_string_parsing() {
        let integrity = "sha512-abc123";
        let info = IntegrityInfo::parse(integrity).unwrap();
        assert_eq!(info.algorithm, HashAlgorithm::Sha512);
        assert_eq!(info.hash, "abc123");

        let integrity = "sha256-xyz789";
        let info = IntegrityInfo::parse(integrity).unwrap();
        assert_eq!(info.algorithm, HashAlgorithm::Sha256);
        assert_eq!(info.hash, "xyz789");
    }

    #[test]
    fn test_compute_integrity() {
        let data = b"hello world";
        let integrity = ChecksumVerifier::compute_integrity(data);

        assert!(integrity.starts_with("sha512-"));
        assert!(ChecksumVerifier::verify(data, &integrity).unwrap());
    }

    #[test]
    fn test_base64_roundtrip() {
        let data = b"Hello, World!";
        let encoded = base64_encode(data);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(data.as_slice(), decoded.as_slice());
    }

    #[test]
    fn test_audit_with_integrity() {
        let caps = SecurityCapabilities::for_install("./node_modules");
        let auditor = SecurityAuditor::new(caps);

        let data = b"package content";
        let integrity = ChecksumVerifier::compute_integrity(data);

        // Valid integrity should pass
        let result = auditor
            .audit_package_with_integrity(
                Path::new("./node_modules/test"),
                data,
                &integrity,
                data.len() as u64,
            )
            .unwrap();

        assert!(result.passed);

        // Invalid integrity should fail
        let result = auditor
            .audit_package_with_integrity(
                Path::new("./node_modules/test"),
                b"different content",
                &integrity,
                15,
            )
            .unwrap();

        assert!(!result.passed);
        assert!(result.issues.iter().any(|i| i.category == IssueCategory::ChecksumMismatch));
    }

    #[test]
    fn test_security_capabilities_clone() {
        let caps = SecurityCapabilities::default();
        let cloned = caps.clone();
        assert_eq!(cloned.max_package_size, caps.max_package_size);
        assert_eq!(cloned.allow_scripts, caps.allow_scripts);
    }

    #[test]
    fn test_hash_algorithm_from_prefix() {
        assert_eq!(HashAlgorithm::from_integrity_prefix("sha512-abc"), Some(HashAlgorithm::Sha512));
        assert_eq!(HashAlgorithm::from_integrity_prefix("sha256-xyz"), Some(HashAlgorithm::Sha256));
        assert_eq!(HashAlgorithm::from_integrity_prefix("xxhash64-123"), Some(HashAlgorithm::XxHash64));
        assert_eq!(HashAlgorithm::from_integrity_prefix("xxhash128-456"), Some(HashAlgorithm::XxHash128));
        assert_eq!(HashAlgorithm::from_integrity_prefix("invalid-hash"), None);
    }

    #[test]
    fn test_integrity_info_constructors() {
        let sha512 = IntegrityInfo::sha512("test_hash");
        assert_eq!(sha512.algorithm, HashAlgorithm::Sha512);
        assert_eq!(sha512.hash, "test_hash");

        let sha256 = IntegrityInfo::sha256("another_hash");
        assert_eq!(sha256.algorithm, HashAlgorithm::Sha256);
        assert_eq!(sha256.hash, "another_hash");
    }

    #[test]
    fn test_xxhash_verification() {
        let data = b"test data for xxhash";
        let hash = dx_pkg_core::hash::xxhash64(data);
        let hex_hash = format!("{:016x}", hash);

        assert!(ChecksumVerifier::verify_xxhash64(data, &hex_hash).unwrap());
        assert!(!ChecksumVerifier::verify_xxhash64(b"wrong data", &hex_hash).unwrap());
    }

    #[test]
    fn test_xxhash128_verification() {
        let data = b"test data for xxhash128";
        let hash = dx_pkg_core::hash::xxhash128(data);
        let hex_hash = format!("{:032x}", hash);

        assert!(ChecksumVerifier::verify_xxhash128(data, &hex_hash).unwrap());
        assert!(!ChecksumVerifier::verify_xxhash128(b"wrong data", &hex_hash).unwrap());
    }

    #[test]
    fn test_audit_result_fields() {
        let result = AuditResult {
            passed: true,
            issues: vec![],
            risk_score: 0,
        };
        assert!(result.passed);
        assert!(result.issues.is_empty());
        assert_eq!(result.risk_score, 0);
    }

    #[test]
    fn test_security_issue_severity() {
        let issue = SecurityIssue {
            severity: Severity::Critical,
            category: IssueCategory::PathTraversal,
            description: "Test issue".to_string(),
        };
        assert_eq!(issue.severity, Severity::Critical);
        assert_eq!(issue.category, IssueCategory::PathTraversal);
    }

    #[test]
    fn test_network_access_check() {
        let caps = SecurityCapabilities::default();
        let auditor = SecurityAuditor::new(caps);

        assert!(auditor.check_network_access("registry.dx.dev").is_ok());
        assert!(auditor.check_network_access("evil.com").is_err());
    }

    #[test]
    fn test_empty_integrity_string() {
        let caps = SecurityCapabilities::for_install("./node_modules");
        let auditor = SecurityAuditor::new(caps);

        // Empty integrity string should pass (no verification)
        let result = auditor
            .audit_package_with_integrity(
                Path::new("./node_modules/test"),
                b"data",
                "",
                4,
            )
            .unwrap();

        assert!(result.passed);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: production-readiness, Property 23: Checksum Verification
        /// Validates: Requirements 23.2
        /// For any data, computing a checksum and verifying it should always succeed
        #[test]
        fn prop_checksum_sha512_roundtrip(data in prop::collection::vec(any::<u8>(), 0..10000)) {
            let hash = ChecksumVerifier::compute_sha512(&data);
            let result = ChecksumVerifier::verify_sha512(&data, &hash);
            prop_assert!(result.is_ok());
            prop_assert!(result.unwrap());
        }

        /// Feature: production-readiness, Property 23: Checksum Verification
        /// Validates: Requirements 23.2
        /// For any data, computing a SHA-256 checksum and verifying it should always succeed
        #[test]
        fn prop_checksum_sha256_roundtrip(data in prop::collection::vec(any::<u8>(), 0..10000)) {
            let hash = ChecksumVerifier::compute_sha256(&data);
            let result = ChecksumVerifier::verify_sha256(&data, &hash);
            prop_assert!(result.is_ok());
            prop_assert!(result.unwrap());
        }

        /// Feature: production-readiness, Property 23: Checksum Verification
        /// Validates: Requirements 23.2
        /// For any data, computing an integrity string and verifying it should always succeed
        #[test]
        fn prop_integrity_roundtrip(data in prop::collection::vec(any::<u8>(), 0..10000)) {
            let integrity = ChecksumVerifier::compute_integrity(&data);
            let result = ChecksumVerifier::verify(&data, &integrity);
            prop_assert!(result.is_ok());
            prop_assert!(result.unwrap());
        }

        /// Feature: production-readiness, Property 23: Checksum Verification
        /// Validates: Requirements 23.2
        /// For any two different data inputs, their checksums should be different
        #[test]
        fn prop_checksum_collision_resistance(
            data1 in prop::collection::vec(any::<u8>(), 1..1000),
            data2 in prop::collection::vec(any::<u8>(), 1..1000)
        ) {
            // Only test when data is actually different
            if data1 != data2 {
                let hash1 = ChecksumVerifier::compute_sha512(&data1);
                let hash2 = ChecksumVerifier::compute_sha512(&data2);
                prop_assert_ne!(hash1, hash2);
            }
        }

        /// Feature: production-readiness, Property 23: Checksum Verification
        /// Validates: Requirements 23.2
        /// Verifying data against wrong checksum should fail
        #[test]
        fn prop_checksum_detects_tampering(
            original in prop::collection::vec(any::<u8>(), 1..1000),
            tampered in prop::collection::vec(any::<u8>(), 1..1000)
        ) {
            // Only test when data is actually different
            if original != tampered {
                let hash = ChecksumVerifier::compute_sha512(&original);
                let result = ChecksumVerifier::verify_sha512(&tampered, &hash);
                prop_assert!(result.is_ok());
                prop_assert!(!result.unwrap());
            }
        }

        /// Feature: production-readiness, Property 23: Checksum Verification
        /// Validates: Requirements 23.2
        /// Base64 encoding/decoding should be a round-trip
        #[test]
        fn prop_base64_roundtrip(data in prop::collection::vec(any::<u8>(), 0..1000)) {
            let encoded = base64_encode(&data);
            let decoded = base64_decode(&encoded);
            prop_assert!(decoded.is_ok());
            prop_assert_eq!(data, decoded.unwrap());
        }
    }
}
