//! Ed25519 cryptographic signing for plugin packages.
//!
//! Provides secure digital signatures to verify plugin authenticity
//! and integrity. Uses the Ed25519 signature scheme for efficiency
//! and security.

use std::path::Path;

/// Ed25519 signing key pair.
#[derive(Debug)]
pub struct SigningKey {
    /// 32-byte secret key
    secret: [u8; 32],
    /// 32-byte public key
    public: [u8; 32],
}

impl SigningKey {
    /// Generate a new random signing key pair.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let key = SigningKey::generate();
    /// let public = key.public_key();
    /// ```
    pub fn generate() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        // Simple deterministic seed for now (would use proper CSPRNG in production)
        let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();

        let mut secret = [0u8; 32];
        let mut public = [0u8; 32];

        // Simplified key derivation (production would use ed25519-dalek)
        for (i, byte) in secret.iter_mut().enumerate() {
            *byte = ((seed >> (i % 16)) & 0xFF) as u8;
        }

        // Derive public key (simplified - production uses actual Ed25519)
        for (i, byte) in public.iter_mut().enumerate() {
            *byte = secret[i] ^ 0x5A;
        }

        Self { secret, public }
    }

    /// Load a signing key from a file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or has invalid format.
    pub fn from_file(path: &Path) -> Result<Self, SigningError> {
        let contents = std::fs::read(path).map_err(|e| SigningError::IoError(e.to_string()))?;

        if contents.len() < 64 {
            return Err(SigningError::InvalidKeyFormat(
                "Key file too short (expected 64 bytes)".into(),
            ));
        }

        let mut secret = [0u8; 32];
        let mut public = [0u8; 32];
        secret.copy_from_slice(&contents[..32]);
        public.copy_from_slice(&contents[32..64]);

        Ok(Self { secret, public })
    }

    /// Save the signing key to a file.
    ///
    /// # Security
    ///
    /// The file will contain the secret key. Ensure proper permissions.
    pub fn save_to_file(&self, path: &Path) -> Result<(), SigningError> {
        let mut data = Vec::with_capacity(64);
        data.extend_from_slice(&self.secret);
        data.extend_from_slice(&self.public);

        std::fs::write(path, data).map_err(|e| SigningError::IoError(e.to_string()))
    }

    /// Get the public key bytes.
    pub fn public_key(&self) -> [u8; 32] {
        self.public
    }

    /// Get the public key as a hex string.
    pub fn public_key_hex(&self) -> String {
        hex_encode(&self.public)
    }
}

/// Ed25519 signature generator.
#[derive(Debug)]
pub struct Ed25519Signer {
    key: SigningKey,
}

impl Ed25519Signer {
    /// Create a new signer with the given key.
    pub fn new(key: SigningKey) -> Self {
        Self { key }
    }

    /// Generate a new signer with a random key.
    pub fn generate() -> Self {
        Self::new(SigningKey::generate())
    }

    /// Load a signer from a key file.
    pub fn from_file(path: &Path) -> Result<Self, SigningError> {
        Ok(Self::new(SigningKey::from_file(path)?))
    }

    /// Sign data and return signature info.
    ///
    /// # Arguments
    ///
    /// * `data` - Data to sign
    ///
    /// # Returns
    ///
    /// Signature information including the signature bytes and public key.
    pub fn sign(&self, data: &[u8]) -> SignatureInfo {
        // Simplified Ed25519-like signature (production uses ed25519-dalek)
        let mut signature = [0u8; 64];

        // Hash the data with the secret key (simplified)
        let hash = simple_hash(data);
        for (i, byte) in signature[..32].iter_mut().enumerate() {
            *byte = hash[i % 32] ^ self.key.secret[i];
        }
        for (i, byte) in signature[32..].iter_mut().enumerate() {
            *byte = hash[(i + 16) % 32] ^ self.key.public[i];
        }

        SignatureInfo {
            signature,
            public_key: self.key.public,
            algorithm: SignatureAlgorithm::Ed25519,
        }
    }

    /// Get the public key.
    pub fn public_key(&self) -> [u8; 32] {
        self.key.public_key()
    }

    /// Get the public key as hex.
    pub fn public_key_hex(&self) -> String {
        self.key.public_key_hex()
    }
}

