//! Secrets Management
//!
//! Secure storage and retrieval of secrets with AES-256-GCM encryption.
//! Supports environment variables, files, and external secret stores.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use rand::{RngCore, rngs::OsRng};
use zeroize::Zeroize;

use super::SecurityError;

/// Secret value with metadata
#[derive(Clone)]
pub struct Secret {
    /// Encrypted value
    encrypted: Vec<u8>,
    /// Nonce used for encryption
    nonce: [u8; 12],
    /// Secret name
    pub name: String,
    /// Secret category
    pub category: SecretCategory,
    /// Created timestamp (Unix)
    pub created_at: u64,
    /// Last accessed timestamp (Unix)
    pub accessed_at: u64,
    /// Access count
    pub access_count: u64,
    /// Rotation required
    pub rotation_required: bool,
}

/// Secret category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretCategory {
    /// API keys
    ApiKey,
    /// Database credentials
    Database,
    /// OAuth tokens
    OAuthToken,
    /// SSH keys
    SshKey,
    /// TLS certificates
    TlsCert,
    /// Generic password
    Password,
    /// Environment variable
    EnvVar,
    /// Other
    Other,
}

impl Secret {
    /// Create new encrypted secret
    fn new(
        name: &str,
        value: &[u8],
        category: SecretCategory,
        cipher: &Aes256Gcm,
    ) -> Result<Self, SecurityError> {
        // Generate random nonce
        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);

        // Encrypt value
        let nonce_ga = Nonce::from_slice(&nonce);
        let encrypted = cipher
            .encrypt(nonce_ga, value)
            .map_err(|e| SecurityError::EncryptionError(e.to_string()))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(Self {
            encrypted,
            nonce,
            name: name.to_string(),
            category,
            created_at: now,
            accessed_at: now,
            access_count: 0,
            rotation_required: false,
        })
    }

    /// Decrypt secret value
    fn decrypt(&mut self, cipher: &Aes256Gcm) -> Result<Vec<u8>, SecurityError> {
        let nonce = Nonce::from_slice(&self.nonce);
        let plaintext = cipher
            .decrypt(nonce, self.encrypted.as_ref())
            .map_err(|e| SecurityError::EncryptionError(e.to_string()))?;

        // Update access tracking
        self.accessed_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.access_count += 1;

        Ok(plaintext)
    }
}

/// Secrets manager
pub struct SecretsManager {
    /// Encryption cipher
    cipher: Aes256Gcm,
    /// Master key (zeroized on drop)
    master_key: [u8; 32],
    /// Stored secrets
    secrets: RwLock<HashMap<String, Secret>>,
    /// Storage path
    storage_path: Option<PathBuf>,
    /// Environment prefix for env vars
    env_prefix: String,
}

impl SecretsManager {
    /// Create new secrets manager with generated key
    pub fn new() -> Result<Self, SecurityError> {
        let mut master_key = [0u8; 32];
        OsRng.fill_bytes(&mut master_key);

        let cipher = Aes256Gcm::new_from_slice(&master_key)
            .map_err(|e| SecurityError::EncryptionError(e.to_string()))?;

        Ok(Self {
            cipher,
            master_key,
            secrets: RwLock::new(HashMap::new()),
            storage_path: None,
            env_prefix: "DX_".into(),
        })
    }

    /// Create from existing master key
    pub fn from_key(key: &[u8; 32]) -> Result<Self, SecurityError> {
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| SecurityError::EncryptionError(e.to_string()))?;

        let mut master_key = [0u8; 32];
        master_key.copy_from_slice(key);

