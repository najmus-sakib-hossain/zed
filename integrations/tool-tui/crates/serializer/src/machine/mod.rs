//! DX-Machine: Pure RKYV + LZ4 Implementation
//!
//! Binary serialization format using RKYV for zero-copy deserialization
//! and LZ4 for fast compression.
//!
//! ## Format
//! - **RKYV**: Zero-copy binary serialization
//! - **LZ4**: Fast compression (enabled by default with `compression` feature)
//! - **Arena-based**: Flattened value storage to avoid recursive types
//!
//! ## Features
//! - Serializable & deserializable with full round-trip support
//! - 41-74% smaller than LLM text format
//! - Zero-copy deserialization for maximum performance
//! - LZ4 compression for reduced wire size
//!
//! ## Usage
//! ```rust,ignore
//! use serializer::{llm_to_machine, machine_to_document};
//!
//! // Convert LLM format to machine format
//! let machine = llm_to_machine(llm_text)?;
//!
//! // Deserialize back to document
//! let doc = machine_to_document(&machine)?;
//! ```
//! - **DX-SIMD512**: AVX-512 operations
//!
//! These are preserved for future use but not currently active to keep the
//! implementation simple and focused on pure RKYV.

// Core modules (RKYV-based implementation)
pub mod api;
pub mod builder;
pub mod footer;
pub mod format;
pub mod header;
pub mod machine_types;
pub mod rkyv_compat;
pub mod serde_compat;
pub mod simd;
pub mod slot;
pub mod traits;
pub mod types;

// COMMENTED OUT - Advanced optimization modules (available but not default)
// pub mod optimized_rkyv;  // Adaptive RKYV wrapper with DX features
// pub mod arena;
// pub mod arena_batch;
pub mod compress;
// pub mod direct_io;
// pub mod inline;
// pub mod intern;
// pub mod mmap;
// #[cfg(feature = "parallel")]
// pub mod parallel;
// pub mod prefetch;
// pub mod quantum;
// pub mod simd512;
// pub mod static_ser;
// pub mod blocking;
// #[cfg(target_os = "linux")]
// pub mod io_uring;
// #[cfg(target_os = "windows")]
// pub mod iocp;
// #[cfg(target_os = "macos")]
// pub mod kqueue;

// COMMENTED OUT - Async file I/O trait (for optimized_rkyv)
// /// Unified async file I/O trait
// ///
// /// Provides a platform-agnostic interface for file operations.
// /// Implementations use the best available platform-specific API.
// pub trait AsyncFileIO: Send + Sync {
//     /// Read entire file synchronously (blocking)
//     fn read_sync(&self, path: &std::path::Path) -> std::io::Result<Vec<u8>>;
//
//     /// Write entire file synchronously (blocking)
//     fn write_sync(&self, path: &std::path::Path, data: &[u8]) -> std::io::Result<()>;
//
//     /// Read multiple files in batch
//     fn read_batch_sync(
//         &self,
//         paths: &[&std::path::Path],
//     ) -> std::io::Result<Vec<std::io::Result<Vec<u8>>>>;
//
//     /// Write multiple files in batch
//     fn write_batch_sync(
//         &self,
//         files: &[(&std::path::Path, &[u8])],
//     ) -> std::io::Result<Vec<std::io::Result<()>>>;
//
//     /// Get the name of the I/O backend
//     fn backend_name(&self) -> &'static str;
//
//     /// Check if this backend is available on the current platform
//     fn is_available(&self) -> bool;
// }

// Property tests (COMMENTED OUT - use advanced features)
// #[cfg(test)]
// mod io_props;
// #[cfg(test)]
// mod machine_props;
// #[cfg(test)]
// mod safe_deserialize_props;

// Core exports (RKYV-based)
pub use api::{deserialize, deserialize_batch, serialize, serialize_batch};
pub use builder::DxMachineBuilder;
pub use format::{DxFormat, FormatMode, detect_format, parse_auto};
pub use header::{
    DxMachineHeader, FLAG_HAS_HEAP, FLAG_HAS_INTERN, FLAG_HAS_LENGTH_TABLE, FLAG_LITTLE_ENDIAN,
};
pub use serde_compat::{from_bytes as from_bytes_serde, to_bytes as to_bytes_serde};
pub use slot::{DxMachineSlot, HEAP_MARKER, INLINE_MARKER, MAX_INLINE_SIZE};
pub use traits::{DxMachineDeserialize, DxMachineSerialize};
pub use types::DxMachineError;

// COMMENTED OUT - Advanced optimization exports (available but not default)
// pub use optimized_rkyv::OptimizedRkyv;
// #[cfg(feature = "arena")]
// pub use optimized_rkyv::ArenaRkyv;
// #[cfg(feature = "compression")]
// pub use optimized_rkyv::CompressedRkyv;
// #[cfg(feature = "mmap")]
// pub use optimized_rkyv::MmapRkyv;
// pub use arena::{DxArena, DxArenaPool, DxBatchBuilder};
// pub use arena_batch::{DxArenaBatch, DxDeserialize, DxSerialize};
pub use compress::{CompressionLevel, DxCompressed};
// pub use direct_io::{
//     AlignedBuffer, DirectIoConfig, DirectIoWriter, DEFAULT_ALIGNMENT, DEFAULT_DIRECT_IO_THRESHOLD,
// };
// pub use footer::{
//     compute_crc16, deserialize_with_footer, extract_data, DxFooter, FooterError, FOOTER_MAGIC,
//     FOOTER_SIZE, FOOTER_VERSION,
// };
// pub use inline::{DxInlineBytes, DxInlineString, MAX_INLINE_BYTES, MAX_INLINE_STRING};
// pub use intern::{InternError, InternPool, InterningDeserializer, InterningSerializer};
// pub use mmap::{DxMmap, DxMmapBatch};
// pub use prefetch::{prefetch, prefetch_lines, prefetch_range, PrefetchHint, PrefetchProcessor};
// pub use quantum::{QuantumLayout, QuantumReader, QuantumType, QuantumWriter};
// pub use simd512::runtime::{detect_simd_level, has_avx2, has_avx512, has_sse42, SimdLevel};

/// DX-Machine magic bytes: 0x5A 0x44 ("ZD" little-endian)
pub const MAGIC: [u8; 2] = [0x5A, 0x44];

/// DX-Machine format version
pub const VERSION: u8 = 0x01;

/// Slot size in bytes (16 bytes for inline optimization)
pub const SLOT_SIZE: usize = 16;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic_bytes() {
        assert_eq!(MAGIC, [0x5A, 0x44]);
        assert_eq!(VERSION, 0x01);
    }

    #[test]
    fn test_slot_size() {
        assert_eq!(SLOT_SIZE, 16);
        assert_eq!(MAX_INLINE_SIZE, 14);
    }
}
