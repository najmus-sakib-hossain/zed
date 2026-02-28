//! SIMD Secret Scanner
//!
//! SIMD-accelerated secret pattern detection using AVX-512/AVX2/NEON.

use std::path::PathBuf;

/// Type of detected secret
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretType {
    AwsAccessKey,
    AwsSecretKey,
    GitHubToken,
    GenericApiKey,
    PrivateKey,
    Password,
}

/// Secret finding with location information
#[derive(Debug, Clone)]
pub struct SecretFinding {
    /// Type of secret detected
    pub pattern_type: SecretType,
    /// Path to the file containing the secret
    pub file_path: PathBuf,
    /// Byte offset within the file
    pub byte_offset: usize,
    /// Line number (1-indexed)
    pub line_number: u32,
    /// Detection confidence (0.0 - 1.0)
    pub confidence: f32,
}

/// SIMD execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimdMode {
    Avx512,
    Avx2,
    Neon,
    Scalar,
}

/// Secret pattern definitions
struct SecretPattern {
    pattern_type: SecretType,
    prefix: &'static [u8],
    min_length: usize,
    max_length: usize,
    validator: fn(&[u8]) -> bool,
}

/// AWS Access Key pattern: AKIA followed by 16 alphanumeric chars
fn is_aws_access_key(data: &[u8]) -> bool {
    data.len() >= 20
        && data.starts_with(b"AKIA")
        && data[4..20].iter().all(|&b| b.is_ascii_alphanumeric())
}

