//! Plugin Signature Verification
//!
//! Ed25519 signature verification for native plugins. Uses `ed25519-dalek`
//! to verify that a plugin binary was signed by a trusted publisher.
//!
//! # Workflow
//!
//! 1. Plugin author generates an Ed25519 keypair
//! 2. Author signs the plugin binary (SHA-512 hash of file contents)
//! 3. Signature is stored as `<plugin>.sig` alongside the binary
//! 4. At load time the CLI verifies the signature against trusted public keys

use std::path::Path;

use anyhow::{Context, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha512};

/// A trusted verifying key with optional label
#[derive(Clone)]
pub struct TrustedKey {
    /// Human-readable label
    pub label: String,
    /// The Ed25519 verifying (public) key
    pub key: VerifyingKey,
}

impl std::fmt::Debug for TrustedKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrustedKey")
            .field("label", &self.label)
            .field("key", &hex::encode(self.key.as_bytes()))
            .finish()
    }
}

impl TrustedKey {
    /// Create from raw 32-byte public key
    pub fn from_bytes(label: &str, bytes: &[u8; 32]) -> Result<Self> {
        let key = VerifyingKey::from_bytes(bytes)
            .map_err(|e| anyhow::anyhow!("Invalid public key: {}", e))?;
        Ok(Self {
            label: label.to_string(),
            key,
        })
    }

    /// Create from hex-encoded public key
    pub fn from_hex(label: &str, hex_str: &str) -> Result<Self> {
        let bytes = hex::decode(hex_str).context("Invalid hex")?;
        if bytes.len() != 32 {
            anyhow::bail!("Public key must be 32 bytes, got {}", bytes.len());
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Self::from_bytes(label, &arr)
    }
}

/// Signature verifier for plugin binaries
pub struct SignatureVerifier {
    /// Trusted public keys
    trusted_keys: Vec<TrustedKey>,
    /// Whether unsigned plugins are allowed
    allow_unsigned: bool,
}

impl SignatureVerifier {
    /// Create a new verifier with no trusted keys
    pub fn new() -> Self {
        Self {
            trusted_keys: Vec::new(),
            allow_unsigned: false,
        }
    }

    /// Create a permissive verifier that allows unsigned plugins
    pub fn permissive() -> Self {
        Self {
            trusted_keys: Vec::new(),
            allow_unsigned: true,
        }
    }

    /// Add a trusted key
    pub fn add_trusted_key(&mut self, key: TrustedKey) {
        self.trusted_keys.push(key);
    }

    /// Set whether unsigned plugins are allowed
    pub fn set_allow_unsigned(&mut self, allow: bool) {
        self.allow_unsigned = allow;
    }

    /// Verify a plugin file against its signature file.
    ///
    /// The signature file is expected at `<plugin_path>.sig` and contains
    /// the raw 64-byte Ed25519 signature.
    pub async fn verify_plugin(&self, plugin_path: &Path) -> Result<VerificationResult> {
        // Build the expected signature path
        let sig_path = plugin_path.with_extension(format!(
            "{}.sig",
            plugin_path.extension().and_then(|e| e.to_str()).unwrap_or("bin")
        ));

        if !sig_path.exists() {
            if self.allow_unsigned {
                return Ok(VerificationResult::Unsigned);
            } else {
                anyhow::bail!("Signature file not found: {:?}", sig_path);
            }
        }

        // Read plugin binary
        let plugin_bytes =
            tokio::fs::read(plugin_path).await.context("Failed to read plugin file")?;

        // SHA-512 digest of the plugin binary (what was signed)
        let digest = Sha512::digest(&plugin_bytes);

        // Read signature
        let sig_bytes =
            tokio::fs::read(&sig_path).await.context("Failed to read signature file")?;

        // Support both raw (64 bytes) and hex-encoded signatures
        let sig_raw = if sig_bytes.len() == 64 {
            sig_bytes
        } else {
            // Try hex decode
            let hex_str = std::str::from_utf8(&sig_bytes)
                .context("Signature file is not valid UTF-8 or raw bytes")?
                .trim();
            hex::decode(hex_str).context("Failed to hex-decode signature")?
        };

        if sig_raw.len() != 64 {
            anyhow::bail!("Invalid signature length: expected 64 bytes, got {}", sig_raw.len());
        }

        let signature = Signature::from_slice(&sig_raw)
            .map_err(|e| anyhow::anyhow!("Invalid Ed25519 signature: {}", e))?;

        // Try each trusted key
        for trusted in &self.trusted_keys {
            if trusted.key.verify(&digest, &signature).is_ok() {
                return Ok(VerificationResult::Verified {
                    signer: trusted.label.clone(),
                });
            }
        }

        Ok(VerificationResult::Untrusted)
    }

    /// Verify a plugin from raw bytes and a raw signature
    pub fn verify_bytes(
        &self,
        plugin_bytes: &[u8],
        signature_bytes: &[u8; 64],
    ) -> Result<VerificationResult> {
        let digest = Sha512::digest(plugin_bytes);
        let signature = Signature::from_slice(signature_bytes)
            .map_err(|e| anyhow::anyhow!("Invalid signature: {}", e))?;

        for trusted in &self.trusted_keys {
            if trusted.key.verify(&digest, &signature).is_ok() {
                return Ok(VerificationResult::Verified {
                    signer: trusted.label.clone(),
                });
            }
        }

        Ok(VerificationResult::Untrusted)
    }

