
# Design Document: DX-Py Hardening & Production Readiness

## Overview

This design document specifies the architecture and implementation details for hardening DX-Py into a production-ready Python package manager. The focus is on replacing placeholder implementations with real functionality, adding robust error handling, and ensuring cross-platform compatibility.

## Architecture

The hardening effort builds on the existing 5-crate architecture: @tree[]

## Components and Interfaces

### 1. PEP 440 Version Parser (dx-py-core)

```rust
/// Full PEP 440 version representation pub struct Pep440Version { pub epoch: u32, pub release: Vec<u32>, pub pre: Option<PreRelease>, pub post: Option<u32>, pub dev: Option<u32>, pub local: Option<String>, }
pub enum PreRelease { Alpha(u32), Beta(u32), ReleaseCandidate(u32), }
impl Pep440Version { pub fn parse(s: &str) -> Result<Self>;
pub fn to_string(&self) -> String;
}
impl Ord for Pep440Version { // PEP 440 ordering: epoch > release > pre > post > dev }
```

### 2. Environment Marker Evaluator (dx-py-compat)

```rust
/// Environment context for marker evaluation pub struct MarkerEnvironment { pub python_version: String, pub python_full_version: String, pub os_name: String, pub sys_platform: String, pub platform_system: String, pub platform_machine: String, pub platform_release: String, pub implementation_name: String, pub implementation_version: String, }
impl MarkerEnvironment { pub fn current() -> Self;
}
/// Marker expression evaluator pub struct MarkerEvaluator;
impl MarkerEvaluator { pub fn parse(marker: &str) -> Result<MarkerExpr>;
pub fn evaluate(expr: &MarkerExpr, env: &MarkerEnvironment, extras: &[String]) -> bool;
}
pub enum MarkerExpr { Compare { left: MarkerVar, op: CompareOp, right: MarkerValue }, And(Box<MarkerExpr>, Box<MarkerExpr>), Or(Box<MarkerExpr>, Box<MarkerExpr>), Not(Box<MarkerExpr>), }
```

### 3. Wheel Tag Parser and Selector (dx-py-core)

```rust
/// Parsed wheel filename pub struct WheelTag { pub name: String, pub version: String, pub build: Option<String>, pub python_tags: Vec<String>, pub abi_tags: Vec<String>, pub platform_tags: Vec<String>, }
impl WheelTag { pub fn parse(filename: &str) -> Result<Self>;
pub fn is_compatible(&self, env: &PlatformEnvironment) -> bool;
pub fn specificity_score(&self) -> u32;
}
/// Platform detection pub struct PlatformEnvironment { pub os: Os, pub arch: Arch, pub python_impl: PythonImpl, pub python_version: (u32, u32), pub abi: String, pub manylinux: Option<ManylinuxVersion>, }
impl PlatformEnvironment { pub fn detect() -> Self;
}
```

### 4. Async Download Manager (dx-py-package-manager)

```rust
/// Async download manager with retry and parallelism pub struct DownloadManager { client: reqwest::Client, max_concurrent: usize, retry_count: u32, retry_delay: Duration, }
impl DownloadManager { pub async fn download_many(&self, urls: Vec<DownloadRequest>) -> Vec<Result<Vec<u8>>>;
pub async fn download_with_progress(&self, url: &str, expected_hash: &str) -> Result<Vec<u8>>;
}
pub struct DownloadRequest { pub url: String, pub expected_sha256: String, pub filename: String, }
```

### 5. Real PyPI Registry Client (dx-py-package-manager)

```rust
/// Async PyPI client pub struct AsyncPyPiClient { client: reqwest::Client, base_url: String, extra_indexes: Vec<String>, }
impl AsyncPyPiClient { pub async fn get_package(&self, name: &str) -> Result<PyPiPackageInfo>;
pub async fn get_versions(&self, name: &str) -> Result<Vec<Pep440Version>>;
pub async fn get_dependencies(&self, name: &str, version: &str) -> Result<Vec<DependencySpec>>;
pub async fn find_best_wheel(&self, name: &str, version: &str, env: &PlatformEnvironment) -> Result<ReleaseFile>;
}
```

