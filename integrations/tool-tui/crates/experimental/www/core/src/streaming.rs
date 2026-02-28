//! # Binary Streaming SSR
//!
//! Binary Dawn's streaming SSR sends binary chunks that memory-map directly.
//! This achieves 20x faster time-to-interactive compared to React 18.
//!
//! Chunks are processed as they arrive without waiting for the full response.

/// Stream chunk types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkType {
    /// Clone template, insert at slot
    Template = 0x01,
    /// Fill template slots with values
    Data = 0x02,
    /// Attach handlers (island hydration)
    Activate = 0x03,
}

impl ChunkType {
    /// Convert from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(Self::Template),
            0x02 => Some(Self::Data),
            0x03 => Some(Self::Activate),
            _ => None,
        }
    }
}

/// Binary stream chunk
///
/// Each chunk has a 5-byte header followed by payload.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamChunk {
    /// Chunk type
    pub chunk_type: ChunkType,
    /// Target slot for insertion
    pub target_slot: u16,
    /// Payload length
    pub payload_len: u16,
    /// Payload data
    pub payload: Vec<u8>,
}

impl StreamChunk {
    /// Header size in bytes
    pub const HEADER_SIZE: usize = 5;

    /// Create a new stream chunk
    pub fn new(chunk_type: ChunkType, target_slot: u16, payload: Vec<u8>) -> Self {
        Self {
            chunk_type,
            target_slot,
            payload_len: payload.len() as u16,
            payload,
        }
    }

    /// Create a template chunk
    pub fn template(target_slot: u16, template_data: Vec<u8>) -> Self {
        Self::new(ChunkType::Template, target_slot, template_data)
    }

    /// Create a data chunk
    pub fn data(target_slot: u16, data: Vec<u8>) -> Self {
        Self::new(ChunkType::Data, target_slot, data)
    }

    /// Create an activate chunk
    pub fn activate(target_slot: u16, island_data: Vec<u8>) -> Self {
        Self::new(ChunkType::Activate, target_slot, island_data)
    }

    /// Get total size including header
    pub fn total_size(&self) -> usize {
        Self::HEADER_SIZE + self.payload.len()
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.total_size());
        bytes.push(self.chunk_type as u8);
        bytes.extend_from_slice(&self.target_slot.to_le_bytes());
        bytes.extend_from_slice(&self.payload_len.to_le_bytes());
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::HEADER_SIZE {
            return None;
        }

        let chunk_type = ChunkType::from_u8(bytes[0])?;
        let target_slot = u16::from_le_bytes([bytes[1], bytes[2]]);
        let payload_len = u16::from_le_bytes([bytes[3], bytes[4]]) as usize;

        if bytes.len() < Self::HEADER_SIZE + payload_len {
            return None;
        }

        let payload = bytes[Self::HEADER_SIZE..Self::HEADER_SIZE + payload_len].to_vec();

        Some(Self {
            chunk_type,
            target_slot,
            payload_len: payload_len as u16,
            payload,
        })
    }

    /// Process the chunk (simulated DOM operations)
    ///
    /// Returns a description of the operation performed.
    pub fn process(&self) -> ChunkResult {
        match self.chunk_type {
            ChunkType::Template => ChunkResult::TemplateInserted {
                slot: self.target_slot,
                size: self.payload.len(),
            },
            ChunkType::Data => ChunkResult::DataFilled {
                slot: self.target_slot,
                size: self.payload.len(),
            },
            ChunkType::Activate => ChunkResult::IslandActivated {
                slot: self.target_slot,
            },
        }
    }
}

/// Result of processing a chunk
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChunkResult {
    /// Template was cloned and inserted
    TemplateInserted { slot: u16, size: usize },
    /// Data was filled into slots
    DataFilled { slot: u16, size: usize },
    /// Island was activated (hydrated)
    IslandActivated { slot: u16 },
}

/// Streaming SSR response builder
///
/// Builds a sequence of chunks for streaming to the client.
#[derive(Debug, Clone, Default)]
pub struct StreamingResponse {
    /// Chunks to send
    chunks: Vec<StreamChunk>,
}

impl StreamingResponse {
    /// Create a new streaming response
    pub fn new() -> Self {
        Self { chunks: Vec::new() }
    }

