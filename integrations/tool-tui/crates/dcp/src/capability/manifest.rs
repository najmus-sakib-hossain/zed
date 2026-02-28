//! Pre-computed capability manifest for instant capability negotiation.
//!
//! The CapabilityManifest uses bitsets to represent supported tools, resources,
//! and prompts, enabling O(1) capability intersection using bitwise AND operations.

use crate::DCPError;

/// Pre-computed capability manifest
///
/// Uses bitsets for efficient capability representation and intersection.
/// - tools: 8192 bits (128 × u64) = supports up to 8192 tools
/// - resources: 1024 bits (16 × u64) = supports up to 1024 resources
/// - prompts: 512 bits (8 × u64) = supports up to 512 prompts
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityManifest {
    /// Protocol version
    pub version: u16,
    /// Reserved for alignment
    _reserved: u16,
    /// Reserved for future use
    _reserved2: u32,
    /// Supported tools bitset (8192 bits = 1024 bytes)
    pub tools: [u64; 128],
    /// Supported resources bitset (1024 bits = 128 bytes)
    pub resources: [u64; 16],
    /// Supported prompts bitset (512 bits = 64 bytes)
    pub prompts: [u64; 8],
    /// Extension flags
    pub extensions: u64,
    /// Ed25519 signature
    pub signature: [u8; 64],
}

impl CapabilityManifest {
    /// Size of the manifest in bytes
    pub const SIZE: usize = 8 + 1024 + 128 + 64 + 8 + 64; // 1296 bytes

    /// Maximum number of tools supported
    pub const MAX_TOOLS: usize = 8192;
    /// Maximum number of resources supported
    pub const MAX_RESOURCES: usize = 1024;
    /// Maximum number of prompts supported
    pub const MAX_PROMPTS: usize = 512;

    /// Create a new empty capability manifest
    pub fn new(version: u16) -> Self {
        Self {
            version,
            _reserved: 0,
            _reserved2: 0,
            tools: [0u64; 128],
            resources: [0u64; 16],
            prompts: [0u64; 8],
            extensions: 0,
            signature: [0u8; 64],
        }
    }

    /// Parse manifest from bytes
    #[inline(always)]
    pub fn from_bytes(bytes: &[u8]) -> Result<&Self, DCPError> {
        if bytes.len() < Self::SIZE {
            return Err(DCPError::InsufficientData);
        }
        // SAFETY: We've verified the slice is at least SIZE bytes
        Ok(unsafe { &*(bytes.as_ptr() as *const Self) })
    }

