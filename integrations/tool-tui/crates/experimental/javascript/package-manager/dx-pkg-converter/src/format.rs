//! DXP File Format
//! Binary package format with LZ4 compression and metadata

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// DXP Package File
#[derive(Debug, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct DxpFile {
    pub version: u32,
    pub metadata: HashMap<String, String>,
    pub entries: Vec<DxpFileEntry>,
}

/// File entry in DXP package
#[derive(Debug, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct DxpFileEntry {
    pub path: String,
    pub size: u64,
    pub compressed_size: u64,
    pub hash: String,
    pub data: Vec<u8>,
}

impl DxpFile {
    /// Write DXP file to disk
    pub fn write(&self, path: &Path) -> Result<()> {
        let mut file = File::create(path)?;

        // Write magic bytes
        file.write_all(b"DXPK")?;

        // Write version
        file.write_all(&self.version.to_le_bytes())?;

        // Serialize entire structure with bincode
        let encoded = bincode::encode_to_vec(self, bincode::config::standard())?;

        // Write length + data
        file.write_all(&(encoded.len() as u64).to_le_bytes())?;
        file.write_all(&encoded)?;

        Ok(())
    }

    /// Read DXP file from disk
    #[allow(dead_code)]
    pub fn read(path: &Path) -> Result<Self> {
        use std::io::Read;

        let mut file = File::open(path)?;

        // Read magic
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;
        if &magic != b"DXPK" {
            anyhow::bail!("Invalid DXP file: bad magic");
        }

        // Read version
        let mut version_bytes = [0u8; 4];
        file.read_exact(&mut version_bytes)?;
        let _version = u32::from_le_bytes(version_bytes);

        // Read data size
        let mut size_bytes = [0u8; 8];
        file.read_exact(&mut size_bytes)?;
        let data_size = u64::from_le_bytes(size_bytes);

        // Read and decode data
        let mut data = vec![0u8; data_size as usize];
        file.read_exact(&mut data)?;

        let (dxp, _): (DxpFile, _) =
            bincode::decode_from_slice(&data, bincode::config::standard())?;

        Ok(dxp)
    }
}
