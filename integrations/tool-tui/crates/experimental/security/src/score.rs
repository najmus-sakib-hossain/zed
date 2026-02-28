//! Security Score Calculator
//!
//! Computes deterministic security scores (0-100) based on scan findings.

use crate::signer::{BinaryReport, ReportSigner, SignedReport};
use ed25519_dalek::{SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

/// Security scan findings used for score calculation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanFindings {
    /// Number of critical CVEs detected
    pub critical_cves: u32,
    /// Number of high-severity CVEs detected
    pub high_cves: u32,
    /// Number of medium-severity CVEs detected
    pub medium_cves: u32,
    /// Number of low-severity CVEs detected
    pub low_cves: u32,
    /// Number of leaked secrets detected
    pub secrets_leaked: u32,
    /// Whether binary inoculation is active
    pub binary_inoculation_active: bool,
    /// Whether supply chain XOR verification passed
    pub supply_chain_xor_verified: bool,
}

/// Cryptographically signed security score
#[derive(Debug, Clone)]
pub struct SecurityScore {
    /// Score value (0-100)
    pub score: u8,
    /// Unix timestamp
    pub timestamp: u64,
    /// Git commit hash
    pub git_hash: [u8; 20],
    /// Ed25519 signature
    pub signature: [u8; 64],
}

const CRITICAL_CVE_PENALTY: i32 = 25;
const HIGH_CVE_PENALTY: i32 = 10;
const SECRET_PENALTY: i32 = 50;
const INOCULATION_BONUS: i32 = 5;
const XOR_VERIFIED_BONUS: i32 = 5;

/// Calculate security score from findings
///
/// Formula: clamp(100 - (critical * 25) - (high * 10) - (secrets * 50) + bonuses, 0, 100)
pub fn calculate_score(findings: &ScanFindings) -> u8 {
    let mut score: i32 = 100;

    score -= (findings.critical_cves as i32) * CRITICAL_CVE_PENALTY;
    score -= (findings.high_cves as i32) * HIGH_CVE_PENALTY;
    score -= (findings.secrets_leaked as i32) * SECRET_PENALTY;

    if findings.binary_inoculation_active {
        score += INOCULATION_BONUS;
    }
    if findings.supply_chain_xor_verified {
        score += XOR_VERIFIED_BONUS;
    }

    score.clamp(0, 100) as u8
}

/// Sign a security score with Ed25519
pub fn sign_score(
    score: u8,
    git_hash: [u8; 20],
    findings_count: u32,
    key: &SigningKey,
) -> SecurityScore {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let report = BinaryReport::new(score, timestamp, git_hash, findings_count);
    let signed = ReportSigner::sign(&report, key);

    SecurityScore {
        score,
        timestamp,
        git_hash,
        signature: signed.signature,
    }
}

