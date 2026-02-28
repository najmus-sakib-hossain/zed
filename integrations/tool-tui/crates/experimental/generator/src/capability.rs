//! Capability-Based Template Security - Feature #11
//!
//! Templates have explicit capability manifests with Ed25519 signing.
//! Prevents malicious templates from generating harmful code.

use crate::error::{GeneratorError, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

// ============================================================================
// Capabilities
// ============================================================================

/// Individual capability flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Capability {
    /// Can create new files.
    CreateFiles = 0x0001,
    /// Can modify existing files.
    ModifyFiles = 0x0002,
    /// Can delete files.
    DeleteFiles = 0x0004,
    /// Can modify Cargo.toml.
    ModifyCargo = 0x0010,
    /// Can add dependencies.
    AddDependencies = 0x0020,
    /// Can execute shell commands.
    ExecuteShell = 0x0100,
    /// Can generate unsafe code.
    GenerateUnsafe = 0x0200,
    /// Can access network.
    NetworkAccess = 0x0400,
    /// Can read environment variables.
    ReadEnv = 0x0800,
    /// Can include other templates.
    IncludeTemplates = 0x1000,
}

impl Capability {
    /// Get all capabilities.
    pub const ALL: &'static [Capability] = &[
        Capability::CreateFiles,
        Capability::ModifyFiles,
        Capability::DeleteFiles,
        Capability::ModifyCargo,
        Capability::AddDependencies,
        Capability::ExecuteShell,
        Capability::GenerateUnsafe,
        Capability::NetworkAccess,
        Capability::ReadEnv,
        Capability::IncludeTemplates,
    ];

    /// Get the capability name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::CreateFiles => "create_files",
            Self::ModifyFiles => "modify_files",
            Self::DeleteFiles => "delete_files",
            Self::ModifyCargo => "modify_cargo",
            Self::AddDependencies => "add_dependencies",
            Self::ExecuteShell => "execute_shell",
            Self::GenerateUnsafe => "generate_unsafe",
            Self::NetworkAccess => "network_access",
            Self::ReadEnv => "read_env",
            Self::IncludeTemplates => "include_templates",
        }
    }

    /// Check if this is a dangerous capability.
    #[must_use]
    pub const fn is_dangerous(&self) -> bool {
        matches!(
            self,
            Self::DeleteFiles | Self::ExecuteShell | Self::GenerateUnsafe | Self::NetworkAccess
        )
    }
}

// ============================================================================
// Capability Manifest
// ============================================================================

/// Capability manifest for a template.
///
/// Defines what operations a template is allowed to perform.
#[derive(Clone, Debug, Default)]
pub struct CapabilityManifest {
    /// Capability flags.
    flags: u32,
    /// Allowed file patterns for creation.
    pub allowed_files: Vec<String>,
    /// Allowed dependencies.
    pub allowed_deps: Vec<String>,
    /// Maximum output size in bytes.
    pub max_output_size: usize,
    /// Template description.
    pub description: String,
    /// Author information.
    pub author: String,
}

impl CapabilityManifest {
    /// Create an empty manifest (no capabilities).
    #[must_use]
    pub fn new() -> Self {
        Self {
            max_output_size: 1024 * 1024, // 1 MB default
            ..Default::default()
        }
    }

    /// Create a manifest with basic file creation.
    #[must_use]
    pub fn basic() -> Self {
        Self::new().with_capability(Capability::CreateFiles)
    }

    /// Create a manifest with full capabilities.
    #[must_use]
    pub fn full() -> Self {
        let mut manifest = Self::new();
        for cap in Capability::ALL {
            manifest = manifest.with_capability(*cap);
        }
        manifest
    }

    /// Add a capability.
    #[must_use]
    pub fn with_capability(mut self, cap: Capability) -> Self {
        self.flags |= cap as u32;
        self
    }

    /// Remove a capability.
    #[must_use]
    pub fn without_capability(mut self, cap: Capability) -> Self {
        self.flags &= !(cap as u32);
        self
    }

    /// Check if a capability is granted.
    #[must_use]
    pub fn has_capability(&self, cap: Capability) -> bool {
        (self.flags & cap as u32) != 0
    }

    /// Require a capability or return error.
    pub fn require(&self, cap: Capability) -> Result<()> {
        if self.has_capability(cap) {
            Ok(())
        } else {
            Err(GeneratorError::capability_violation(cap.name()))
        }
    }

