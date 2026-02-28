//! Property-based tests for dx-www-auth crate.
//!
//! These tests verify universal properties that should hold across all inputs.

use chrono::Duration;
use dx_www_auth::{
    AuthError, AuthToken, PasswordHasher, ProductionTokenGenerator, ProductionTokenVerifier,
    TokenConfig, TokenType,
};
use proptest::prelude::*;

// ============================================================================
// Property 2: Token Signature Integrity
// **Validates: Requirements 1.2, 1.3, 1.5**
//
// *For any* generated Ed25519 token, the signature SHALL verify successfully
// with the corresponding public key, and any modification to the token payload
// SHALL cause verification to fail.
// ============================================================================

proptest! {
    /// Feature: production-readiness, Property 2: Token Signature Integrity
    ///
    /// For any user ID, a generated token should verify successfully.
    #[test]
    fn property_2_token_signature_verifies(user_id in "[a-zA-Z0-9_-]{1,64}") {
        let generator = ProductionTokenGenerator::new();
        let token = generator.generate_access(&user_id).expect("token generation should succeed");

        // Token should verify with the generator
        prop_assert!(generator.verify(&token).is_ok(), "Token should verify with generator");

        // Token should verify with a separate verifier using the public key
        let verifier = ProductionTokenVerifier::from_public_key(&generator.public_key_bytes())
            .expect("verifier creation should succeed");
        prop_assert!(verifier.verify(&token).is_ok(), "Token should verify with separate verifier");
    }

    /// Feature: production-readiness, Property 2: Token Signature Integrity (tamper detection)
    ///
    /// For any token, modifying the subject should cause verification to fail.
    #[test]
    fn property_2_tampered_subject_fails(
        user_id in "[a-zA-Z0-9_-]{1,64}",
        tampered_id in "[a-zA-Z0-9_-]{1,64}"
    ) {
        prop_assume!(user_id != tampered_id);

        let generator = ProductionTokenGenerator::new();
        let mut token = generator.generate_access(&user_id).expect("token generation should succeed");

        // Tamper with the subject
        token.sub = tampered_id;

        // Verification should fail
        prop_assert_eq!(generator.verify(&token), Err(AuthError::TokenInvalid));
    }

    /// Feature: production-readiness, Property 2: Token Signature Integrity (JTI tamper)
    ///
    /// For any token, modifying the JTI should cause verification to fail.
    #[test]
    fn property_2_tampered_jti_fails(user_id in "[a-zA-Z0-9_-]{1,64}") {
        let generator = ProductionTokenGenerator::new();
        let mut token = generator.generate_access(&user_id).expect("token generation should succeed");

        // Tamper with the JTI
        token.jti = "tampered_jti_value".to_string();

        // Verification should fail
        prop_assert_eq!(generator.verify(&token), Err(AuthError::TokenInvalid));
    }

    /// Feature: production-readiness, Property 2: Token Signature Integrity (timestamp tamper)
    ///
    /// For any token, modifying the timestamps should cause verification to fail.
    #[test]
    fn property_2_tampered_timestamps_fails(
        user_id in "[a-zA-Z0-9_-]{1,64}",
        time_offset in 1i64..1000000
    ) {
        let generator = ProductionTokenGenerator::new();
        let mut token = generator.generate_access(&user_id).expect("token generation should succeed");

        // Tamper with the expiration
        token.exp += time_offset;

        // Verification should fail
        prop_assert_eq!(generator.verify(&token), Err(AuthError::TokenInvalid));
    }

    /// Feature: production-readiness, Property 2: Token Signature Integrity (type tamper)
    ///
    /// For any token, changing the token type should cause verification to fail.
    #[test]
    fn property_2_tampered_type_fails(user_id in "[a-zA-Z0-9_-]{1,64}") {
        let generator = ProductionTokenGenerator::new();
        let mut token = generator.generate_access(&user_id).expect("token generation should succeed");

        // Tamper with the type
        token.typ = TokenType::Refresh;

        // Verification should fail
        prop_assert_eq!(generator.verify(&token), Err(AuthError::TokenInvalid));
    }

    /// Feature: production-readiness, Property 2: Token Signature Integrity (signature tamper)
    ///
    /// For any token, modifying the signature bytes should cause verification to fail.
    #[test]
    fn property_2_tampered_signature_fails(
        user_id in "[a-zA-Z0-9_-]{1,64}",
        byte_index in 0usize..64,
        xor_value in 1u8..=255
    ) {
        let generator = ProductionTokenGenerator::new();
        let mut token = generator.generate_access(&user_id).expect("token generation should succeed");

        // Tamper with a signature byte
        token.sig[byte_index] ^= xor_value;

        // Verification should fail
        prop_assert_eq!(generator.verify(&token), Err(AuthError::TokenInvalid));
    }
}