    /// Serialize manifest to bytes
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: CapabilityManifest is repr(C) with predictable layout
        unsafe { std::slice::from_raw_parts(self as *const Self as *const u8, Self::SIZE) }
    }

    /// Get the bytes that are signed (everything before the signature)
    pub fn signed_bytes(&self) -> &[u8] {
        &self.as_bytes()[..Self::SIZE - 64]
    }

    /// Compute capability intersection - single CPU instruction per word
    ///
    /// Returns a new manifest containing only capabilities present in both manifests.
    #[inline]
    pub fn intersect(&self, other: &Self) -> Self {
        let mut result = Self::new(self.version.min(other.version));

        // Intersect tools bitset
        for i in 0..128 {
            result.tools[i] = self.tools[i] & other.tools[i];
        }

        // Intersect resources bitset
        for i in 0..16 {
            result.resources[i] = self.resources[i] & other.resources[i];
        }

        // Intersect prompts bitset
        for i in 0..8 {
            result.prompts[i] = self.prompts[i] & other.prompts[i];
        }

        // Intersect extensions
        result.extensions = self.extensions & other.extensions;

        result
    }

    /// Set a tool capability
    #[inline]
    pub fn set_tool(&mut self, tool_id: u16) {
        let id = tool_id as usize;
        if id < Self::MAX_TOOLS {
            let word = id / 64;
            let bit = id % 64;
            self.tools[word] |= 1u64 << bit;
        }
    }

    /// Clear a tool capability
    #[inline]
    pub fn clear_tool(&mut self, tool_id: u16) {
        let id = tool_id as usize;
        if id < Self::MAX_TOOLS {
            let word = id / 64;
            let bit = id % 64;
            self.tools[word] &= !(1u64 << bit);
        }
    }

    /// Check if a tool is supported
    #[inline]
    pub fn has_tool(&self, tool_id: u16) -> bool {
        let id = tool_id as usize;
        if id >= Self::MAX_TOOLS {
            return false;
        }
        let word = id / 64;
        let bit = id % 64;
        self.tools[word] & (1u64 << bit) != 0
    }

    /// Set a resource capability
    #[inline]
    pub fn set_resource(&mut self, resource_id: u16) {
        let id = resource_id as usize;
        if id < Self::MAX_RESOURCES {
            let word = id / 64;
            let bit = id % 64;
            self.resources[word] |= 1u64 << bit;
        }
    }

    /// Clear a resource capability
    #[inline]
    pub fn clear_resource(&mut self, resource_id: u16) {
        let id = resource_id as usize;
        if id < Self::MAX_RESOURCES {
            let word = id / 64;
            let bit = id % 64;
            self.resources[word] &= !(1u64 << bit);
        }
    }

    /// Check if a resource is supported
    #[inline]
    pub fn has_resource(&self, resource_id: u16) -> bool {
        let id = resource_id as usize;
        if id >= Self::MAX_RESOURCES {
            return false;
        }
        let word = id / 64;
        let bit = id % 64;
        self.resources[word] & (1u64 << bit) != 0
    }

    /// Set a prompt capability
    #[inline]
    pub fn set_prompt(&mut self, prompt_id: u16) {
        let id = prompt_id as usize;
        if id < Self::MAX_PROMPTS {
            let word = id / 64;
            let bit = id % 64;
            self.prompts[word] |= 1u64 << bit;
        }
    }

    /// Clear a prompt capability
    #[inline]
    pub fn clear_prompt(&mut self, prompt_id: u16) {
        let id = prompt_id as usize;
        if id < Self::MAX_PROMPTS {
            let word = id / 64;
            let bit = id % 64;
            self.prompts[word] &= !(1u64 << bit);
        }
    }

    /// Check if a prompt is supported
    #[inline]
    pub fn has_prompt(&self, prompt_id: u16) -> bool {
        let id = prompt_id as usize;
        if id >= Self::MAX_PROMPTS {
            return false;
        }
        let word = id / 64;
        let bit = id % 64;
        self.prompts[word] & (1u64 << bit) != 0
    }

    /// Set an extension flag
    #[inline]
    pub fn set_extension(&mut self, bit: u8) {
        if bit < 64 {
            self.extensions |= 1u64 << bit;
        }
    }

    /// Clear an extension flag
    #[inline]
    pub fn clear_extension(&mut self, bit: u8) {
        if bit < 64 {
            self.extensions &= !(1u64 << bit);
        }
    }

    /// Check if an extension is supported
    #[inline]
    pub fn has_extension(&self, bit: u8) -> bool {
        if bit >= 64 {
            return false;
        }
        self.extensions & (1u64 << bit) != 0
    }

    /// Count the number of supported tools
    pub fn tool_count(&self) -> u32 {
        self.tools.iter().map(|w| w.count_ones()).sum()
    }

    /// Count the number of supported resources
    pub fn resource_count(&self) -> u32 {
        self.resources.iter().map(|w| w.count_ones()).sum()
    }

    /// Count the number of supported prompts
    pub fn prompt_count(&self) -> u32 {
        self.prompts.iter().map(|w| w.count_ones()).sum()
    }

    /// Count the number of enabled extensions
    pub fn extension_count(&self) -> u32 {
        self.extensions.count_ones()
    }

    /// Get an iterator over all supported tool IDs
    pub fn tool_ids(&self) -> impl Iterator<Item = u16> + '_ {
        self.tools.iter().enumerate().flat_map(|(word_idx, &word)| {
            (0..64).filter_map(move |bit| {
                if word & (1u64 << bit) != 0 {
                    Some((word_idx * 64 + bit) as u16)
                } else {
                    None
                }
            })
        })
    }

    /// Get an iterator over all supported resource IDs
    pub fn resource_ids(&self) -> impl Iterator<Item = u16> + '_ {
        self.resources.iter().enumerate().flat_map(|(word_idx, &word)| {
            (0..64).filter_map(move |bit| {
                if word & (1u64 << bit) != 0 {
                    Some((word_idx * 64 + bit) as u16)
                } else {
                    None
                }
            })
        })
    }

    /// Get an iterator over all supported prompt IDs
    pub fn prompt_ids(&self) -> impl Iterator<Item = u16> + '_ {
        self.prompts.iter().enumerate().flat_map(|(word_idx, &word)| {
            (0..64).filter_map(move |bit| {
                if word & (1u64 << bit) != 0 {
                    Some((word_idx * 64 + bit) as u16)
                } else {
                    None
                }
            })
        })
    }
}