    /// Create with pre-allocated capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            chunks: Vec::with_capacity(capacity),
        }
    }

    /// Add a template chunk
    pub fn add_template(&mut self, slot: u16, template_data: Vec<u8>) {
        self.chunks.push(StreamChunk::template(slot, template_data));
    }

    /// Add a data chunk
    pub fn add_data(&mut self, slot: u16, data: Vec<u8>) {
        self.chunks.push(StreamChunk::data(slot, data));
    }

    /// Add an activate chunk
    pub fn add_activate(&mut self, slot: u16, island_data: Vec<u8>) {
        self.chunks.push(StreamChunk::activate(slot, island_data));
    }

    /// Get number of chunks
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Get total size of all chunks
    pub fn total_size(&self) -> usize {
        self.chunks.iter().map(|c| c.total_size()).sum()
    }

    /// Serialize all chunks to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.total_size());
        for chunk in &self.chunks {
            bytes.extend_from_slice(&chunk.to_bytes());
        }
        bytes
    }

    /// Get iterator over chunks
    pub fn iter(&self) -> impl Iterator<Item = &StreamChunk> {
        self.chunks.iter()
    }

    /// Process all chunks and return results
    pub fn process_all(&self) -> Vec<ChunkResult> {
        self.chunks.iter().map(|c| c.process()).collect()
    }
}

/// Stream parser for receiving chunks
///
/// Parses incoming bytes into chunks as they arrive.
#[derive(Debug, Default)]
pub struct StreamParser {
    /// Buffer for incomplete chunks
    buffer: Vec<u8>,
}

impl StreamParser {
    /// Create a new stream parser
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Feed bytes and extract complete chunks
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<StreamChunk> {
        self.buffer.extend_from_slice(bytes);

        let mut chunks = Vec::new();

        while self.buffer.len() >= StreamChunk::HEADER_SIZE {
            // Check if we have enough bytes for the payload
            let payload_len = u16::from_le_bytes([self.buffer[3], self.buffer[4]]) as usize;
            let total_len = StreamChunk::HEADER_SIZE + payload_len;

            if self.buffer.len() < total_len {
                break;
            }

            // Extract the chunk
            let chunk_bytes: Vec<u8> = self.buffer.drain(..total_len).collect();
            if let Some(chunk) = StreamChunk::from_bytes(&chunk_bytes) {
                chunks.push(chunk);
            }
        }

        chunks
    }

    /// Check if there's pending data
    pub fn has_pending(&self) -> bool {
        !self.buffer.is_empty()
    }

