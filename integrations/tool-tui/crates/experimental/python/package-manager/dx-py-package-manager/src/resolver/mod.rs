//! Dependency resolver with SIMD acceleration
//!
//! Implements a PubGrub-based dependency resolver with:
//! - Version constraint satisfaction
//! - Conflict detection and reporting
//! - SIMD-accelerated version comparison
//! - Resolution hint cache for fast re-resolution
//! - Full PubGrub backtracking via pubgrub_resolver module
//! - Extras resolution for optional dependencies
//! - Pre-release version handling per PEP 440
//! - Circular dependency detection and handling

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use dx_py_core::version::{compare_versions, PackedVersion};

use crate::{Error, Result};

pub mod circular;
pub mod extras;
pub mod prerelease;
pub mod pubgrub_resolver;

pub use circular::{CircularDependencyDetector, CycleHandling, DependencyCycle, DependencyGraph};
pub use extras::{
    normalize_package_name, parse_dependency_with_extras, ExtrasResolver, PackageWithExtras,
};
pub use prerelease::{compare_versions_pep440, is_prerelease, PreReleaseFilter, PreReleasePolicy};
pub use pubgrub_resolver::{ConflictExplanation, PubGrubResolver, SemanticVersion};

/// Version constraint for dependency resolution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionConstraint {
    /// Any version (*)
    Any,
    /// Exact version (==1.2.3)
    Exact(PackedVersion),
    /// Greater than or equal (>=1.2.3)
    Gte(PackedVersion),
    /// Less than (<2.0.0)
    Lt(PackedVersion),
    /// Range constraint (>=1.2.3,<2.0.0)
    Range {
        min: PackedVersion,
        max: PackedVersion,
    },
    /// Compatible release (~=1.2.3 means >=1.2.3,<1.3.0)
    Compatible(PackedVersion),
}

impl VersionConstraint {
    /// Parse a version constraint string
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();
        if s.is_empty() || s == "*" {
            return Ok(Self::Any);
        }

        // Handle range constraints (e.g., ">=1.0,<2.0")
        if s.contains(',') {
            let parts: Vec<&str> = s.split(',').collect();
            if parts.len() == 2 {
                let min_part = parts[0].trim();
                let max_part = parts[1].trim();

                let min = if let Some(version_str) = min_part.strip_prefix(">=") {
                    PackedVersion::parse(version_str)
                        .ok_or_else(|| Error::InvalidVersion(min_part.to_string()))?
                } else {
                    return Err(Error::InvalidVersion(s.to_string()));
                };

                let max = if let Some(version_str) = max_part.strip_prefix('<') {
                    if !max_part.starts_with("<=") {
                        PackedVersion::parse(version_str)
                            .ok_or_else(|| Error::InvalidVersion(max_part.to_string()))?
                    } else {
                        return Err(Error::InvalidVersion(s.to_string()));
                    }
                } else {
                    return Err(Error::InvalidVersion(s.to_string()));
                };

                return Ok(Self::Range { min, max });
            }
        }

        // Handle single constraints
        if let Some(version_str) = s.strip_prefix("==") {
            let version = PackedVersion::parse(version_str)
                .ok_or_else(|| Error::InvalidVersion(s.to_string()))?;
            Ok(Self::Exact(version))
        } else if let Some(version_str) = s.strip_prefix(">=") {
            let version = PackedVersion::parse(version_str)
                .ok_or_else(|| Error::InvalidVersion(s.to_string()))?;
            Ok(Self::Gte(version))
        } else if let Some(version_str) = s.strip_prefix("~=") {
            let version = PackedVersion::parse(version_str)
                .ok_or_else(|| Error::InvalidVersion(s.to_string()))?;
            Ok(Self::Compatible(version))
        } else if let Some(version_str) = s.strip_prefix('<') {
            if !s.starts_with("<=") {
                let version = PackedVersion::parse(version_str)
                    .ok_or_else(|| Error::InvalidVersion(s.to_string()))?;
                Ok(Self::Lt(version))
            } else {
                let version =
                    PackedVersion::parse(s).ok_or_else(|| Error::InvalidVersion(s.to_string()))?;
                Ok(Self::Exact(version))
            }
        } else if let Some(version_str) = s.strip_prefix('>') {
            if !s.starts_with(">=") {
                // >1.0.0 is treated as >=1.0.1 (next patch)
                let version = PackedVersion::parse(version_str)
                    .ok_or_else(|| Error::InvalidVersion(s.to_string()))?;
                Ok(Self::Gte(PackedVersion::new(version.major, version.minor, version.patch + 1)))
            } else {
                let version =
                    PackedVersion::parse(s).ok_or_else(|| Error::InvalidVersion(s.to_string()))?;
                Ok(Self::Exact(version))
            }
        } else {
            // Assume exact version if no operator
            let version =
                PackedVersion::parse(s).ok_or_else(|| Error::InvalidVersion(s.to_string()))?;
            Ok(Self::Exact(version))
        }
    }

    /// Check if a version satisfies this constraint
    pub fn satisfies(&self, version: &PackedVersion) -> bool {
        match self {
            Self::Any => true,
            Self::Exact(v) => version == v,
            Self::Gte(min) => version >= min,
            Self::Lt(max) => version < max,
            Self::Range { min, max } => version >= min && version < max,
            Self::Compatible(v) => {
                // ~=1.2.3 means >=1.2.3,<1.3.0
                let max = PackedVersion::new(v.major, v.minor + 1, 0);
                version >= v && version < &max
            }
        }
    }

    /// Get the minimum version for SIMD comparison
    pub fn min_version(&self) -> Option<PackedVersion> {
        match self {
            Self::Any => None,
            Self::Exact(v) => Some(*v),
            Self::Gte(v) => Some(*v),
            Self::Lt(_) => None,
            Self::Range { min, .. } => Some(*min),
            Self::Compatible(v) => Some(*v),
        }
    }
}

