//! Report Signer
//!
//! Ed25519 cryptographic attestation for security reports.

use crate::error::{Result, SecurityError};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use std::path::Path;

/// Binary report format
#[repr(C)]
#[derive(Debug, Clone)]
pub struct BinaryReport {
    /// Magic bytes "SR\0"
    pub magic: [u8; 4],
    /// Format version
    pub version: u8,
    /// Security score (0-100)
    pub score: u8,
    /// Unix timestamp
    pub timestamp: u64,
    /// Git commit hash
    pub git_hash: [u8; 20],
    /// Number of findings
    pub findings_count: u32,
}

impl BinaryReport {
    /// Create a new binary report
    pub fn new(score: u8, timestamp: u64, git_hash: [u8; 20], findings_count: u32) -> Self {
        Self {
            magic: *b"SR\0",
            version: 1,
            score,
            timestamp,
            git_hash,
            findings_count,
        }
    }

    /// Serialize report to bytes for signing
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(64);
        bytes.extend_from_slice(&self.magic);
        bytes.push(self.version);
        bytes.push(self.score);
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        bytes.extend_from_slice(&self.git_hash);
        bytes.extend_from_slice(&self.findings_count.to_le_bytes());
        bytes
    }
}

/// Signed report with Ed25519 signature
#[derive(Debug, Clone)]
pub struct SignedReport {
    /// The report data
    pub report: BinaryReport,
    /// Ed25519 signature
    pub signature: [u8; 64],
    /// Signer's public key
    pub signer_public_key: [u8; 32],
}

/// Report signer for cryptographic attestation
pub struct ReportSigner;

impl ReportSigner {
    /// Sign a security report
    pub fn sign(report: &BinaryReport, key: &SigningKey) -> SignedReport {
        let message = report.to_bytes();
        let signature: Signature = key.sign(&message);

        SignedReport {
            report: report.clone(),
            signature: signature.to_bytes(),
            signer_public_key: key.verifying_key().to_bytes(),
        }
    }

    /// Verify report signature
    pub fn verify(signed_report: &SignedReport, key: &VerifyingKey) -> bool {
        let message = signed_report.report.to_bytes();
        let signature = Signature::from_bytes(&signed_report.signature);

        key.verify(&message, &signature).is_ok()
    }