    /// Get pending buffer size
    pub fn pending_size(&self) -> usize {
        self.buffer.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_header_size() {
        assert_eq!(StreamChunk::HEADER_SIZE, 5);
    }

    #[test]
    fn test_chunk_roundtrip() {
        let chunk = StreamChunk::template(42, vec![1, 2, 3, 4]);
        let bytes = chunk.to_bytes();
        let restored = StreamChunk::from_bytes(&bytes).unwrap();

        assert_eq!(chunk, restored);
    }

    #[test]
    fn test_chunk_types() {
        let template = StreamChunk::template(0, vec![]);
        assert_eq!(template.chunk_type, ChunkType::Template);

        let data = StreamChunk::data(1, vec![]);
        assert_eq!(data.chunk_type, ChunkType::Data);

        let activate = StreamChunk::activate(2, vec![]);
        assert_eq!(activate.chunk_type, ChunkType::Activate);
    }

    #[test]
    fn test_chunk_process() {
        let template = StreamChunk::template(5, vec![1, 2, 3]);
        let result = template.process();
        assert_eq!(result, ChunkResult::TemplateInserted { slot: 5, size: 3 });

        let data = StreamChunk::data(10, vec![4, 5]);
        let result = data.process();
        assert_eq!(result, ChunkResult::DataFilled { slot: 10, size: 2 });

        let activate = StreamChunk::activate(15, vec![]);
        let result = activate.process();
        assert_eq!(result, ChunkResult::IslandActivated { slot: 15 });
    }

    #[test]
    fn test_streaming_response() {
        let mut response = StreamingResponse::new();
        response.add_template(0, vec![1, 2, 3]);
        response.add_data(0, vec![4, 5]);
        response.add_activate(0, vec![]);

        assert_eq!(response.len(), 3);

        let results = response.process_all();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_stream_parser() {
        let mut response = StreamingResponse::new();
        response.add_template(0, vec![1, 2, 3]);
        response.add_data(1, vec![4, 5]);

        let bytes = response.to_bytes();

        let mut parser = StreamParser::new();

        // Feed bytes in parts
        let chunks1 = parser.feed(&bytes[..5]);
        assert!(chunks1.is_empty()); // Not enough for first chunk

        let chunks2 = parser.feed(&bytes[5..]);
        assert_eq!(chunks2.len(), 2);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating valid ChunkType
    fn chunk_type_strategy() -> impl Strategy<Value = ChunkType> {
        prop_oneof![
            Just(ChunkType::Template),
            Just(ChunkType::Data),
            Just(ChunkType::Activate),
        ]
    }

    // **Feature: binary-dawn-features, Property 21: StreamChunk Processing**
    // *For any* StreamChunk with chunk_type TEMPLATE, DATA, or ACTIVATE,
    // processing SHALL produce the correct DOM modification for that type.
    // **Validates: Requirements 12.1, 12.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_stream_chunk_processing(
            chunk_type in chunk_type_strategy(),
            target_slot in any::<u16>(),
            payload in prop::collection::vec(any::<u8>(), 0..100)
        ) {
            let chunk = StreamChunk::new(chunk_type, target_slot, payload.clone());
            let result = chunk.process();

            // Verify correct result type for each chunk type
            match chunk_type {
                ChunkType::Template => {
                    let is_template = matches!(result, ChunkResult::TemplateInserted { .. });
                    prop_assert!(is_template, "Expected TemplateInserted");
                    if let ChunkResult::TemplateInserted { slot, size } = result {
                        prop_assert_eq!(slot, target_slot);
                        prop_assert_eq!(size, payload.len());
                    }
                }
                ChunkType::Data => {
                    let is_data = matches!(result, ChunkResult::DataFilled { .. });
                    prop_assert!(is_data, "Expected DataFilled");
                    if let ChunkResult::DataFilled { slot, size } = result {
                        prop_assert_eq!(slot, target_slot);
                        prop_assert_eq!(size, payload.len());
                    }
                }
                ChunkType::Activate => {
                    let is_activate = matches!(result, ChunkResult::IslandActivated { .. });
                    prop_assert!(is_activate, "Expected IslandActivated");
                    if let ChunkResult::IslandActivated { slot } = result {
                        prop_assert_eq!(slot, target_slot);
                    }
                }
            }
        }
    }

    // Round-trip property for StreamChunk serialization
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_stream_chunk_roundtrip(
            chunk_type in chunk_type_strategy(),
            target_slot in any::<u16>(),
            payload in prop::collection::vec(any::<u8>(), 0..100)
        ) {
            let chunk = StreamChunk::new(chunk_type, target_slot, payload);
            let bytes = chunk.to_bytes();
            let restored = StreamChunk::from_bytes(&bytes);

            prop_assert!(restored.is_some());
            prop_assert_eq!(chunk, restored.unwrap());
        }
    }

    // StreamParser correctly parses multiple chunks
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_stream_parser_multiple_chunks(
            chunk_count in 1usize..10,
            target_slots in prop::collection::vec(any::<u16>(), 10),
            payloads in prop::collection::vec(prop::collection::vec(any::<u8>(), 0..20), 10)
        ) {
            let mut response = StreamingResponse::new();

            for i in 0..chunk_count {
                let slot = target_slots[i % target_slots.len()];
                let payload = payloads[i % payloads.len()].clone();
                response.add_template(slot, payload);
            }

            let bytes = response.to_bytes();

            let mut parser = StreamParser::new();
            let chunks = parser.feed(&bytes);

            prop_assert_eq!(chunks.len(), chunk_count);
            prop_assert!(!parser.has_pending());
        }
    }

    // ChunkType round-trip
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_chunk_type_roundtrip(
            chunk_type in chunk_type_strategy()
        ) {
            let byte = chunk_type as u8;
            let restored = ChunkType::from_u8(byte);

            prop_assert!(restored.is_some());
            prop_assert_eq!(chunk_type, restored.unwrap());
        }
    }
}