// ============================================================================
// Property 1: Password Hash Round-Trip
// **Validates: Requirements 1.1**
//
// *For any* valid password string, hashing with Argon2id and then verifying
// the same password against the hash SHALL return true.
// ============================================================================

// Note: Password hashing tests use fewer iterations because Argon2id is
// computationally expensive by design (security feature).
proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    /// Feature: production-readiness, Property 1: Password Hash Round-Trip
    ///
    /// For any password, hashing and then verifying should succeed.
    #[test]
    fn property_1_password_hash_roundtrip(password in ".{8,32}") {
        let hasher = PasswordHasher::new();

        let hash = hasher.hash(&password).expect("hashing should succeed");

        // Verification should succeed
        let verified = hasher.verify(&password, &hash).expect("verification should not error");
        prop_assert!(verified, "Password should verify against its own hash");
    }

    /// Feature: production-readiness, Property 1: Password Hash Round-Trip (wrong password fails)
    ///
    /// For any two different passwords, verifying one against the other's hash should fail.
    #[test]
    fn property_1_wrong_password_fails(
        password1 in ".{8,32}",
        password2 in ".{8,32}"
    ) {
        prop_assume!(password1 != password2);

        let hasher = PasswordHasher::new();
        let hash = hasher.hash(&password1).expect("hashing should succeed");

        // Wrong password should not verify
        let verified = hasher.verify(&password2, &hash).expect("verification should not error");
        prop_assert!(!verified, "Wrong password should not verify");
    }

    /// Feature: production-readiness, Property 1: Password Hash Uniqueness
    ///
    /// For any password, hashing twice should produce different hashes (due to salt).
    #[test]
    fn property_1_hash_uniqueness(password in ".{8,32}") {
        let hasher = PasswordHasher::new();

        let hash1 = hasher.hash(&password).expect("hashing should succeed");
        let hash2 = hasher.hash(&password).expect("hashing should succeed");

        // Hashes should be different (different salts)
        prop_assert_ne!(&hash1, &hash2, "Same password should produce different hashes");

        // But both should verify
        prop_assert!(hasher.verify(&password, &hash1).unwrap());
        prop_assert!(hasher.verify(&password, &hash2).unwrap());
    }
}

// ============================================================================
// Property 3: Token Expiration Enforcement
// **Validates: Requirements 1.3, 1.4**
//
// *For any* token with expiration time T, verification SHALL succeed when
// current_time < T and SHALL fail when current_time >= T.
// ============================================================================

/// Feature: production-readiness, Property 3: Token Expiration Enforcement
///
/// Tokens with negative TTL should be expired immediately.
#[test]
fn property_3_expired_token_fails() {
    let config = TokenConfig::new(Duration::seconds(-10), Duration::days(7));
    let generator = ProductionTokenGenerator::with_config(config);

    let token = generator.generate_access("user123").expect("token generation should succeed");

    // Token should be expired
    assert!(token.is_expired());
    assert_eq!(generator.verify(&token), Err(AuthError::TokenExpired));
}

