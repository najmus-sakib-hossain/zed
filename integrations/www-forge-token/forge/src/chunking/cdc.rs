#[derive(Debug, Clone)]
pub struct ChunkConfig {
    pub min_size: u32,
    pub avg_size: u32,
    pub max_size: u32,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            min_size: 64 * 1024,
            avg_size: 256 * 1024,
            max_size: 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChunkResult {
    pub offset: usize,
    pub length: usize,
    pub hash: blake3::Hash,
}

pub fn chunk_data(data: &[u8], config: &ChunkConfig) -> Vec<ChunkResult> {
    if data.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    for chunk in fastcdc::v2020::FastCDC::new(
        data,
        config.min_size,
        config.avg_size,
        config.max_size,
    ) {
        let payload = &data[chunk.offset..chunk.offset + chunk.length];
        out.push(ChunkResult {
            offset: chunk.offset,
            length: chunk.length,
            hash: blake3::hash(payload),
        });
    }
    out
}
