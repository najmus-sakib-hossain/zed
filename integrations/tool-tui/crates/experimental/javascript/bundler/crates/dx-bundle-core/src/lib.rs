//! DX Bundle Core - High-performance JavaScript bundler infrastructure
//!
//! This crate provides the foundation for 3x faster bundling than Bun through:
//!
//! - **Arena allocator**: Zero-allocation transforms using thread-local arenas
//! - **Binary representations**: Core types optimized for memory efficiency
//! - **Parallel processing**: Thread-local arenas for lock-free parallel bundling
//! - **Watch mode**: File system watching with debouncing for development
//! - **Module resolution**: AST-based import/export parsing and Node.js resolution
//!
//! # Architecture
//!
//! The bundler uses a multi-phase architecture:
//!
//! 1. **Scanning**: Fast regex-based import detection
//! 2. **Parsing**: Full AST parsing with OXC
//! 3. **Resolution**: Node.js-compatible module resolution
//! 4. **Transformation**: Arena-allocated AST transforms
//! 5. **Code generation**: Optimized output generation
//!
//! # Examples
//!
//! ```no_run
//! use dx_bundle_core::{BundleConfig, ModuleFormat, Target};
//!
//! let config = BundleConfig {
//!     entries: vec!["src/index.ts".into()],
//!     out_dir: "dist".into(),
//!     format: ModuleFormat::ESM,
//!     target: Target::ES2020,
//!     minify: true,
//!     ..Default::default()
//! };
//! ```
//!
//! # Watch Mode
//!
//! ```no_run
//! use dx_bundle_core::{FileWatcher, WatchConfig};
//!
//! let mut watcher = FileWatcher::new(WatchConfig::default()).unwrap();
//! watcher.watch(std::path::Path::new("src")).unwrap();
//!
//! // Wait for changes
//! let changed_files = watcher.wait_for_changes();
//! println!("Changed: {:?}", changed_files);
//! ```

#![allow(unsafe_code)]

pub mod arena;
pub mod code_split;
pub mod config;
pub mod css;
pub mod error;
pub mod hash;
pub mod resolve;
pub mod tree_shake;
pub mod types;
pub mod watch;

pub use arena::{with_arena, ArenaOutput, BundleArena};
pub use code_split::{ChunkInfo, CodeSplitStats, CodeSplitter, DynamicImportInfo};
pub use config::{BundleConfig, ModuleFormat, Target};
pub use css::{AssetReference, CssBundleOutput, CssBundler, CssImport, CssModuleExport};
pub use error::BundleError;
pub use hash::ContentHash;
pub use resolve::{
    parse_module, ImportedName, ModuleParseResult, ModuleResolver, PackageJson, ParsedExport,
    ParsedImport, ResolveConditions,
};
pub use tree_shake::{ModuleInfo, TreeShakeStats, TreeShakenModule, TreeShaker};
pub use types::*;
pub use watch::{FileWatcher, HmrServer, HmrUpdate, WatchConfig, WatchError};
