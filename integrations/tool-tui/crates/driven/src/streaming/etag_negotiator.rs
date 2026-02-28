//! ETag Negotiator
//!
//! HTTP-style cache validation for rule synchronization.

use crate::binary::checksum::compute_blake3;

/// Negotiation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NegotiationResult {
    /// Content is fresh, no update needed
    Fresh,
    /// Content is stale, full update needed
    Stale,
    /// Content can be patched (delta update)
    Patch,
}

/// ETag for content versioning
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ETag {
    /// Strong ETag (content hash)
    strong: [u8; 16],
    /// Weak ETag (version number)
    weak: u64,
}

impl ETag {
    /// Create from content
    pub fn from_content(content: &[u8], version: u64) -> Self {
        Self {
            strong: compute_blake3(content),
            weak: version,
        }
    }

    /// Create a weak ETag (version only)
    pub fn weak(version: u64) -> Self {
        Self {
            strong: [0; 16],
            weak: version,
        }
    }

    /// Get strong hash
    pub fn strong(&self) -> &[u8; 16] {
        &self.strong
    }

    /// Get weak version
    pub fn weak_version(&self) -> u64 {
        self.weak
    }

    /// Check if ETags match strongly
    pub fn strong_match(&self, other: &ETag) -> bool {
        self.strong != [0; 16] && self.strong == other.strong
    }

    /// Check if ETags match weakly (version only)
    pub fn weak_match(&self, other: &ETag) -> bool {
        self.weak == other.weak
    }

    /// Serialize to string
    pub fn to_string(&self) -> String {
        let hash_hex: String = self.strong.iter().map(|b| format!("{:02x}", b)).collect();
        format!("\"{}:{}\"", hash_hex, self.weak)
    }

    /// Parse from string
    pub fn from_string(s: &str) -> Option<Self> {
        let s = s.trim_matches('"');
        let mut parts = s.split(':');

        let hash_hex = parts.next()?;
        let version = parts.next()?.parse().ok()?;

        if hash_hex.len() != 32 {
            return None;
        }

        let mut strong = [0u8; 16];
        for (i, chunk) in hash_hex.as_bytes().chunks(2).enumerate() {
            let hex = std::str::from_utf8(chunk).ok()?;
            strong[i] = u8::from_str_radix(hex, 16).ok()?;
        }

        Some(Self {
            strong,
            weak: version,
        })
    }
}

/// ETag negotiator for cache validation
#[derive(Debug, Default)]
pub struct ETagNegotiator {
    /// Local ETags by resource ID
    local: std::collections::HashMap<String, ETag>,
}

impl ETagNegotiator {
    /// Create a new negotiator
    pub fn new() -> Self {
        Self::default()
    }

    /// Set local ETag for a resource
    pub fn set_local(&mut self, resource_id: &str, etag: ETag) {
        self.local.insert(resource_id.to_string(), etag);
    }

    /// Get local ETag for a resource
    pub fn get_local(&self, resource_id: &str) -> Option<&ETag> {
        self.local.get(resource_id)
    }

    /// Negotiate with remote ETag
    pub fn negotiate(&self, resource_id: &str, remote: &ETag) -> NegotiationResult {
        match self.local.get(resource_id) {
            None => NegotiationResult::Stale,
            Some(local) => {
                if local.strong_match(remote) {
                    NegotiationResult::Fresh
                } else if local.weak_match(remote) {
                    // Same version but different content - likely patching possible
                    NegotiationResult::Patch
                } else {
                    NegotiationResult::Stale
                }
            }
        }
    }

    /// Update local ETag after sync
    pub fn update(&mut self, resource_id: &str, new_content: &[u8], new_version: u64) {
        let etag = ETag::from_content(new_content, new_version);
        self.local.insert(resource_id.to_string(), etag);
    }

    /// Clear all ETags
    pub fn clear(&mut self) {
        self.local.clear();
    }

    /// Get If-None-Match header value
    pub fn if_none_match(&self, resource_id: &str) -> Option<String> {
        self.local.get(resource_id).map(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_etag_creation() {
        let content = b"Hello, World!";
        let etag = ETag::from_content(content, 1);

        assert_ne!(etag.strong(), &[0; 16]);
        assert_eq!(etag.weak_version(), 1);
    }

    #[test]
    fn test_etag_roundtrip() {
        let content = b"Test content";
        let original = ETag::from_content(content, 42);
        let serialized = original.to_string();
        let parsed = ETag::from_string(&serialized).unwrap();

        assert_eq!(original.strong(), parsed.strong());
        assert_eq!(original.weak_version(), parsed.weak_version());
    }

    #[test]
    fn test_negotiation() {
        let mut negotiator = ETagNegotiator::new();

        let content = b"Test content";
        let etag = ETag::from_content(content, 1);
        negotiator.set_local("resource1", etag.clone());

        // Same content - Fresh
        assert_eq!(negotiator.negotiate("resource1", &etag), NegotiationResult::Fresh);

        // Unknown resource - Stale
        assert_eq!(negotiator.negotiate("unknown", &etag), NegotiationResult::Stale);

        // Different content - Stale
        let other = ETag::from_content(b"Different", 2);
        assert_eq!(negotiator.negotiate("resource1", &other), NegotiationResult::Stale);
    }
}