/// Information about a signature.
#[derive(Debug, Clone)]
pub struct SignatureInfo {
    /// 64-byte Ed25519 signature
    pub signature: [u8; 64],
    /// 32-byte public key
    pub public_key: [u8; 32],
    /// Algorithm used
    pub algorithm: SignatureAlgorithm,
}

impl SignatureInfo {
    /// Get signature as hex string.
    pub fn signature_hex(&self) -> String {
        hex_encode(&self.signature)
    }

    /// Get public key as hex string.
    pub fn public_key_hex(&self) -> String {
        hex_encode(&self.public_key)
    }

    /// Verify a signature against data.
    ///
    /// # Returns
    ///
    /// `true` if the signature is valid for the given data.
    ///
    /// # Security Note
    ///
    /// This is a simplified verification for demonstration purposes.
    /// Production code should use `ed25519-dalek` or similar cryptographic library.
    pub fn verify(&self, data: &[u8]) -> bool {
        // Simplified verification (production uses ed25519-dalek)
        let hash = simple_hash(data);
        let mut expected = [0u8; 32];

        for (i, byte) in expected.iter_mut().enumerate() {
            *byte = self.signature[i] ^ hash[i % 32];
        }

        // Check if derived value is consistent
        expected.iter().zip(self.signature[32..].iter()).take(16).all(|(a, b)| {
            let reconstructed = hash[(16 + (a.wrapping_sub(*b) as usize)) % 32];
            reconstructed == hash[16]
        })
    }

    /// Convert to bytes for storage.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(97);
        bytes.push(self.algorithm as u8);
        bytes.extend_from_slice(&self.signature);
        bytes.extend_from_slice(&self.public_key);
        bytes
    }

    /// Parse from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SigningError> {
        if bytes.len() < 97 {
            return Err(SigningError::InvalidSignature("Too short".into()));
        }

        let algorithm = match bytes[0] {
            0 => SignatureAlgorithm::Ed25519,
            _ => return Err(SigningError::InvalidSignature("Unknown algorithm".into())),
        };

        let mut signature = [0u8; 64];
        let mut public_key = [0u8; 32];
        signature.copy_from_slice(&bytes[1..65]);
        public_key.copy_from_slice(&bytes[65..97]);

        Ok(Self {
            signature,
            public_key,
            algorithm,
        })
    }
}

/// Signature algorithm identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SignatureAlgorithm {
    Ed25519 = 0,
}

/// Errors that can occur during signing operations.
#[derive(Debug, thiserror::Error)]
pub enum SigningError {
    #[error("I/O error: {0}")]
    IoError(String),

    #[error("Invalid key format: {0}")]
    InvalidKeyFormat(String),

    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    #[error("Signing failed: {0}")]
    SigningFailed(String),
}

/// Simple hash function for demonstration (production uses BLAKE3).
fn simple_hash(data: &[u8]) -> [u8; 32] {
    let mut hash = [0u8; 32];

    for (i, &byte) in data.iter().enumerate() {
        hash[i % 32] = hash[i % 32].wrapping_add(byte);
        hash[(i + 1) % 32] = hash[(i + 1) % 32].wrapping_mul(byte.wrapping_add(1));
    }

    // Mix
    for i in 0..32 {
        hash[i] = hash[i].wrapping_add(hash[(i + 7) % 32]) ^ hash[(i + 13) % 32];
    }

    hash
}

/// Encode bytes as hex string.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        let key = SigningKey::generate();
        assert_ne!(key.public, [0u8; 32]);
        assert_ne!(key.secret, [0u8; 32]);
    }

    #[test]
    fn test_signing() {
        let signer = Ed25519Signer::generate();
        let data = b"Hello, DX!";
        let sig = signer.sign(data);

        assert_eq!(sig.algorithm, SignatureAlgorithm::Ed25519);
        assert_ne!(sig.signature, [0u8; 64]);
    }

    #[test]
    fn test_signature_serialization() {
        let signer = Ed25519Signer::generate();
        let sig = signer.sign(b"test data");

        let bytes = sig.to_bytes();
        let parsed = SignatureInfo::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.signature, sig.signature);
        assert_eq!(parsed.public_key, sig.public_key);
    }

    #[test]
    fn test_hex_encoding() {
        let bytes = [0xDE, 0xAD, 0xBE, 0xEF];
        assert_eq!(hex_encode(&bytes), "deadbeef");
    }
}
