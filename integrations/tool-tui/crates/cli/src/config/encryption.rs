//! Configuration Secret Encryption
//!
//! AES-256-GCM encryption for sensitive configuration values like API keys.
//! Encrypted values use a `dx:enc:` prefix to identify them.

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use base64::{Engine as _, engine::general_purpose};

/// Prefix for encrypted values in configuration files
const ENCRYPTED_PREFIX: &str = "dx:enc:";

/// Configuration encryption handler using AES-256-GCM
pub struct ConfigEncryption {
    /// AES-256-GCM cipher
    cipher: Aes256Gcm,
}

impl ConfigEncryption {
    /// Create a new encryption handler with the given key
    pub fn new(key: &[u8; 32]) -> Self {
        let cipher = Aes256Gcm::new_from_slice(key).expect("valid 32-byte key");
        Self { cipher }
    }

    /// Create from a password using Argon2 key derivation
    pub fn from_password(password: &str) -> Result<(Self, [u8; 16]), EncryptionError> {
        let salt = generate_salt();
        let key = derive_key(password, &salt)?;
        Ok((Self::new(&key), salt))
    }

    /// Create from a password and existing salt
    pub fn from_password_and_salt(
        password: &str,
        salt: &[u8; 16],
    ) -> Result<Self, EncryptionError> {
        let key = derive_key(password, salt)?;
        Ok(Self::new(&key))
    }

    /// Encrypt a plaintext value and return prefixed ciphertext
    pub fn encrypt(&self, plaintext: &str) -> Result<String, EncryptionError> {
        // Generate random nonce (96 bits)
        let mut nonce_bytes = [0u8; 12];
        getrandom::getrandom(&mut nonce_bytes)
            .map_err(|e| EncryptionError::RandomError(e.to_string()))?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| EncryptionError::EncryptError(e.to_string()))?;

        // Encode as: prefix + base64(nonce + ciphertext)
        let mut combined = Vec::with_capacity(12 + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        Ok(format!("{}{}", ENCRYPTED_PREFIX, base64_encode(&combined)))
    }

    /// Decrypt a prefixed ciphertext value
    pub fn decrypt(&self, encrypted: &str) -> Result<String, EncryptionError> {
        // Check prefix
        let encoded = encrypted
            .strip_prefix(ENCRYPTED_PREFIX)
            .ok_or_else(|| EncryptionError::NotEncrypted)?;

        // Decode base64
        let combined = base64_decode(encoded)?;

        if combined.len() < 12 {
            return Err(EncryptionError::DecryptError("Ciphertext too short".to_string()));
        }

        // Extract nonce and ciphertext
        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| EncryptionError::DecryptError(e.to_string()))?;

        String::from_utf8(plaintext)
            .map_err(|e| EncryptionError::DecryptError(format!("Invalid UTF-8: {}", e)))
    }

    /// Check if a value is encrypted
    pub fn is_encrypted(value: &str) -> bool {
        value.starts_with(ENCRYPTED_PREFIX)
    }

    /// Encrypt all sensitive values in a YAML configuration
    pub fn encrypt_secrets(&self, value: &mut serde_yaml::Value) -> Result<usize, EncryptionError> {
        let mut count = 0;
        encrypt_sensitive_fields(value, &self, &SENSITIVE_KEYS, &mut count)?;
        Ok(count)
    }

    /// Decrypt all encrypted values in a YAML configuration
    pub fn decrypt_secrets(&self, value: &mut serde_yaml::Value) -> Result<usize, EncryptionError> {
        let mut count = 0;
        decrypt_all_fields(value, &self, &mut count)?;
        Ok(count)
    }
}

/// Keys that contain sensitive values and should be encrypted
const SENSITIVE_KEYS: &[&str] = &[
    "api_key",
    "token",
    "secret",
    "password",
    "private_key",
    "encryption_key",
];

