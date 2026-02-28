use crate::chunking::cdc::{self, ChunkConfig, ChunkResult};

pub const UASSET_MAGIC: u32 = 0x9E2A83C1;

pub fn chunk_uasset(data: &[u8], config: &ChunkConfig) -> Vec<ChunkResult> {
    if data.len() < 28 {
        return cdc::chunk_data(data, config);
    }

    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if magic != UASSET_MAGIC {
        return cdc::chunk_data(data, config);
    }

    let mut header_size = u32::from_le_bytes([data[24], data[25], data[26], data[27]]) as usize;
    if header_size == 0 {
        return cdc::chunk_data(data, config);
    }
    header_size = header_size.min(data.len());

    let mut out = Vec::new();
    let header = &data[..header_size];
    out.push(ChunkResult {
        offset: 0,
        length: header.len(),
        hash: blake3::hash(header),
    });

    if header_size < data.len() {
        let rest = &data[header_size..];
        let mut rest_chunks = cdc::chunk_data(rest, config);
        for chunk in &mut rest_chunks {
            chunk.offset += header_size;
        }
        out.extend(rest_chunks);
    }

    out
}
