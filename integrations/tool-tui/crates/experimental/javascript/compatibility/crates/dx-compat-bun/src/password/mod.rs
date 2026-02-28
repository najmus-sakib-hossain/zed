//! Bun.password hashing.
//!
//! Secure password hashing using Argon2id (recommended) or bcrypt.

use crate::error::{BunError, BunResult};
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};

/// Password hashing algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Algorithm {
    /// Argon2id (default, recommended)
    ///
    /// Argon2id is the winner of the Password Hashing Competition and is
    /// recommended for most use cases. It provides resistance against both
    /// GPU and side-channel attacks.
    #[default]
    Argon2id,
    /// bcrypt
    ///
    /// bcrypt is a well-established algorithm that's still secure but
    /// has some limitations compared to Argon2id.
    Bcrypt,
}

/// Argon2 configuration options.
#[derive(Debug, Clone)]
pub struct Argon2Options {
    /// Memory cost in KiB (default: 19456 = 19 MiB)
    pub memory_cost: u32,
    /// Time cost (iterations, default: 2)
    pub time_cost: u32,
    /// Parallelism (default: 1)
    pub parallelism: u32,
}

impl Default for Argon2Options {
    fn default() -> Self {
        Self {
            memory_cost: 19456, // 19 MiB
            time_cost: 2,
            parallelism: 1,
        }
    }
}

/// bcrypt configuration options.
#[derive(Debug, Clone)]
pub struct BcryptOptions {
    /// Cost factor (default: 12, range: 4-31)
    pub cost: u32,
}

impl Default for BcryptOptions {
    fn default() -> Self {
        Self {
            cost: bcrypt::DEFAULT_COST,
        }
    }
}

/// Hash options.
#[derive(Debug, Clone, Default)]
pub struct HashOptions {
    /// Algorithm to use
    pub algorithm: Algorithm,
    /// Argon2 options (only used if algorithm is Argon2id)
    pub argon2: Option<Argon2Options>,
    /// bcrypt options (only used if algorithm is Bcrypt)
    pub bcrypt: Option<BcryptOptions>,
}

impl HashOptions {
    /// Create options for Argon2id with default settings.
    pub fn argon2() -> Self {
        Self {
            algorithm: Algorithm::Argon2id,
            argon2: Some(Argon2Options::default()),
            bcrypt: None,
        }
    }

    /// Create options for bcrypt with default settings.
    pub fn bcrypt() -> Self {
        Self {
            algorithm: Algorithm::Bcrypt,
            argon2: None,
            bcrypt: Some(BcryptOptions::default()),
        }
    }

    /// Set memory cost for Argon2.
    pub fn memory_cost(mut self, cost: u32) -> Self {
        self.argon2.get_or_insert_with(Argon2Options::default).memory_cost = cost;
        self
    }

    /// Set time cost for Argon2.
    pub fn time_cost(mut self, cost: u32) -> Self {
        self.argon2.get_or_insert_with(Argon2Options::default).time_cost = cost;
        self
    }

    /// Set bcrypt cost.
    pub fn cost(mut self, cost: u32) -> Self {
        self.bcrypt.get_or_insert_with(BcryptOptions::default).cost = cost;
        self
    }
}

/// Hash a password using the specified algorithm.
///
/// # Arguments
/// * `password` - The password to hash
/// * `options` - Optional hashing options (defaults to Argon2id)
///
/// # Returns
/// The hashed password as a string in PHC format.
///
/// # Example
/// ```ignore
/// let hashed = hash("my_password", None)?;
/// assert!(verify("my_password", &hashed)?);
/// ```
pub fn hash(password: &str, options: Option<HashOptions>) -> BunResult<String> {
    let options = options.unwrap_or_default();

    match options.algorithm {
        Algorithm::Argon2id => hash_argon2(password, options.argon2.unwrap_or_default()),
        Algorithm::Bcrypt => hash_bcrypt(password, options.bcrypt.unwrap_or_default()),
    }
}