/// A dependency requirement
#[derive(Debug, Clone)]
pub struct Dependency {
    /// Package name
    pub name: String,
    /// Version constraint
    pub constraint: VersionConstraint,
    /// Optional extras
    pub extras: Vec<String>,
    /// Environment markers (not evaluated in this implementation)
    pub markers: Option<String>,
}

impl Dependency {
    /// Create a new dependency
    pub fn new(name: &str, constraint: VersionConstraint) -> Self {
        Self {
            name: name.to_string(),
            constraint,
            extras: Vec::new(),
            markers: None,
        }
    }

    /// Create a dependency with extras
    pub fn with_extras(name: &str, constraint: VersionConstraint, extras: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            constraint,
            extras,
            markers: None,
        }
    }
}

/// A resolved package in the dependency graph
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPackage {
    /// Package name
    pub name: String,
    /// Resolved version
    pub version: PackedVersion,
    /// Version string (original format)
    pub version_string: String,
    /// Dependencies of this package
    pub dependencies: Vec<String>,
    /// Content hash for integrity
    pub content_hash: [u8; 32],
}

impl ResolvedPackage {
    /// Create a new resolved package
    pub fn new(name: &str, version: PackedVersion, version_string: &str) -> Self {
        Self {
            name: name.to_string(),
            version,
            version_string: version_string.to_string(),
            dependencies: Vec::new(),
            content_hash: [0u8; 32],
        }
    }
}

/// Resolution result
#[derive(Debug, Clone)]
pub struct Resolution {
    /// Resolved packages
    pub packages: Vec<ResolvedPackage>,
    /// Resolution time in milliseconds
    pub resolution_time_ms: u64,
    /// Whether this was from cache
    pub from_cache: bool,
}

impl Resolution {
    /// Create a new resolution
    pub fn new(packages: Vec<ResolvedPackage>, resolution_time_ms: u64) -> Self {
        Self {
            packages,
            resolution_time_ms,
            from_cache: false,
        }
    }

    /// Create a cached resolution
    pub fn from_cache(packages: Vec<ResolvedPackage>) -> Self {
        Self {
            packages,
            resolution_time_ms: 0,
            from_cache: true,
        }
    }
}

/// Package version provider trait
///
/// Implementations provide available versions and dependencies for packages.
pub trait VersionProvider {
    /// Get all available versions for a package (sorted newest first)
    fn get_versions(&self, package: &str) -> Result<Vec<(PackedVersion, String)>>;

    /// Get dependencies for a specific package version
    fn get_dependencies(&self, package: &str, version: &PackedVersion) -> Result<Vec<Dependency>>;
}

/// In-memory version provider for testing
#[derive(Default, Debug, Clone)]
pub struct InMemoryProvider {
    /// Package -> [(version, version_string, dependencies)]
    packages: HashMap<String, Vec<(PackedVersion, String, Vec<Dependency>)>>,
}

impl InMemoryProvider {
    /// Create a new in-memory provider
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a package version
    pub fn add_package(&mut self, name: &str, version: &str, deps: Vec<Dependency>) {
        let packed = PackedVersion::parse(version).unwrap_or_default();
        self.packages.entry(name.to_string()).or_default().push((
            packed,
            version.to_string(),
            deps,
        ));
    }
}

impl VersionProvider for InMemoryProvider {
    fn get_versions(&self, package: &str) -> Result<Vec<(PackedVersion, String)>> {
        let versions = self
            .packages
            .get(package)
            .map(|v| v.iter().map(|(pv, vs, _)| (*pv, vs.clone())).collect())
            .unwrap_or_default();
        Ok(versions)
    }

    fn get_dependencies(&self, package: &str, version: &PackedVersion) -> Result<Vec<Dependency>> {
        let deps = self
            .packages
            .get(package)
            .and_then(|versions| {
                versions.iter().find(|(v, _, _)| v == version).map(|(_, _, deps)| deps.clone())
            })
            .unwrap_or_default();
        Ok(deps)
    }
}

/// PubGrub-based dependency resolver
///
/// Uses a simplified PubGrub algorithm for version constraint satisfaction
/// with SIMD-accelerated version filtering.
pub struct Resolver<P: VersionProvider> {
    /// Version provider
    provider: P,
    /// Resolution hint cache
    hints: HintCache,
}