### 6. Real Resolver with PyPI Integration (dx-py-package-manager)

```rust
/// Resolver that fetches from PyPI pub struct PyPiResolver { client: AsyncPyPiClient, cache: HintCache, marker_env: MarkerEnvironment, platform_env: PlatformEnvironment, }
impl PyPiResolver { pub async fn resolve(&mut self, deps: &[DependencySpec]) -> Result<Resolution>;
}
```

### 7. Real Installer (dx-py-package-manager)

```rust
/// Installer that extracts wheels to site-packages pub struct WheelInstaller { cache: GlobalCache, site_packages: PathBuf, strategy: InstallStrategy, }
impl WheelInstaller { pub fn install_wheel(&self, wheel_path: &Path) -> Result<InstalledPackage>;
pub fn install_from_cache(&self, hash: &[u8; 32]) -> Result<InstalledPackage>;
pub fn uninstall(&self, package: &str) -> Result<()>;
}
pub struct InstalledPackage { pub name: String, pub version: String, pub files: Vec<PathBuf>, pub dist_info: PathBuf, }
```

### 8. Real Virtual Environment Manager (dx-py-project-manager)

```rust
/// Real venv creation pub struct RealVenvManager { python_manager: PythonManager, }
impl RealVenvManager { pub fn create(&self, path: &Path, python: &Path) -> Result<Venv>;
pub fn create_with_packages(&self, path: &Path, python: &Path, packages: &[&str]) -> Result<Venv>;
}
pub struct Venv { pub path: PathBuf, pub python: PathBuf, pub site_packages: PathBuf, pub bin_dir: PathBuf, }
impl Venv { pub fn run(&self, cmd: &str, args: &[&str]) -> Result<ExitStatus>;
pub fn pip_install(&self, packages: &[&str]) -> Result<()>;
}
```

### 9. Real Python Manager (dx-py-project-manager)

```rust
/// Python version manager with download support pub struct RealPythonManager { install_dir: PathBuf, client: reqwest::Client, }
impl RealPythonManager { pub async fn list_available(&self) -> Result<Vec<PythonRelease>>;
pub async fn install(&self, version: &str) -> Result<PythonInstallation>;
pub fn is_installed(&self, version: &str) -> bool;
}
pub struct PythonRelease { pub version: String, pub url: String, pub sha256: String, pub platform: String, pub arch: String, }
```

### 10. Real Build System (dx-py-cli)

```rust
/// PEP 517 build system pub struct BuildSystem { project_dir: PathBuf, build_backend: String, }
impl BuildSystem { pub fn build_wheel(&self, output_dir: &Path) -> Result<PathBuf>;
pub fn build_sdist(&self, output_dir: &Path) -> Result<PathBuf>;
}
```

### 11. Real Publish System (dx-py-cli)

```rust
/// PyPI upload client pub struct PublishClient { client: reqwest::Client, repository_url: String, }
impl PublishClient { pub async fn upload(&self, file: &Path, token: &str) -> Result<()>;
}
```

### 12. Configuration System (dx-py-compat)

```rust
/// Configuration with layered sources pub struct Config { pub index_url: String, pub extra_index_urls: Vec<String>, pub trusted_hosts: Vec<String>, pub cache_dir: PathBuf, pub python_downloads: bool, }
impl Config { pub fn load() -> Result<Self>; // Merges env vars, global config, project config }
```

## Data Models

### DependencySpec (Enhanced)

```rust
pub struct DependencySpec { pub name: String, pub extras: Vec<String>, pub version_constraint: Option<VersionConstraint>, pub markers: Option<MarkerExpr>, pub url: Option<String>, // For URL dependencies pub path: Option<PathBuf>, // For path dependencies }
```