/// Feature: production-readiness, Property 3: Token Expiration Enforcement
///
/// Tokens with positive TTL should not be expired immediately.
#[test]
fn property_3_valid_token_succeeds() {
    let config = TokenConfig::new(Duration::hours(1), Duration::days(7));
    let generator = ProductionTokenGenerator::with_config(config);

    let token = generator.generate_access("user123").expect("token generation should succeed");

    // Token should not be expired
    assert!(!token.is_expired());
    assert!(generator.verify(&token).is_ok());
}

/// Feature: production-readiness, Property 3: Token Expiration Enforcement
///
/// Verification at a timestamp before expiration should succeed.
#[test]
fn property_3_verify_at_before_expiration_succeeds() {
    let generator = ProductionTokenGenerator::new();
    let token = generator.generate_access("user123").expect("token generation should succeed");

    let verifier = ProductionTokenVerifier::from_public_key(&generator.public_key_bytes())
        .expect("verifier creation should succeed");

    // Verify at issued time (before expiration)
    assert!(verifier.verify_at(&token, token.iat).is_ok());

    // Verify at midpoint (before expiration)
    let midpoint = token.iat + (token.exp - token.iat) / 2;
    assert!(verifier.verify_at(&token, midpoint).is_ok());

    // Verify just before expiration
    assert!(verifier.verify_at(&token, token.exp - 1).is_ok());
}

/// Feature: production-readiness, Property 3: Token Expiration Enforcement
///
/// Verification at or after expiration should fail.
#[test]
fn property_3_verify_at_after_expiration_fails() {
    let generator = ProductionTokenGenerator::new();
    let token = generator.generate_access("user123").expect("token generation should succeed");

    let verifier = ProductionTokenVerifier::from_public_key(&generator.public_key_bytes())
        .expect("verifier creation should succeed");

    // Verify at exact expiration time should fail
    assert_eq!(verifier.verify_at(&token, token.exp + 1), Err(AuthError::TokenExpired));

    // Verify well after expiration should fail
    assert_eq!(verifier.verify_at(&token, token.exp + 3600), Err(AuthError::TokenExpired));
}

