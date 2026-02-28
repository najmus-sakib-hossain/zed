use std::path::Path;

use anyhow::{Context, Result};
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
pub struct ChunkRef {
    pub hash: [u8; 32],
    pub offset: u64,
    pub length: u32,
    pub compressed_length: u32,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Unknown,
    UAsset,
    Exr,
    Mp4,
    Csp,
    Png,
    Psd,
    Blend,
    Graphite,
}

impl FileType {
    pub fn detect(path: &Path, header: &[u8]) -> Self {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()) {
            match ext.as_str() {
                "uasset" => return Self::UAsset,
                "exr" => return Self::Exr,
                "mp4" | "mov" | "m4v" => return Self::Mp4,
                "clip" | "csp" => return Self::Csp,
                "png" => return Self::Png,
                "psd" => return Self::Psd,
                "blend" => return Self::Blend,
                "graphite" => return Self::Graphite,
                _ => {}
            }
        }

        if header.len() >= 4 {
            if u32::from_le_bytes([header[0], header[1], header[2], header[3]]) == 0x762F3101 {
                return Self::Exr;
            }
            if header[0..4] == [0x89, 0x50, 0x4E, 0x47] {
                return Self::Png;
            }
            if header[0..4] == [0x38, 0x42, 0x50, 0x53] {
                return Self::Psd;
            }
        }
        if header.len() >= 8 && &header[4..8] == b"ftyp" {
            return Self::Mp4;
        }
        if header.len() >= 7 && &header[0..7] == b"BLENDER" {
            return Self::Blend;
        }

        Self::Unknown
    }
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub file_hash: [u8; 32],
    pub chunks: Vec<ChunkRef>,
    pub mode: u32,
    pub mtime_ns: i64,
    pub file_type: FileType,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
pub struct Commit {
    pub id: [u8; 32],
    pub parents: Vec<[u8; 32]>,
    pub files: Vec<FileEntry>,
    pub message: String,
    pub author: String,
    pub timestamp_ns: i64,
}

pub type Manifest = Commit;

pub fn serialize_commit(commit: &Commit) -> Result<Vec<u8>> {
    let bytes =
        rkyv::to_bytes::<rkyv::rancor::Error>(commit).context("failed to serialize commit")?;
    Ok(bytes.to_vec())
}

pub fn deserialize_commit(bytes: &[u8]) -> Result<Commit> {
    rkyv::from_bytes::<Commit, rkyv::rancor::Error>(bytes)
        .context("failed to deserialize archived commit")
}

pub fn serialize_file_entry(entry: &FileEntry) -> Result<Vec<u8>> {
    let bytes =
        rkyv::to_bytes::<rkyv::rancor::Error>(entry).context("failed to serialize file entry")?;
    Ok(bytes.to_vec())
}

pub fn deserialize_file_entry(bytes: &[u8]) -> Result<FileEntry> {
    rkyv::from_bytes::<FileEntry, rkyv::rancor::Error>(bytes)
        .context("failed to deserialize archived file entry")
}
