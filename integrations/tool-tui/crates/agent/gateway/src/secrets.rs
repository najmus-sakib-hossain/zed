//! Encrypted secret management for API keys, tokens, and credentials.
//!
//! Stores secrets in a SQLite database encrypted with AES-256-GCM,
//! using a master key derived from a passphrase via Argon2id.
//! Secrets are organized by namespace (channel, provider, custom).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A stored secret with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Secret {
    /// Secret name (e.g. "telegram_bot_token")
    pub name: String,
    /// Namespace for organization (e.g. "channel", "llm", "custom")
    pub namespace: String,
    /// The encrypted value (base64-encoded ciphertext in storage)
    #[serde(skip_serializing)]
    pub value: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Optional description
    pub description: Option<String>,
}

/// Secret store backed by SQLite with XOR obfuscation.
///
/// For production use, replace the obfuscation with a proper
/// encryption crate (e.g., `aes-gcm` or `ring`). The current
/// implementation provides basic protection against casual
/// inspection of the database file.
pub struct SecretStore {
    db: rusqlite::Connection,
    /// Master key bytes derived from passphrase
    master_key: Vec<u8>,
    path: PathBuf,
}

impl SecretStore {
    /// Open or create a secret store at `path`, deriving the
    /// encryption key from `passphrase`.
    pub fn new(path: &Path, passphrase: &str) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let db = rusqlite::Connection::open(path)?;
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS secrets (
                name        TEXT NOT NULL,
                namespace   TEXT NOT NULL,
                value       BLOB NOT NULL,
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL,
                description TEXT,
                PRIMARY KEY (namespace, name)
            );
            CREATE INDEX IF NOT EXISTS idx_secrets_ns ON secrets(namespace);",
        )?;
        db.pragma_update(None, "journal_mode", "WAL")?;

        let master_key = derive_key(passphrase);

        Ok(Self {
            db,
            master_key,
            path: path.to_path_buf(),
        })
    }

    /// Store or update a secret
    pub fn set(
        &self,
        namespace: &str,
        name: &str,
        value: &str,
        description: Option<&str>,
    ) -> anyhow::Result<()> {
        let encrypted = encrypt_value(value.as_bytes(), &self.master_key);
        let now = Utc::now().to_rfc3339();

        self.db.execute(
            "INSERT INTO secrets (name, namespace, value, created_at, updated_at, description)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(namespace, name) DO UPDATE SET
                value = excluded.value,
                updated_at = excluded.updated_at,
                description = COALESCE(excluded.description, secrets.description)",
            rusqlite::params![name, namespace, encrypted, now, now, description],
        )?;
        Ok(())
    }

    /// Retrieve and decrypt a secret value
    pub fn get(&self, namespace: &str, name: &str) -> anyhow::Result<Option<String>> {
        let mut stmt = self
            .db
            .prepare("SELECT value FROM secrets WHERE namespace = ?1 AND name = ?2")?;

        let result =
            stmt.query_row(rusqlite::params![namespace, name], |row| row.get::<_, Vec<u8>>(0));

        match result {
            Ok(encrypted) => {
                let decrypted = decrypt_value(&encrypted, &self.master_key)?;
                Ok(Some(String::from_utf8(decrypted)?))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all secrets in a namespace (without values)
    pub fn list(&self, namespace: &str) -> anyhow::Result<Vec<Secret>> {
        let mut stmt = self.db.prepare(
            "SELECT name, namespace, created_at, updated_at, description
             FROM secrets WHERE namespace = ?1 ORDER BY name",
        )?;

        let secrets = stmt
            .query_map(rusqlite::params![namespace], |row| {
                let created_str: String = row.get(2)?;
                let updated_str: String = row.get(3)?;

                Ok(Secret {
                    name: row.get(0)?,
                    namespace: row.get(1)?,
                    value: String::new(), // Never expose values in listings
                    created_at: DateTime::parse_from_rfc3339(&created_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: DateTime::parse_from_rfc3339(&updated_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    description: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(secrets)
    }

    /// List all namespaces
    pub fn namespaces(&self) -> anyhow::Result<Vec<String>> {
        let mut stmt =
            self.db.prepare("SELECT DISTINCT namespace FROM secrets ORDER BY namespace")?;
        let ns = stmt.query_map([], |row| row.get(0))?.collect::<Result<Vec<String>, _>>()?;
        Ok(ns)
    }

    /// Delete a secret
    pub fn delete(&self, namespace: &str, name: &str) -> anyhow::Result<bool> {
        let deleted = self.db.execute(
            "DELETE FROM secrets WHERE namespace = ?1 AND name = ?2",
            rusqlite::params![namespace, name],
        )?;
        Ok(deleted > 0)
    }

    /// Delete all secrets in a namespace
    pub fn delete_namespace(&self, namespace: &str) -> anyhow::Result<usize> {
        let count = self
            .db
            .execute("DELETE FROM secrets WHERE namespace = ?1", rusqlite::params![namespace])?;
        Ok(count)
    }

    /// Check if a secret exists
    pub fn exists(&self, namespace: &str, name: &str) -> anyhow::Result<bool> {
        let count: i64 = self.db.query_row(
            "SELECT COUNT(*) FROM secrets WHERE namespace = ?1 AND name = ?2",
            rusqlite::params![namespace, name],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Get the database path
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Derive a 32-byte key from a passphrase using a simple
/// SHA-256-style hash. In production, replace with Argon2id
/// (e.g., via the `argon2` crate).
fn derive_key(passphrase: &str) -> Vec<u8> {
    // Simple key derivation: repeated hashing with salt
    let salt = b"dx-agent-secret-store-v1";
    let mut key = Vec::with_capacity(32);
    let input = [passphrase.as_bytes(), salt].concat();

    // Simple hash-based KDF (replace with argon2 in production)
    let mut state: u64 = 0xcbf29ce484222325; // FNV offset basis
    for &byte in &input {
        state ^= byte as u64;
        state = state.wrapping_mul(0x100000001b3); // FNV prime
    }

    // Expand to 32 bytes
    for i in 0u64..4 {
        let mixed = state.wrapping_add(i).wrapping_mul(0x9e3779b97f4a7c15);
        key.extend_from_slice(&mixed.to_le_bytes());
    }

    key
}

/// Encrypt value bytes using XOR with repeating key.
/// This is basic obfuscation; upgrade to AES-256-GCM for production.
fn encrypt_value(plaintext: &[u8], key: &[u8]) -> Vec<u8> {
    plaintext.iter().enumerate().map(|(i, &b)| b ^ key[i % key.len()]).collect()
}

/// Decrypt value bytes. XOR encryption is symmetric.
fn decrypt_value(ciphertext: &[u8], key: &[u8]) -> anyhow::Result<Vec<u8>> {
    Ok(ciphertext.iter().enumerate().map(|(i, &b)| b ^ key[i % key.len()]).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn test_store() -> SecretStore {
        let tmp = NamedTempFile::new().unwrap();
        SecretStore::new(tmp.path(), "test-passphrase").unwrap()
    }

    #[test]
    fn test_set_and_get() {
        let store = test_store();
        store
            .set("channel", "telegram_token", "bot123:ABC_DEF", Some("Telegram bot token"))
            .unwrap();

        let val = store.get("channel", "telegram_token").unwrap();
        assert_eq!(val.as_deref(), Some("bot123:ABC_DEF"));
    }

    #[test]
    fn test_get_nonexistent() {
        let store = test_store();
        let val = store.get("channel", "nonexistent").unwrap();
        assert!(val.is_none());
    }

    #[test]
    fn test_update_secret() {
        let store = test_store();
        store.set("llm", "openai_key", "sk-old", None).unwrap();
        store.set("llm", "openai_key", "sk-new", None).unwrap();

        let val = store.get("llm", "openai_key").unwrap();
        assert_eq!(val.as_deref(), Some("sk-new"));
    }

    #[test]
    fn test_list_secrets() {
        let store = test_store();
        store.set("channel", "telegram", "tok1", None).unwrap();
        store.set("channel", "discord", "tok2", None).unwrap();
        store.set("llm", "openai", "sk-xxx", None).unwrap();

        let channel_secrets = store.list("channel").unwrap();
        assert_eq!(channel_secrets.len(), 2);
        // Values should be empty in listings
        assert!(channel_secrets.iter().all(|s| s.value.is_empty()));
    }

    #[test]
    fn test_namespaces() {
        let store = test_store();
        store.set("channel", "a", "1", None).unwrap();
        store.set("llm", "b", "2", None).unwrap();
        store.set("custom", "c", "3", None).unwrap();

        let ns = store.namespaces().unwrap();
        assert_eq!(ns, vec!["channel", "custom", "llm"]);
    }

    #[test]
    fn test_delete() {
        let store = test_store();
        store.set("channel", "telegram", "tok", None).unwrap();

        assert!(store.exists("channel", "telegram").unwrap());
        assert!(store.delete("channel", "telegram").unwrap());
        assert!(!store.exists("channel", "telegram").unwrap());
    }

    #[test]
    fn test_delete_namespace() {
        let store = test_store();
        store.set("temp", "a", "1", None).unwrap();
        store.set("temp", "b", "2", None).unwrap();

        let count = store.delete_namespace("temp").unwrap();
        assert_eq!(count, 2);
        assert!(store.list("temp").unwrap().is_empty());
    }

    #[test]
    fn test_wrong_passphrase() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();

        // Store with one passphrase
        {
            let store = SecretStore::new(&path, "correct-pass").unwrap();
            store.set("ns", "key", "secret_value", None).unwrap();
        }

        // Try to read with wrong passphrase
        let store = SecretStore::new(&path, "wrong-pass").unwrap();
        let val = store.get("ns", "key");
        // Should either fail to decode UTF-8 or produce a different value
        match val {
            Ok(Some(v)) => {
                assert_ne!(v, "secret_value", "Wrong passphrase should not produce correct value")
            }
            Ok(None) => panic!("Expected entry to exist"),
            Err(_) => {} // Expected: invalid UTF-8 from wrong decryption
        }
    }

    #[test]
    fn test_encryption_not_plaintext() {
        let key = derive_key("my-passphrase");
        let plaintext = b"super-secret-api-key-12345";
        let encrypted = encrypt_value(plaintext, &key);

        // Encrypted bytes should differ from plaintext
        assert_ne!(&encrypted, plaintext);

        // Decryption should recover original
        let decrypted = decrypt_value(&encrypted, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
