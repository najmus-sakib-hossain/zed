//! Capability Manifest
//!
//! Declarative security permissions for rule files.

use std::collections::HashSet;

/// Capability types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Capability {
    /// Read file system
    FileRead = 0,
    /// Write file system
    FileWrite = 1,
    /// Execute commands
    Execute = 2,
    /// Network access
    Network = 3,
    /// Environment variables
    Environment = 4,
    /// Modify settings
    Settings = 5,
    /// Access secrets/credentials
    Secrets = 6,
    /// Create processes
    Process = 7,
    /// Inter-process communication
    Ipc = 8,
    /// Full system access (dangerous)
    System = 255,
}

impl Capability {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "file-read" | "fs:read" => Some(Self::FileRead),
            "file-write" | "fs:write" => Some(Self::FileWrite),
            "execute" | "exec" => Some(Self::Execute),
            "network" | "net" => Some(Self::Network),
            "environment" | "env" => Some(Self::Environment),
            "settings" | "config" => Some(Self::Settings),
            "secrets" | "credentials" => Some(Self::Secrets),
            "process" | "spawn" => Some(Self::Process),
            "ipc" => Some(Self::Ipc),
            "system" | "all" => Some(Self::System),
            _ => None,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FileRead => "fs:read",
            Self::FileWrite => "fs:write",
            Self::Execute => "execute",
            Self::Network => "network",
            Self::Environment => "environment",
            Self::Settings => "settings",
            Self::Secrets => "secrets",
            Self::Process => "process",
            Self::Ipc => "ipc",
            Self::System => "system",
        }
    }

    /// Check if capability implies another
    pub fn implies(&self, other: &Capability) -> bool {
        // System implies everything
        if *self == Self::System {
            return true;
        }

        // FileWrite implies FileRead
        if *self == Self::FileWrite && *other == Self::FileRead {
            return true;
        }

        // Process implies Execute
        if *self == Self::Process && *other == Self::Execute {
            return true;
        }

        *self == *other
    }
}

/// Capability manifest for a rule file
#[derive(Debug, Clone, Default)]
pub struct CapabilityManifest {
    /// Required capabilities
    required: HashSet<Capability>,
    /// Optional capabilities (requested but not required)
    optional: HashSet<Capability>,
    /// Denied capabilities (explicitly blocked)
    denied: HashSet<Capability>,
    /// Resource restrictions (e.g., allowed paths)
    restrictions: Vec<CapabilityRestriction>,
}

/// Restriction on a capability
#[derive(Debug, Clone)]
pub struct CapabilityRestriction {
    /// Capability this restricts
    pub capability: Capability,
    /// Restriction type
    pub restriction: RestrictionType,
    /// Allowed values (paths, domains, etc.)
    pub allowed: Vec<String>,
}

/// Types of restrictions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestrictionType {
    /// Restrict to specific paths
    Path,
    /// Restrict to specific domains
    Domain,
    /// Restrict to specific commands
    Command,
    /// Restrict to specific environment variables
    EnvVar,
}

impl CapabilityManifest {
    /// Create an empty manifest
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a manifest with default safe capabilities
    pub fn safe() -> Self {
        let mut manifest = Self::new();
        manifest.require(Capability::FileRead);
        manifest
    }

    /// Create a manifest with all capabilities
    pub fn unrestricted() -> Self {
        let mut manifest = Self::new();
        manifest.require(Capability::System);
        manifest
    }

    /// Require a capability
    pub fn require(&mut self, cap: Capability) {
        self.required.insert(cap);
        self.optional.remove(&cap);
        self.denied.remove(&cap);
    }

    /// Request an optional capability
    pub fn request(&mut self, cap: Capability) {
        if !self.required.contains(&cap) && !self.denied.contains(&cap) {
            self.optional.insert(cap);
        }
    }

    /// Deny a capability
    pub fn deny(&mut self, cap: Capability) {
        self.required.remove(&cap);
        self.optional.remove(&cap);
        self.denied.insert(cap);
    }