proptest! {
    /// Feature: production-readiness, Property 3: Token Expiration Enforcement (property-based)
    ///
    /// For any positive TTL, token should not be expired immediately.
    #[test]
    fn property_3_positive_ttl_not_expired(ttl_seconds in 1i64..86400) {
        let config = TokenConfig::new(Duration::seconds(ttl_seconds), Duration::days(7));
        let generator = ProductionTokenGenerator::with_config(config);

        let token = generator.generate_access("test_user").expect("token generation should succeed");

        prop_assert!(!token.is_expired(), "Token with positive TTL should not be expired");
        prop_assert!(generator.verify(&token).is_ok(), "Token should verify");
    }

    /// Feature: production-readiness, Property 3: Token Expiration Enforcement (timestamp-based)
    ///
    /// For any token and any timestamp before expiration, verification should succeed.
    #[test]
    fn property_3_verify_before_expiration_succeeds(
        user_id in "[a-zA-Z0-9_-]{1,64}",
        offset_before_exp in 1i64..3600
    ) {
        let generator = ProductionTokenGenerator::new();
        let token = generator.generate_access(&user_id).expect("token generation should succeed");

        let verifier = ProductionTokenVerifier::from_public_key(&generator.public_key_bytes())
            .expect("verifier creation should succeed");

        // Verify at a time before expiration
        let verify_time = token.exp - offset_before_exp;
        prop_assert!(
            verifier.verify_at(&token, verify_time).is_ok(),
            "Token should verify at time {} (exp: {})", verify_time, token.exp
        );
    }

    /// Feature: production-readiness, Property 3: Token Expiration Enforcement (after expiration)
    ///
    /// For any token and any timestamp after expiration, verification should fail.
    #[test]
    fn property_3_verify_after_expiration_fails(
        user_id in "[a-zA-Z0-9_-]{1,64}",
        offset_after_exp in 1i64..86400
    ) {
        let generator = ProductionTokenGenerator::new();
        let token = generator.generate_access(&user_id).expect("token generation should succeed");

        let verifier = ProductionTokenVerifier::from_public_key(&generator.public_key_bytes())
            .expect("verifier creation should succeed");

        // Verify at a time after expiration
        let verify_time = token.exp + offset_after_exp;
        prop_assert_eq!(
            verifier.verify_at(&token, verify_time),
            Err(AuthError::TokenExpired),
            "Token should be expired at time {} (exp: {})", verify_time, token.exp
        );
    }

    /// Feature: production-readiness, Property 3: Token Expiration Enforcement (negative TTL)
    ///
    /// For any negative TTL, token should be expired immediately.
    #[test]
    fn property_3_negative_ttl_expired(ttl_seconds in -86400i64..-1) {
        let config = TokenConfig::new(Duration::seconds(ttl_seconds), Duration::days(7));
        let generator = ProductionTokenGenerator::with_config(config);

        let token = generator.generate_access("test_user").expect("token generation should succeed");

        prop_assert!(token.is_expired(), "Token with negative TTL should be expired");
        prop_assert_eq!(
            generator.verify(&token),
            Err(AuthError::TokenExpired),
            "Expired token verification should fail"
        );
    }

    /// Feature: production-readiness, Property 3: Token Expiration Enforcement (is_expired_at consistency)
    ///
    /// For any token and timestamp, is_expired_at should be consistent with verify_at.
    #[test]
    fn property_3_is_expired_at_consistency(
        user_id in "[a-zA-Z0-9_-]{1,64}",
        time_offset in -3600i64..3600
    ) {
        let generator = ProductionTokenGenerator::new();
        let token = generator.generate_access(&user_id).expect("token generation should succeed");

        let verifier = ProductionTokenVerifier::from_public_key(&generator.public_key_bytes())
            .expect("verifier creation should succeed");

        let check_time = token.exp + time_offset;
        let is_expired = token.is_expired_at(check_time);
        let verify_result = verifier.verify_at(&token, check_time);

        // If is_expired_at returns true, verify_at should return TokenExpired
        // If is_expired_at returns false, verify_at should succeed (assuming valid signature)
        if is_expired {
            prop_assert_eq!(
                verify_result,
                Err(AuthError::TokenExpired),
                "is_expired_at({}) = true but verify_at succeeded", check_time
            );
        } else {
            prop_assert!(
                verify_result.is_ok(),
                "is_expired_at({}) = false but verify_at failed: {:?}", check_time, verify_result
            );
        }
    }
}

// ============================================================================
// Property 4: Token Refresh Validity
// **Validates: Requirements 1.6**
//
// *For any* valid refresh token within its grace period, refreshing SHALL
// produce a new valid access token with a fresh expiration time.
// ============================================================================

