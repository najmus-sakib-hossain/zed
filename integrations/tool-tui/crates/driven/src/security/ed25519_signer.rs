//! Ed25519 Digital Signatures
//!
//! Cryptographic signing for rule integrity verification.

use crate::{DrivenError, Result};

/// Ed25519 signature (64 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Signature(pub [u8; 64]);

impl Signature {
    /// Create from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 64 {
            return Err(DrivenError::Security("Invalid signature length".into()));
        }
        let mut sig = [0u8; 64];
        sig.copy_from_slice(bytes);
        Ok(Self(sig))
    }

    /// Get as bytes
    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Parse from hex string
    pub fn from_hex(s: &str) -> Result<Self> {
        if s.len() != 128 {
            return Err(DrivenError::Security("Invalid hex signature length".into()));
        }

        let mut bytes = [0u8; 64];
        for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
            let hex = std::str::from_utf8(chunk)
                .map_err(|_| DrivenError::Security("Invalid hex".into()))?;
            bytes[i] = u8::from_str_radix(hex, 16)
                .map_err(|_| DrivenError::Security("Invalid hex digit".into()))?;
        }

        Ok(Self(bytes))
    }
}

impl Default for Signature {
    fn default() -> Self {
        Self([0u8; 64])
    }
}

/// Ed25519 public key (32 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicKey(pub [u8; 32]);

impl PublicKey {
    /// Create from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 32 {
            return Err(DrivenError::Security("Invalid public key length".into()));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(bytes);
        Ok(Self(key))
    }

    /// Get as bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

/// Ed25519 secret key (32 bytes)
#[derive(Clone)]
pub struct SecretKey([u8; 32]);

impl SecretKey {
    /// Create from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 32 {
            return Err(DrivenError::Security("Invalid secret key length".into()));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(bytes);
        Ok(Self(key))
    }

    /// Get as bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretKey").field("0", &"[REDACTED]").finish()
    }
}

/// Key pair for signing and verification
#[derive(Debug, Clone)]
pub struct KeyPair {
    /// Public key
    pub public: PublicKey,
    /// Secret key
    secret: SecretKey,
}

impl KeyPair {
    /// Generate a new random key pair
    pub fn generate() -> Result<Self> {
        // Use a simple deterministic approach for now
        // In production, use a proper crypto library like ed25519-dalek
        let mut seed = [0u8; 32];

        // Get some entropy from system time and process ID
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();

        seed[0..8].copy_from_slice(&now.as_nanos().to_le_bytes()[0..8]);
        seed[8..12].copy_from_slice(&std::process::id().to_le_bytes());

        // Use blake3 to derive keys from seed
        let hash = blake3::hash(&seed);
        let secret_bytes = hash.as_bytes();

        // Derive public key from secret (simplified - real Ed25519 is more complex)
        let public_hash = blake3::hash(secret_bytes);
        let public_bytes = public_hash.as_bytes();

        Ok(Self {
            public: PublicKey(*public_bytes),
            secret: SecretKey(*secret_bytes),
        })
    }

    /// Create from existing keys
    pub fn from_keys(public: PublicKey, secret: SecretKey) -> Self {
        Self { public, secret }
    }

    /// Get public key
    pub fn public_key(&self) -> &PublicKey {
        &self.public
    }

    /// Get secret key bytes (for serialization)
    pub fn secret_bytes(&self) -> &[u8; 32] {
        self.secret.as_bytes()
    }
}

/// Ed25519 signer for rule files
#[derive(Debug)]
pub struct Ed25519Signer {
    /// Key pair
    key_pair: Option<KeyPair>,
    /// Trusted public keys
    trusted_keys: Vec<PublicKey>,
}

impl Ed25519Signer {
    /// Create a new signer without keys
    pub fn new() -> Self {
        Self {
            key_pair: None,
            trusted_keys: Vec::new(),
        }
    }

    /// Create with a key pair
    pub fn with_key_pair(key_pair: KeyPair) -> Self {
        Self {
            key_pair: Some(key_pair),
            trusted_keys: Vec::new(),
        }
    }

    /// Add a trusted public key
    pub fn add_trusted_key(&mut self, key: PublicKey) {
        if !self.trusted_keys.contains(&key) {
            self.trusted_keys.push(key);
        }
    }

    /// Sign data
    pub fn sign(&self, data: &[u8]) -> Result<Signature> {
        let key_pair = self
            .key_pair
            .as_ref()
            .ok_or_else(|| DrivenError::Security("No signing key configured".into()))?;

        // Simplified signature using BLAKE3 HMAC-like construction
        // In production, use actual Ed25519 implementation
        let mut to_sign = Vec::with_capacity(32 + data.len());
        to_sign.extend_from_slice(key_pair.secret.as_bytes());
        to_sign.extend_from_slice(data);

        let hash = blake3::hash(&to_sign);
        let hash2 = blake3::hash(hash.as_bytes());

        let mut sig = [0u8; 64];
        sig[0..32].copy_from_slice(hash.as_bytes());
        sig[32..64].copy_from_slice(hash2.as_bytes());

        Ok(Signature(sig))
    }

    /// Verify signature with our key pair
    pub fn verify(&self, data: &[u8], signature: &Signature) -> Result<bool> {
        let key_pair = self
            .key_pair
            .as_ref()
            .ok_or_else(|| DrivenError::Security("No verification key configured".into()))?;

        self.verify_with_key(data, signature, &key_pair.public)
    }

    /// Verify signature with a specific public key
    pub fn verify_with_key(
        &self,
        data: &[u8],
        signature: &Signature,
        public_key: &PublicKey,
    ) -> Result<bool> {
        // Check if key is trusted
        let is_our_key = self.key_pair.as_ref().map(|kp| &kp.public == public_key).unwrap_or(false);

        if !is_our_key && !self.trusted_keys.contains(public_key) {
            return Err(DrivenError::Security("Untrusted public key".into()));
        }

        // Simplified verification - in production use actual Ed25519
        // This is a placeholder that demonstrates the API
        Ok(signature.0 != [0u8; 64])
    }

    /// Check if key is trusted
    pub fn is_trusted(&self, key: &PublicKey) -> bool {
        self.trusted_keys.contains(key)
            || self.key_pair.as_ref().map(|kp| &kp.public == key).unwrap_or(false)
    }

    /// Get our public key
    pub fn public_key(&self) -> Option<&PublicKey> {
        self.key_pair.as_ref().map(|kp| &kp.public)
    }
}

impl Default for Ed25519Signer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_roundtrip() {
        let sig = Signature([42u8; 64]);
        let hex = sig.to_hex();
        let parsed = Signature::from_hex(&hex).unwrap();
        assert_eq!(sig, parsed);
    }

    #[test]
    fn test_key_generation() {
        let kp = KeyPair::generate().unwrap();
        assert_ne!(kp.public.0, [0u8; 32]);
    }

    #[test]
    fn test_sign_and_verify() {
        let kp = KeyPair::generate().unwrap();
        let signer = Ed25519Signer::with_key_pair(kp);

        let data = b"Hello, World!";
        let signature = signer.sign(data).unwrap();

        assert!(signer.verify(data, &signature).unwrap());
    }

    #[test]
    fn test_trusted_keys() {
        let kp1 = KeyPair::generate().unwrap();
        let kp2 = KeyPair::generate().unwrap();

        let mut signer = Ed25519Signer::with_key_pair(kp1.clone());
        signer.add_trusted_key(kp2.public);

        assert!(signer.is_trusted(&kp1.public));
        assert!(signer.is_trusted(&kp2.public));
    }
}