impl<P: VersionProvider> Resolver<P> {
    /// Create a new resolver
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            hints: HintCache::new(),
        }
    }

    /// Create a resolver with a hint cache
    pub fn with_hints(provider: P, hints: HintCache) -> Self {
        Self { provider, hints }
    }

    /// Resolve dependencies
    pub fn resolve(&mut self, deps: &[Dependency]) -> Result<Resolution> {
        let start = Instant::now();

        // Check hint cache first
        let input_hash = self.hash_dependencies(deps);
        if let Some(cached) = self.hints.lookup(input_hash) {
            if cached.is_valid() {
                return Ok(Resolution::from_cache(cached.packages.clone()));
            }
        }

        // Try delta resolution if similar resolution exists
        if let Some(delta) = self.hints.find_similar_and_patch(deps, &self.provider)? {
            return Ok(delta);
        }

        // Full resolution using PubGrub-style algorithm
        let resolution = self.pubgrub_resolve(deps)?;

        // Cache the result
        self.hints.store(input_hash, &resolution);

        let elapsed = start.elapsed();
        Ok(Resolution::new(resolution.packages, elapsed.as_millis() as u64))
    }

    /// Hash dependencies for cache lookup
    fn hash_dependencies(&self, deps: &[Dependency]) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for dep in deps {
            dep.name.hash(&mut hasher);
            format!("{:?}", dep.constraint).hash(&mut hasher);
        }
        hasher.finish()
    }

    /// PubGrub-style resolution algorithm
    fn pubgrub_resolve(&self, deps: &[Dependency]) -> Result<Resolution> {
        let start = Instant::now();

        // Track selected versions
        let mut selected: HashMap<String, ResolvedPackage> = HashMap::new();
        // Track constraints per package
        let mut constraints: HashMap<String, Vec<VersionConstraint>> = HashMap::new();
        // Work queue
        let mut queue: VecDeque<Dependency> = deps.iter().cloned().collect();
        // Track visited to avoid cycles
        let mut visited: HashSet<(String, PackedVersion)> = HashSet::new();

        while let Some(dep) = queue.pop_front() {
            // Skip if already resolved with compatible version
            if let Some(resolved) = selected.get(&dep.name) {
                if dep.constraint.satisfies(&resolved.version) {
                    continue;
                } else {
                    // Conflict! Need to backtrack or report error
                    return Err(Error::DependencyConflict(format!(
                        "Package {} requires {:?} but {} is already selected",
                        dep.name, dep.constraint, resolved.version_string
                    )));
                }
            }

            // Add constraint
            constraints.entry(dep.name.clone()).or_default().push(dep.constraint.clone());

            // Get available versions
            let versions = self.provider.get_versions(&dep.name)?;
            if versions.is_empty() {
                return Err(Error::PackageNotFound(dep.name.clone()));
            }

            // Find best version satisfying all constraints using SIMD
            let best = self.find_best_version(&dep.name, &constraints, &versions)?;

            // Check for cycles
            if !visited.insert((dep.name.clone(), best.0)) {
                continue;
            }

            // Create resolved package
            let mut resolved = ResolvedPackage::new(&dep.name, best.0, &best.1);

            // Get dependencies of this version
            let sub_deps = self.provider.get_dependencies(&dep.name, &best.0)?;
            resolved.dependencies = sub_deps.iter().map(|d| d.name.clone()).collect();

            // Add to selected
            selected.insert(dep.name.clone(), resolved);

            // Queue sub-dependencies
            for sub_dep in sub_deps {
                queue.push_back(sub_dep);
            }
        }

        let elapsed = start.elapsed();
        let packages: Vec<_> = selected.into_values().collect();

        Ok(Resolution::new(packages, elapsed.as_millis() as u64))
    }

    /// Find the best version satisfying all constraints using SIMD acceleration
    fn find_best_version(
        &self,
        package: &str,
        constraints: &HashMap<String, Vec<VersionConstraint>>,
        versions: &[(PackedVersion, String)],
    ) -> Result<(PackedVersion, String)> {
        let package_constraints = constraints.get(package).map(|c| c.as_slice()).unwrap_or(&[]);

        // Extract packed versions for SIMD comparison
        let packed: Vec<PackedVersion> = versions.iter().map(|(v, _)| *v).collect();

        // Use SIMD to filter versions that satisfy minimum constraint
        let mut candidates: Vec<(PackedVersion, String)> =
            if let Some(min) = package_constraints.iter().find_map(|c| c.min_version()) {
                let matches = compare_versions(&min, &packed);
                versions
                    .iter()
                    .zip(matches.iter())
                    .filter(|(_, &matches)| matches)
                    .map(|((v, s), _)| (*v, s.clone()))
                    .collect()
            } else {
                versions.to_vec()
            };

        // Filter by all constraints (scalar for complex constraints)
        candidates.retain(|(v, _)| package_constraints.iter().all(|c| c.satisfies(v)));

        // Sort by version descending (prefer newest)
        candidates.sort_by(|a, b| b.0.cmp(&a.0));

        candidates.into_iter().next().ok_or_else(|| Error::NoMatchingVersion {
            package: package.to_string(),
            constraint: format!("{:?}", package_constraints),
        })
    }
}

/// Resolution snapshot for hint cache
#[derive(Debug, Clone)]
pub struct ResolutionSnapshot {
    /// Hash of input dependencies
    pub input_hash: u64,
    /// Resolved packages
    pub packages: Vec<ResolvedPackage>,
    /// Creation timestamp
    pub created_at: u64,
    /// Validity duration in seconds
    pub valid_for: u64,
}

impl ResolutionSnapshot {
    /// Create a new snapshot
    pub fn new(input_hash: u64, packages: Vec<ResolvedPackage>) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

        Self {
            input_hash,
            packages,
            created_at: now,
            valid_for: 3600, // 1 hour default validity
        }
    }

    /// Check if the snapshot is still valid
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

        now < self.created_at + self.valid_for
    }
}

/// Resolution hint cache
///
/// Caches successful resolutions for fast re-resolution of similar dependency sets.
#[derive(Default)]
pub struct HintCache {
    /// Cached resolutions by input hash
    hints: HashMap<u64, ResolutionSnapshot>,
    /// Maximum cache size
    max_size: usize,
}

impl HintCache {
    /// Create a new hint cache
    pub fn new() -> Self {
        Self {
            hints: HashMap::new(),
            max_size: 1000,
        }
    }

