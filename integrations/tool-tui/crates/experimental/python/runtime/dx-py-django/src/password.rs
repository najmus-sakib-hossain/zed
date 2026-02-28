//! Password hashing compatible with Django's password hashers
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PasswordError {
    #[error("Hash error: {0}")]
    HashError(String),
    #[error("Invalid hash format: {0}")]
    InvalidFormat(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    BCryptSHA256,
    Argon2,
    PBKDF2SHA256,
}

impl HashAlgorithm {
    pub fn django_identifier(&self) -> &'static str {
        match self {
            Self::BCryptSHA256 => "bcrypt_sha256",
            Self::Argon2 => "argon2",
            Self::PBKDF2SHA256 => "pbkdf2_sha256",
        }
    }
}

pub struct PasswordHasher {
    bcrypt_cost: u32,
}

impl Default for PasswordHasher {
    fn default() -> Self {
        Self { bcrypt_cost: 12 }
    }
}

impl PasswordHasher {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn bcrypt() -> Self {
        Self::default()
    }

    pub fn hash(&self, password: &str) -> Result<String, PasswordError> {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        let prehash = format!("{:x}", hasher.finalize());
        let hash = bcrypt::hash(&prehash, self.bcrypt_cost)
            .map_err(|e| PasswordError::HashError(e.to_string()))?;
        Ok(format!("bcrypt_sha256${}", hash))
    }

    pub fn verify(&self, password: &str, hash: &str) -> Result<bool, PasswordError> {
        let bcrypt_hash = hash
            .strip_prefix("bcrypt_sha256$")
            .ok_or_else(|| PasswordError::InvalidFormat("Invalid".into()))?;
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        let prehash = format!("{:x}", hasher.finalize());
        bcrypt::verify(&prehash, bcrypt_hash).map_err(|e| PasswordError::HashError(e.to_string()))
    }
}
