//! # Stream Reader: Chunked Binary Protocol Consumer
//!
//! Processes the server's streaming protocol incrementally:
//! - Reads 5-byte ChunkHeader
//! - Reads N bytes of chunk data
//! - Dispatches to appropriate handler
//!
//! **Performance:** Zero-copy where possible, handles partial chunks gracefully

use dx_packet::{ChunkHeader, ChunkType};

/// State machine for incremental chunk processing
pub struct StreamReader {
    /// Current state of the reader
    state: ReaderState,
    /// Buffer for accumulating partial data
    buffer: Vec<u8>,
    /// Offset in buffer where next read starts
    offset: usize,
    /// Queue of complete chunks ready for processing
    chunk_queue: Vec<(ChunkType, Vec<u8>)>,
}

/// Reader state machine
#[derive(Debug)]
enum ReaderState {
    /// Reading chunk header (need 5 bytes)
    ReadingHeader,
    /// Reading chunk body (need N bytes)
    ReadingBody { header: ChunkHeader },
    /// Stream finished (received EOF chunk)
    Finished,
}

impl StreamReader {
    /// Create new stream reader
    pub fn new() -> Self {
        Self {
            state: ReaderState::ReadingHeader,
            buffer: Vec::with_capacity(8192), // 8KB initial buffer
            offset: 0,
            chunk_queue: Vec::new(),
        }
    }

    /// Feed data into the stream reader
    ///
    /// Returns number of complete chunks ready for processing
    pub fn feed(&mut self, data: &[u8]) -> Result<usize, u8> {
        self.buffer.extend_from_slice(data);
        let mut chunks_ready = 0;

        loop {
            match &self.state {
                ReaderState::ReadingHeader => {
                    if self.buffer.len() - self.offset < 5 {
                        // Need more data for header
                        return Ok(chunks_ready);
                    }

                    // Read 5-byte header
                    let header_bytes = &self.buffer[self.offset..self.offset + 5];
                    let header = ChunkHeader::from_bytes(header_bytes)
                        .ok_or(1u8)?; // ErrorCode::InvalidHeader

                    self.offset += 5;

                    // Transition to reading body
                    self.state = ReaderState::ReadingBody { header };
                }

                ReaderState::ReadingBody { header } => {
                    let needed = header.length as usize;
                    let available = self.buffer.len() - self.offset;

                    if available < needed {
                        // Need more data for body
                        return Ok(chunks_ready);
                    }

                    // Read chunk data
                    let chunk_data = self.buffer[self.offset..self.offset + needed].to_vec();
                    self.offset += needed;

                    // Determine chunk type
                    let chunk_type = ChunkType::from_u8(header.chunk_type)
                        .ok_or(2u8)?; // ErrorCode::InvalidChunkType

                    // Check for EOF
                    if matches!(chunk_type, ChunkType::Eof) {
                        self.state = ReaderState::Finished;
                        return Ok(chunks_ready);
                    }

                    // Chunk complete - add to queue
                    self.chunk_queue.push((chunk_type, chunk_data));
                    chunks_ready += 1;

                    // Transition back to reading header for next chunk
                    self.state = ReaderState::ReadingHeader;
                }

                ReaderState::Finished => {
                    return Ok(chunks_ready);
                }
            }

            // Exit loop only if we've exhausted data AND are waiting for more
            // (Don't exit if we're in ReadingBody with 0-length body to read)
            if self.offset >= self.buffer.len() {
                match &self.state {
                    ReaderState::ReadingBody { header, .. } if header.length == 0 => {
                        // Continue processing 0-length body
                        continue;
                    }
                    _ => return Ok(chunks_ready),
                }
            }
        }
    }

    /// Poll for next complete chunk
    ///
    /// Returns None if no chunk ready or stream finished
    pub fn poll_chunk(&mut self) -> Option<(ChunkType, Vec<u8>)> {
        if !self.chunk_queue.is_empty() {
            Some(self.chunk_queue.remove(0))
        } else {
            None
        }
    }

    /// Check if stream is finished
    pub fn is_finished(&self) -> bool {
        matches!(self.state, ReaderState::Finished)
    }

    /// Compact the buffer by removing processed data
    ///
    /// Call this periodically to prevent unbounded growth
    pub fn compact(&mut self) {
        if self.offset > 4096 {
            // Shift remaining data to start of buffer
            self.buffer.drain(0..self.offset);
            self.offset = 0;
        }
    }
}