    /// Create a hint cache with custom max size
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            hints: HashMap::new(),
            max_size,
        }
    }

    /// Look up a cached resolution
    pub fn lookup(&self, input_hash: u64) -> Option<&ResolutionSnapshot> {
        self.hints.get(&input_hash).filter(|s| s.is_valid())
    }

    /// Store a resolution in the cache
    pub fn store(&mut self, input_hash: u64, resolution: &Resolution) {
        // Evict old entries if at capacity
        if self.hints.len() >= self.max_size {
            self.evict_oldest();
        }

        let snapshot = ResolutionSnapshot::new(input_hash, resolution.packages.clone());
        self.hints.insert(input_hash, snapshot);
    }

    /// Find a similar resolution and compute delta
    pub fn find_similar_and_patch<P: VersionProvider>(
        &self,
        deps: &[Dependency],
        _provider: &P,
    ) -> Result<Option<Resolution>> {
        // Find the closest cached resolution
        let dep_names: HashSet<_> = deps.iter().map(|d| &d.name).collect();

        let mut best_match: Option<(&ResolutionSnapshot, usize)> = None;

        for snapshot in self.hints.values() {
            if !snapshot.is_valid() {
                continue;
            }

            let cached_names: HashSet<_> = snapshot.packages.iter().map(|p| &p.name).collect();
            let overlap = dep_names.intersection(&cached_names).count();

            if let Some((_, best_overlap)) = best_match {
                if overlap > best_overlap {
                    best_match = Some((snapshot, overlap));
                }
            } else if overlap > 0 {
                best_match = Some((snapshot, overlap));
            }
        }

        // Only use delta if >= 90% overlap
        if let Some((snapshot, overlap)) = best_match {
            if overlap * 10 >= dep_names.len() * 9 {
                // Compute delta
                let cached_map: HashMap<_, _> =
                    snapshot.packages.iter().map(|p| (&p.name, p)).collect();

                let mut packages = Vec::new();
                let mut needs_full_resolve = false;

                for dep in deps {
                    if let Some(cached) = cached_map.get(&dep.name) {
                        // Check if cached version still satisfies constraint
                        if dep.constraint.satisfies(&cached.version) {
                            packages.push((*cached).clone());
                        } else {
                            // Need to resolve this package
                            needs_full_resolve = true;
                            break;
                        }
                    } else {
                        // New package, need full resolve
                        needs_full_resolve = true;
                        break;
                    }
                }

                if !needs_full_resolve {
                    return Ok(Some(Resolution::from_cache(packages)));
                }
            }
        }

        Ok(None)
    }

    /// Evict the oldest entry
    fn evict_oldest(&mut self) {
        if let Some(oldest_key) =
            self.hints.iter().min_by_key(|(_, v)| v.created_at).map(|(k, _)| *k)
        {
            self.hints.remove(&oldest_key);
        }
    }

    /// Clear all cached hints
    pub fn clear(&mut self) {
        self.hints.clear();
    }

    /// Get the number of cached hints
    pub fn len(&self) -> usize {
        self.hints.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.hints.is_empty()
    }
}

/// Async resolver that fetches from PyPI
///
/// This resolver implements full dependency resolution with:
/// - Recursive transitive dependency resolution
/// - Highest compatible version selection (per Requirement 8.4)
/// - Circular dependency detection and handling
/// - Environment marker evaluation
/// - Conflict detection and reporting
///
/// # Example
/// ```ignore
/// use dx_py_package_manager::{AsyncPyPiClient, PyPiResolver, DependencySpec};
///
/// let client = AsyncPyPiClient::new();
/// let mut resolver = PyPiResolver::new(client);
///
/// let deps = vec![DependencySpec::parse("requests>=2.0")?];
/// let resolution = resolver.resolve(&deps).await?;
///
/// for pkg in resolution.packages {
///     println!("{} == {}", pkg.name, pkg.version_string);
/// }
/// ```
pub struct PyPiResolver {
    /// Async PyPI client
    client: crate::AsyncPyPiClient,
    /// Resolution hint cache
    hints: HintCache,
    /// Marker environment for filtering dependencies
    marker_env: dx_py_compat::markers::MarkerEnvironment,
    /// Platform environment for wheel selection
    platform_env: dx_py_core::wheel::PlatformEnvironment,
    /// Active extras for marker evaluation
    active_extras: HashSet<String>,
    /// Circular dependency handling strategy
    cycle_handling: circular::CycleHandling,
    /// Maximum resolution depth to prevent infinite loops
    max_depth: usize,
}

impl PyPiResolver {
    /// Create a new PyPI resolver
    pub fn new(client: crate::AsyncPyPiClient) -> Self {
        Self {
            client,
            hints: HintCache::new(),
            marker_env: dx_py_compat::markers::MarkerEnvironment::current(),
            platform_env: dx_py_core::wheel::PlatformEnvironment::detect(),
            active_extras: HashSet::new(),
            cycle_handling: circular::CycleHandling::Warn,
            max_depth: 100,
        }
    }

    /// Create a resolver with custom environments
    pub fn with_environments(
        client: crate::AsyncPyPiClient,
        marker_env: dx_py_compat::markers::MarkerEnvironment,
        platform_env: dx_py_core::wheel::PlatformEnvironment,
    ) -> Self {
        Self {
            client,
            hints: HintCache::new(),
            marker_env,
            platform_env,
            active_extras: HashSet::new(),
            cycle_handling: circular::CycleHandling::Warn,
            max_depth: 100,
        }
    }

    /// Set active extras for marker evaluation
    pub fn with_extras(mut self, extras: HashSet<String>) -> Self {
        self.active_extras = extras;
        self
    }

    /// Set circular dependency handling strategy
    pub fn with_cycle_handling(mut self, handling: circular::CycleHandling) -> Self {
        self.cycle_handling = handling;
        self
    }

    /// Set maximum resolution depth
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Get the marker environment
    pub fn marker_env(&self) -> &dx_py_compat::markers::MarkerEnvironment {
        &self.marker_env
    }

    /// Get the platform environment
    pub fn platform_env(&self) -> &dx_py_core::wheel::PlatformEnvironment {
        &self.platform_env
    }