/// Verify a signed security score
pub fn verify_score(score: &SecurityScore, findings_count: u32, key: &VerifyingKey) -> bool {
    let report = BinaryReport::new(score.score, score.timestamp, score.git_hash, findings_count);
    let signed = SignedReport {
        report,
        signature: score.signature,
        signer_public_key: key.to_bytes(),
    };

    ReportSigner::verify(&signed, key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_score() {
        let findings = ScanFindings::default();
        assert_eq!(calculate_score(&findings), 100);
    }

    #[test]
    fn test_critical_cve_penalty() {
        let findings = ScanFindings {
            critical_cves: 1,
            ..Default::default()
        };
        assert_eq!(calculate_score(&findings), 75);
    }

    #[test]
    fn test_high_cve_penalty() {
        let findings = ScanFindings {
            high_cves: 1,
            ..Default::default()
        };
        assert_eq!(calculate_score(&findings), 90);
    }

    #[test]
    fn test_secret_penalty() {
        let findings = ScanFindings {
            secrets_leaked: 1,
            ..Default::default()
        };
        assert_eq!(calculate_score(&findings), 50);
    }

    #[test]
    fn test_bonuses() {
        let findings = ScanFindings {
            binary_inoculation_active: true,
            supply_chain_xor_verified: true,
            ..Default::default()
        };
        assert_eq!(calculate_score(&findings), 100);
    }

    #[test]
    fn test_score_clamps_to_zero() {
        let findings = ScanFindings {
            critical_cves: 10,
            ..Default::default()
        };
        assert_eq!(calculate_score(&findings), 0);
    }

    #[test]
    fn test_combined_penalties_and_bonuses() {
        let findings = ScanFindings {
            critical_cves: 2,
            high_cves: 3,
            binary_inoculation_active: true,
            supply_chain_xor_verified: true,
            ..Default::default()
        };
        assert_eq!(calculate_score(&findings), 30);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate arbitrary ScanFindings for property testing
    fn arb_scan_findings() -> impl Strategy<Value = ScanFindings> {
        (
            0u32..100,     // critical_cves
            0u32..100,     // high_cves
            0u32..100,     // medium_cves
            0u32..100,     // low_cves
            0u32..100,     // secrets_leaked
            any::<bool>(), // binary_inoculation_active
            any::<bool>(), // supply_chain_xor_verified
        )
            .prop_map(
                |(critical, high, medium, low, secrets, inoculation, xor_verified)| ScanFindings {
                    critical_cves: critical,
                    high_cves: high,
                    medium_cves: medium,
                    low_cves: low,
                    secrets_leaked: secrets,
                    binary_inoculation_active: inoculation,
                    supply_chain_xor_verified: xor_verified,
                },
            )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: dx-security, Property 1: Score Calculation Invariants**
        /// **Validates: Requirements 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7**
        ///
        /// For any ScanFindings input, the calculated security score SHALL:
        /// - Always be in the range [0, 100]
        /// - Equal: clamp(100 - (critical_cves × 25) - (high_cves × 10) - (secrets × 50) + bonuses, 0, 100)
        #[test]
        fn prop_score_always_in_valid_range(findings in arb_scan_findings()) {
            let score = calculate_score(&findings);
            prop_assert!(score <= 100, "Score {} exceeds 100", score);
        }

        #[test]
        fn prop_score_matches_formula(findings in arb_scan_findings()) {
            let score = calculate_score(&findings);

            // Calculate expected score using the formula
            let mut expected: i32 = 100;
            expected -= (findings.critical_cves as i32) * 25;
            expected -= (findings.high_cves as i32) * 10;
            expected -= (findings.secrets_leaked as i32) * 50;

            if findings.binary_inoculation_active {
                expected += 5;
            }
            if findings.supply_chain_xor_verified {
                expected += 5;
            }

            let expected = expected.clamp(0, 100) as u8;

            prop_assert_eq!(
                score, expected,
                "Score {} doesn't match expected {} for findings {:?}",
                score, expected, findings
            );
        }

        #[test]
        fn prop_critical_cve_penalty_is_25(
            base in arb_scan_findings(),
            additional_critical in 0u32..4
        ) {
            // Only test when we won't overflow and score won't be clamped
            if base.critical_cves < 3 && additional_critical > 0 {
                let mut modified = base.clone();
                modified.critical_cves += additional_critical;

                let base_score = calculate_score(&base) as i32;
                let modified_score = calculate_score(&modified) as i32;

                // The difference should be 25 * additional_critical (unless clamped)
                let expected_diff = (additional_critical as i32) * 25;
                let actual_diff = base_score - modified_score;

                // Account for clamping at 0
                if modified_score > 0 {
                    prop_assert_eq!(
                        actual_diff, expected_diff,
                        "Critical CVE penalty should be 25 per CVE"
                    );
                }
            }
        }

        #[test]
        fn prop_high_cve_penalty_is_10(
            base in arb_scan_findings(),
            additional_high in 0u32..10
        ) {
            if base.high_cves < 5 && additional_high > 0 {
                let mut modified = base.clone();
                modified.high_cves += additional_high;

                let base_score = calculate_score(&base) as i32;
                let modified_score = calculate_score(&modified) as i32;

                let expected_diff = (additional_high as i32) * 10;
                let actual_diff = base_score - modified_score;

                if modified_score > 0 {
                    prop_assert_eq!(
                        actual_diff, expected_diff,
                        "High CVE penalty should be 10 per CVE"
                    );
                }
            }
        }

        #[test]
        fn prop_secret_penalty_is_50(
            base in arb_scan_findings(),
            additional_secrets in 0u32..2
        ) {
            if base.secrets_leaked < 2 && additional_secrets > 0 {
                let mut modified = base.clone();
                modified.secrets_leaked += additional_secrets;

                let base_score = calculate_score(&base) as i32;
                let modified_score = calculate_score(&modified) as i32;

                let expected_diff = (additional_secrets as i32) * 50;
                let actual_diff = base_score - modified_score;

                if modified_score > 0 {
                    prop_assert_eq!(
                        actual_diff, expected_diff,
                        "Secret penalty should be 50 per secret"
                    );
                }
            }
        }

        #[test]
        fn prop_inoculation_bonus_is_5(findings in arb_scan_findings()) {
            let mut with_inoculation = findings.clone();
            with_inoculation.binary_inoculation_active = true;

            let mut without_inoculation = findings.clone();
            without_inoculation.binary_inoculation_active = false;

            let score_with = calculate_score(&with_inoculation) as i32;
            let score_without = calculate_score(&without_inoculation) as i32;

            // Bonus should be 5 (unless clamped at 100 or 0)
            let diff = score_with - score_without;
            prop_assert!(
                diff == 5 || (score_with == 100 && diff < 5) || (score_without == 0 && diff < 5),
                "Inoculation bonus should be 5 (got diff {})", diff
            );
        }

        #[test]
        fn prop_xor_verified_bonus_is_5(findings in arb_scan_findings()) {
            let mut with_xor = findings.clone();
            with_xor.supply_chain_xor_verified = true;

            let mut without_xor = findings.clone();
            without_xor.supply_chain_xor_verified = false;

            let score_with = calculate_score(&with_xor) as i32;
            let score_without = calculate_score(&without_xor) as i32;

            let diff = score_with - score_without;
            prop_assert!(
                diff == 5 || (score_with == 100 && diff < 5) || (score_without == 0 && diff < 5),
                "XOR verified bonus should be 5 (got diff {})", diff
            );
        }
    }
}