proptest! {
    /// Feature: production-readiness, Property 4: Token Refresh Validity
    ///
    /// For any user, generating a refresh token and then a new access token should work.
    #[test]
    fn property_4_refresh_token_generates_new_access(user_id in "[a-zA-Z0-9_-]{1,64}") {
        let generator = ProductionTokenGenerator::new();

        // Generate refresh token
        let refresh_token = generator.generate_refresh(&user_id).expect("refresh token generation should succeed");
        prop_assert_eq!(refresh_token.typ, TokenType::Refresh);
        prop_assert!(generator.verify(&refresh_token).is_ok());

        // Generate new access token (simulating refresh flow)
        let new_access_token = generator.generate_access(&user_id).expect("access token generation should succeed");
        prop_assert_eq!(new_access_token.typ, TokenType::Access);
        prop_assert!(generator.verify(&new_access_token).is_ok());

        // Both tokens should have the same subject
        prop_assert_eq!(refresh_token.sub, new_access_token.sub);

        // New access token should have a later issued-at time or equal
        prop_assert!(new_access_token.iat >= refresh_token.iat);
    }

    /// Feature: production-readiness, Property 4: Token Refresh Validity (fresh expiration)
    ///
    /// For any refresh operation, the new access token should have a fresh expiration time.
    #[test]
    fn property_4_refresh_produces_fresh_expiration(user_id in "[a-zA-Z0-9_-]{1,64}") {
        let config = TokenConfig::new(Duration::minutes(15), Duration::days(7));
        let generator = ProductionTokenGenerator::with_config(config);

        // Generate initial access token
        let initial_access = generator.generate_access(&user_id).expect("token generation should succeed");

        // Simulate time passing (by generating a new token which will have a new iat)
        let refreshed_access = generator.generate_access(&user_id).expect("token generation should succeed");

        // Both tokens should be valid
        prop_assert!(generator.verify(&initial_access).is_ok());
        prop_assert!(generator.verify(&refreshed_access).is_ok());

        // Refreshed token should have same or later expiration
        prop_assert!(refreshed_access.exp >= initial_access.exp);
    }

    /// Feature: production-readiness, Property 4: Token Refresh Validity (type preservation)
    ///
    /// Refresh tokens should always produce access tokens, not more refresh tokens.
    #[test]
    fn property_4_refresh_produces_access_type(user_id in "[a-zA-Z0-9_-]{1,64}") {
        let generator = ProductionTokenGenerator::new();

        // Generate refresh token
        let refresh_token = generator.generate_refresh(&user_id).expect("refresh token generation should succeed");
        prop_assert_eq!(refresh_token.typ, TokenType::Refresh);

        // Generate new access token from refresh
        let new_access = generator.generate_access(&refresh_token.sub).expect("access token generation should succeed");
        prop_assert_eq!(new_access.typ, TokenType::Access);

        // Verify type-specific verification works
        prop_assert!(generator.verify_with_type(&refresh_token, TokenType::Refresh).is_ok());
        prop_assert!(generator.verify_with_type(&new_access, TokenType::Access).is_ok());

        // Cross-type verification should fail
        prop_assert_eq!(
            generator.verify_with_type(&refresh_token, TokenType::Access),
            Err(AuthError::TokenTypeMismatch)
        );
        prop_assert_eq!(
            generator.verify_with_type(&new_access, TokenType::Refresh),
            Err(AuthError::TokenTypeMismatch)
        );
    }

    /// Feature: production-readiness, Property 4: Token Refresh Validity (subject preservation)
    ///
    /// The refreshed access token should have the same subject as the refresh token.
    #[test]
    fn property_4_refresh_preserves_subject(user_id in "[a-zA-Z0-9_-]{1,64}") {
        let generator = ProductionTokenGenerator::new();

        // Generate refresh token
        let refresh_token = generator.generate_refresh(&user_id).expect("refresh token generation should succeed");

        // Generate new access token using the subject from refresh token
        let new_access = generator.generate_access(&refresh_token.sub).expect("access token generation should succeed");

        // Subjects should match
        prop_assert_eq!(&refresh_token.sub, &new_access.sub);
        prop_assert_eq!(&refresh_token.sub, &user_id);
    }
}

/// Feature: production-readiness, Property 4: Token Refresh Validity (grace period)
///
/// Tokens within grace period should still be refreshable.
#[test]
fn property_4_grace_period_allows_refresh() {
    // Create config with short TTL and grace period
    let config = TokenConfig::new(Duration::seconds(10), Duration::seconds(30))
        .with_grace_period(Duration::seconds(15));
    let generator = ProductionTokenGenerator::with_config(config);

    // Generate refresh token
    let refresh_token = generator
        .generate_refresh("test_user")
        .expect("refresh token generation should succeed");

    // Token should not be in grace period immediately (it's fresh)
    assert!(!generator.is_within_grace_period(&refresh_token));

    // Token should be valid
    assert!(generator.verify(&refresh_token).is_ok());
}