    /// Resolve dependencies from PyPI
    ///
    /// This method implements full dependency resolution per Requirements 8.3 and 8.4:
    /// - Recursively resolves all transitive dependencies
    /// - Selects the highest compatible version for each package
    /// - Handles circular dependencies according to the configured strategy
    /// - Evaluates environment markers to filter platform-specific dependencies
    ///
    /// # Arguments
    /// * `deps` - Root dependencies to resolve (in PEP 508 format)
    ///
    /// # Returns
    /// * `Ok(Resolution)` - Successful resolution with all packages
    /// * `Err(Error)` - Resolution failed (conflict, package not found, etc.)
    pub async fn resolve(&mut self, deps: &[crate::DependencySpec]) -> Result<Resolution> {
        let start = std::time::Instant::now();

        // Convert DependencySpec to Dependency
        let converted_deps: Vec<Dependency> = deps
            .iter()
            .filter(|d| self.evaluate_markers(d))
            .filter_map(|d| self.convert_dependency(d))
            .collect();

        // Check hint cache first
        let input_hash = self.hash_dependency_specs(deps);
        if let Some(cached) = self.hints.lookup(input_hash) {
            if cached.is_valid() {
                return Ok(Resolution::from_cache(cached.packages.clone()));
            }
        }

        // Full resolution with recursive dependency fetching
        let resolution = self.resolve_recursive(&converted_deps).await?;

        // Cache the result
        self.hints.store(input_hash, &resolution);

        let elapsed = start.elapsed();
        Ok(Resolution::new(resolution.packages, elapsed.as_millis() as u64))
    }

    /// Evaluate markers for a dependency
    fn evaluate_markers(&self, dep: &crate::DependencySpec) -> bool {
        if let Some(ref markers) = dep.markers {
            let extras: Vec<String> = self.active_extras.iter().cloned().collect();
            dx_py_compat::markers::MarkerEvaluator::evaluate(markers, &self.marker_env, &extras)
        } else {
            true
        }
    }

    /// Convert DependencySpec to Dependency
    fn convert_dependency(&self, spec: &crate::DependencySpec) -> Option<Dependency> {
        let constraint = if let Some(ref vc) = spec.version_constraint {
            VersionConstraint::parse(vc).ok()?
        } else {
            VersionConstraint::Any
        };

        Some(Dependency {
            name: spec.name.clone(),
            constraint,
            extras: spec.extras.clone(),
            markers: spec.markers.clone(),
        })
    }