    /// Number of trusted keys
    pub fn trusted_key_count(&self) -> usize {
        self.trusted_keys.len()
    }
}

impl Default for SignatureVerifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a signature verification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationResult {
    /// Plugin was signed by a trusted key
    Verified { signer: String },
    /// Plugin has a signature but no trusted key matched
    Untrusted,
    /// Plugin has no signature file (only returned when `allow_unsigned` is true)
    Unsigned,
}

impl VerificationResult {
    /// Whether the plugin is verified (signed by a trusted signer)
    pub fn is_verified(&self) -> bool {
        matches!(self, Self::Verified { .. })
    }

    /// Whether the result allows loading (verified OR unsigned when allowed)
    pub fn is_loadable(&self) -> bool {
        matches!(self, Self::Verified { .. } | Self::Unsigned)
    }
}

// ----- Signing utility (for plugin authors) -----

/// Sign a plugin binary with a secret key.
/// Returns the 64-byte Ed25519 signature.
pub fn sign_plugin(plugin_bytes: &[u8], signing_key: &SigningKey) -> [u8; 64] {
    let digest = Sha512::digest(plugin_bytes);
    let signature = signing_key.sign(&digest);
    signature.to_bytes()
}

/// Generate a new Ed25519 keypair for plugin signing.
/// Returns `(signing_key_bytes, verifying_key_bytes)`.
pub fn generate_keypair() -> ([u8; 32], [u8; 32]) {
    let signing_key = SigningKey::generate(&mut rand::rngs::OsRng);
    let verifying_key = signing_key.verifying_key();
    (signing_key.to_bytes(), verifying_key.to_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_keypair() -> (SigningKey, VerifyingKey) {
        let sk = SigningKey::generate(&mut rand::rngs::OsRng);
        let vk = sk.verifying_key();
        (sk, vk)
    }

    #[test]
    fn test_sign_and_verify() {
        let (sk, vk) = make_keypair();
        let data = b"Hello, this is a fake plugin binary!";

        let sig = sign_plugin(data, &sk);

        let mut verifier = SignatureVerifier::new();
        verifier.add_trusted_key(TrustedKey {
            label: "test-key".to_string(),
            key: vk,
        });

        let result = verifier.verify_bytes(data, &sig).unwrap();
        assert!(result.is_verified());
        assert!(result.is_loadable());

        if let VerificationResult::Verified { signer } = result {
            assert_eq!(signer, "test-key");
        }
    }

    #[test]
    fn test_verify_wrong_key() {
        let (sk, _vk) = make_keypair();
        let (_sk2, vk2) = make_keypair();
        let data = b"plugin binary";

        let sig = sign_plugin(data, &sk);

        let mut verifier = SignatureVerifier::new();
        verifier.add_trusted_key(TrustedKey {
            label: "wrong-key".to_string(),
            key: vk2,
        });

        let result = verifier.verify_bytes(data, &sig).unwrap();
        assert_eq!(result, VerificationResult::Untrusted);
        assert!(!result.is_verified());
        assert!(!result.is_loadable());
    }

    #[test]
    fn test_verify_tampered_data() {
        let (sk, vk) = make_keypair();
        let data = b"original plugin binary";
        let tampered = b"tampered plugin binary";

        let sig = sign_plugin(data, &sk);

        let mut verifier = SignatureVerifier::new();
        verifier.add_trusted_key(TrustedKey {
            label: "author".to_string(),
            key: vk,
        });

        // Verify with tampered data should fail
        let result = verifier.verify_bytes(tampered, &sig).unwrap();
        assert_eq!(result, VerificationResult::Untrusted);
    }

    #[test]
    fn test_generate_keypair() {
        let (sk_bytes, vk_bytes) = generate_keypair();
        assert_eq!(sk_bytes.len(), 32);
        assert_eq!(vk_bytes.len(), 32);

        // Round-trip: create key objects
        let sk = SigningKey::from_bytes(&sk_bytes);
        let vk = VerifyingKey::from_bytes(&vk_bytes).unwrap();

        // Verify they match
        assert_eq!(sk.verifying_key(), vk);
    }

    #[test]
    fn test_trusted_key_from_hex() {
        let (_sk, vk) = make_keypair();
        let hex_str = hex::encode(vk.as_bytes());
        let trusted = TrustedKey::from_hex("test", &hex_str);
        assert!(trusted.is_ok());
    }

    #[test]
    fn test_trusted_key_from_hex_invalid() {
        let result = TrustedKey::from_hex("bad", "not_hex");
        assert!(result.is_err());
    }

    #[test]
    fn test_trusted_key_wrong_length() {
        let result = TrustedKey::from_hex("short", "aabbccdd");
        assert!(result.is_err());
    }

    #[test]
    fn test_verification_result_variants() {
        assert!(
            VerificationResult::Verified {
                signer: "a".to_string()
            }
            .is_loadable()
        );
        assert!(VerificationResult::Unsigned.is_loadable());
        assert!(!VerificationResult::Untrusted.is_loadable());
    }

    #[test]
    fn test_permissive_verifier() {
        let verifier = SignatureVerifier::permissive();
        assert_eq!(verifier.trusted_key_count(), 0);
    }
}
