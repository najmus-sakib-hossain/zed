//! DX Binary Dawn - Binary-First Rule Format
//!
//! Leverages DX Serializer's world-record format principles for
//! 73% smaller rules with zero-copy parsing.

pub mod checksum;
mod infinity_format;
mod memory_map;
mod rule_schema;
mod simd_tokenizer;
mod string_table;

pub use checksum::{Blake3Checksum, Blake3Hasher, compute_blake3, verify_blake3};
pub use infinity_format::{INFINITY_MAGIC, InfinityHeader, InfinityRule};
pub use memory_map::{MappedRule, RuleMapping};
pub use rule_schema::{BinaryRule, BinaryStep, RuleFlags, SectionOffsets};
pub use simd_tokenizer::{SimdTokenizer, Token, TokenType};
pub use string_table::{StringId, StringTable, StringTableBuilder};

use crate::Result;

/// DX ∞ Infinity Format version
pub const INFINITY_VERSION: u16 = 1;

/// Maximum rule size (10MB safety limit)
pub const MAX_RULE_SIZE: usize = 10 * 1024 * 1024;

/// Load rules from binary format with zero-copy
pub fn load_zero_copy(path: &std::path::Path) -> Result<MappedRule> {
    MappedRule::open(path)
}

/// Verify binary integrity
pub fn verify_integrity(data: &[u8]) -> Result<bool> {
    if data.len() < std::mem::size_of::<InfinityHeader>() {
        return Ok(false);
    }

    let header = InfinityHeader::from_bytes(data)?;

    // Verify magic
    if &header.magic != INFINITY_MAGIC {
        return Ok(false);
    }

    // Verify checksum
    let payload = &data[std::mem::size_of::<InfinityHeader>()..];
    Ok(verify_blake3(payload, &header.checksum))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infinity_version() {
        assert_eq!(INFINITY_VERSION, 1);
    }

    #[test]
    fn test_max_rule_size() {
        assert_eq!(MAX_RULE_SIZE, 10 * 1024 * 1024);
    }
}