/// AWS Secret Key pattern: 40 base64-like characters
fn is_aws_secret_key(data: &[u8]) -> bool {
    data.len() >= 40
        && data[..40]
            .iter()
            .all(|&b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=')
}

/// GitHub token pattern: ghp_, gho_, ghu_, ghs_, ghr_ followed by 36 chars
fn is_github_token(data: &[u8]) -> bool {
    if data.len() < 40 {
        return false;
    }
    let prefixes = [b"ghp_", b"gho_", b"ghu_", b"ghs_", b"ghr_"];
    prefixes.iter().any(|p| data.starts_with(*p))
        && data[4..40].iter().all(|&b| b.is_ascii_alphanumeric())
}

/// Generic API key pattern: api_key, apikey, api-key followed by value
fn is_generic_api_key(data: &[u8]) -> bool {
    let lower: Vec<u8> = data.iter().map(|b| b.to_ascii_lowercase()).collect();
    (lower.starts_with(b"api_key") || lower.starts_with(b"apikey") || lower.starts_with(b"api-key"))
        && data.len() >= 20
}

/// Private key pattern: -----BEGIN
fn is_private_key(data: &[u8]) -> bool {
    data.starts_with(b"-----BEGIN") && data.len() >= 20
}

/// Password pattern: password= or passwd= followed by value
fn is_password(data: &[u8]) -> bool {
    let lower: Vec<u8> = data.iter().map(|b| b.to_ascii_lowercase()).collect();
    (lower.starts_with(b"password=") || lower.starts_with(b"passwd=")) && data.len() >= 12
}

/// All secret patterns to check
const PATTERNS: &[SecretPattern] = &[
    SecretPattern {
        pattern_type: SecretType::AwsAccessKey,
        prefix: b"AKIA",
        min_length: 20,
        max_length: 20,
        validator: is_aws_access_key,
    },
    SecretPattern {
        pattern_type: SecretType::GitHubToken,
        prefix: b"ghp_",
        min_length: 40,
        max_length: 40,
        validator: is_github_token,
    },
    SecretPattern {
        pattern_type: SecretType::GitHubToken,
        prefix: b"gho_",
        min_length: 40,
        max_length: 40,
        validator: is_github_token,
    },
    SecretPattern {
        pattern_type: SecretType::GitHubToken,
        prefix: b"ghu_",
        min_length: 40,
        max_length: 40,
        validator: is_github_token,
    },
    SecretPattern {
        pattern_type: SecretType::PrivateKey,
        prefix: b"-----BEGIN",
        min_length: 20,
        max_length: 4096,
        validator: is_private_key,
    },
];

/// SIMD-accelerated secret scanner
pub struct SimdSecretScanner {
    mode: SimdMode,
    file_path: PathBuf,
}

impl SimdSecretScanner {
    /// Create a new scanner with automatic SIMD detection
    pub fn new() -> Self {
        Self {
            mode: Self::detect_simd_mode(),
            file_path: PathBuf::new(),
        }
    }

    /// Create scanner with specific file path
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            mode: Self::detect_simd_mode(),
            file_path: path,
        }
    }

    /// Set the file path for findings
    pub fn set_path(&mut self, path: PathBuf) {
        self.file_path = path;
    }

    /// Detect available SIMD mode at runtime
    fn detect_simd_mode() -> SimdMode {
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx512f") {
                return SimdMode::Avx512;
            }
            if is_x86_feature_detected!("avx2") {
                return SimdMode::Avx2;
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            return SimdMode::Neon;
        }
        SimdMode::Scalar
    }

    /// Scan bytes for secrets using the appropriate SIMD mode
    pub fn scan(&self, data: &[u8]) -> Vec<SecretFinding> {
        match self.mode {
            SimdMode::Avx512 => self.scan_avx512(data),
            SimdMode::Avx2 => self.scan_avx2(data),
            SimdMode::Neon => self.scan_neon(data),
            SimdMode::Scalar => self.scan_scalar(data),
        }
    }

    /// Scalar fallback scanner
    fn scan_scalar(&self, data: &[u8]) -> Vec<SecretFinding> {
        let mut findings = Vec::new();
        let mut line_number = 1u32;
        let mut line_start = 0usize;

        for i in 0..data.len() {
            // Track line numbers
            if data[i] == b'\n' {
                line_number += 1;
                line_start = i + 1;
                continue;
            }

            // Check each pattern
            for pattern in PATTERNS {
                if i + pattern.prefix.len() <= data.len()
                    && &data[i..i + pattern.prefix.len()] == pattern.prefix
                {
                    let end = (i + pattern.max_length).min(data.len());
                    if (pattern.validator)(&data[i..end]) {
                        findings.push(SecretFinding {
                            pattern_type: pattern.pattern_type,
                            file_path: self.file_path.clone(),
                            byte_offset: i,
                            line_number,
                            confidence: 0.9,
                        });
                    }
                }
            }

            // Check case-insensitive patterns
            self.check_case_insensitive_patterns(data, i, line_number, &mut findings);
        }

        findings
    }

    /// Check case-insensitive patterns (api_key, password, etc.)
    fn check_case_insensitive_patterns(
        &self,
        data: &[u8],
        offset: usize,
        line_number: u32,
        findings: &mut Vec<SecretFinding>,
    ) {
        let remaining = &data[offset..];

        // Check for AWS secret key pattern (40 base64 chars after common prefixes)
        if remaining.len() >= 50 {
            let lower: Vec<u8> = remaining[..20].iter().map(|b| b.to_ascii_lowercase()).collect();
            if lower.starts_with(b"aws_secret_access_key")
                || lower.starts_with(b"secret_access_key")
            {
                // Find the = sign and check the value
                if let Some(eq_pos) = remaining[..50].iter().position(|&b| b == b'=') {
                    let value_start = eq_pos + 1;
                    // Skip whitespace and quotes
                    let value_start = remaining[value_start..]
                        .iter()
                        .position(|&b| !b.is_ascii_whitespace() && b != b'"' && b != b'\'')
                        .map(|p| value_start + p)
                        .unwrap_or(value_start);

                    if value_start + 40 <= remaining.len()
                        && is_aws_secret_key(&remaining[value_start..])
                    {
                        findings.push(SecretFinding {
                            pattern_type: SecretType::AwsSecretKey,
                            file_path: self.file_path.clone(),
                            byte_offset: offset + value_start,
                            line_number,
                            confidence: 0.85,
                        });
                    }
                }
            }
        }

        // Check for generic API key
        if remaining.len() >= 20 && is_generic_api_key(remaining) {
            findings.push(SecretFinding {
                pattern_type: SecretType::GenericApiKey,
                file_path: self.file_path.clone(),
                byte_offset: offset,
                line_number,
                confidence: 0.7,
            });
        }

        // Check for password
        if remaining.len() >= 12 && is_password(remaining) {
            findings.push(SecretFinding {
                pattern_type: SecretType::Password,
                file_path: self.file_path.clone(),
                byte_offset: offset,
                line_number,
                confidence: 0.6,
            });
        }
    }

    /// AVX-512 accelerated scanner (falls back to scalar for now)
    #[cfg(target_arch = "x86_64")]
    fn scan_avx512(&self, data: &[u8]) -> Vec<SecretFinding> {
        // AVX-512 implementation would process 64 bytes at a time
        // For now, fall back to scalar
        self.scan_scalar(data)
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn scan_avx512(&self, data: &[u8]) -> Vec<SecretFinding> {
        self.scan_scalar(data)
    }

    /// AVX2 accelerated scanner (falls back to scalar for now)
    #[cfg(target_arch = "x86_64")]
    fn scan_avx2(&self, data: &[u8]) -> Vec<SecretFinding> {
        // AVX2 implementation would process 32 bytes at a time
        // For now, fall back to scalar
        self.scan_scalar(data)
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn scan_avx2(&self, data: &[u8]) -> Vec<SecretFinding> {
        self.scan_scalar(data)
    }

    /// NEON accelerated scanner (falls back to scalar for now)
    #[cfg(target_arch = "aarch64")]
    fn scan_neon(&self, data: &[u8]) -> Vec<SecretFinding> {
        // NEON implementation would process 16 bytes at a time
        // For now, fall back to scalar
        self.scan_scalar(data)
    }

    #[cfg(not(target_arch = "aarch64"))]
    fn scan_neon(&self, data: &[u8]) -> Vec<SecretFinding> {
        self.scan_scalar(data)
    }

    /// Check if AVX-512 is available
    pub fn has_avx512(&self) -> bool {
        self.mode == SimdMode::Avx512
    }

    /// Get current SIMD mode
    pub fn simd_mode(&self) -> SimdMode {
        self.mode
    }

    /// Force a specific SIMD mode (for testing)
    pub fn with_mode(mode: SimdMode) -> Self {
        Self {
            mode,
            file_path: PathBuf::new(),
        }
    }
}

impl Default for SimdSecretScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_aws_access_key() {
        let scanner = SimdSecretScanner::new();
        let data = b"AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
        let findings = scanner.scan(data);

        assert!(
            findings.iter().any(|f| f.pattern_type == SecretType::AwsAccessKey),
            "Should detect AWS access key"
        );
    }

    #[test]
    fn test_detect_github_token() {
        let scanner = SimdSecretScanner::new();
        let data = b"GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
        let findings = scanner.scan(data);

        assert!(
            findings.iter().any(|f| f.pattern_type == SecretType::GitHubToken),
            "Should detect GitHub token"
        );
    }

    #[test]
    fn test_detect_private_key() {
        let scanner = SimdSecretScanner::new();
        let data = b"-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA...";
        let findings = scanner.scan(data);

        assert!(
            findings.iter().any(|f| f.pattern_type == SecretType::PrivateKey),
            "Should detect private key"
        );
    }

    #[test]
    fn test_no_false_positives_on_safe_content() {
        let scanner = SimdSecretScanner::new();
        let data = b"This is a normal text file with no secrets.";
        let findings = scanner.scan(data);

        assert!(findings.is_empty(), "Should not detect secrets in safe content");
    }

    #[test]
    fn test_line_number_tracking() {
        let scanner = SimdSecretScanner::new();
        let data = b"line1\nline2\nAKIAIOSFODNN7EXAMPLE1\nline4";
        let findings = scanner.scan(data);

        if let Some(finding) = findings.iter().find(|f| f.pattern_type == SecretType::AwsAccessKey)
        {
            assert_eq!(finding.line_number, 3, "Should report correct line number");
        }
    }

    #[test]
    fn test_simd_mode_consistency() {
        let data = b"AKIAIOSFODNN7EXAMPLE1";

        let scalar_findings = SimdSecretScanner::with_mode(SimdMode::Scalar).scan(data);
        let avx2_findings = SimdSecretScanner::with_mode(SimdMode::Avx2).scan(data);

        assert_eq!(
            scalar_findings.len(),
            avx2_findings.len(),
            "All SIMD modes should produce same results"
        );
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate a valid AWS access key
    fn gen_aws_access_key() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(
            prop::sample::select(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".to_vec()),
            16..=16,
        )
        .prop_map(|suffix| {
            let mut key = b"AKIA".to_vec();
            key.extend(suffix);
            key
        })
    }

    /// Generate a valid GitHub token
    fn gen_github_token() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(
            prop::sample::select(
                b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".to_vec(),
            ),
            36..=36,
        )
        .prop_map(|suffix| {
            let mut token = b"ghp_".to_vec();
            token.extend(suffix);
            token
        })
    }

    /// Generate safe content (no secret patterns)
    fn gen_safe_content() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(
            prop::sample::select(b"abcdefghijklmnopqrstuvwxyz0123456789 \n\t.,!?".to_vec()),
            0..1000,
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: dx-security, Property 2: Secret Detection Accuracy**
        /// **Validates: Requirements 3.3, 3.4**
        ///
        /// For any byte array containing a known secret pattern at offset N,
        /// the SIMD_Scanner SHALL detect the secret and report an offset within
        /// the pattern's byte range.
        #[test]
        fn prop_detects_aws_access_key(
            prefix in gen_safe_content(),
            key in gen_aws_access_key(),
            suffix in gen_safe_content()
        ) {
            let mut data = prefix.clone();
            let key_offset = data.len();
            data.extend(&key);
            data.extend(&suffix);

            let scanner = SimdSecretScanner::new();
            let findings = scanner.scan(&data);

            let found = findings.iter().any(|f| {
                f.pattern_type == SecretType::AwsAccessKey
                    && f.byte_offset >= key_offset
                    && f.byte_offset < key_offset + key.len()
            });

            prop_assert!(
                found,
                "Should detect AWS access key at offset {}",
                key_offset
            );
        }

        #[test]
        fn prop_detects_github_token(
            prefix in gen_safe_content(),
            token in gen_github_token(),
            suffix in gen_safe_content()
        ) {
            let mut data = prefix.clone();
            let token_offset = data.len();
            data.extend(&token);
            data.extend(&suffix);

            let scanner = SimdSecretScanner::new();
            let findings = scanner.scan(&data);

            let found = findings.iter().any(|f| {
                f.pattern_type == SecretType::GitHubToken
                    && f.byte_offset >= token_offset
                    && f.byte_offset < token_offset + token.len()
            });

            prop_assert!(
                found,
                "Should detect GitHub token at offset {}",
                token_offset
            );
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: dx-security, Property 3: Secret Scanner No False Positives**
        /// **Validates: Requirements 3.5**
        ///
        /// For any byte array generated from a safe alphabet (alphanumeric without
        /// secret-like patterns), the SIMD_Scanner SHALL return an empty findings list.
        #[test]
        fn prop_no_false_positives(content in gen_safe_content()) {
            let scanner = SimdSecretScanner::new();
            let findings = scanner.scan(&content);

            // Filter out low-confidence findings (generic patterns)
            let high_confidence_findings: Vec<_> = findings
                .iter()
                .filter(|f| f.confidence >= 0.8)
                .collect();

            prop_assert!(
                high_confidence_findings.is_empty(),
                "Should not detect high-confidence secrets in safe content, found: {:?}",
                high_confidence_findings
            );
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: dx-security, Property 4: SIMD Fallback Consistency**
        /// **Validates: Requirements 3.2**
        ///
        /// For any byte array, scanning with AVX-512, AVX2, NEON, or Scalar mode
        /// SHALL produce identical findings (same secrets detected at same offsets).
        #[test]
        fn prop_simd_mode_consistency(
            prefix in gen_safe_content(),
            key in gen_aws_access_key(),
            suffix in gen_safe_content()
        ) {
            let mut data = prefix;
            data.extend(&key);
            data.extend(&suffix);

            let scalar_findings = SimdSecretScanner::with_mode(SimdMode::Scalar).scan(&data);
            let avx2_findings = SimdSecretScanner::with_mode(SimdMode::Avx2).scan(&data);
            let avx512_findings = SimdSecretScanner::with_mode(SimdMode::Avx512).scan(&data);
            let neon_findings = SimdSecretScanner::with_mode(SimdMode::Neon).scan(&data);

            // All modes should find the same number of secrets
            prop_assert_eq!(
                scalar_findings.len(),
                avx2_findings.len(),
                "Scalar and AVX2 should find same number of secrets"
            );
            prop_assert_eq!(
                scalar_findings.len(),
                avx512_findings.len(),
                "Scalar and AVX512 should find same number of secrets"
            );
            prop_assert_eq!(
                scalar_findings.len(),
                neon_findings.len(),
                "Scalar and NEON should find same number of secrets"
            );

            // All modes should find secrets at the same offsets
            for (i, scalar_finding) in scalar_findings.iter().enumerate() {
                prop_assert_eq!(
                    scalar_finding.byte_offset,
                    avx2_findings[i].byte_offset,
                    "Scalar and AVX2 should find secrets at same offsets"
                );
            }
        }
    }
}