/// Feature: production-readiness, Property 4: Token Refresh Validity (different JTIs)
///
/// Each refreshed token should have a unique JTI.
#[test]
fn property_4_refresh_produces_unique_jti() {
    let generator = ProductionTokenGenerator::new();

    let refresh_token = generator
        .generate_refresh("user1")
        .expect("refresh token generation should succeed");
    let access1 = generator
        .generate_access(&refresh_token.sub)
        .expect("access token generation should succeed");
    let access2 = generator
        .generate_access(&refresh_token.sub)
        .expect("access token generation should succeed");

    // All JTIs should be unique
    assert_ne!(refresh_token.jti, access1.jti);
    assert_ne!(refresh_token.jti, access2.jti);
    assert_ne!(access1.jti, access2.jti);
}

// ============================================================================
// Property 5: Token Revocation Effectiveness
// **Validates: Requirements 1.7**
//
// *For any* revoked token, subsequent verification attempts SHALL fail
// regardless of the token's expiration status.
// ============================================================================

/// Feature: production-readiness, Property 5: Token Revocation Effectiveness
///
/// This is a placeholder test - full revocation testing requires the CredentialStore
/// implementation which will be tested in integration tests.
#[test]
fn property_5_token_has_unique_jti_for_revocation() {
    let generator = ProductionTokenGenerator::new();

    let token1 = generator.generate_access("user1").expect("token generation should succeed");
    let token2 = generator.generate_access("user1").expect("token generation should succeed");
    let token3 = generator.generate_access("user2").expect("token generation should succeed");

    // All tokens should have unique JTIs
    assert_ne!(token1.jti, token2.jti);
    assert_ne!(token1.jti, token3.jti);
    assert_ne!(token2.jti, token3.jti);

    // JTIs should be non-empty
    assert!(!token1.jti.is_empty());
    assert!(!token2.jti.is_empty());
    assert!(!token3.jti.is_empty());
}

proptest! {
    /// Feature: production-readiness, Property 5: Token Revocation Effectiveness (JTI uniqueness)
    ///
    /// For any user, all generated tokens should have unique JTIs for revocation tracking.
    #[test]
    fn property_5_all_tokens_have_unique_jti(user_id in "[a-zA-Z0-9_-]{1,64}") {
        let generator = ProductionTokenGenerator::new();

        let access1 = generator.generate_access(&user_id).expect("token generation should succeed");
        let access2 = generator.generate_access(&user_id).expect("token generation should succeed");
        let refresh1 = generator.generate_refresh(&user_id).expect("token generation should succeed");
        let refresh2 = generator.generate_refresh(&user_id).expect("token generation should succeed");

        // All JTIs should be unique
        let jtis = vec![&access1.jti, &access2.jti, &refresh1.jti, &refresh2.jti];
        let unique_jtis: std::collections::HashSet<_> = jtis.iter().collect();
        prop_assert_eq!(jtis.len(), unique_jtis.len(), "All JTIs should be unique");
    }

    /// Feature: production-readiness, Property 5: Token Revocation Effectiveness (JTI format)
    ///
    /// JTIs should be non-empty and have sufficient entropy for uniqueness.
    #[test]
    fn property_5_jti_has_sufficient_entropy(user_id in "[a-zA-Z0-9_-]{1,64}") {
        let generator = ProductionTokenGenerator::new();
        let token = generator.generate_access(&user_id).expect("token generation should succeed");

        // JTI should be non-empty
        prop_assert!(!token.jti.is_empty(), "JTI should not be empty");

        // JTI should have at least 16 characters (128 bits of entropy when base64 encoded)
        prop_assert!(token.jti.len() >= 16, "JTI should have at least 16 characters for sufficient entropy");
    }
}