/// Recursively encrypt sensitive fields
fn encrypt_sensitive_fields(
    value: &mut serde_yaml::Value,
    enc: &ConfigEncryption,
    sensitive_keys: &[&str],
    count: &mut usize,
) -> Result<(), EncryptionError> {
    match value {
        serde_yaml::Value::Mapping(map) => {
            let keys: Vec<serde_yaml::Value> = map.keys().cloned().collect();
            for key in keys {
                let key_str = key.as_str().unwrap_or("");
                let is_sensitive = sensitive_keys.iter().any(|sk| key_str.contains(sk));

                if let Some(v) = map.get_mut(&key) {
                    if is_sensitive {
                        if let serde_yaml::Value::String(s) = v {
                            if !s.is_empty() && !ConfigEncryption::is_encrypted(s) {
                                *s = enc.encrypt(s)?;
                                *count += 1;
                            }
                        }
                    } else {
                        encrypt_sensitive_fields(v, enc, sensitive_keys, count)?;
                    }
                }
            }
        }
        serde_yaml::Value::Sequence(seq) => {
            for item in seq.iter_mut() {
                encrypt_sensitive_fields(item, enc, sensitive_keys, count)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Recursively decrypt all encrypted fields
fn decrypt_all_fields(
    value: &mut serde_yaml::Value,
    enc: &ConfigEncryption,
    count: &mut usize,
) -> Result<(), EncryptionError> {
    match value {
        serde_yaml::Value::String(s) => {
            if ConfigEncryption::is_encrypted(s) {
                *s = enc.decrypt(s)?;
                *count += 1;
            }
        }
        serde_yaml::Value::Mapping(map) => {
            let keys: Vec<serde_yaml::Value> = map.keys().cloned().collect();
            for key in keys {
                if let Some(v) = map.get_mut(&key) {
                    decrypt_all_fields(v, enc, count)?;
                }
            }
        }
        serde_yaml::Value::Sequence(seq) => {
            for item in seq.iter_mut() {
                decrypt_all_fields(item, enc, count)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Derive an AES-256 key from a password using Argon2id
fn derive_key(password: &str, salt: &[u8; 16]) -> Result<[u8; 32], EncryptionError> {
    use argon2::Argon2;

    let mut key = [0u8; 32];
    let argon2 = Argon2::default();
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| EncryptionError::KeyDerivationError(e.to_string()))?;
    Ok(key)
}

/// Generate a random salt
fn generate_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    getrandom::getrandom(&mut salt).expect("getrandom failed");
    salt
}

/// Generate a random AES-256 key
pub fn generate_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    getrandom::getrandom(&mut key).expect("getrandom failed");
    key
}

/// Save encryption key to a file
pub fn save_key_to_file(key: &[u8; 32], path: &std::path::Path) -> Result<(), EncryptionError> {
    let encoded = hex::encode(key);
    std::fs::write(path, &encoded).map_err(|e| EncryptionError::IoError(e.to_string()))?;

    // Set restrictive permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| EncryptionError::IoError(e.to_string()))?;
    }

    Ok(())
}

/// Load encryption key from a file
pub fn load_key_from_file(path: &std::path::Path) -> Result<[u8; 32], EncryptionError> {
    let content =
        std::fs::read_to_string(path).map_err(|e| EncryptionError::IoError(e.to_string()))?;
    let bytes = hex::decode(content.trim()).map_err(|e| EncryptionError::IoError(e.to_string()))?;
    if bytes.len() != 32 {
        return Err(EncryptionError::IoError(format!(
            "Key file has wrong size: {} bytes (expected 32)",
            bytes.len()
        )));
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(key)
}

/// Base64 encode
fn base64_encode(data: &[u8]) -> String {
    general_purpose::STANDARD.encode(data)
}

/// Base64 decode
fn base64_decode(s: &str) -> Result<Vec<u8>, EncryptionError> {
    general_purpose::STANDARD
        .decode(s)
        .map_err(|e| EncryptionError::DecryptError(format!("Base64 error: {}", e)))
}

/// Encryption errors
#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    #[error("Encryption failed: {0}")]
    EncryptError(String),

    #[error("Decryption failed: {0}")]
    DecryptError(String),

    #[error("Key derivation failed: {0}")]
    KeyDerivationError(String),

    #[error("Random number generation failed: {0}")]
    RandomError(String),

    #[error("Value is not encrypted")]
    NotEncrypted,

    #[error("IO error: {0}")]
    IoError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        for i in 0..32 {
            key[i] = i as u8;
        }
        key
    }

    #[test]
    fn test_encrypt_decrypt() {
        let enc = ConfigEncryption::new(&test_key());
        let plaintext = "sk-secret-api-key-12345";
        let encrypted = enc.encrypt(plaintext).unwrap();

        assert!(encrypted.starts_with(ENCRYPTED_PREFIX));
        assert_ne!(encrypted, plaintext);

        let decrypted = enc.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_different_nonces() {
        let enc = ConfigEncryption::new(&test_key());
        let plaintext = "same-text";
        let enc1 = enc.encrypt(plaintext).unwrap();
        let enc2 = enc.encrypt(plaintext).unwrap();

        // Different encryptions of same plaintext should produce different ciphertexts
        assert_ne!(enc1, enc2);

        // Both should decrypt to the same thing
        assert_eq!(enc.decrypt(&enc1).unwrap(), plaintext);
        assert_eq!(enc.decrypt(&enc2).unwrap(), plaintext);
    }

    #[test]
    fn test_is_encrypted() {
        assert!(ConfigEncryption::is_encrypted("dx:enc:abc123"));
        assert!(!ConfigEncryption::is_encrypted("plain-text"));
        assert!(!ConfigEncryption::is_encrypted(""));
    }

    #[test]
    fn test_wrong_key_fails() {
        let enc1 = ConfigEncryption::new(&test_key());
        let encrypted = enc1.encrypt("secret").unwrap();

        let mut wrong_key = [0u8; 32];
        wrong_key[0] = 255; // Different key
        let enc2 = ConfigEncryption::new(&wrong_key);
        assert!(enc2.decrypt(&encrypted).is_err());
    }

    #[test]
    fn test_not_encrypted_error() {
        let enc = ConfigEncryption::new(&test_key());
        let result = enc.decrypt("not-encrypted");
        assert!(matches!(result, Err(EncryptionError::NotEncrypted)));
    }

    #[test]
    fn test_encrypt_yaml_secrets() {
        let enc = ConfigEncryption::new(&test_key());
        let mut value: serde_yaml::Value = serde_yaml::from_str(
            r#"
llm:
  providers:
    openai:
      api_key: "sk-test-key"
      base_url: "https://api.openai.com"
channels:
  discord:
    token: "bot-token-123"
    enabled: true
gateway:
  port: 31337
"#,
        )
        .unwrap();

        let count = enc.encrypt_secrets(&mut value).unwrap();
        assert_eq!(count, 2); // api_key and token

        // Verify api_key is encrypted
        let api_key = value["llm"]["providers"]["openai"]["api_key"].as_str().unwrap();
        assert!(ConfigEncryption::is_encrypted(api_key));

        // Verify non-sensitive fields are NOT encrypted
        let port = &value["gateway"]["port"];
        assert!(port.is_number());
    }

    #[test]
    fn test_decrypt_yaml_secrets() {
        let enc = ConfigEncryption::new(&test_key());
        let mut value: serde_yaml::Value = serde_yaml::from_str(
            r#"
llm:
  providers:
    openai:
      api_key: "sk-test-key"
"#,
        )
        .unwrap();

        // Encrypt first
        enc.encrypt_secrets(&mut value).unwrap();

        // Then decrypt
        let count = enc.decrypt_secrets(&mut value).unwrap();
        assert_eq!(count, 1);

        let api_key = value["llm"]["providers"]["openai"]["api_key"].as_str().unwrap();
        assert_eq!(api_key, "sk-test-key");
    }

    #[test]
    fn test_key_file_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join("key.hex");

        let key = test_key();
        save_key_to_file(&key, &key_path).unwrap();
        let loaded = load_key_from_file(&key_path).unwrap();
        assert_eq!(key, loaded);
    }

    #[test]
    fn test_generate_key() {
        let key1 = generate_key();
        let key2 = generate_key();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_from_password() {
        let (enc, salt) = ConfigEncryption::from_password("my-password").unwrap();
        let encrypted = enc.encrypt("secret").unwrap();

        let enc2 = ConfigEncryption::from_password_and_salt("my-password", &salt).unwrap();
        let decrypted = enc2.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, "secret");
    }
}
