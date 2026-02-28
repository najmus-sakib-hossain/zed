//! dx-py-package-manager: High-performance Python package manager
//!
//! This crate provides binary package operations including DPP format handling,
//! dependency resolution, and zero-copy installation.

pub mod build;
pub mod cache;
pub mod converter;
pub mod download;
pub mod formats;
pub mod installer;
pub mod publish;
pub mod registry;
pub mod resolver;

pub use build::{BuildEnvironment, BuildFrontend, DEFAULT_BUILD_BACKEND, DEFAULT_BUILD_REQUIRES};
pub use cache::GlobalCache;
pub use converter::{
    BytecodeCompiler, BytecodeLoadError, BytecodeLoader, CompiledBytecode, CoverageStats,
    DppBuilder, DppBytecodeEntry, DppNativeEntry, FallbackBehavior, LoadedBytecode,
    MultiVersionStore, NativeExtension, NativePackager, PlatformTag, PythonVersion,
    SelectionResult, SourceMarker, ValidationResult, VersionSelector, WheelFile,
};
pub use download::{
    compute_sha256, verify_sha256, DownloadManager, DownloadRequest, DownloadResult,
    FileDigestsInfo, PackageInfoDetails, PackageMetadata, PlatformEnvironment, PyPiDownloader,
    ReleaseFileInfo, WheelTag, PYPI_BASE_URL,
};
pub use dx_py_core::{Error, Result};
pub use formats::{DplBuilder, DplLockFile, DppPackage};
pub use installer::{
    DistInfoMetadata, EditableInstall, EditableInstaller, InstallFile, InstallPackage,
    InstallResult, InstallStrategy, InstalledPackage, Installer, RecordEntry, WheelInstaller,
};
pub use publish::{PublishClient, UploadResult, DEFAULT_REPOSITORY_URL, TEST_PYPI_URL};
pub use registry::{
    AsyncPyPiClient, CredentialProvider, DependencySpec, EnvironmentCredentialProvider,
    FileDigests, NetrcCredentialProvider, PackageInfo, PyPiClient, PyPiPackageInfo, RegistryConfig,
    RegistryCredentials, RegistryManager, ReleaseFile, SslConfig,
};
pub use resolver::{
    compare_versions_pep440, is_prerelease, normalize_package_name, parse_dependency_with_extras,
    CircularDependencyDetector, ConflictExplanation, CycleHandling, Dependency, DependencyCycle,
    DependencyGraph, ExtrasResolver, HintCache, InMemoryProvider, PackageWithExtras,
    PreReleaseFilter, PreReleasePolicy, PubGrubResolver, PyPiResolver, Resolution,
    ResolutionSnapshot, ResolvedPackage, Resolver, SemanticVersion, VersionConstraint,
    VersionProvider,
};
