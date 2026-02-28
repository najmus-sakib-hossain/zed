//! Cipher operations for Node.js crypto compatibility.
//!
//! This module provides encryption and decryption functions compatible
//! with Node.js crypto.createCipheriv and crypto.createDecipheriv APIs.

use aes::cipher::KeyIvInit;
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};

/// Error type for cipher operations.
#[derive(Debug, thiserror::Error)]
pub enum CipherError {
    /// Invalid key length.
    #[error("Invalid key length: expected {expected}, got {got}")]
    InvalidKeyLength { expected: usize, got: usize },
    /// Invalid IV length.
    #[error("Invalid IV length: expected {expected}, got {got}")]
    InvalidIvLength { expected: usize, got: usize },
    /// Encryption failed.
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    /// Decryption failed.
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
    /// Unsupported algorithm.
    #[error("Unsupported algorithm: {0}")]
    UnsupportedAlgorithm(String),
    /// Invalid padding.
    #[error("Invalid padding")]
    InvalidPadding,
}

/// Cipher algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipherAlgorithm {
    /// AES-128-CBC
    Aes128Cbc,
    /// AES-192-CBC
    Aes192Cbc,
    /// AES-256-CBC
    Aes256Cbc,
    /// AES-128-CTR
    Aes128Ctr,
    /// AES-192-CTR
    Aes192Ctr,
    /// AES-256-CTR
    Aes256Ctr,
    /// ChaCha20-Poly1305 (AEAD)
    ChaCha20Poly1305,
}

impl CipherAlgorithm {
    /// Get the required key length for this algorithm.
    pub fn key_length(&self) -> usize {
        match self {
            CipherAlgorithm::Aes128Cbc | CipherAlgorithm::Aes128Ctr => 16,
            CipherAlgorithm::Aes192Cbc | CipherAlgorithm::Aes192Ctr => 24,
            CipherAlgorithm::Aes256Cbc | CipherAlgorithm::Aes256Ctr => 32,
            CipherAlgorithm::ChaCha20Poly1305 => 32,
        }
    }

    /// Get the required IV/nonce length for this algorithm.
    pub fn iv_length(&self) -> usize {
        match self {
            CipherAlgorithm::Aes128Cbc
            | CipherAlgorithm::Aes192Cbc
            | CipherAlgorithm::Aes256Cbc
            | CipherAlgorithm::Aes128Ctr
            | CipherAlgorithm::Aes192Ctr
            | CipherAlgorithm::Aes256Ctr => 16,
            CipherAlgorithm::ChaCha20Poly1305 => 12,
        }
    }

    /// Check if this is an AEAD cipher.
    pub fn is_aead(&self) -> bool {
        matches!(self, CipherAlgorithm::ChaCha20Poly1305)
    }
}

// Type aliases for AES-CBC
type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;
type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

// Type aliases for AES-CTR
type Aes128Ctr = ctr::Ctr64BE<aes::Aes128>;
type Aes256Ctr = ctr::Ctr64BE<aes::Aes256>;

