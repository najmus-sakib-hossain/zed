use crate::chunking::cdc::{self, ChunkConfig, ChunkResult};

pub fn chunk_exr(data: &[u8], config: &ChunkConfig) -> Vec<ChunkResult> {
    if data.len() < 16 {
        return cdc::chunk_data(data, config);
    }

    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if magic != 0x762F3101 {
        return cdc::chunk_data(data, config);
    }

    let mut pos = 8usize;
    while pos + 1 < data.len() {
        if data[pos] == 0 && data[pos + 1] == 0 {
            pos += 2;
            break;
        }
        pos += 1;
    }

    if pos >= data.len() {
        return cdc::chunk_data(data, config);
    }

    let header_end = pos.min(data.len());
    let mut out = Vec::new();
    let header = &data[..header_end];
    out.push(ChunkResult {
        offset: 0,
        length: header.len(),
        hash: blake3::hash(header),
    });

    if header_end < data.len() {
        let pixel = &data[header_end..];
        let mut pixel_chunks = cdc::chunk_data(pixel, config);
        for chunk in &mut pixel_chunks {
            chunk.offset += header_end;
        }
        out.extend(pixel_chunks);
    }

    out
}
