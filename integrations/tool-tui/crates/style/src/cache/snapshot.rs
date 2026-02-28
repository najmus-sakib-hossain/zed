use ahash::AHashSet;
use memmap2::Mmap;
use std::fs::{self, File};
use std::hash::Hasher;
use std::io::Write;

/// Get the default cache directory path
#[allow(dead_code)]
fn default_cache_dir() -> String {
    ".dx/cache".to_string()
}

fn snapshot_path(cache_dir: &str) -> String {
    format!("{}/snapshot.bin", cache_dir)
}
const MAGIC: u32 = 0x44585334;
const VERSION: u16 = 4;

#[allow(dead_code)]
pub fn load_snapshot() -> Option<(AHashSet<String>, u64, u64)> {
    load_snapshot_from_dir(&default_cache_dir())
}

pub fn load_snapshot_from_dir(cache_dir: &str) -> Option<(AHashSet<String>, u64, u64)> {
    let f = File::open(snapshot_path(cache_dir)).ok()?;
    let mmap = unsafe { Mmap::map(&f).ok()? };
    if mmap.len() < 4 + 2 + 4 + 8 + 8 {
        return None;
    }
    let magic = u32::from_le_bytes(mmap[0..4].try_into().unwrap());
    if magic != MAGIC {
        return None;
    }
    let version = u16::from_le_bytes(mmap[4..6].try_into().unwrap());
    if version != VERSION {
        return None;
    }
    let count = u32::from_le_bytes(mmap[6..10].try_into().unwrap()) as usize;
    let html_hash = u64::from_le_bytes(mmap[10..18].try_into().unwrap());
    let stored_checksum = u64::from_le_bytes(mmap[18..26].try_into().unwrap());
    let mut set = AHashSet::with_capacity(count.next_power_of_two());
    let mut offset = 26usize;
    let mut checksum_hasher = ahash::AHasher::default();
    for _ in 0..count {
        if offset + 2 > mmap.len() {
            return None;
        }
        let len = u16::from_le_bytes(mmap[offset..offset + 2].try_into().unwrap()) as usize;
        offset += 2;
        if offset + len > mmap.len() {
            return None;
        }
        if let Ok(st) = std::str::from_utf8(&mmap[offset..offset + len]) {
            checksum_hasher.write(st.as_bytes());
            set.insert(st.to_owned());
        }
        offset += len;
    }
    if checksum_hasher.finish() != stored_checksum {
        return None;
    }
    Some((set, html_hash, stored_checksum))
}

#[allow(dead_code)]
pub fn save_snapshot(cache: &AHashSet<String>, html_hash: u64) {
    save_snapshot_to_dir(cache, html_hash, &default_cache_dir())
}

pub fn save_snapshot_to_dir(cache: &AHashSet<String>, html_hash: u64, cache_dir: &str) {
    if fs::create_dir_all(cache_dir).is_err() {
        return;
    }
    if let Ok(mut f) = File::create(snapshot_path(cache_dir)) {
        let mut header = Vec::with_capacity(4 + 2 + 4 + 8 + 8);
        let mut checksum_hasher = ahash::AHasher::default();
        for s in cache.iter() {
            checksum_hasher.write(s.as_bytes());
        }
        let checksum = checksum_hasher.finish();
        header.extend_from_slice(&MAGIC.to_le_bytes());
        header.extend_from_slice(&VERSION.to_le_bytes());
        header.extend_from_slice(&(cache.len() as u32).to_le_bytes());
        header.extend_from_slice(&html_hash.to_le_bytes());
        header.extend_from_slice(&checksum.to_le_bytes());
        let _ = f.write_all(&header);
        for s in cache.iter() {
            if s.len() > u16::MAX as usize {
                continue;
            }
            let len = s.len() as u16;
            let _ = f.write_all(&len.to_le_bytes());
            let _ = f.write_all(s.as_bytes());
        }
    }
}