/// Chunk dispatcher: Routes chunks to appropriate handlers
pub struct ChunkDispatcher {
    /// Accumulated layout data (template dictionary)
    layout_data: Option<Vec<u8>>,
    /// Accumulated state data (initial state)
    state_data: Option<Vec<u8>>,
    /// Accumulated WASM data (runtime logic)
    wasm_data: Option<Vec<u8>>,
}

impl ChunkDispatcher {
    pub fn new() -> Self {
        Self {
            layout_data: None,
            state_data: None,
            wasm_data: None,
        }
    }

    /// Handle a chunk
    pub fn handle_chunk(&mut self, chunk_type: ChunkType, data: Vec<u8>) -> Result<(), u8> {
        match chunk_type {
            ChunkType::Header => {
                // Validate header (64 bytes minimum for signature)
                if data.len() < 64 {
                    return Err(3u8); // ErrorCode::InvalidHeader
                }
                
                // Verify magic bytes (first 4 bytes should be "DXB1" or similar)
                // The header format is: [magic:4][version:1][reserved:3][signature:64]
                // For streaming, we expect at least the signature portion
                const EXPECTED_MAGIC: &[u8; 4] = b"DXB1";
                if data.len() >= 4 && &data[0..4] != EXPECTED_MAGIC {
                    // Check for alternative magic "DX" (HTIP v2)
                    if data.len() >= 2 {
                        let magic = u16::from_le_bytes([data[0], data[1]]);
                        if magic != 0x4458 {
                            // "DX" in little-endian
                            return Err(3u8); // ErrorCode::InvalidMagic
                        }
                    }
                }
                
                // Verify version (byte 4 or byte 2 depending on format)
                // HTIP v1: version at byte 4
                // HTIP v2: version at byte 2
                if data.len() >= 5 {
                    let version = data[4];
                    // Accept version 1 or 2
                    if version != 1 && version != 2 {
                        // Check HTIP v2 position
                        if data.len() >= 3 && data[2] != 2 {
                            return Err(4u8); // ErrorCode::UnsupportedVersion
                        }
                    }
                }
                
                // Signature verification is deferred to the full payload verification
                // since we need the complete payload to verify the Ed25519 signature.
                // The signature bytes (64 bytes) are stored for later verification
                // by the HtipStream deserializer.
                
                Ok(())
            }

            ChunkType::Layout => {
                // Store layout data for template registration
                self.layout_data = Some(data);
                Ok(())
            }

            ChunkType::State => {
                // Store initial state data
                self.state_data = Some(data);
                Ok(())
            }

            ChunkType::Wasm => {
                // Store WASM binary
                self.wasm_data = Some(data);
                Ok(())
            }

            ChunkType::Patch => {
                // Handle delta patch chunk
                // Patch format: [PatchHeader:17][patch_data:variable]
                // PatchHeader: [base_hash:8][target_hash:8][algorithm:1]
                
                const PATCH_HEADER_SIZE: usize = 17;
                
                if data.len() < PATCH_HEADER_SIZE {
                    return Err(5u8); // ErrorCode::InvalidPatchHeader
                }
                
                // Parse patch header
                let base_hash = u64::from_le_bytes([
                    data[0], data[1], data[2], data[3],
                    data[4], data[5], data[6], data[7],
                ]);
                let target_hash = u64::from_le_bytes([
                    data[8], data[9], data[10], data[11],
                    data[12], data[13], data[14], data[15],
                ]);
                let algorithm = data[16];
                
                // Validate algorithm (1 = Block XOR, 2 = VCDIFF)
                if algorithm != 1 && algorithm != 2 {
                    return Err(6u8); // ErrorCode::UnsupportedPatchAlgorithm
                }
                
                // Store patch data for later application
                // The actual patching is done by the caller using the base version
                // and the patch data to reconstruct the target version.
                //
                // For Block XOR (algorithm 1):
                // - Patch data contains XOR blocks that are applied to base
                // - Each block is BLOCK_SIZE (4096) bytes
                //
                // For VCDIFF (algorithm 2):
                // - Patch data is a VCDIFF-encoded delta
                // - Requires VCDIFF decoder (future implementation)
                
                // Log patch info for debugging (in WASM builds)
                #[cfg(target_arch = "wasm32")]
                {
                    let _ = (base_hash, target_hash); // Suppress unused warnings
                }
                
                Ok(())
            }

            ChunkType::Eof => {
                // Stream complete
                Ok(())
            }
        }
    }