    /// Add allowed file pattern.
    #[must_use]
    pub fn with_allowed_file(mut self, pattern: impl Into<String>) -> Self {
        self.allowed_files.push(pattern.into());
        self
    }

    /// Add allowed dependency.
    #[must_use]
    pub fn with_allowed_dep(mut self, dep: impl Into<String>) -> Self {
        self.allowed_deps.push(dep.into());
        self
    }

    /// Set maximum output size.
    #[must_use]
    pub fn with_max_output(mut self, size: usize) -> Self {
        self.max_output_size = size;
        self
    }

    /// Set description.
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set author.
    #[must_use]
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = author.into();
        self
    }

    /// Check if file path is allowed.
    #[must_use]
    pub fn is_file_allowed(&self, path: &str) -> bool {
        if self.allowed_files.is_empty() {
            return true; // No restrictions
        }

        self.allowed_files.iter().any(|pattern| {
            // Simple glob matching
            if let Some(idx) = pattern.find('*') {
                let prefix = &pattern[..idx];
                let suffix = &pattern[idx + 1..];
                path.starts_with(prefix) && path.ends_with(suffix)
            } else {
                path == pattern
            }
        })
    }

    /// Check if dependency is allowed.
    #[must_use]
    pub fn is_dep_allowed(&self, dep: &str) -> bool {
        if self.allowed_deps.is_empty() {
            return true;
        }
        self.allowed_deps.iter().any(|d| d == dep)
    }

    /// Serialize manifest to bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();

        // Flags
        out.extend_from_slice(&self.flags.to_le_bytes());

        // Max output size
        out.extend_from_slice(&(self.max_output_size as u64).to_le_bytes());

        // Allowed files count
        out.extend_from_slice(&(self.allowed_files.len() as u16).to_le_bytes());
        for f in &self.allowed_files {
            let bytes = f.as_bytes();
            out.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
            out.extend_from_slice(bytes);
        }

        // Allowed deps count
        out.extend_from_slice(&(self.allowed_deps.len() as u16).to_le_bytes());
        for d in &self.allowed_deps {
            let bytes = d.as_bytes();
            out.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
            out.extend_from_slice(bytes);
        }

        // Description
        let desc_bytes = self.description.as_bytes();
        out.extend_from_slice(&(desc_bytes.len() as u16).to_le_bytes());
        out.extend_from_slice(desc_bytes);

        // Author
        let author_bytes = self.author.as_bytes();
        out.extend_from_slice(&(author_bytes.len() as u16).to_le_bytes());
        out.extend_from_slice(author_bytes);

        out
    }

    /// Get all granted capabilities.
    #[must_use]
    pub fn capabilities(&self) -> Vec<Capability> {
        Capability::ALL
            .iter()
            .filter(|&&cap| self.has_capability(cap))
            .copied()
            .collect()
    }

    /// Check if any dangerous capability is granted.
    #[must_use]
    pub fn has_dangerous_capability(&self) -> bool {
        self.capabilities().iter().any(|c| c.is_dangerous())
    }
}

// ============================================================================
// Signed Template
// ============================================================================

/// A signed template with cryptographic verification.
#[derive(Clone, Debug)]
pub struct SignedTemplate {
    /// Template bytes.
    pub template_data: Vec<u8>,
    /// Capability manifest.
    pub manifest: CapabilityManifest,
    /// Ed25519 signature.
    pub signature: [u8; 64],
    /// Signer's public key.
    pub public_key: [u8; 32],
}

impl SignedTemplate {
    /// Sign a template with a signing key.
    pub fn sign(
        template_data: Vec<u8>,
        manifest: CapabilityManifest,
        signing_key: &SigningKey,
    ) -> Self {
        // Create message to sign: template + manifest
        let manifest_bytes = manifest.to_bytes();
        let mut message = Vec::with_capacity(template_data.len() + manifest_bytes.len());
        message.extend_from_slice(&template_data);
        message.extend_from_slice(&manifest_bytes);

        // Sign
        let signature = signing_key.sign(&message);

        Self {
            template_data,
            manifest,
            signature: signature.to_bytes(),
            public_key: signing_key.verifying_key().to_bytes(),
        }
    }

