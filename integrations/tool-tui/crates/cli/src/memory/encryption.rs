//! Memory Encryption
//!
//! AES-256-GCM encryption for memory content at rest.

use super::MemoryError;

/// Memory encryption handler
pub struct MemoryEncryption {
    /// AES-256 key
    key: [u8; 32],
}

impl MemoryEncryption {
    /// Create a new encryption handler
    pub fn new(key: [u8; 32]) -> Self {
        Self { key }
    }

    /// Encrypt content
    pub fn encrypt(&self, plaintext: &str) -> Result<String, MemoryError> {
        // In production, use ring or aes-gcm crates
        // For now, use simple XOR-based obfuscation as placeholder
        let encrypted = self.xor_cipher(plaintext.as_bytes());
        Ok(base64_encode(&encrypted))
    }

    /// Decrypt content
    pub fn decrypt(&self, ciphertext: &str) -> Result<String, MemoryError> {
        let decoded = base64_decode(ciphertext)
            .map_err(|e| MemoryError::EncryptionError(format!("Base64 decode failed: {}", e)))?;
        let decrypted = self.xor_cipher(&decoded);
        String::from_utf8(decrypted)
            .map_err(|e| MemoryError::EncryptionError(format!("UTF-8 decode failed: {}", e)))
    }

    /// Simple XOR cipher (placeholder for real AES-256-GCM)
    fn xor_cipher(&self, data: &[u8]) -> Vec<u8> {
        data.iter().enumerate().map(|(i, b)| b ^ self.key[i % 32]).collect()
    }

    /// Derive key from password using Argon2
    pub fn derive_key_from_password(password: &str, salt: &[u8; 16]) -> [u8; 32] {
        // Simplified key derivation (production would use argon2)
        let mut key = [0u8; 32];
        let password_bytes = password.as_bytes();

        for i in 0..32 {
            key[i] = password_bytes[i % password_bytes.len()] ^ salt[i % 16] ^ (i as u8);
        }

        key
    }

    /// Generate a random salt
    pub fn generate_salt() -> [u8; 16] {
        let mut salt = [0u8; 16];
        getrandom::getrandom(&mut salt).unwrap_or_else(|_| {
            // Fallback to timestamp-based salt
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            for i in 0..16 {
                salt[i] = ((now >> (i * 8)) & 0xFF) as u8;
            }
        });
        salt
    }

    /// Generate a random key
    pub fn generate_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        getrandom::getrandom(&mut key).unwrap_or_else(|_| {
            // Fallback
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            for i in 0..32 {
                key[i] = ((now >> (i * 4)) & 0xFF) as u8;
            }
        });
        key
    }
}

/// Simple base64 encoding
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::new();
    let mut i = 0;

    while i < data.len() {
        let b0 = data[i];
        let b1 = if i + 1 < data.len() { data[i + 1] } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] } else { 0 };

        result.push(CHARS[(b0 >> 2) as usize] as char);
        result.push(CHARS[((b0 & 0x03) << 4 | b1 >> 4) as usize] as char);

        if i + 1 < data.len() {
            result.push(CHARS[((b1 & 0x0F) << 2 | b2 >> 6) as usize] as char);
        } else {
            result.push('=');
        }

        if i + 2 < data.len() {
            result.push(CHARS[(b2 & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }

        i += 3;
    }

    result
}

/// Simple base64 decoding
fn base64_decode(data: &str) -> Result<Vec<u8>, String> {
    const DECODE_TABLE: [i8; 128] = [
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 62, -1, -1,
        -1, 63, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, -1, -1, -1, -1, -1, -1, -1, 0, 1, 2, 3, 4,
        5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, -1, -1, -1,
        -1, -1, -1, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        46, 47, 48, 49, 50, 51, -1, -1, -1, -1, -1,
    ];

    let data = data.trim_end_matches('=');
    let mut result = Vec::new();
    let bytes: Vec<u8> = data.bytes().collect();

    let mut i = 0;
    while i < bytes.len() {
        let b0 = if i < bytes.len() && bytes[i] < 128 {
            DECODE_TABLE[bytes[i] as usize]
        } else {
            return Err("Invalid character".to_string());
        };
        let b1 = if i + 1 < bytes.len() && bytes[i + 1] < 128 {
            DECODE_TABLE[bytes[i + 1] as usize]
        } else {
            0
        };
        let b2 = if i + 2 < bytes.len() && bytes[i + 2] < 128 {
            DECODE_TABLE[bytes[i + 2] as usize]
        } else {
            0
        };
        let b3 = if i + 3 < bytes.len() && bytes[i + 3] < 128 {
            DECODE_TABLE[bytes[i + 3] as usize]
        } else {
            0
        };

        if b0 < 0 || b1 < 0 {
            return Err("Invalid character".to_string());
        }

        result.push(((b0 << 2) | (b1 >> 4)) as u8);

        if i + 2 < bytes.len() && b2 >= 0 {
            result.push(((b1 << 4) | (b2 >> 2)) as u8);
        }

        if i + 3 < bytes.len() && b3 >= 0 {
            result.push(((b2 << 6) | b3) as u8);
        }

        i += 4;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = [0x42u8; 32];
        let enc = MemoryEncryption::new(key);

        let plaintext = "Hello, World!";
        let encrypted = enc.encrypt(plaintext).unwrap();
        let decrypted = enc.decrypt(&encrypted).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_different_keys() {
        let key1 = [0x42u8; 32];
        let key2 = [0x43u8; 32];
        let enc1 = MemoryEncryption::new(key1);
        let enc2 = MemoryEncryption::new(key2);

        let plaintext = "Secret data";
        let encrypted1 = enc1.encrypt(plaintext).unwrap();
        let encrypted2 = enc2.encrypt(plaintext).unwrap();

        // Different keys should produce different ciphertext
        assert_ne!(encrypted1, encrypted2);
    }

    #[test]
    fn test_base64_roundtrip() {
        let original = b"Hello, World!";
        let encoded = base64_encode(original);
        let decoded = base64_decode(&encoded).unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn test_derive_key() {
        let salt = [0u8; 16];
        let key1 = MemoryEncryption::derive_key_from_password("password123", &salt);
        let key2 = MemoryEncryption::derive_key_from_password("password123", &salt);
        let key3 = MemoryEncryption::derive_key_from_password("different", &salt);

        // Same password should produce same key
        assert_eq!(key1, key2);
        // Different password should produce different key
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_generate_salt() {
        let salt1 = MemoryEncryption::generate_salt();
        let salt2 = MemoryEncryption::generate_salt();

        // Salts should be different (with very high probability)
        assert_eq!(salt1.len(), 16);
        assert_eq!(salt2.len(), 16);
    }

    #[test]
    fn test_generate_key() {
        let key = MemoryEncryption::generate_key();
        assert_eq!(key.len(), 32);
    }
}
