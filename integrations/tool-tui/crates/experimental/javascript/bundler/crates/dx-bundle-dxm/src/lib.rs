//! DX Module Binary Format (.dxm)
//!
//! Pre-compiled binary representation of JavaScript modules.
//! Zero-parse bundling through binary fusion.

mod atomizer;
mod format;
mod fusion;

pub use atomizer::*;
pub use format::*;
pub use fusion::*;
