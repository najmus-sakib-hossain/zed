//! Fast tarball extraction for dx-pkg

pub mod direct;
pub mod simd;

pub use direct::DirectExtractor;
pub use simd::{FastExtractor, ParallelExtractor, SimdGzipDecompressor};