fn hash_argon2(password: &str, options: Argon2Options) -> BunResult<String> {
    let salt = SaltString::generate(&mut rand::thread_rng());

    let params =
        argon2::Params::new(options.memory_cost, options.time_cost, options.parallelism, None)
            .map_err(|e| BunError::Password(format!("Invalid Argon2 params: {}", e)))?;

    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| BunError::Password(format!("Argon2 hash failed: {}", e)))
}

fn hash_bcrypt(password: &str, options: BcryptOptions) -> BunResult<String> {
    bcrypt::hash(password, options.cost)
        .map_err(|e| BunError::Password(format!("bcrypt hash failed: {}", e)))
}

/// Verify a password against a hash.
///
/// Automatically detects the algorithm from the hash format.
///
/// # Arguments
/// * `password` - The password to verify
/// * `hash` - The hash to verify against
///
/// # Returns
/// `true` if the password matches, `false` otherwise.
pub fn verify(password: &str, hash: &str) -> BunResult<bool> {
    // Detect algorithm from hash format
    if hash.starts_with("$argon2") {
        verify_argon2(password, hash)
    } else if hash.starts_with("$2") {
        verify_bcrypt(password, hash)
    } else {
        Err(BunError::Password("Unknown hash format".to_string()))
    }
}

fn verify_argon2(password: &str, hash: &str) -> BunResult<bool> {
    let parsed_hash =
        PasswordHash::new(hash).map_err(|e| BunError::Password(format!("Invalid hash: {}", e)))?;

    let argon2 = Argon2::default();
    Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
}

fn verify_bcrypt(password: &str, hash: &str) -> BunResult<bool> {
    bcrypt::verify(password, hash)
        .map_err(|e| BunError::Password(format!("bcrypt verify failed: {}", e)))
}

/// Hash a password asynchronously.
///
/// Runs the CPU-intensive hashing in a blocking task pool.
pub async fn hash_async(password: String, options: Option<HashOptions>) -> BunResult<String> {
    tokio::task::spawn_blocking(move || hash(&password, options))
        .await
        .map_err(|e| BunError::Password(format!("Task join error: {}", e)))?
}

/// Verify a password asynchronously.
///
/// Runs the CPU-intensive verification in a blocking task pool.
pub async fn verify_async(password: String, hash: String) -> BunResult<bool> {
    tokio::task::spawn_blocking(move || verify(&password, &hash))
        .await
        .map_err(|e| BunError::Password(format!("Task join error: {}", e)))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argon2_hash_verify() {
        let password = "my_secure_password";
        let hashed = hash(password, Some(HashOptions::argon2())).unwrap();

        assert!(hashed.starts_with("$argon2"));
        assert!(verify(password, &hashed).unwrap());
        assert!(!verify("wrong_password", &hashed).unwrap());
    }

    #[test]
    fn test_bcrypt_hash_verify() {
        let password = "my_secure_password";
        let hashed = hash(password, Some(HashOptions::bcrypt())).unwrap();

        assert!(hashed.starts_with("$2"));
        assert!(verify(password, &hashed).unwrap());
        assert!(!verify("wrong_password", &hashed).unwrap());
    }

    #[test]
    fn test_default_is_argon2() {
        let password = "test";
        let hashed = hash(password, None).unwrap();
        assert!(hashed.starts_with("$argon2"));
    }

    #[test]
    fn test_custom_argon2_params() {
        let password = "test";
        let options = HashOptions::argon2().memory_cost(4096).time_cost(1);
        let hashed = hash(password, Some(options)).unwrap();

        assert!(verify(password, &hashed).unwrap());
    }

    #[test]
    fn test_custom_bcrypt_cost() {
        let password = "test";
        let options = HashOptions::bcrypt().cost(4); // Minimum cost for fast tests
        let hashed = hash(password, Some(options)).unwrap();

        assert!(verify(password, &hashed).unwrap());
    }

    #[tokio::test]
    async fn test_async_hash_verify() {
        let password = "async_test";
        let options = HashOptions::argon2().memory_cost(4096).time_cost(1);
        let hashed = hash_async(password.to_string(), Some(options)).await.unwrap();

        assert!(verify_async(password.to_string(), hashed).await.unwrap());
    }

    #[test]
    fn test_unknown_hash_format() {
        let result = verify("password", "invalid_hash");
        assert!(result.is_err());
    }
}
