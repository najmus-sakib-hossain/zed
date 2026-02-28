use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use anyhow::{Context, Result};

use crate::store::cas::ChunkStore;

#[derive(Debug, Clone)]
pub struct PackFile;

#[derive(Debug, Clone)]
pub struct PackIndexEntry {
    pub hash: [u8; 32],
    pub offset: u64,
    pub length: u32,
}

#[derive(Debug, Clone)]
pub struct PackIndex {
    pub entries: Vec<PackIndexEntry>,
}

pub fn create_pack(store: &ChunkStore, chunk_hashes: &[[u8; 32]], output: &Path) -> Result<PackIndex> {
    let mut file = File::create(output).with_context(|| format!("create pack {}", output.display()))?;
    let mut offset = 0u64;
    let mut entries = Vec::with_capacity(chunk_hashes.len());

    for hash_arr in chunk_hashes {
        let hash = blake3::Hash::from(*hash_arr);
        let data = store.read(&hash)?;
        file.write_all(&data).context("write chunk to pack")?;
        entries.push(PackIndexEntry {
            hash: *hash_arr,
            offset,
            length: data.len() as u32,
        });
        offset += data.len() as u64;
    }

    let mut trailer = Vec::new();
    trailer.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for e in &entries {
        trailer.extend_from_slice(&e.hash);
        trailer.extend_from_slice(&e.offset.to_le_bytes());
        trailer.extend_from_slice(&e.length.to_le_bytes());
    }

    file.write_all(&trailer).context("write index trailer")?;
    file.write_all(&(trailer.len() as u64).to_le_bytes())
        .context("write index trailer size")?;

    Ok(PackIndex { entries })
}

pub fn read_from_pack(pack_path: &Path, index_entry: &PackIndexEntry) -> Result<Vec<u8>> {
    let mut file = File::open(pack_path).with_context(|| format!("open pack {}", pack_path.display()))?;
    file.seek(SeekFrom::Start(index_entry.offset))
        .context("seek to chunk in pack")?;
    let mut buf = vec![0u8; index_entry.length as usize];
    file.read_exact(&mut buf).context("read chunk from pack")?;
    Ok(buf)
}