        Ok(Self {
            cipher,
            master_key,
            secrets: RwLock::new(HashMap::new()),
            storage_path: None,
            env_prefix: "DX_".into(),
        })
    }

    /// Derive key from password using Argon2
    pub fn from_password(password: &str, salt: &[u8]) -> Result<Self, SecurityError> {
        use argon2::{
            Argon2,
            password_hash::{PasswordHasher, SaltString},
        };

        let salt_string = SaltString::encode_b64(salt)
            .map_err(|e| SecurityError::EncryptionError(e.to_string()))?;

        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(password.as_bytes(), &salt_string)
            .map_err(|e| SecurityError::EncryptionError(e.to_string()))?;

        let hash_bytes = hash
            .hash
            .ok_or_else(|| SecurityError::EncryptionError("Failed to get hash bytes".into()))?;

        let mut key = [0u8; 32];
        key.copy_from_slice(&hash_bytes.as_bytes()[..32]);

        Self::from_key(&key)
    }

    /// Set storage path
    pub fn with_storage(mut self, path: PathBuf) -> Self {
        self.storage_path = Some(path);
        self
    }

    /// Set environment variable prefix
    pub fn with_env_prefix(mut self, prefix: &str) -> Self {
        self.env_prefix = prefix.to_string();
        self
    }

    /// Store a secret
    pub fn set(
        &self,
        name: &str,
        value: &[u8],
        category: SecretCategory,
    ) -> Result<(), SecurityError> {
        let secret = Secret::new(name, value, category, &self.cipher)?;

        let mut secrets = self.secrets.write().unwrap();
        secrets.insert(name.to_string(), secret);

        Ok(())
    }

    /// Store string secret
    pub fn set_string(
        &self,
        name: &str,
        value: &str,
        category: SecretCategory,
    ) -> Result<(), SecurityError> {
        self.set(name, value.as_bytes(), category)
    }

    /// Get secret value
    pub fn get(&self, name: &str) -> Result<Vec<u8>, SecurityError> {
        let mut secrets = self.secrets.write().unwrap();
        let secret = secrets
            .get_mut(name)
            .ok_or_else(|| SecurityError::SecretNotFound(name.into()))?;

        secret.decrypt(&self.cipher)
    }

    /// Get secret as string
    pub fn get_string(&self, name: &str) -> Result<String, SecurityError> {
        let bytes = self.get(name)?;
        String::from_utf8(bytes).map_err(|_| SecurityError::EncryptionError("Invalid UTF-8".into()))
    }

    /// Check if secret exists
    pub fn exists(&self, name: &str) -> bool {
        self.secrets.read().unwrap().contains_key(name)
    }

    /// Delete secret
    pub fn delete(&self, name: &str) -> bool {
        self.secrets.write().unwrap().remove(name).is_some()
    }

    /// List secret names
    pub fn list(&self) -> Vec<String> {
        self.secrets.read().unwrap().keys().cloned().collect()
    }

    /// List secrets by category
    pub fn list_by_category(&self, category: SecretCategory) -> Vec<String> {
        self.secrets
            .read()
            .unwrap()
            .iter()
            .filter(|(_, s)| s.category == category)
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// Load secret from environment variable
    pub fn load_from_env(&self, name: &str) -> Result<(), SecurityError> {
        let env_name = format!("{}{}", self.env_prefix, name.to_uppercase().replace('-', "_"));

        let value = std::env::var(&env_name)
            .map_err(|_| SecurityError::SecretNotFound(env_name.clone()))?;

        self.set_string(name, &value, SecretCategory::EnvVar)
    }

    /// Load all secrets from environment
    pub fn load_all_from_env(&self) -> Vec<String> {
        let mut loaded = Vec::new();

        for (key, value) in std::env::vars() {
            if key.starts_with(&self.env_prefix) {
                let name = key[self.env_prefix.len()..].to_lowercase().replace('_', "-");

                if self.set_string(&name, &value, SecretCategory::EnvVar).is_ok() {
                    loaded.push(name);
                }
            }
        }

        loaded
    }

    /// Export master key (base64 encoded)
    pub fn export_key(&self) -> String {
        BASE64.encode(self.master_key)
    }

    /// Get secret metadata
    pub fn metadata(&self, name: &str) -> Option<SecretMetadata> {
        self.secrets.read().unwrap().get(name).map(|s| SecretMetadata {
            name: s.name.clone(),
            category: s.category,
            created_at: s.created_at,
            accessed_at: s.accessed_at,
            access_count: s.access_count,
            rotation_required: s.rotation_required,
        })
    }

    /// Mark secret for rotation
    pub fn mark_for_rotation(&self, name: &str) -> bool {
        if let Some(secret) = self.secrets.write().unwrap().get_mut(name) {
            secret.rotation_required = true;
            true
        } else {
            false
        }
    }

    /// Get secrets requiring rotation
    pub fn secrets_requiring_rotation(&self) -> Vec<String> {
        self.secrets
            .read()
            .unwrap()
            .iter()
            .filter(|(_, s)| s.rotation_required)
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// Rotate a secret
    pub fn rotate(&self, name: &str, new_value: &[u8]) -> Result<(), SecurityError> {
        let category = {
            let secrets = self.secrets.read().unwrap();
            secrets
                .get(name)
                .map(|s| s.category)
                .ok_or_else(|| SecurityError::SecretNotFound(name.into()))?
        };

        // Create new secret with same category
        let mut secret = Secret::new(name, new_value, category, &self.cipher)?;
        secret.rotation_required = false;

        let mut secrets = self.secrets.write().unwrap();
        secrets.insert(name.to_string(), secret);

        Ok(())
    }

    /// Save to file (encrypted)
    pub async fn save(&self) -> Result<(), SecurityError> {
        let path = self
            .storage_path
            .as_ref()
            .ok_or_else(|| SecurityError::EncryptionError("No storage path configured".into()))?;

        let secrets = self.secrets.read().unwrap();

        // Serialize secrets
        let mut data = Vec::new();
        for (name, secret) in secrets.iter() {
            // Format: name_len(4) | name | category(1) | nonce(12) | encrypted_len(4) | encrypted
            data.extend_from_slice(&(name.len() as u32).to_le_bytes());
            data.extend_from_slice(name.as_bytes());
            data.push(secret.category as u8);
            data.extend_from_slice(&secret.nonce);
            data.extend_from_slice(&(secret.encrypted.len() as u32).to_le_bytes());
            data.extend_from_slice(&secret.encrypted);
        }

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(path, data).await?;

        Ok(())
    }

    /// Load from file
    pub async fn load(&self) -> Result<usize, SecurityError> {
        let path = self
            .storage_path
            .as_ref()
            .ok_or_else(|| SecurityError::EncryptionError("No storage path configured".into()))?;

        if !path.exists() {
            return Ok(0);
        }

        let data = tokio::fs::read(path).await?;
        let mut secrets = self.secrets.write().unwrap();
        let mut loaded = 0;
        let mut offset = 0;

        while offset < data.len() {
            // Read name length
            if offset + 4 > data.len() {
                break;
            }
            let name_len = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
            offset += 4;

            // Read name
            if offset + name_len > data.len() {
                break;
            }
            let name = String::from_utf8_lossy(&data[offset..offset + name_len]).to_string();
            offset += name_len;

            // Read category
            if offset >= data.len() {
                break;
            }
            let category = match data[offset] {
                0 => SecretCategory::ApiKey,
                1 => SecretCategory::Database,
                2 => SecretCategory::OAuthToken,
                3 => SecretCategory::SshKey,
                4 => SecretCategory::TlsCert,
                5 => SecretCategory::Password,
                6 => SecretCategory::EnvVar,
                _ => SecretCategory::Other,
            };
            offset += 1;

            // Read nonce
            if offset + 12 > data.len() {
                break;
            }
            let mut nonce = [0u8; 12];
            nonce.copy_from_slice(&data[offset..offset + 12]);
            offset += 12;

            // Read encrypted length
            if offset + 4 > data.len() {
                break;
            }
            let encrypted_len = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
            offset += 4;

            // Read encrypted
            if offset + encrypted_len > data.len() {
                break;
            }
            let encrypted = data[offset..offset + encrypted_len].to_vec();
            offset += encrypted_len;

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            secrets.insert(
                name.clone(),
                Secret {
                    encrypted,
                    nonce,
                    name,
                    category,
                    created_at: now,
                    accessed_at: now,
                    access_count: 0,
                    rotation_required: false,
                },
            );
            loaded += 1;
        }

        Ok(loaded)
    }
}

impl Drop for SecretsManager {
    fn drop(&mut self) {
        self.master_key.zeroize();
    }
}

impl Default for SecretsManager {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

/// Secret metadata (without value)
#[derive(Debug)]
pub struct SecretMetadata {
    pub name: String,
    pub category: SecretCategory,
    pub created_at: u64,
    pub accessed_at: u64,
    pub access_count: u64,
    pub rotation_required: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_get_secret() {
        let manager = SecretsManager::new().unwrap();

        manager.set_string("api-key", "sk-test-123", SecretCategory::ApiKey).unwrap();

        let value = manager.get_string("api-key").unwrap();
        assert_eq!(value, "sk-test-123");
    }

    #[test]
    fn test_secret_not_found() {
        let manager = SecretsManager::new().unwrap();
        assert!(manager.get("nonexistent").is_err());
    }

    #[test]
    fn test_secret_deletion() {
        let manager = SecretsManager::new().unwrap();

        manager.set_string("temp", "value", SecretCategory::Other).unwrap();

        assert!(manager.exists("temp"));
        assert!(manager.delete("temp"));
        assert!(!manager.exists("temp"));
    }

    #[test]
    fn test_secret_rotation() {
        let manager = SecretsManager::new().unwrap();

        manager.set_string("rotating", "v1", SecretCategory::Password).unwrap();

        manager.mark_for_rotation("rotating");
        assert_eq!(manager.secrets_requiring_rotation(), vec!["rotating"]);

        manager.rotate("rotating", b"v2").unwrap();
        assert!(manager.secrets_requiring_rotation().is_empty());

        let value = manager.get_string("rotating").unwrap();
        assert_eq!(value, "v2");
    }

    #[test]
    fn test_list_by_category() {
        let manager = SecretsManager::new().unwrap();

        manager.set_string("key1", "v1", SecretCategory::ApiKey).unwrap();
        manager.set_string("key2", "v2", SecretCategory::ApiKey).unwrap();
        manager.set_string("db", "v3", SecretCategory::Database).unwrap();

        let api_keys = manager.list_by_category(SecretCategory::ApiKey);
        assert_eq!(api_keys.len(), 2);
    }

    #[test]
    fn test_from_password() {
        let salt = b"random_salt_12345678";
        let manager = SecretsManager::from_password("secure_password", salt).unwrap();

        manager.set_string("secret", "value", SecretCategory::Other).unwrap();

        // Re-derive from same password
        let manager2 = SecretsManager::from_password("secure_password", salt).unwrap();

        // Keys should be same (but secrets won't transfer without save/load)
        assert_eq!(manager.export_key(), manager2.export_key());
    }
}
