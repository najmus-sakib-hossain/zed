use crate::chunking::cdc::{self, ChunkConfig, ChunkResult};

const SQLITE_HEADER: &[u8; 16] = b"SQLite format 3\0";

pub fn chunk_csp(data: &[u8], config: &ChunkConfig) -> Vec<ChunkResult> {
    if data.len() < 18 || &data[0..16] != SQLITE_HEADER {
        return cdc::chunk_data(data, config);
    }

    let page_size = u16::from_be_bytes([data[16], data[17]]) as usize;
    let page_size = if page_size == 1 { 65_536 } else { page_size };

    if page_size < 512 || page_size > 65_536 {
        return cdc::chunk_data(data, config);
    }

    let mut out = Vec::new();
    let mut offset = 0usize;
    while offset < data.len() {
        let end = (offset + page_size).min(data.len());
        let page = &data[offset..end];
        out.push(ChunkResult {
            offset,
            length: page.len(),
            hash: blake3::hash(page),
        });
        offset = end;
    }

    if out.is_empty() {
        cdc::chunk_data(data, config)
    } else {
        out
    }
}
