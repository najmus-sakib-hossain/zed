//! Ed25519 signing and verification for DCP protocol.
//!
//! Provides cryptographic signing for tool definitions and invocations
//! using Ed25519 signatures.

use blake3;
use ed25519_dalek::{
    Signature, Signer as DalekSigner, SigningKey, Verifier as DalekVerifier, VerifyingKey,
};

use crate::binary::{SignedInvocation, SignedToolDef};
use crate::SecurityError;

/// Ed25519 signer for creating signatures
pub struct Signer {
    signing_key: SigningKey,
}

impl Signer {
    /// Create a new signer from a 32-byte seed
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        Self {
            signing_key: SigningKey::from_bytes(seed),
        }
    }

    /// Generate a new random signer
    pub fn generate() -> Self {
        use ed25519_dalek::SigningKey;
        let mut rng = rand::thread_rng();
        Self {
            signing_key: SigningKey::generate(&mut rng),
        }
    }

    /// Get the public key bytes
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    /// Sign a tool definition
    pub fn sign_tool_def(
        &self,
        tool_id: u32,
        schema_hash: [u8; 32],
        capabilities: u64,
    ) -> SignedToolDef {
        // Build the message to sign
        let mut message = Vec::with_capacity(44);
        message.extend_from_slice(&tool_id.to_le_bytes());
        message.extend_from_slice(&schema_hash);
        message.extend_from_slice(&capabilities.to_le_bytes());

        // Sign the message
        let signature = self.signing_key.sign(&message);

        SignedToolDef {
            tool_id,
            schema_hash,
            capabilities,
            signature: signature.to_bytes(),
            public_key: self.public_key_bytes(),
        }
    }

    /// Sign an invocation
    pub fn sign_invocation(
        &self,
        tool_id: u32,
        nonce: u64,
        timestamp: u64,
        args: &[u8],
    ) -> SignedInvocation {
        // Compute args hash
        let args_hash = *blake3::hash(args).as_bytes();

        // Build the message to sign
        let mut message = Vec::with_capacity(52);
        message.extend_from_slice(&tool_id.to_le_bytes());
        message.extend_from_slice(&nonce.to_le_bytes());
        message.extend_from_slice(&timestamp.to_le_bytes());
        message.extend_from_slice(&args_hash);

        // Sign the message
        let signature = self.signing_key.sign(&message);

        SignedInvocation {
            tool_id,
            nonce,
            timestamp,
            args_hash,
            signature: signature.to_bytes(),
        }
    }
}

/// Ed25519 verifier for checking signatures
pub struct Verifier;

impl Verifier {
    /// Verify a signed tool definition
    pub fn verify_tool_def(def: &SignedToolDef) -> Result<(), SecurityError> {
        // Reconstruct the message that was signed
        let mut message = Vec::with_capacity(44);
        message.extend_from_slice(&def.tool_id.to_le_bytes());
        message.extend_from_slice(&def.schema_hash);
        message.extend_from_slice(&def.capabilities.to_le_bytes());

        // Parse the public key
        let verifying_key = VerifyingKey::from_bytes(&def.public_key)
            .map_err(|_| SecurityError::InvalidSignature)?;

        // Parse the signature
        let signature = Signature::from_bytes(&def.signature);

        // Verify
        verifying_key
            .verify(&message, &signature)
            .map_err(|_| SecurityError::InvalidSignature)
    }

    /// Verify a signed invocation with a known public key
    pub fn verify_invocation(
        inv: &SignedInvocation,
        public_key: &[u8; 32],
    ) -> Result<(), SecurityError> {
        // Reconstruct the message that was signed
        let mut message = Vec::with_capacity(52);
        message.extend_from_slice(&inv.tool_id.to_le_bytes());
        message.extend_from_slice(&inv.nonce.to_le_bytes());
        message.extend_from_slice(&inv.timestamp.to_le_bytes());
        message.extend_from_slice(&inv.args_hash);

        // Parse the public key
        let verifying_key =
            VerifyingKey::from_bytes(public_key).map_err(|_| SecurityError::InvalidSignature)?;

        // Parse the signature
        let signature = Signature::from_bytes(&inv.signature);

        // Verify
        verifying_key
            .verify(&message, &signature)
            .map_err(|_| SecurityError::InvalidSignature)
    }

    /// Verify that the args hash matches the provided arguments
    pub fn verify_args_hash(inv: &SignedInvocation, args: &[u8]) -> bool {
        let computed_hash = *blake3::hash(args).as_bytes();
        computed_hash == inv.args_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_and_verify_tool_def() {
        let signer = Signer::from_seed(&[42u8; 32]);

        let def = signer.sign_tool_def(123, [0xAB; 32], 0x1234);

        assert_eq!(def.tool_id, 123);
        assert_eq!(def.schema_hash, [0xAB; 32]);
        assert_eq!(def.capabilities, 0x1234);
        assert_eq!(def.public_key, signer.public_key_bytes());

        // Verify should succeed
        assert!(Verifier::verify_tool_def(&def).is_ok());
    }

    #[test]
    fn test_sign_and_verify_invocation() {
        let signer = Signer::from_seed(&[42u8; 32]);
        let args = b"test arguments";

        let inv = signer.sign_invocation(456, 0xDEADBEEF, 1234567890, args);

        assert_eq!(inv.tool_id, 456);
        assert_eq!(inv.nonce, 0xDEADBEEF);
        assert_eq!(inv.timestamp, 1234567890);

        // Verify should succeed
        let public_key = signer.public_key_bytes();
        assert!(Verifier::verify_invocation(&inv, &public_key).is_ok());

        // Args hash should match
        assert!(Verifier::verify_args_hash(&inv, args));
        assert!(!Verifier::verify_args_hash(&inv, b"wrong args"));
    }

    #[test]
    fn test_tampered_tool_def_fails() {
        let signer = Signer::from_seed(&[42u8; 32]);

        let mut def = signer.sign_tool_def(123, [0xAB; 32], 0x1234);

        // Tamper with the tool_id
        def.tool_id = 999;

        // Verify should fail
        assert!(Verifier::verify_tool_def(&def).is_err());
    }

    #[test]
    fn test_tampered_invocation_fails() {
        let signer = Signer::from_seed(&[42u8; 32]);

        let mut inv = signer.sign_invocation(456, 0xDEADBEEF, 1234567890, b"args");

        // Tamper with the nonce
        inv.nonce = 0xCAFEBABE;

        // Verify should fail
        let public_key = signer.public_key_bytes();
        assert!(Verifier::verify_invocation(&inv, &public_key).is_err());
    }

    #[test]
    fn test_wrong_public_key_fails() {
        let signer1 = Signer::from_seed(&[1u8; 32]);
        let signer2 = Signer::from_seed(&[2u8; 32]);

        let inv = signer1.sign_invocation(456, 0xDEADBEEF, 1234567890, b"args");

        // Verify with wrong public key should fail
        let wrong_key = signer2.public_key_bytes();
        assert!(Verifier::verify_invocation(&inv, &wrong_key).is_err());
    }

    #[test]
    fn test_generate_random_signer() {
        let signer1 = Signer::generate();
        let signer2 = Signer::generate();

        // Different signers should have different public keys
        assert_ne!(signer1.public_key_bytes(), signer2.public_key_bytes());
    }
}