### VersionConstraint (Enhanced)

```rust
pub enum VersionConstraint { Any, Exact(Pep440Version), NotEqual(Pep440Version), Gte(Pep440Version), Gt(Pep440Version), Lte(Pep440Version), Lt(Pep440Version), Compatible(Pep440Version), // ~= Arbitrary(String), // === And(Vec<VersionConstraint>), }
```

## Correctness Properties

A property is a characteristic or behavior that should hold true across all valid executions of a system-, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.

### Property 1: PEP 440 Version Round-Trip

For any valid PEP 440 version string, parsing then formatting SHALL produce a semantically equivalent version string. Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5, 2.7

### Property 2: PEP 440 Version Ordering

For any two valid PEP 440 versions v1 and v2, the comparison result SHALL match the PEP 440 specification ordering rules. Validates: Requirements 2.6

### Property 3: PEP 508 Dependency Parsing Round-Trip

For any valid PEP 508 dependency string, parsing then formatting SHALL produce a semantically equivalent dependency string. Validates: Requirements 1.2

### Property 4: Marker Evaluation Consistency

For any valid marker expression and environment, evaluating the marker SHALL produce a deterministic boolean result consistent with PEP 508 semantics. Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6

### Property 5: Wheel Tag Parsing

For any valid wheel filename, parsing SHALL extract the correct name, version, and compatibility tags. Validates: Requirements 4.4

### Property 6: Wheel Selection Priority

For any set of compatible wheels, the selector SHALL prefer platform-specific wheels over universal wheels, and newer manylinux versions over older ones. Validates: Requirements 4.5, 4.6

### Property 7: SHA256 Verification

For any downloaded content, if the computed SHA256 hash does not match the expected hash, the download SHALL be rejected. Validates: Requirements 1.3, 8.2

### Property 8: Cleanup on Failure

For any operation that fails, the system SHALL leave no partial state (corrupted cache entries, incomplete venvs, etc.). Validates: Requirements 5.6

### Property 9: Configuration Layering

For any configuration key, the value SHALL be determined by the highest-priority source (env var > project config > global config > default). Validates: Requirements 11.1, 11.3

### Property 10: Workspace Member Enumeration

For any workspace configuration with glob patterns, the enumerated members SHALL match exactly the directories that match the patterns. Validates: Requirements 12.1, 12.2

### Property 11: Activation Script Validity

For any generated activation script (bash, zsh, fish, PowerShell), the script SHALL be syntactically valid for its target shell. Validates: Requirements 7.3

## Error Handling

### Error Categories

- Network Errors: Retry with exponential backoff, clear timeout messages
- Parse Errors: Show the invalid input and expected format
- Resolution Errors: Show conflicting requirements and suggest fixes
- Platform Errors: Show required vs available platform tags
- Permission Errors: Suggest running with appropriate permissions
- Integrity Errors: Show expected vs actual hash, suggest re-download

### Error Recovery

- All file operations use atomic writes (write to temp, then rename)
- Failed downloads are cleaned up from cache
- Failed venv creation removes partial directory
- Failed installs roll back to previous state

## Testing Strategy

### Unit Tests

- PEP 440 version parsing edge cases
- Marker expression parsing and evaluation
- Wheel tag parsing
- Configuration loading

### Property-Based Tests

- Version round-trip (Property 1)
- Version ordering (Property 2)
- Dependency parsing round-trip (Property 3)
- Marker evaluation (Property 4)
- Wheel tag parsing (Property 5)
- SHA256 verification (Property 7)
- Configuration layering (Property 9)

### Integration Tests

- Real PyPI package resolution
- Real wheel download and installation
- Real venv creation and activation
- Cross-platform tests (Windows, macOS, Linux)

### Performance Tests

- Resolution benchmark (1000 packages)
- Download benchmark (parallel vs sequential)
- Installation benchmark (hard link vs copy)