/// Encrypt data using the specified algorithm.
pub fn encrypt(
    algorithm: CipherAlgorithm,
    key: &[u8],
    iv: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>, CipherError> {
    // Validate key length
    if key.len() != algorithm.key_length() {
        return Err(CipherError::InvalidKeyLength {
            expected: algorithm.key_length(),
            got: key.len(),
        });
    }

    // Validate IV length
    if iv.len() != algorithm.iv_length() {
        return Err(CipherError::InvalidIvLength {
            expected: algorithm.iv_length(),
            got: iv.len(),
        });
    }

    match algorithm {
        CipherAlgorithm::Aes128Cbc => encrypt_aes128_cbc(key, iv, plaintext),
        CipherAlgorithm::Aes256Cbc => encrypt_aes256_cbc(key, iv, plaintext),
        CipherAlgorithm::Aes128Ctr => encrypt_aes128_ctr(key, iv, plaintext),
        CipherAlgorithm::Aes256Ctr => encrypt_aes256_ctr(key, iv, plaintext),
        CipherAlgorithm::ChaCha20Poly1305 => encrypt_chacha20_poly1305(key, iv, plaintext),
        _ => Err(CipherError::UnsupportedAlgorithm(format!("{:?}", algorithm))),
    }
}

/// Decrypt data using the specified algorithm.
pub fn decrypt(
    algorithm: CipherAlgorithm,
    key: &[u8],
    iv: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, CipherError> {
    // Validate key length
    if key.len() != algorithm.key_length() {
        return Err(CipherError::InvalidKeyLength {
            expected: algorithm.key_length(),
            got: key.len(),
        });
    }

    // Validate IV length
    if iv.len() != algorithm.iv_length() {
        return Err(CipherError::InvalidIvLength {
            expected: algorithm.iv_length(),
            got: iv.len(),
        });
    }

    match algorithm {
        CipherAlgorithm::Aes128Cbc => decrypt_aes128_cbc(key, iv, ciphertext),
        CipherAlgorithm::Aes256Cbc => decrypt_aes256_cbc(key, iv, ciphertext),
        CipherAlgorithm::Aes128Ctr => decrypt_aes128_ctr(key, iv, ciphertext),
        CipherAlgorithm::Aes256Ctr => decrypt_aes256_ctr(key, iv, ciphertext),
        CipherAlgorithm::ChaCha20Poly1305 => decrypt_chacha20_poly1305(key, iv, ciphertext),
        _ => Err(CipherError::UnsupportedAlgorithm(format!("{:?}", algorithm))),
    }
}

// AES-128-CBC encryption with PKCS7 padding
fn encrypt_aes128_cbc(key: &[u8], iv: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, CipherError> {
    use aes::cipher::block_padding::Pkcs7;
    use cbc::cipher::BlockEncryptMut;
    
    let cipher = Aes128CbcEnc::new_from_slices(key, iv)
        .map_err(|e| CipherError::EncryptionFailed(e.to_string()))?;
    
    // Calculate padded length
    let block_size = 16;
    let padding_len = block_size - (plaintext.len() % block_size);
    let padded_len = plaintext.len() + padding_len;
    
    // Create buffer with plaintext and space for padding
    let mut buffer = vec![0u8; padded_len];
    buffer[..plaintext.len()].copy_from_slice(plaintext);
    
    // Apply PKCS7 padding
    for i in plaintext.len()..padded_len {
        buffer[i] = padding_len as u8;
    }
    
    // Encrypt in place
    cipher.encrypt_padded_mut::<Pkcs7>(&mut buffer, plaintext.len())
        .map_err(|_| CipherError::EncryptionFailed("Padding error".into()))?;
    
    Ok(buffer)
}

// AES-128-CBC decryption
fn decrypt_aes128_cbc(key: &[u8], iv: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, CipherError> {
    use aes::cipher::block_padding::Pkcs7;
    use cbc::cipher::BlockDecryptMut;
    
    let cipher = Aes128CbcDec::new_from_slices(key, iv)
        .map_err(|e| CipherError::DecryptionFailed(e.to_string()))?;
    
    let mut buffer = ciphertext.to_vec();
    let decrypted = cipher.decrypt_padded_mut::<Pkcs7>(&mut buffer)
        .map_err(|_| CipherError::InvalidPadding)?;
    
    Ok(decrypted.to_vec())
}

// AES-256-CBC encryption
fn encrypt_aes256_cbc(key: &[u8], iv: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, CipherError> {
    use aes::cipher::block_padding::Pkcs7;
    use cbc::cipher::BlockEncryptMut;
    
    let cipher = Aes256CbcEnc::new_from_slices(key, iv)
        .map_err(|e| CipherError::EncryptionFailed(e.to_string()))?;
    
    // Calculate padded length
    let block_size = 16;
    let padding_len = block_size - (plaintext.len() % block_size);
    let padded_len = plaintext.len() + padding_len;
    
    // Create buffer with plaintext and space for padding
    let mut buffer = vec![0u8; padded_len];
    buffer[..plaintext.len()].copy_from_slice(plaintext);
    
    // Apply PKCS7 padding
    for i in plaintext.len()..padded_len {
        buffer[i] = padding_len as u8;
    }
    
    // Encrypt in place
    cipher.encrypt_padded_mut::<Pkcs7>(&mut buffer, plaintext.len())
        .map_err(|_| CipherError::EncryptionFailed("Padding error".into()))?;
    
    Ok(buffer)
}

// AES-256-CBC decryption
fn decrypt_aes256_cbc(key: &[u8], iv: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, CipherError> {
    use aes::cipher::block_padding::Pkcs7;
    use cbc::cipher::BlockDecryptMut;
    
    let cipher = Aes256CbcDec::new_from_slices(key, iv)
        .map_err(|e| CipherError::DecryptionFailed(e.to_string()))?;
    
    let mut buffer = ciphertext.to_vec();
    let decrypted = cipher.decrypt_padded_mut::<Pkcs7>(&mut buffer)
        .map_err(|_| CipherError::InvalidPadding)?;
    
    Ok(decrypted.to_vec())
}

// AES-128-CTR encryption (same as decryption for CTR mode)
fn encrypt_aes128_ctr(key: &[u8], iv: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, CipherError> {
    use aes::cipher::StreamCipher;
    
    let mut cipher = Aes128Ctr::new_from_slices(key, iv)
        .map_err(|e| CipherError::EncryptionFailed(e.to_string()))?;
    
    let mut output = plaintext.to_vec();
    cipher.apply_keystream(&mut output);
    Ok(output)
}

// AES-128-CTR decryption (same as encryption for CTR mode)
fn decrypt_aes128_ctr(key: &[u8], iv: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, CipherError> {
    encrypt_aes128_ctr(key, iv, ciphertext)
}

// AES-256-CTR encryption
fn encrypt_aes256_ctr(key: &[u8], iv: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, CipherError> {
    use aes::cipher::StreamCipher;
    
    let mut cipher = Aes256Ctr::new_from_slices(key, iv)
        .map_err(|e| CipherError::EncryptionFailed(e.to_string()))?;
    
    let mut output = plaintext.to_vec();
    cipher.apply_keystream(&mut output);
    Ok(output)
}

// AES-256-CTR decryption
fn decrypt_aes256_ctr(key: &[u8], iv: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, CipherError> {
    encrypt_aes256_ctr(key, iv, ciphertext)
}

// ChaCha20-Poly1305 encryption (AEAD)
fn encrypt_chacha20_poly1305(key: &[u8], nonce: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, CipherError> {
    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| CipherError::EncryptionFailed(e.to_string()))?;
    
    let nonce = Nonce::from_slice(nonce);
    cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| CipherError::EncryptionFailed(e.to_string()))
}

// ChaCha20-Poly1305 decryption (AEAD)
fn decrypt_chacha20_poly1305(key: &[u8], nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, CipherError> {
    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| CipherError::DecryptionFailed(e.to_string()))?;
    
    let nonce = Nonce::from_slice(nonce);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| CipherError::DecryptionFailed(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aes128_cbc_round_trip() {
        let key = [0u8; 16];
        let iv = [0u8; 16];
        let plaintext = b"Hello, World!";

        let ciphertext = encrypt(CipherAlgorithm::Aes128Cbc, &key, &iv, plaintext).unwrap();
        let decrypted = decrypt(CipherAlgorithm::Aes128Cbc, &key, &iv, &ciphertext).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_aes256_cbc_round_trip() {
        let key = [0u8; 32];
        let iv = [0u8; 16];
        let plaintext = b"Hello, World!";

        let ciphertext = encrypt(CipherAlgorithm::Aes256Cbc, &key, &iv, plaintext).unwrap();
        let decrypted = decrypt(CipherAlgorithm::Aes256Cbc, &key, &iv, &ciphertext).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_aes128_ctr_round_trip() {
        let key = [0u8; 16];
        let iv = [0u8; 16];
        let plaintext = b"Hello, World!";

        let ciphertext = encrypt(CipherAlgorithm::Aes128Ctr, &key, &iv, plaintext).unwrap();
        let decrypted = decrypt(CipherAlgorithm::Aes128Ctr, &key, &iv, &ciphertext).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_aes256_ctr_round_trip() {
        let key = [0u8; 32];
        let iv = [0u8; 16];
        let plaintext = b"Hello, World!";

        let ciphertext = encrypt(CipherAlgorithm::Aes256Ctr, &key, &iv, plaintext).unwrap();
        let decrypted = decrypt(CipherAlgorithm::Aes256Ctr, &key, &iv, &ciphertext).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_chacha20_poly1305_round_trip() {
        let key = [0u8; 32];
        let nonce = [0u8; 12];
        let plaintext = b"Hello, World!";

        let ciphertext = encrypt(CipherAlgorithm::ChaCha20Poly1305, &key, &nonce, plaintext).unwrap();
        let decrypted = decrypt(CipherAlgorithm::ChaCha20Poly1305, &key, &nonce, &ciphertext).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_invalid_key_length() {
        let key = [0u8; 10]; // Wrong length
        let iv = [0u8; 16];
        let plaintext = b"Hello";

        let result = encrypt(CipherAlgorithm::Aes128Cbc, &key, &iv, plaintext);
        assert!(matches!(result, Err(CipherError::InvalidKeyLength { .. })));
    }

    #[test]
    fn test_invalid_iv_length() {
        let key = [0u8; 16];
        let iv = [0u8; 10]; // Wrong length
        let plaintext = b"Hello";

        let result = encrypt(CipherAlgorithm::Aes128Cbc, &key, &iv, plaintext);
        assert!(matches!(result, Err(CipherError::InvalidIvLength { .. })));
    }

    #[test]
    fn test_chacha20_poly1305_tamper_detection() {
        let key = [0u8; 32];
        let nonce = [0u8; 12];
        let plaintext = b"Hello, World!";

        let mut ciphertext = encrypt(CipherAlgorithm::ChaCha20Poly1305, &key, &nonce, plaintext).unwrap();
        
        // Tamper with the ciphertext
        if !ciphertext.is_empty() {
            ciphertext[0] ^= 0xFF;
        }

        // Decryption should fail due to authentication
        let result = decrypt(CipherAlgorithm::ChaCha20Poly1305, &key, &nonce, &ciphertext);
        assert!(result.is_err());
    }

    #[test]
    fn test_algorithm_properties() {
        assert_eq!(CipherAlgorithm::Aes128Cbc.key_length(), 16);
        assert_eq!(CipherAlgorithm::Aes256Cbc.key_length(), 32);
        assert_eq!(CipherAlgorithm::ChaCha20Poly1305.key_length(), 32);
        
        assert_eq!(CipherAlgorithm::Aes128Cbc.iv_length(), 16);
        assert_eq!(CipherAlgorithm::ChaCha20Poly1305.iv_length(), 12);
        
        assert!(!CipherAlgorithm::Aes128Cbc.is_aead());
        assert!(CipherAlgorithm::ChaCha20Poly1305.is_aead());
    }
}
