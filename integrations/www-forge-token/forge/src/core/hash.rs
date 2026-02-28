use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use memmap2::Mmap;

pub fn hash_bytes(data: &[u8]) -> blake3::Hash {
    blake3::hash(data)
}

pub fn hash_file(path: &Path) -> Result<blake3::Hash> {
    let metadata = fs::metadata(path).with_context(|| format!("stat failed for {}", path.display()))?;
    if metadata.len() < 1024 * 1024 {
        let bytes = fs::read(path).with_context(|| format!("read failed for {}", path.display()))?;
        return Ok(hash_bytes(&bytes));
    }

    let file = fs::File::open(path).with_context(|| format!("open failed for {}", path.display()))?;
    let mapped = unsafe { Mmap::map(&file) }.with_context(|| format!("mmap failed for {}", path.display()))?;
    Ok(hash_bytes(&mapped))
}

pub fn hash_to_hex(hash: &blake3::Hash) -> String {
    hash.to_hex().to_string()
}

pub fn hex_to_hash(hex_str: &str) -> Result<blake3::Hash> {
    let bytes = hex::decode(hex_str).with_context(|| "invalid hex hash")?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("hash must be exactly 32 bytes"))?;
    Ok(blake3::Hash::from(arr))
}