    /// Export signed report to .sr file
    pub fn export_dxs(signed_report: &SignedReport, path: &Path) -> Result<()> {
        let mut bytes = signed_report.report.to_bytes();
        bytes.extend_from_slice(&signed_report.signature);
        bytes.extend_from_slice(&signed_report.signer_public_key);

        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// Import and verify .sr file
    pub fn import_dxs(path: &Path) -> Result<SignedReport> {
        let bytes = std::fs::read(path)?;

        if bytes.len() < 64 + 64 + 32 {
            return Err(SecurityError::InvalidFormat("File too small".to_string()));
        }

        // Parse report
        if &bytes[0..4] != b"SR\0" {
            return Err(SecurityError::InvalidFormat("Invalid magic".to_string()));
        }

        let version = bytes[4];
        let score = bytes[5];
        let timestamp = u64::from_le_bytes(bytes[6..14].try_into().unwrap());
        let mut git_hash = [0u8; 20];
        git_hash.copy_from_slice(&bytes[14..34]);
        let findings_count = u32::from_le_bytes(bytes[34..38].try_into().unwrap());

        let report = BinaryReport {
            magic: *b"SR\0",
            version,
            score,
            timestamp,
            git_hash,
            findings_count,
        };

        // Parse signature and public key
        let sig_start = bytes.len() - 64 - 32;
        let mut signature = [0u8; 64];
        signature.copy_from_slice(&bytes[sig_start..sig_start + 64]);

        let mut signer_public_key = [0u8; 32];
        signer_public_key.copy_from_slice(&bytes[sig_start + 64..]);

        Ok(SignedReport {
            report,
            signature,
            signer_public_key,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    fn create_test_keypair() -> SigningKey {
        let secret_bytes: [u8; 32] = [
            0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec,
            0x2c, 0xc4, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03,
            0x1c, 0xae, 0x7f, 0x60,
        ];
        SigningKey::from_bytes(&secret_bytes)
    }

    #[test]
    fn test_sign_and_verify() {
        let key = create_test_keypair();
        let report = BinaryReport::new(85, 1234567890, [0u8; 20], 5);

        let signed = ReportSigner::sign(&report, &key);
        assert!(ReportSigner::verify(&signed, &key.verifying_key()));
    }

    #[test]
    fn test_tampered_report_fails_verification() {
        let key = create_test_keypair();
        let report = BinaryReport::new(85, 1234567890, [0u8; 20], 5);

        let mut signed = ReportSigner::sign(&report, &key);
        // Tamper with the score
        signed.report.score = 100;

        assert!(!ReportSigner::verify(&signed, &key.verifying_key()));
    }

    #[test]
    fn test_wrong_key_fails_verification() {
        let key1 = create_test_keypair();
        let key2_bytes: [u8; 32] = [
            0x4c, 0xcd, 0x08, 0x9b, 0x28, 0xff, 0x96, 0xda, 0x9d, 0xb6, 0xc3, 0x46, 0xec, 0x11,
            0x4e, 0x0f, 0x5b, 0x8a, 0x31, 0x9f, 0x35, 0xab, 0xa6, 0x24, 0xda, 0x8c, 0xf6, 0xed,
            0x4f, 0xb8, 0xa6, 0xfb,
        ];
        let key2 = SigningKey::from_bytes(&key2_bytes);

        let report = BinaryReport::new(85, 1234567890, [0u8; 20], 5);
        let signed = ReportSigner::sign(&report, &key1);

        assert!(!ReportSigner::verify(&signed, &key2.verifying_key()));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use proptest::prelude::*;

    /// Generate arbitrary BinaryReport for property testing
    fn arb_binary_report() -> impl Strategy<Value = BinaryReport> {
        (
            0u8..=100,         // score
            any::<u64>(),      // timestamp
            any::<[u8; 20]>(), // git_hash
            any::<u32>(),      // findings_count
        )
            .prop_map(|(score, timestamp, git_hash, findings_count)| {
                BinaryReport::new(score, timestamp, git_hash, findings_count)
            })
    }

    /// Generate arbitrary Ed25519 signing key
    fn arb_signing_key() -> impl Strategy<Value = SigningKey> {
        any::<[u8; 32]>().prop_map(|bytes| SigningKey::from_bytes(&bytes))
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: dx-security, Property 8: Cryptographic Signing Round-Trip**
        /// **Validates: Requirements 8.1, 8.2, 8.3, 8.4**
        ///
        /// For any BinaryReport and valid Ed25519 keypair:
        /// - sign(report, private_key) → signed_report
        /// - verify(signed_report, public_key) SHALL return true
        #[test]
        fn prop_sign_verify_roundtrip(
            report in arb_binary_report(),
            key in arb_signing_key()
        ) {
            let signed = ReportSigner::sign(&report, &key);
            let verified = ReportSigner::verify(&signed, &key.verifying_key());

            prop_assert!(
                verified,
                "Signed report should verify with correct key"
            );
        }

        /// Tampered reports should fail verification
        #[test]
        fn prop_tampered_report_fails(
            report in arb_binary_report(),
            key in arb_signing_key()
        ) {
            let mut signed = ReportSigner::sign(&report, &key);

            // Tamper with the score (a simple, reliable way to tamper)
            let original_score = signed.report.score;
            signed.report.score = if original_score < 100 {
                original_score + 1
            } else {
                original_score - 1
            };

            let verified = ReportSigner::verify(&signed, &key.verifying_key());
            prop_assert!(
                !verified,
                "Tampered report should fail verification"
            );
        }

        /// Wrong key should fail verification
        #[test]
        fn prop_wrong_key_fails(
            report in arb_binary_report(),
            key1 in arb_signing_key(),
            key2 in arb_signing_key()
        ) {
            // Only test when keys are different
            if key1.to_bytes() != key2.to_bytes() {
                let signed = ReportSigner::sign(&report, &key1);
                let verified = ReportSigner::verify(&signed, &key2.verifying_key());

                prop_assert!(
                    !verified,
                    "Report signed with key1 should not verify with key2"
                );
            }
        }
    }
}