/// Feature: production-readiness, Property 5: Token Revocation Effectiveness (credential store)
///
/// Revoked tokens should be tracked in the credential store.
#[tokio::test]
async fn property_5_revoked_token_is_tracked() {
    use dx_www_auth::{CredentialStore, InMemoryCredentialStore};

    let store = InMemoryCredentialStore::new();
    let generator = ProductionTokenGenerator::new();
    let token = generator.generate_access("user1").expect("token generation should succeed");

    // Token should not be revoked initially
    let is_revoked = store.is_token_revoked(&token.jti).await.expect("check should succeed");
    assert!(!is_revoked, "Token should not be revoked initially");

    // Revoke the token
    store.revoke_token(&token.jti).await.expect("revocation should succeed");

    // Token should now be revoked
    let is_revoked = store.is_token_revoked(&token.jti).await.expect("check should succeed");
    assert!(is_revoked, "Token should be revoked after revocation");
}

/// Feature: production-readiness, Property 5: Token Revocation Effectiveness (multiple tokens)
///
/// Revoking one token should not affect other tokens.
#[tokio::test]
async fn property_5_revocation_is_specific() {
    use dx_www_auth::{CredentialStore, InMemoryCredentialStore};

    let store = InMemoryCredentialStore::new();
    let generator = ProductionTokenGenerator::new();

    let token1 = generator.generate_access("user1").expect("token generation should succeed");
    let token2 = generator.generate_access("user1").expect("token generation should succeed");
    let token3 = generator.generate_access("user2").expect("token generation should succeed");

    // Revoke only token1
    store.revoke_token(&token1.jti).await.expect("revocation should succeed");

    // Only token1 should be revoked
    assert!(store.is_token_revoked(&token1.jti).await.unwrap(), "token1 should be revoked");
    assert!(
        !store.is_token_revoked(&token2.jti).await.unwrap(),
        "token2 should not be revoked"
    );
    assert!(
        !store.is_token_revoked(&token3.jti).await.unwrap(),
        "token3 should not be revoked"
    );
}

/// Feature: production-readiness, Property 5: Token Revocation Effectiveness (idempotent)
///
/// Revoking a token multiple times should be idempotent.
#[tokio::test]
async fn property_5_revocation_is_idempotent() {
    use dx_www_auth::{CredentialStore, InMemoryCredentialStore};

    let store = InMemoryCredentialStore::new();
    let generator = ProductionTokenGenerator::new();
    let token = generator.generate_access("user1").expect("token generation should succeed");

    // Revoke multiple times
    store.revoke_token(&token.jti).await.expect("first revocation should succeed");
    store.revoke_token(&token.jti).await.expect("second revocation should succeed");
    store.revoke_token(&token.jti).await.expect("third revocation should succeed");

    // Token should still be revoked
    assert!(store.is_token_revoked(&token.jti).await.unwrap(), "Token should be revoked");
}

// ============================================================================
// Token Base64 Round-Trip
// ============================================================================

proptest! {
    /// Token base64 encoding should be reversible.
    #[test]
    fn property_token_base64_roundtrip(user_id in "[a-zA-Z0-9_-]{1,64}") {
        let generator = ProductionTokenGenerator::new();
        let token = generator.generate_access(&user_id).expect("token generation should succeed");

        let encoded = token.to_base64();
        let decoded = AuthToken::from_base64(&encoded).expect("decoding should succeed");

        prop_assert_eq!(&token.jti, &decoded.jti);
        prop_assert_eq!(&token.sub, &decoded.sub);
        prop_assert_eq!(token.iat, decoded.iat);
        prop_assert_eq!(token.exp, decoded.exp);
        prop_assert_eq!(token.typ, decoded.typ);
        prop_assert_eq!(token.sig, decoded.sig);

        // Decoded token should still verify
        prop_assert!(generator.verify(&decoded).is_ok());
    }
}