    /// Verify the signature.
    pub fn verify(&self) -> Result<()> {
        let verifying_key = VerifyingKey::from_bytes(&self.public_key)
            .map_err(|_| GeneratorError::SignatureInvalid)?;

        let signature = Signature::from_bytes(&self.signature);

        // Recreate message
        let manifest_bytes = self.manifest.to_bytes();
        let mut message = Vec::with_capacity(self.template_data.len() + manifest_bytes.len());
        message.extend_from_slice(&self.template_data);
        message.extend_from_slice(&manifest_bytes);

        verifying_key
            .verify(&message, &signature)
            .map_err(|_| GeneratorError::SignatureInvalid)
    }

    /// Verify with a specific trusted key.
    pub fn verify_with_key(&self, trusted_key: &VerifyingKey) -> Result<()> {
        // Check key matches
        if trusted_key.to_bytes() != self.public_key {
            return Err(GeneratorError::SignatureInvalid);
        }

        self.verify()
    }
}

// ============================================================================
// Capability Checker
// ============================================================================

/// Runtime capability checker.
#[derive(Clone, Debug)]
pub struct CapabilityChecker {
    /// Required capabilities for the operation.
    required: u32,
    /// Whether to allow unsigned templates.
    allow_unsigned: bool,
}

impl CapabilityChecker {
    /// Create a new checker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            required: 0,
            allow_unsigned: false,
        }
    }

    /// Require a capability.
    #[must_use]
    pub fn require(mut self, cap: Capability) -> Self {
        self.required |= cap as u32;
        self
    }

    /// Allow unsigned templates.
    #[must_use]
    pub fn allow_unsigned(mut self) -> Self {
        self.allow_unsigned = true;
        self
    }

    /// Check if a manifest satisfies requirements.
    pub fn check(&self, manifest: &CapabilityManifest) -> Result<()> {
        for cap in Capability::ALL {
            if (self.required & *cap as u32) != 0 && !manifest.has_capability(*cap) {
                return Err(GeneratorError::capability_violation(cap.name()));
            }
        }
        Ok(())
    }
}

impl Default for CapabilityChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    #[test]
    fn test_capability_manifest() {
        let manifest = CapabilityManifest::new()
            .with_capability(Capability::CreateFiles)
            .with_capability(Capability::ModifyFiles);

        assert!(manifest.has_capability(Capability::CreateFiles));
        assert!(manifest.has_capability(Capability::ModifyFiles));
        assert!(!manifest.has_capability(Capability::DeleteFiles));
    }

    #[test]
    fn test_require_capability() {
        let manifest = CapabilityManifest::new().with_capability(Capability::CreateFiles);

        assert!(manifest.require(Capability::CreateFiles).is_ok());
        assert!(manifest.require(Capability::DeleteFiles).is_err());
    }

    #[test]
    fn test_file_pattern_matching() {
        let manifest = CapabilityManifest::new()
            .with_allowed_file("src/*.rs")
            .with_allowed_file("*.md");

        assert!(manifest.is_file_allowed("src/lib.rs"));
        assert!(manifest.is_file_allowed("README.md"));
        assert!(!manifest.is_file_allowed("Cargo.toml"));
    }

    #[test]
    fn test_sign_and_verify() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let manifest = CapabilityManifest::basic();
        let template_data = b"template content".to_vec();

        let signed = SignedTemplate::sign(template_data, manifest, &signing_key);

        assert!(signed.verify().is_ok());
    }

    #[test]
    fn test_tampered_signature() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let manifest = CapabilityManifest::basic();
        let template_data = b"template content".to_vec();

        let mut signed = SignedTemplate::sign(template_data, manifest, &signing_key);

        // Tamper with data
        signed.template_data[0] ^= 0xFF;

        assert!(signed.verify().is_err());
    }

    #[test]
    fn test_dangerous_capability() {
        assert!(Capability::ExecuteShell.is_dangerous());
        assert!(Capability::DeleteFiles.is_dangerous());
        assert!(!Capability::CreateFiles.is_dangerous());
    }

    #[test]
    fn test_capability_checker() {
        let manifest = CapabilityManifest::new().with_capability(Capability::CreateFiles);

        let checker = CapabilityChecker::new().require(Capability::CreateFiles);

        assert!(checker.check(&manifest).is_ok());

        let checker2 = CapabilityChecker::new().require(Capability::DeleteFiles);

        assert!(checker2.check(&manifest).is_err());
    }
}