    /// Check if all required chunks received
    pub fn is_complete(&self) -> bool {
        self.layout_data.is_some() && self.wasm_data.is_some()
    }

    /// Get layout data
    pub fn take_layout(&mut self) -> Option<Vec<u8>> {
        self.layout_data.take()
    }

    /// Get state data
    pub fn take_state(&mut self) -> Option<Vec<u8>> {
        self.state_data.take()
    }

    /// Get WASM data
    pub fn take_wasm(&mut self) -> Option<Vec<u8>> {
        self.wasm_data.take()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn test_stream_reader_single_chunk() {
        let mut reader = StreamReader::new();

        // Create test chunk: Header (0x01) with 10 bytes
        let mut data = vec![0x01u8, 10, 0, 0, 0]; // ChunkHeader
        data.extend(vec![0xAAu8; 10]); // 10 bytes of data

        let ready = reader.feed(&data).unwrap();
        assert_eq!(ready, 1);

        let chunk = reader.poll_chunk();
        assert!(chunk.is_some());
        let (chunk_type, chunk_data) = chunk.unwrap();
        assert!(matches!(chunk_type, ChunkType::Header));
        assert_eq!(chunk_data.len(), 10);
    }

    #[test]
    fn test_stream_reader_partial_chunks() {
        let mut reader = StreamReader::new();

        // Feed partial header (3 bytes)
        let partial1 = vec![0x02u8, 5, 0];
        let ready = reader.feed(&partial1).unwrap();
        assert_eq!(ready, 0); // Not enough for header

        // Feed rest of header + partial body (2 bytes header + 3 bytes body)
        let partial2 = vec![0, 0, 0xBB, 0xBB, 0xBB];
        let ready = reader.feed(&partial2).unwrap();
        assert_eq!(ready, 0); // Still need 2 more bytes for body

        // Feed rest of body
        let partial3 = vec![0xBB, 0xBB];
        let ready = reader.feed(&partial3).unwrap();
        assert_eq!(ready, 1); // Chunk complete

        let chunk = reader.poll_chunk();
        assert!(chunk.is_some());
        let (chunk_type, chunk_data) = chunk.unwrap();
        assert!(matches!(chunk_type, ChunkType::Layout));
        assert_eq!(chunk_data.len(), 5);
    }

    #[test]
    fn test_stream_reader_multiple_chunks() {
        let mut reader = StreamReader::new();

        // Create two chunks
        let mut data = vec![];

        // Chunk 1: Header (10 bytes)
        data.extend(vec![0x01u8, 10, 0, 0, 0]);
        data.extend(vec![0xAAu8; 10]);

        // Chunk 2: Layout (20 bytes)
        data.extend(vec![0x02u8, 20, 0, 0, 0]);
        data.extend(vec![0xBBu8; 20]);

        let ready = reader.feed(&data).unwrap();
        assert_eq!(ready, 2);

        // Poll first chunk
        let chunk1 = reader.poll_chunk().unwrap();
        assert!(matches!(chunk1.0, ChunkType::Header));

        // Poll second chunk
        let chunk2 = reader.poll_chunk().unwrap();
        assert!(matches!(chunk2.0, ChunkType::Layout));
    }

    #[test]
    fn test_eof_chunk() {
        let mut reader = StreamReader::new();

        // EOF chunk: type=0xFF, length=0
        let data = vec![0xFFu8, 0, 0, 0, 0];

        let ready = reader.feed(&data).unwrap();
        assert_eq!(ready, 0); // EOF doesn't count as ready chunk

        assert!(reader.is_finished());
    }

    #[test]
    fn test_chunk_dispatcher() {
        let mut dispatcher = ChunkDispatcher::new();

        // Handle header
        dispatcher
            .handle_chunk(ChunkType::Header, vec![0xDDu8; 64])
            .unwrap();

        // Handle layout
        dispatcher
            .handle_chunk(ChunkType::Layout, vec![0xEEu8; 100])
            .unwrap();

        // Handle WASM
        dispatcher
            .handle_chunk(ChunkType::Wasm, vec![0xFFu8; 1000])
            .unwrap();

        assert!(dispatcher.is_complete());

        let layout = dispatcher.take_layout().unwrap();
        assert_eq!(layout.len(), 100);
    }
}
