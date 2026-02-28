//! Wheel to DPP converter
//!
//! Converts standard Python wheel files to the DPP binary format.

pub mod bytecode;
pub mod dpp_builder;
pub mod loader;
pub mod native;
pub mod version_selector;
pub mod wheel;

pub use bytecode::{BytecodeCompiler, CompiledBytecode, PythonVersion, SourceMarker};
pub use dpp_builder::{inspect_dpp, DppBuilder, DppBytecodeEntry, DppNativeEntry};
pub use loader::{BytecodeLoadError, BytecodeLoader, LoadedBytecode, ValidationResult};
pub use native::{NativeExtension, NativePackager, PlatformTag};
pub use version_selector::{
    CoverageStats, FallbackBehavior, MultiVersionStore, SelectionResult, VersionSelector,
};
pub use wheel::WheelFile;