    /// Add a restriction
    pub fn restrict(&mut self, restriction: CapabilityRestriction) {
        self.restrictions.push(restriction);
    }

    /// Check if capability is required
    pub fn requires(&self, cap: Capability) -> bool {
        self.required.iter().any(|r| r.implies(&cap))
    }

    /// Check if capability is allowed
    pub fn allows(&self, cap: Capability) -> bool {
        if self.denied.iter().any(|d| d.implies(&cap)) {
            return false;
        }
        self.required.iter().any(|r| r.implies(&cap))
            || self.optional.iter().any(|o| o.implies(&cap))
    }

    /// Check if capability is denied
    pub fn denies(&self, cap: Capability) -> bool {
        self.denied.iter().any(|d| d.implies(&cap))
    }

    /// Get all required capabilities
    pub fn required_capabilities(&self) -> &HashSet<Capability> {
        &self.required
    }

    /// Get all optional capabilities
    pub fn optional_capabilities(&self) -> &HashSet<Capability> {
        &self.optional
    }

    /// Get restrictions for a capability
    pub fn restrictions_for(
        &self,
        cap: Capability,
    ) -> impl Iterator<Item = &CapabilityRestriction> {
        self.restrictions.iter().filter(move |r| r.capability == cap)
    }

    /// Check if a path is allowed for file operations
    pub fn is_path_allowed(&self, path: &str, cap: Capability) -> bool {
        if !self.allows(cap) {
            return false;
        }

        let restrictions: Vec<_> = self
            .restrictions_for(cap)
            .filter(|r| r.restriction == RestrictionType::Path)
            .collect();

        // If no path restrictions, allow all paths
        if restrictions.is_empty() {
            return true;
        }

        // Check if path matches any allowed pattern
        restrictions.iter().any(|r| {
            r.allowed.iter().any(|allowed| {
                path.starts_with(allowed)
                    || globset::Glob::new(allowed)
                        .ok()
                        .and_then(|g| g.compile_matcher().is_match(path).then_some(()))
                        .is_some()
            })
        })
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut output = Vec::new();

        // Required count + capabilities
        output.push(self.required.len() as u8);
        for cap in &self.required {
            output.push(*cap as u8);
        }

        // Optional count + capabilities
        output.push(self.optional.len() as u8);
        for cap in &self.optional {
            output.push(*cap as u8);
        }

        // Denied count + capabilities
        output.push(self.denied.len() as u8);
        for cap in &self.denied {
            output.push(*cap as u8);
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_implies() {
        assert!(Capability::System.implies(&Capability::FileRead));
        assert!(Capability::FileWrite.implies(&Capability::FileRead));
        assert!(!Capability::FileRead.implies(&Capability::FileWrite));
    }

    #[test]
    fn test_manifest_require() {
        let mut manifest = CapabilityManifest::new();
        manifest.require(Capability::FileRead);

        assert!(manifest.requires(Capability::FileRead));
        assert!(manifest.allows(Capability::FileRead));
        assert!(!manifest.denies(Capability::FileRead));
    }

    #[test]
    fn test_manifest_deny() {
        let mut manifest = CapabilityManifest::new();
        manifest.deny(Capability::Network);

        assert!(manifest.denies(Capability::Network));
        assert!(!manifest.allows(Capability::Network));
    }

    #[test]
    fn test_path_restrictions() {
        let mut manifest = CapabilityManifest::new();
        manifest.require(Capability::FileRead);
        manifest.restrict(CapabilityRestriction {
            capability: Capability::FileRead,
            restriction: RestrictionType::Path,
            allowed: vec!["/home/user/project".to_string()],
        });

        assert!(manifest.is_path_allowed("/home/user/project/src/main.rs", Capability::FileRead));
        assert!(!manifest.is_path_allowed("/etc/passwd", Capability::FileRead));
    }
}
