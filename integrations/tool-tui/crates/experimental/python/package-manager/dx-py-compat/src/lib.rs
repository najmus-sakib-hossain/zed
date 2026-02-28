//! dx-py-compat: Compatibility layer for Python project formats
//!
//! This crate provides:
//! - pyproject.toml parsing and serialization
//! - Binary pyproject.dx format conversion
//! - Round-trip conversion between formats
//! - PEP 508 environment marker evaluation
//! - Configuration system with layered loading
//! - Lock file compatibility (uv.lock, poetry.lock, Pipfile.lock, requirements.txt)

pub mod config;
pub mod lockfiles;
pub mod markers;
pub mod pyproject;

pub use config::{Config, ConfigFile, DEFAULT_INDEX_URL, ENV_PREFIX};
pub use dx_py_core::{Error, Result};
pub use lockfiles::{
    load_lock_file, save_lock_file, LockFile, LockFileFormat, LockMetadata, LockedDependency,
    LockedPackage, PackageSource, PipfileLockFormat, PoetryLockFormat, RequirementsTxtFormat,
    UvLockFormat,
};
pub use markers::{
    CompareOp, MarkerEnvironment, MarkerEvaluator, MarkerExpr, MarkerParseError, MarkerValue,
    MarkerVar,
};
pub use pyproject::{
    convert_from_binary, convert_to_binary, BuildSystem, ProjectSection, PyProjectToml,
};
