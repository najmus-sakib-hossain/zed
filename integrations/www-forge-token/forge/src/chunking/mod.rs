pub mod cdc;
pub mod structure_aware;

use crate::chunking::cdc::{chunk_data, ChunkConfig, ChunkResult};
use crate::core::manifest::FileType;

pub fn chunk_file(data: &[u8], file_type: FileType, config: &ChunkConfig) -> Vec<ChunkResult> {
    match file_type {
        FileType::UAsset => structure_aware::uasset::chunk_uasset(data, config),
        FileType::Mp4 => structure_aware::mp4::chunk_mp4(data, config),
        FileType::Exr => structure_aware::exr::chunk_exr(data, config),
        FileType::Csp => structure_aware::csp::chunk_csp(data, config),
        _ => chunk_data(data, config),
    }
}
