//! Chunk Streamer
//!
//! Streaming delivery of rule updates in chunks.

use crate::Result;

use super::MAX_CHUNK_SIZE;

/// Stream chunk
#[derive(Debug, Clone)]
pub struct StreamChunk {
    /// Chunk sequence number
    pub sequence: u32,
    /// Chunk flags
    pub flags: ChunkFlags,
    /// Chunk data
    pub data: Vec<u8>,
}

/// Chunk flags
#[derive(Debug, Clone, Copy, Default)]
pub struct ChunkFlags(u8);

impl ChunkFlags {
    /// First chunk in stream
    pub const FIRST: u8 = 1 << 0;
    /// Last chunk in stream
    pub const LAST: u8 = 1 << 1;
    /// Chunk is compressed
    pub const COMPRESSED: u8 = 1 << 2;
    /// Chunk is encrypted
    pub const ENCRYPTED: u8 = 1 << 3;

    pub fn new() -> Self {
        Self(0)
    }

    pub fn is_first(self) -> bool {
        self.0 & Self::FIRST != 0
    }

    pub fn is_last(self) -> bool {
        self.0 & Self::LAST != 0
    }

    pub fn is_compressed(self) -> bool {
        self.0 & Self::COMPRESSED != 0
    }

    pub fn set(&mut self, flag: u8) {
        self.0 |= flag;
    }
}

impl StreamChunk {
    /// Create a new chunk
    pub fn new(sequence: u32, flags: ChunkFlags, data: Vec<u8>) -> Self {
        Self {
            sequence,
            flags,
            data,
        }
    }

    /// Serialize to bytes
    pub fn serialize(&self) -> Vec<u8> {
        let mut output = Vec::with_capacity(8 + self.data.len());
        output.extend_from_slice(&self.sequence.to_le_bytes());
        output.push(self.flags.0);
        output.extend_from_slice(&[0, 0, 0]); // Reserved/padding
        output.extend_from_slice(&self.data);
        output
    }

    /// Deserialize from bytes
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(crate::DrivenError::InvalidBinary("Chunk too small".into()));
        }

        let sequence = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let flags = ChunkFlags(data[4]);
        let chunk_data = data[8..].to_vec();

        Ok(Self {
            sequence,
            flags,
            data: chunk_data,
        })
    }
}

/// Chunk streamer for breaking large payloads into chunks
#[derive(Debug)]
pub struct ChunkStreamer {
    /// Maximum chunk size
    chunk_size: usize,
    /// Current sequence number
    sequence: u32,
}

impl ChunkStreamer {
    /// Create a new streamer
    pub fn new(chunk_size: usize) -> Self {
        Self {
            chunk_size: chunk_size.min(MAX_CHUNK_SIZE),
            sequence: 0,
        }
    }

    /// Split data into chunks
    pub fn chunk(&mut self, data: &[u8]) -> Vec<StreamChunk> {
        let mut chunks = Vec::new();
        let total_chunks = data.len().div_ceil(self.chunk_size);

        for (i, chunk_data) in data.chunks(self.chunk_size).enumerate() {
            let mut flags = ChunkFlags::new();
            if i == 0 {
                flags.set(ChunkFlags::FIRST);
            }
            if i == total_chunks - 1 {
                flags.set(ChunkFlags::LAST);
            }

            chunks.push(StreamChunk::new(self.sequence, flags, chunk_data.to_vec()));
            self.sequence += 1;
        }

        chunks
    }

    /// Reassemble chunks into data
    pub fn reassemble(&self, chunks: &[StreamChunk]) -> Result<Vec<u8>> {
        let mut result = Vec::new();

        // Sort by sequence
        let mut sorted: Vec<_> = chunks.iter().collect();
        sorted.sort_by_key(|c| c.sequence);

        for chunk in sorted {
            result.extend_from_slice(&chunk.data);
        }

        Ok(result)
    }

    /// Reset sequence number
    pub fn reset(&mut self) {
        self.sequence = 0;
    }
}

impl Default for ChunkStreamer {
    fn default() -> Self {
        Self::new(MAX_CHUNK_SIZE)
    }
}

/// Streaming receiver for accumulating chunks
#[derive(Debug)]
pub struct StreamReceiver {
    /// Accumulated chunks
    chunks: Vec<StreamChunk>,
    /// Expected next sequence
    next_sequence: u32,
    /// Complete flag
    complete: bool,
}

impl StreamReceiver {
    /// Create a new receiver
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            next_sequence: 0,
            complete: false,
        }
    }

    /// Receive a chunk
    pub fn receive(&mut self, chunk: StreamChunk) -> bool {
        if chunk.flags.is_first() && chunk.sequence == 0 {
            self.chunks.clear();
            self.next_sequence = 0;
        }

        self.chunks.push(chunk.clone());

        if chunk.flags.is_last() {
            self.complete = true;
        }

        self.complete
    }

    /// Check if complete
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// Get accumulated data
    pub fn data(&self) -> Result<Vec<u8>> {
        ChunkStreamer::default().reassemble(&self.chunks)
    }

    /// Reset receiver
    pub fn reset(&mut self) {
        self.chunks.clear();
        self.next_sequence = 0;
        self.complete = false;
    }
}

impl Default for StreamReceiver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_roundtrip() {
        let mut flags = ChunkFlags::new();
        flags.set(ChunkFlags::FIRST);
        flags.set(ChunkFlags::LAST);

        let chunk = StreamChunk::new(42, flags, vec![1, 2, 3, 4]);
        let bytes = chunk.serialize();
        let parsed = StreamChunk::deserialize(&bytes).unwrap();

        assert_eq!(parsed.sequence, 42);
        assert!(parsed.flags.is_first());
        assert!(parsed.flags.is_last());
        assert_eq!(parsed.data, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_streamer() {
        let mut streamer = ChunkStreamer::new(10);
        let data = b"Hello, World! This is a test of chunking.";

        let chunks = streamer.chunk(data);
        assert!(chunks.len() > 1);
        assert!(chunks[0].flags.is_first());
        assert!(chunks.last().unwrap().flags.is_last());

        let reassembled = streamer.reassemble(&chunks).unwrap();
        assert_eq!(reassembled, data);
    }

    #[test]
    fn test_receiver() {
        let mut streamer = ChunkStreamer::new(10);
        let data = b"Test data for streaming";

        let chunks = streamer.chunk(data);

        let mut receiver = StreamReceiver::new();
        for chunk in chunks {
            receiver.receive(chunk);
        }

        assert!(receiver.is_complete());
        assert_eq!(receiver.data().unwrap(), data);
    }
}