impl Default for CapabilityManifest {
    fn default() -> Self {
        Self::new(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_size() {
        assert_eq!(std::mem::size_of::<CapabilityManifest>(), CapabilityManifest::SIZE);
    }

    #[test]
    fn test_tool_operations() {
        let mut manifest = CapabilityManifest::new(1);

        assert!(!manifest.has_tool(42));
        manifest.set_tool(42);
        assert!(manifest.has_tool(42));
        manifest.clear_tool(42);
        assert!(!manifest.has_tool(42));
    }

    #[test]
    fn test_resource_operations() {
        let mut manifest = CapabilityManifest::new(1);

        assert!(!manifest.has_resource(100));
        manifest.set_resource(100);
        assert!(manifest.has_resource(100));
        manifest.clear_resource(100);
        assert!(!manifest.has_resource(100));
    }

    #[test]
    fn test_prompt_operations() {
        let mut manifest = CapabilityManifest::new(1);

        assert!(!manifest.has_prompt(50));
        manifest.set_prompt(50);
        assert!(manifest.has_prompt(50));
        manifest.clear_prompt(50);
        assert!(!manifest.has_prompt(50));
    }

    #[test]
    fn test_extension_operations() {
        let mut manifest = CapabilityManifest::new(1);

        assert!(!manifest.has_extension(5));
        manifest.set_extension(5);
        assert!(manifest.has_extension(5));
        manifest.clear_extension(5);
        assert!(!manifest.has_extension(5));
    }

    #[test]
    fn test_intersection() {
        let mut m1 = CapabilityManifest::new(1);
        let mut m2 = CapabilityManifest::new(2);

        // Set some tools in m1
        m1.set_tool(1);
        m1.set_tool(2);
        m1.set_tool(3);

        // Set some tools in m2
        m2.set_tool(2);
        m2.set_tool(3);
        m2.set_tool(4);

        // Intersection should have only 2 and 3
        let result = m1.intersect(&m2);
        assert!(!result.has_tool(1));
        assert!(result.has_tool(2));
        assert!(result.has_tool(3));
        assert!(!result.has_tool(4));

        // Version should be minimum
        assert_eq!(result.version, 1);
    }

    #[test]
    fn test_round_trip() {
        let mut manifest = CapabilityManifest::new(1);
        manifest.set_tool(42);
        manifest.set_tool(100);
        manifest.set_resource(5);
        manifest.set_prompt(10);
        manifest.set_extension(3);

        let bytes = manifest.as_bytes();
        let parsed = CapabilityManifest::from_bytes(bytes).unwrap();

        assert_eq!(parsed.version, 1);
        assert!(parsed.has_tool(42));
        assert!(parsed.has_tool(100));
        assert!(parsed.has_resource(5));
        assert!(parsed.has_prompt(10));
        assert!(parsed.has_extension(3));
    }

    #[test]
    fn test_counts() {
        let mut manifest = CapabilityManifest::new(1);
        manifest.set_tool(1);
        manifest.set_tool(2);
        manifest.set_tool(3);
        manifest.set_resource(1);
        manifest.set_resource(2);
        manifest.set_prompt(1);
        manifest.set_extension(0);
        manifest.set_extension(1);

        assert_eq!(manifest.tool_count(), 3);
        assert_eq!(manifest.resource_count(), 2);
        assert_eq!(manifest.prompt_count(), 1);
        assert_eq!(manifest.extension_count(), 2);
    }

    #[test]
    fn test_boundary_ids() {
        let mut manifest = CapabilityManifest::new(1);

        // Test boundary tool IDs
        manifest.set_tool(0);
        manifest.set_tool(63);
        manifest.set_tool(64);
        manifest.set_tool(8191);

        assert!(manifest.has_tool(0));
        assert!(manifest.has_tool(63));
        assert!(manifest.has_tool(64));
        assert!(manifest.has_tool(8191));

        // Out of range should not panic
        assert!(!manifest.has_tool(8192));
    }

    #[test]
    fn test_iterators() {
        let mut manifest = CapabilityManifest::new(1);
        manifest.set_tool(5);
        manifest.set_tool(100);
        manifest.set_tool(1000);

        let tool_ids: Vec<_> = manifest.tool_ids().collect();
        assert_eq!(tool_ids, vec![5, 100, 1000]);
    }
}
