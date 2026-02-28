use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub struct ChunkData {
    pub hash: blake3::Hash,
    pub offset: usize,
    pub length: usize,
    pub data: Vec<u8>,
}

impl Display for ChunkData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({} bytes @ {})",
            self.hash.to_hex(),
            self.length,
            self.offset
        )
    }
}
