//! # Stream Module - The Binary Streamer (Day 16)
//!
//! **The Waterfall Killer**
//!
//! Traditional web loading is sequential:
//! 1. Download HTML → 2. Parse HTML → 3. Download JS → 4. Parse JS → 5. Execute
//!
//! Dx-www streaming is parallel:
//! - Chunk 1 (Layout) → WASM creates templates **while downloading**
//! - Chunk 2 (State) → Memory allocated **while downloading**
//! - Chunk 3 (Logic) → Browser compiles **while downloading**
//!
//! Result: Zero blocking time. Execution starts before download completes.

use bytes::Bytes;
use dx_www_packet::{ChunkHeader, ChunkType, DxbArtifact};
use futures::stream::{self, Stream};
use std::pin::Pin;

/// The Binary Stream Generator
///
/// Creates an async stream that yields chunks in optimal order:
/// 1. Header (64 bytes) - Magic + Version + Signature
/// 2. Layout (templates) - Client starts DOM prep immediately
/// 3. State (initial data) - Client allocates memory
/// 4. WASM (logic) - Client compiles in background
/// 5. EOF marker
pub fn create_stream(
    artifact: &DxbArtifact,
    layout_bin: Vec<u8>,
    wasm_bin: Vec<u8>,
) -> Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>> {
    let chunks = build_chunks(artifact, layout_bin, wasm_bin);

    Box::pin(stream::iter(chunks.into_iter().map(Ok)))
}

/// Build the complete chunk sequence
fn build_chunks(artifact: &DxbArtifact, layout_bin: Vec<u8>, wasm_bin: Vec<u8>) -> Vec<Bytes> {
    let mut chunks = Vec::new();

    // Chunk 0: Header (Magic + Version + Signature)
    chunks.push(create_header_chunk(artifact));

    // Chunk 1: Layout (Templates)
    chunks.push(create_layout_chunk(layout_bin));

    // Chunk 2: State (Initial Memory Snapshot)
    chunks.push(create_state_chunk());

    // Chunk 3: WASM (Runtime Logic)
    chunks.push(create_wasm_chunk(wasm_bin));

    // Chunk 4: EOF
    chunks.push(create_eof_chunk());

    chunks
}

/// Create Header Chunk (64 bytes fixed)
///
/// Format:
/// - [0..4]: Magic bytes "DX\0\0"
/// - [4..5]: Version
/// - [5..64]: Signature (placeholder for crypto signature)
fn create_header_chunk(artifact: &DxbArtifact) -> Bytes {
    let mut header_data = vec![0u8; 64];

    // Magic bytes
    header_data[0..4].copy_from_slice(b"DX\0\0");

    // Version
    header_data[4] = artifact.version;

    // Signature (first 59 bytes of artifact signature)
    let sig_len = artifact.capabilities.signature.len().min(59);
    header_data[5..5 + sig_len].copy_from_slice(&artifact.capabilities.signature[..sig_len]);

    // Wrap with ChunkHeader
    wrap_chunk(ChunkType::Header, header_data)
}

/// Create Layout Chunk (Templates Binary)
///
/// Contains serialized template dictionary
/// Client action: Immediately create <template> tags in DOM
fn create_layout_chunk(layout_bin: Vec<u8>) -> Bytes {
    wrap_chunk(ChunkType::Layout, layout_bin)
}

/// Create State Chunk (Initial State Data)
///
/// Contains initial state snapshot
/// Client action: Allocate SharedArrayBuffer and populate
fn create_state_chunk() -> Bytes {
    // For now, empty state (future: serialize initial component state)
    let state_data = vec![0u8; 0];
    wrap_chunk(ChunkType::State, state_data)
}

/// Create WASM Chunk (Runtime Logic)
///
/// Contains compiled WASM binary
/// Client action: Streaming compilation via WebAssembly.instantiateStreaming()
fn create_wasm_chunk(wasm_bin: Vec<u8>) -> Bytes {
    wrap_chunk(ChunkType::Wasm, wasm_bin)
}

/// Create EOF Chunk (End of Stream)
fn create_eof_chunk() -> Bytes {
    wrap_chunk(ChunkType::Eof, vec![])
}

/// Wrap data with ChunkHeader
///
/// Format:
/// - [0]: chunk_type (u8)
/// - [1..5]: length (u32 little-endian)
/// - [5..]: data
fn wrap_chunk(chunk_type: ChunkType, data: Vec<u8>) -> Bytes {
    let header = ChunkHeader::new(chunk_type, data.len() as u32);
    let header_bytes = header.to_bytes();

    let mut chunk = Vec::with_capacity(5 + data.len());
    chunk.extend_from_slice(&header_bytes);
    chunk.extend_from_slice(&data);

    Bytes::from(chunk)
}

/// Calculate total stream size (for Content-Length header)
pub fn calculate_stream_size(layout_size: usize, wasm_size: usize) -> usize {
    // Header: 5 (chunk header) + 64 (data) = 69
    // Layout: 5 (chunk header) + layout_size
    // State: 5 (chunk header) + 0 (empty for now)
    // WASM: 5 (chunk header) + wasm_size
    // EOF: 5 (chunk header) + 0
    69 + 5 + layout_size + 5 + 5 + wasm_size + 5
}

#[cfg(test)]
mod tests {
    use super::*;
    use dx_www_packet::CapabilitiesManifest;

    #[test]
    fn test_header_chunk_format() {
        let artifact = DxbArtifact {
            version: 1,
            capabilities: CapabilitiesManifest {
                signature: vec![0xAA; 64],
                ..Default::default()
            },
            templates: vec![],
            wasm_size: 0,
        };

        let chunk = create_header_chunk(&artifact);

        // Should be 5 (header) + 64 (data) = 69 bytes
        assert_eq!(chunk.len(), 69);

        // First byte should be Header chunk type
        assert_eq!(chunk[0], ChunkType::Header as u8);

        // Length should be 64 (Little Endian)
        let length = u32::from_le_bytes([chunk[1], chunk[2], chunk[3], chunk[4]]);
        assert_eq!(length, 64);

        // Data should start with magic bytes
        assert_eq!(&chunk[5..9], b"DX\0\0");
    }

    #[test]
    fn test_chunk_wrapping() {
        let data = vec![1, 2, 3, 4, 5];
        let chunk = wrap_chunk(ChunkType::Layout, data.clone());

        // Should be 5 (header) + 5 (data) = 10 bytes
        assert_eq!(chunk.len(), 10);

        // First byte is chunk type
        assert_eq!(chunk[0], ChunkType::Layout as u8);

        // Next 4 bytes are length
        let length = u32::from_le_bytes([chunk[1], chunk[2], chunk[3], chunk[4]]);
        assert_eq!(length, 5);

        // Rest is data
        assert_eq!(&chunk[5..10], &data[..]);
    }

    #[test]
    fn test_stream_size_calculation() {
        let layout_size = 1000;
        let wasm_size = 50000;

        let total = calculate_stream_size(layout_size, wasm_size);

        // Header (69) + Layout (5+1000) + State (5+0) + WASM (5+50000) + EOF (5+0)
        assert_eq!(total, 69 + 1005 + 5 + 50005 + 5);
        assert_eq!(total, 51089);
    }

    #[test]
    fn test_eof_chunk() {
        let chunk = create_eof_chunk();

        // Should be 5 bytes (header only, no data)
        assert_eq!(chunk.len(), 5);
        assert_eq!(chunk[0], ChunkType::Eof as u8);

        let length = u32::from_le_bytes([chunk[1], chunk[2], chunk[3], chunk[4]]);
        assert_eq!(length, 0);
    }
}
