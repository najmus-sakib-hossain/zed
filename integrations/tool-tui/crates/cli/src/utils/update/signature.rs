//! Signature verification for updates

use crate::utils::error::DxError;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

/// Verify Ed25519 signature of update binary
pub fn verify_signature(
    data: &[u8],
    signature_bytes: &[u8],
    public_key_bytes: &[u8],
) -> Result<(), DxError> {
    let public_key = VerifyingKey::from_bytes(
        public_key_bytes.try_into().map_err(|_| DxError::SignatureInvalid)?,
    )
    .map_err(|_| DxError::SignatureInvalid)?;

    let signature =
        Signature::from_bytes(signature_bytes.try_into().map_err(|_| DxError::SignatureInvalid)?);

    public_key.verify(data, &signature).map_err(|_| DxError::SignatureInvalid)
}

/// Verify signature from hex-encoded strings
pub fn verify_signature_hex(
    data: &[u8],
    signature_hex: &str,
    public_key_hex: &str,
) -> Result<(), DxError> {
    let signature_bytes = hex_decode(signature_hex)?;
    let public_key_bytes = hex_decode(public_key_hex)?;
    verify_signature(data, &signature_bytes, &public_key_bytes)
}

fn hex_decode(hex: &str) -> Result<Vec<u8>, DxError> {
    let hex = hex.trim();
    if !hex.len().is_multiple_of(2) {
        return Err(DxError::SignatureInvalid);
    }

    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| DxError::SignatureInvalid))
        .collect()
}
