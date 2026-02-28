use crate::chunking::cdc::{self, ChunkConfig, ChunkResult};

pub fn find_box(data: &[u8], box_type: &[u8; 4]) -> Option<(usize, usize)> {
    let mut offset = 0usize;
    while offset + 8 <= data.len() {
        let size32 = u32::from_be_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        let typ = &data[offset + 4..offset + 8];

        let box_size = if size32 == 0 {
            data.len().saturating_sub(offset)
        } else if size32 == 1 {
            if offset + 16 > data.len() {
                return None;
            }
            let size64 = u64::from_be_bytes([
                data[offset + 8],
                data[offset + 9],
                data[offset + 10],
                data[offset + 11],
                data[offset + 12],
                data[offset + 13],
                data[offset + 14],
                data[offset + 15],
            ]) as usize;
            size64
        } else {
            size32
        };

        if box_size < 8 || offset + box_size > data.len() {
            return None;
        }

        if typ == box_type {
            return Some((offset, box_size));
        }

        offset += box_size;
    }

    None
}

pub fn chunk_mp4(data: &[u8], config: &ChunkConfig) -> Vec<ChunkResult> {
    if data.len() < 16 {
        return cdc::chunk_data(data, config);
    }

    let Some((mdat_offset, mdat_size)) = find_box(data, b"mdat") else {
        return cdc::chunk_data(data, config);
    };

    if mdat_size < 8 {
        return cdc::chunk_data(data, config);
    }

    let header_size = if u32::from_be_bytes([
        data[mdat_offset],
        data[mdat_offset + 1],
        data[mdat_offset + 2],
        data[mdat_offset + 3],
    ]) == 1
    {
        16
    } else {
        8
    };

    if mdat_offset + header_size > data.len() {
        return cdc::chunk_data(data, config);
    }

    let mdat_payload_offset = mdat_offset + header_size;
    let mdat_payload_end = (mdat_offset + mdat_size).min(data.len());
    if mdat_payload_offset >= mdat_payload_end {
        return cdc::chunk_data(data, config);
    }

    let mut out = Vec::new();
    if mdat_offset > 0 {
        let metadata = &data[..mdat_offset];
        out.push(ChunkResult {
            offset: 0,
            length: metadata.len(),
            hash: blake3::hash(metadata),
        });
    }

    let payload = &data[mdat_payload_offset..mdat_payload_end];
    let mut payload_chunks = cdc::chunk_data(payload, config);
    for chunk in &mut payload_chunks {
        chunk.offset += mdat_payload_offset;
    }
    out.extend(payload_chunks);

    if mdat_payload_end < data.len() {
        let tail = &data[mdat_payload_end..];
        out.push(ChunkResult {
            offset: mdat_payload_end,
            length: tail.len(),
            hash: blake3::hash(tail),
        });
    }

    out
}