    /// Hash dependency specs for cache lookup
    fn hash_dependency_specs(&self, deps: &[crate::DependencySpec]) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for dep in deps {
            dep.name.hash(&mut hasher);
            dep.version_constraint.hash(&mut hasher);
            dep.extras.hash(&mut hasher);
            dep.markers.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Recursive dependency resolution with circular dependency detection
    ///
    /// This method implements the core resolution algorithm:
    /// 1. For each dependency, fetch available versions from PyPI
    /// 2. Select the highest version satisfying all constraints (Requirement 8.4)
    /// 3. Parse Requires-Dist from package metadata (PEP 508 format)
    /// 4. Recursively resolve transitive dependencies (Requirement 8.3)
    /// 5. Detect and handle circular dependencies
    /// 6. Report conflicts when constraints cannot be satisfied
    async fn resolve_recursive(&self, deps: &[Dependency]) -> Result<Resolution> {
        let start = std::time::Instant::now();

        // Track selected versions: package name -> resolved package
        let mut selected: HashMap<String, ResolvedPackage> = HashMap::new();
        // Track constraints per package: package name -> list of constraints
        let mut constraints: HashMap<String, Vec<VersionConstraint>> = HashMap::new();
        // Work queue: (dependency, depth)
        let mut queue: VecDeque<(Dependency, usize)> =
            deps.iter().map(|d| (d.clone(), 0)).collect();
        // Circular dependency detector
        let mut cycle_detector = circular::CircularDependencyDetector::new(self.cycle_handling);
        // Track packages being processed to detect cycles
        let mut in_progress: HashSet<String> = HashSet::new();

        while let Some((dep, depth)) = queue.pop_front() {
            // Check depth limit
            if depth > self.max_depth {
                return Err(Error::Resolution(format!(
                    "Maximum resolution depth ({}) exceeded while resolving {}",
                    self.max_depth, dep.name
                )));
            }

            let normalized_name = normalize_package_name(&dep.name);

            // Check for circular dependency
            if in_progress.contains(&normalized_name) {
                match cycle_detector.enter(&normalized_name) {
                    Ok(should_continue) => {
                        if !should_continue {
                            // Skip this dependency (cycle detected and handled)
                            continue;
                        }
                    }
                    Err(e) => return Err(e),
                }
            }

            // Skip if already resolved with compatible version
            if let Some(resolved) = selected.get(&normalized_name) {
                if dep.constraint.satisfies(&resolved.version) {
                    continue;
                } else {
                    // Conflict! The new constraint is incompatible with selected version
                    return Err(Error::DependencyConflict(format!(
                        "Package '{}' version {} is already selected, but new constraint {:?} is incompatible.\n\
                         This conflict may be caused by:\n\
                         - {} requiring {}=={}\n\
                         - Another package requiring {} with constraint {:?}",
                        normalized_name,
                        resolved.version_string,
                        dep.constraint,
                        resolved.dependencies.first().unwrap_or(&"root".to_string()),
                        normalized_name,
                        resolved.version_string,
                        normalized_name,
                        dep.constraint
                    )));
                }
            }

            // Mark as in progress
            in_progress.insert(normalized_name.clone());

            // Add constraint
            constraints
                .entry(normalized_name.clone())
                .or_default()
                .push(dep.constraint.clone());

            // Fetch versions from PyPI
            let versions = match self.client.get_versions(&normalized_name).await {
                Ok(v) => v,
                Err(Error::PackageNotFound(_)) => {
                    return Err(Error::PackageNotFound(format!(
                        "Package '{}' not found on PyPI. Please check the package name.",
                        dep.name
                    )));
                }
                Err(e) => return Err(e),
            };

            if versions.is_empty() {
                return Err(Error::PackageNotFound(format!(
                    "No versions available for package '{}'",
                    dep.name
                )));
            }

            // Convert to PackedVersion and find best match
            let packed_versions: Vec<(PackedVersion, String)> = versions
                .iter()
                .filter_map(|v| PackedVersion::parse(v).map(|pv| (pv, v.clone())))
                .collect();

            // Find best version satisfying all constraints (highest compatible version)
            let best = self.find_best_version(&normalized_name, &constraints, &packed_versions)?;

            // Create resolved package
            let mut resolved = ResolvedPackage::new(&normalized_name, best.0, &best.1);

            // Fetch dependencies for this version (Requires-Dist parsing)
            let sub_deps = self.client.get_dependencies(&normalized_name, &best.1).await?;

            // Filter by markers and convert, then add to queue
            for sub_dep in sub_deps {
                // Evaluate environment markers
                if !self.evaluate_markers(&sub_dep) {
                    continue;
                }

                // Convert to internal Dependency type
                if let Some(converted) = self.convert_dependency(&sub_dep) {
                    resolved.dependencies.push(converted.name.clone());

                    // Add to queue for recursive resolution
                    queue.push_back((converted, depth + 1));
                }
            }

            // Store resolved package
            selected.insert(normalized_name.clone(), resolved);

            // Mark as no longer in progress
            in_progress.remove(&normalized_name);
            cycle_detector.leave(&normalized_name);
        }

        // Check for any detected cycles and report them
        if cycle_detector.has_cycles() {
            let cycles = cycle_detector.cycles();
            let cycle_descriptions: Vec<String> =
                cycles.iter().map(|c| c.description()).collect();

            // Log warning about cycles (they were handled according to policy)
            eprintln!(
                "Warning: Circular dependencies detected and handled:\n{}",
                cycle_descriptions.join("\n")
            );
        }

        let elapsed = start.elapsed();
        let packages: Vec<_> = selected.into_values().collect();

        Ok(Resolution::new(packages, elapsed.as_millis() as u64))
    }

    /// Find the best version satisfying all constraints
    ///
    /// Implements Requirement 8.4: "WHEN multiple versions satisfy constraints,
    /// THE Package_Manager SHALL select the highest compatible version"
    ///
    /// # Algorithm
    /// 1. Filter versions by all accumulated constraints for the package
    /// 2. Sort remaining versions in descending order (highest first)
    /// 3. Return the highest version that satisfies all constraints
    ///
    /// # Arguments
    /// * `package` - Package name for error reporting
    /// * `constraints` - Map of package names to their accumulated constraints
    /// * `versions` - Available versions from PyPI
    ///
    /// # Returns
    /// * `Ok((PackedVersion, String))` - Best version (packed and string form)
    /// * `Err(Error::NoMatchingVersion)` - No version satisfies all constraints
    fn find_best_version(
        &self,
        package: &str,
        constraints: &HashMap<String, Vec<VersionConstraint>>,
        versions: &[(PackedVersion, String)],
    ) -> Result<(PackedVersion, String)> {
        let package_constraints = constraints.get(package).map(|c| c.as_slice()).unwrap_or(&[]);

        // Filter by all constraints
        let mut candidates: Vec<(PackedVersion, String)> = versions
            .iter()
            .filter(|(v, _)| package_constraints.iter().all(|c| c.satisfies(v)))
            .cloned()
            .collect();

        // Sort by version descending (prefer newest - Requirement 8.4)
        candidates.sort_by(|a, b| b.0.cmp(&a.0));

        candidates.into_iter().next().ok_or_else(|| {
            // Build a helpful error message
            let constraint_strs: Vec<String> = package_constraints
                .iter()
                .map(|c| format!("{:?}", c))
                .collect();

            let available_versions: Vec<String> = versions
                .iter()
                .take(10) // Show first 10 versions
                .map(|(_, v)| v.clone())
                .collect();

            Error::NoMatchingVersion {
                package: package.to_string(),
                constraint: format!(
                    "Constraints: [{}]\nAvailable versions: [{}]{}",
                    constraint_strs.join(", "),
                    available_versions.join(", "),
                    if versions.len() > 10 {
                        format!(" ... and {} more", versions.len() - 10)
                    } else {
                        String::new()
                    }
                ),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_constraint_parse() {
        assert!(matches!(VersionConstraint::parse("*").unwrap(), VersionConstraint::Any));
        assert!(matches!(
            VersionConstraint::parse("==1.2.3").unwrap(),
            VersionConstraint::Exact(_)
        ));
        assert!(matches!(
            VersionConstraint::parse(">=1.0.0").unwrap(),
            VersionConstraint::Gte(_)
        ));
        assert!(matches!(VersionConstraint::parse("<2.0.0").unwrap(), VersionConstraint::Lt(_)));
        assert!(matches!(
            VersionConstraint::parse(">=1.0.0,<2.0.0").unwrap(),
            VersionConstraint::Range { .. }
        ));
        assert!(matches!(
            VersionConstraint::parse("~=1.2.3").unwrap(),
            VersionConstraint::Compatible(_)
        ));
    }

    #[test]
    fn test_version_constraint_satisfies() {
        let v1 = PackedVersion::new(1, 5, 0);
        let v2 = PackedVersion::new(2, 0, 0);

        assert!(VersionConstraint::Any.satisfies(&v1));
        assert!(VersionConstraint::Exact(v1).satisfies(&v1));
        assert!(!VersionConstraint::Exact(v1).satisfies(&v2));
        assert!(VersionConstraint::Gte(v1).satisfies(&v1));
        assert!(VersionConstraint::Gte(v1).satisfies(&v2));
        assert!(VersionConstraint::Lt(v2).satisfies(&v1));
        assert!(!VersionConstraint::Lt(v2).satisfies(&v2));

        let range = VersionConstraint::Range {
            min: PackedVersion::new(1, 0, 0),
            max: PackedVersion::new(2, 0, 0),
        };
        assert!(range.satisfies(&v1));
        assert!(!range.satisfies(&v2));
    }

    #[test]
    fn test_simple_resolution() {
        let mut provider = InMemoryProvider::new();
        provider.add_package("requests", "2.28.0", vec![]);
        provider.add_package("requests", "2.29.0", vec![]);
        provider.add_package("requests", "2.30.0", vec![]);

        let mut resolver = Resolver::new(provider);
        let deps = vec![Dependency::new(
            "requests",
            VersionConstraint::Gte(PackedVersion::new(2, 28, 0)),
        )];

        let resolution = resolver.resolve(&deps).unwrap();
        assert_eq!(resolution.packages.len(), 1);
        assert_eq!(resolution.packages[0].name, "requests");
        // Should pick newest version
        assert_eq!(resolution.packages[0].version, PackedVersion::new(2, 30, 0));
    }

    #[test]
    fn test_resolution_with_dependencies() {
        let mut provider = InMemoryProvider::new();
        provider.add_package(
            "requests",
            "2.30.0",
            vec![Dependency::new(
                "urllib3",
                VersionConstraint::Gte(PackedVersion::new(1, 21, 0)),
            )],
        );
        provider.add_package("urllib3", "1.26.0", vec![]);
        provider.add_package("urllib3", "2.0.0", vec![]);

        let mut resolver = Resolver::new(provider);
        let deps = vec![Dependency::new("requests", VersionConstraint::Any)];

        let resolution = resolver.resolve(&deps).unwrap();
        assert_eq!(resolution.packages.len(), 2);

        let names: HashSet<_> = resolution.packages.iter().map(|p| &p.name).collect();
        assert!(names.contains(&"requests".to_string()));
        assert!(names.contains(&"urllib3".to_string()));
    }

    #[test]
    fn test_resolution_conflict() {
        let mut provider = InMemoryProvider::new();
        provider.add_package(
            "a",
            "1.0.0",
            vec![Dependency::new(
                "c",
                VersionConstraint::Exact(PackedVersion::new(1, 0, 0)),
            )],
        );
        provider.add_package(
            "b",
            "1.0.0",
            vec![Dependency::new(
                "c",
                VersionConstraint::Exact(PackedVersion::new(2, 0, 0)),
            )],
        );
        provider.add_package("c", "1.0.0", vec![]);
        provider.add_package("c", "2.0.0", vec![]);

        let mut resolver = Resolver::new(provider);
        let deps = vec![
            Dependency::new("a", VersionConstraint::Any),
            Dependency::new("b", VersionConstraint::Any),
        ];

        let result = resolver.resolve(&deps);
        assert!(result.is_err());
    }

    #[test]
    fn test_hint_cache_basic() {
        let mut cache = HintCache::new();
        assert!(cache.is_empty());

        let packages = vec![ResolvedPackage::new(
            "requests",
            PackedVersion::new(2, 30, 0),
            "2.30.0",
        )];
        let resolution = Resolution::new(packages.clone(), 10);

        cache.store(12345, &resolution);
        assert_eq!(cache.len(), 1);

        let cached = cache.lookup(12345).unwrap();
        assert_eq!(cached.packages.len(), 1);
        assert_eq!(cached.packages[0].name, "requests");
    }

    #[test]
    fn test_hint_cache_miss() {
        let cache = HintCache::new();
        assert!(cache.lookup(99999).is_none());
    }

    #[test]
    fn test_resolver_uses_cache() {
        let mut provider = InMemoryProvider::new();
        provider.add_package("requests", "2.30.0", vec![]);

        let mut resolver = Resolver::new(provider);
        let deps = vec![Dependency::new("requests", VersionConstraint::Any)];

        // First resolution
        let res1 = resolver.resolve(&deps).unwrap();
        assert!(!res1.from_cache);

        // Second resolution should use cache
        let res2 = resolver.resolve(&deps).unwrap();
        assert!(res2.from_cache);
        assert_eq!(res1.packages[0].name, res2.packages[0].name);
    }

    /// Test deep transitive dependency resolution (Requirement 8.3)
    #[test]
    fn test_deep_transitive_resolution() {
        let mut provider = InMemoryProvider::new();

        // Create a chain: a -> b -> c -> d -> e
        provider.add_package(
            "a",
            "1.0.0",
            vec![Dependency::new("b", VersionConstraint::Any)],
        );
        provider.add_package(
            "b",
            "1.0.0",
            vec![Dependency::new("c", VersionConstraint::Any)],
        );
        provider.add_package(
            "c",
            "1.0.0",
            vec![Dependency::new("d", VersionConstraint::Any)],
        );
        provider.add_package(
            "d",
            "1.0.0",
            vec![Dependency::new("e", VersionConstraint::Any)],
        );
        provider.add_package("e", "1.0.0", vec![]);

        let mut resolver = Resolver::new(provider);
        let deps = vec![Dependency::new("a", VersionConstraint::Any)];

        let resolution = resolver.resolve(&deps).unwrap();

        // All 5 packages should be resolved
        assert_eq!(resolution.packages.len(), 5);

        let names: HashSet<_> = resolution.packages.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains("a"));
        assert!(names.contains("b"));
        assert!(names.contains("c"));
        assert!(names.contains("d"));
        assert!(names.contains("e"));
    }

    /// Test highest compatible version selection (Requirement 8.4)
    #[test]
    fn test_highest_compatible_version_selection() {
        let mut provider = InMemoryProvider::new();

        // Add multiple versions of a package
        provider.add_package("pkg", "1.0.0", vec![]);
        provider.add_package("pkg", "1.5.0", vec![]);
        provider.add_package("pkg", "2.0.0", vec![]);
        provider.add_package("pkg", "2.5.0", vec![]);
        provider.add_package("pkg", "3.0.0", vec![]);

        let mut resolver = Resolver::new(provider);

        // Request >=1.0.0,<3.0.0 - should get 2.5.0 (highest in range)
        let deps = vec![Dependency::new(
            "pkg",
            VersionConstraint::Range {
                min: PackedVersion::new(1, 0, 0),
                max: PackedVersion::new(3, 0, 0),
            },
        )];

        let resolution = resolver.resolve(&deps).unwrap();
        assert_eq!(resolution.packages.len(), 1);
        assert_eq!(resolution.packages[0].version, PackedVersion::new(2, 5, 0));
    }

    /// Test that multiple constraints are combined correctly
    /// Note: This test uses PubGrubResolver which has backtracking support
    #[test]
    fn test_multiple_constraints_combined() {
        let mut provider = InMemoryProvider::new();

        // Package a requires c>=1.0.0
        // Package b requires c<2.0.0
        // Should resolve to highest version in [1.0.0, 2.0.0)
        provider.add_package(
            "a",
            "1.0.0",
            vec![Dependency::new(
                "c",
                VersionConstraint::Gte(PackedVersion::new(1, 0, 0)),
            )],
        );
        provider.add_package(
            "b",
            "1.0.0",
            vec![Dependency::new(
                "c",
                VersionConstraint::Lt(PackedVersion::new(2, 0, 0)),
            )],
        );
        provider.add_package("c", "0.5.0", vec![]);
        provider.add_package("c", "1.0.0", vec![]);
        provider.add_package("c", "1.5.0", vec![]);
        provider.add_package("c", "2.0.0", vec![]);
        provider.add_package("c", "2.5.0", vec![]);

        // Use PubGrubResolver which has proper backtracking
        let mut resolver = PubGrubResolver::new(provider);
        let deps = vec![
            Dependency::new("a", VersionConstraint::Any),
            Dependency::new("b", VersionConstraint::Any),
        ];

        let resolution = resolver.resolve(&deps).unwrap();

        // Find package c
        let c_pkg = resolution.packages.iter().find(|p| p.name == "c").unwrap();

        // Should be 1.5.0 (highest satisfying both >=1.0.0 and <2.0.0)
        assert_eq!(c_pkg.version, PackedVersion::new(1, 5, 0));
    }

    /// Test diamond dependency pattern
    #[test]
    fn test_diamond_dependency() {
        let mut provider = InMemoryProvider::new();

        // Diamond pattern:
        //     a
        //    / \
        //   b   c
        //    \ /
        //     d
        provider.add_package(
            "a",
            "1.0.0",
            vec![
                Dependency::new("b", VersionConstraint::Any),
                Dependency::new("c", VersionConstraint::Any),
            ],
        );
        provider.add_package(
            "b",
            "1.0.0",
            vec![Dependency::new(
                "d",
                VersionConstraint::Gte(PackedVersion::new(1, 0, 0)),
            )],
        );
        provider.add_package(
            "c",
            "1.0.0",
            vec![Dependency::new(
                "d",
                VersionConstraint::Gte(PackedVersion::new(1, 5, 0)),
            )],
        );
        provider.add_package("d", "1.0.0", vec![]);
        provider.add_package("d", "1.5.0", vec![]);
        provider.add_package("d", "2.0.0", vec![]);

        let mut resolver = Resolver::new(provider);
        let deps = vec![Dependency::new("a", VersionConstraint::Any)];

        let resolution = resolver.resolve(&deps).unwrap();

        // All 4 packages should be resolved
        assert_eq!(resolution.packages.len(), 4);

        // d should be 2.0.0 (highest satisfying both >=1.0.0 and >=1.5.0)
        let d_pkg = resolution.packages.iter().find(|p| p.name == "d").unwrap();
        assert_eq!(d_pkg.version, PackedVersion::new(2, 0, 0));
    }

    /// Test package name normalization
    #[test]
    fn test_package_name_normalization() {
        // Test the normalize_package_name function
        assert_eq!(normalize_package_name("My-Package"), "my_package");
        assert_eq!(normalize_package_name("my.package"), "my_package");
        assert_eq!(normalize_package_name("MY_PACKAGE"), "my_package");
        assert_eq!(normalize_package_name("my-package.name"), "my_package_name");
    }

    /// Test resolution with no matching version
    #[test]
    fn test_no_matching_version() {
        let mut provider = InMemoryProvider::new();
        provider.add_package("pkg", "1.0.0", vec![]);
        provider.add_package("pkg", "1.5.0", vec![]);

        let mut resolver = Resolver::new(provider);

        // Request version that doesn't exist
        let deps = vec![Dependency::new(
            "pkg",
            VersionConstraint::Gte(PackedVersion::new(2, 0, 0)),
        )];

        let result = resolver.resolve(&deps);
        assert!(result.is_err());

        // Check error type
        match result.unwrap_err() {
            Error::NoMatchingVersion { package, .. } => {
                assert_eq!(package, "pkg");
            }
            _ => panic!("Expected NoMatchingVersion error"),
        }
    }

    /// Test resolution with package not found
    #[test]
    fn test_package_not_found() {
        let provider = InMemoryProvider::new();
        let mut resolver = Resolver::new(provider);

        let deps = vec![Dependency::new("nonexistent", VersionConstraint::Any)];

        let result = resolver.resolve(&deps);
        assert!(result.is_err());

        // Check error type
        match result.unwrap_err() {
            Error::PackageNotFound(name) => {
                assert_eq!(name, "nonexistent");
            }
            _ => panic!("Expected PackageNotFound error"),
        }
    }
}
