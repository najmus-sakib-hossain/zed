//! Security Module - Ed25519 Signing & Capability-Based Access
//!
//! Cryptographic integrity for rule files.

mod capability_manifest;
mod ed25519_signer;
mod integrity_guard;
mod sandbox;

pub use capability_manifest::{Capability, CapabilityManifest};
pub use ed25519_signer::{Ed25519Signer, KeyPair, PublicKey, SecretKey, Signature};
pub use integrity_guard::{IntegrityGuard, IntegrityStatus};
pub use sandbox::{Sandbox, SandboxConfig};

/// Security version
pub const SECURITY_VERSION: u8 = 1;

/// Signature algorithm identifier
pub const SIG_ALG_ED25519: u8 = 1;
