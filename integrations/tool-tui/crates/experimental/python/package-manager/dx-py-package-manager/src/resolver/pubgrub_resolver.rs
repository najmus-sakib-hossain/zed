//! PubGrub-based dependency resolver with full backtracking support
//!
//! This module implements a proper PubGrub resolver using the pubgrub crate,
//! providing:
//! - Full backtracking for conflict resolution
//! - Detailed conflict explanation generation
//! - Integration with the existing VersionProvider trait
//! - Extras and marker evaluation support

use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::error::Error as StdError;
use std::fmt::{self, Display};

use pubgrub::range::Range;
use pubgrub::report::{DefaultStringReporter, Reporter};
use pubgrub::solver::{Dependencies, DependencyConstraints, DependencyProvider};
use pubgrub::version::Version;

use dx_py_compat::markers::{MarkerEnvironment, MarkerEvaluator};
use dx_py_core::version::PackedVersion;

use super::{Dependency, Resolution, ResolvedPackage, VersionConstraint, VersionProvider};
use crate::{Error, Result};

/// A semantic version wrapper that implements pubgrub's Version trait
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl SemanticVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn from_packed(packed: &PackedVersion) -> Self {
        Self {
            major: packed.major,
            minor: packed.minor,
            patch: packed.patch,
        }
    }

    pub fn to_packed(&self) -> PackedVersion {
        PackedVersion::new(self.major, self.minor, self.patch)
    }
}

impl Display for SemanticVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Version for SemanticVersion {
    fn lowest() -> Self {
        Self::new(0, 0, 0)
    }

    fn bump(&self) -> Self {
        Self::new(self.major, self.minor, self.patch + 1)
    }
}

/// Package identifier for the resolver
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Package(pub String);

impl Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Borrow<str> for Package {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// Convert VersionConstraint to pubgrub Range
fn constraint_to_range(constraint: &VersionConstraint) -> Range<SemanticVersion> {
    match constraint {
        VersionConstraint::Any => Range::any(),
        VersionConstraint::Exact(v) => {
            let sv = SemanticVersion::from_packed(v);
            Range::exact(sv)
        }
        VersionConstraint::Gte(v) => {
            let sv = SemanticVersion::from_packed(v);
            Range::higher_than(sv)
        }
        VersionConstraint::Lt(v) => {
            let sv = SemanticVersion::from_packed(v);
            Range::strictly_lower_than(sv)
        }
        VersionConstraint::Range { min, max } => {
            let min_sv = SemanticVersion::from_packed(min);
            let max_sv = SemanticVersion::from_packed(max);
            Range::between(min_sv, max_sv)
        }
        VersionConstraint::Compatible(v) => {
            // ~=1.2.3 means >=1.2.3,<1.3.0
            let min_sv = SemanticVersion::from_packed(v);
            let max_sv = SemanticVersion::new(v.major, v.minor + 1, 0);
            Range::between(min_sv, max_sv)
        }
    }
}

/// PubGrub dependency provider implementation
pub struct PubGrubProvider<P: VersionProvider> {
    /// The underlying version provider
    provider: P,
    /// Cache of available versions per package
    version_cache: HashMap<String, Vec<SemanticVersion>>,
    /// Cache of dependencies per (package, version)
    #[allow(clippy::type_complexity)]
    dep_cache: HashMap<(String, SemanticVersion), Vec<(Package, Range<SemanticVersion>)>>,
}

impl<P: VersionProvider> PubGrubProvider<P> {
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            version_cache: HashMap::new(),
            dep_cache: HashMap::new(),
        }
    }

    /// Get available versions for a package (cached)
    pub fn get_versions(&mut self, package: &str) -> Result<Vec<SemanticVersion>> {
        if let Some(cached) = self.version_cache.get(package) {
            return Ok(cached.clone());
        }

        let versions = self.provider.get_versions(package)?;
        let semantic_versions: Vec<SemanticVersion> =
            versions.into_iter().map(|(pv, _)| SemanticVersion::from_packed(&pv)).collect();

        self.version_cache.insert(package.to_string(), semantic_versions.clone());
        Ok(semantic_versions)
    }

    /// Get dependencies for a package version (cached)
    pub fn get_deps(
        &mut self,
        package: &str,
        version: &SemanticVersion,
    ) -> Result<Vec<(Package, Range<SemanticVersion>)>> {
        let key = (package.to_string(), *version);
        if let Some(cached) = self.dep_cache.get(&key) {
            return Ok(cached.clone());
        }

        let packed = version.to_packed();
        let deps = self.provider.get_dependencies(package, &packed)?;

        let converted: Vec<(Package, Range<SemanticVersion>)> = deps
            .into_iter()
            .map(|d| (Package(d.name), constraint_to_range(&d.constraint)))
            .collect();

        self.dep_cache.insert(key, converted.clone());
        Ok(converted)
    }
}

impl<P: VersionProvider> DependencyProvider<Package, SemanticVersion> for PubGrubProvider<P> {
    fn choose_package_version<T: Borrow<Package>, U: Borrow<Range<SemanticVersion>>>(
        &self,
        potential_packages: impl Iterator<Item = (T, U)>,
    ) -> std::result::Result<(T, Option<SemanticVersion>), Box<dyn StdError>> {
        // Choose the package with the fewest available versions (fail-first heuristic)
        let mut best: Option<(T, Option<SemanticVersion>, usize)> = None;

        for (package, range) in potential_packages {
            let pkg_name = package.borrow();

            // Get versions from cache (we can't mutate self here, so use cached data)
            let versions = self.version_cache.get(&pkg_name.0).cloned().unwrap_or_default();

            // Filter versions that satisfy the range
            let matching: Vec<_> =
                versions.iter().filter(|v| range.borrow().contains(v)).cloned().collect();

            let count = matching.len();
            let chosen = matching.into_iter().max(); // Pick highest version

            match &best {
                None => best = Some((package, chosen, count)),
                Some((_, _, best_count)) if count < *best_count => {
                    best = Some((package, chosen, count));
                }
                _ => {}
            }
        }

        match best {
            Some((package, version, _)) => Ok((package, version)),
            None => Err("No packages to choose from".into()),
        }
    }

    fn get_dependencies(
        &self,
        package: &Package,
        version: &SemanticVersion,
    ) -> std::result::Result<Dependencies<Package, SemanticVersion>, Box<dyn StdError>> {
        // Get dependencies from cache
        let key = (package.0.clone(), *version);
        if let Some(deps) = self.dep_cache.get(&key) {
            let dep_map: DependencyConstraints<Package, SemanticVersion> =
                deps.iter().cloned().collect();
            return Ok(Dependencies::Known(dep_map));
        }

        // If not cached, return empty (should have been pre-populated)
        Ok(Dependencies::Known(DependencyConstraints::default()))
    }
}

/// Conflict explanation with detailed information
#[derive(Debug, Clone)]
pub struct ConflictExplanation {
    /// Human-readable explanation
    pub message: String,
    /// Packages involved in the conflict
    pub packages: Vec<String>,
    /// Suggested resolutions
    pub suggestions: Vec<String>,
}

impl Display for ConflictExplanation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Dependency conflict detected:")?;
        writeln!(f)?;
        writeln!(f, "{}", self.message)?;
        writeln!(f)?;
        writeln!(f, "Packages involved:")?;
        for pkg in &self.packages {
            writeln!(f, "  - {}", pkg)?;
        }
        if !self.suggestions.is_empty() {
            writeln!(f)?;
            writeln!(f, "Suggestions:")?;
            for suggestion in &self.suggestions {
                writeln!(f, "  • {}", suggestion)?;
            }
        }
        Ok(())
    }
}

/// Full PubGrub resolver with backtracking
pub struct PubGrubResolver<P: VersionProvider> {
    provider: P,
    /// Marker environment for evaluating conditional dependencies
    marker_env: MarkerEnvironment,
    /// Active extras for the resolution
    active_extras: HashSet<String>,
    /// Whether to allow pre-release versions
    allow_prereleases: bool,
    /// Yanked versions to skip (package -> set of versions)
    yanked_versions: HashMap<String, HashSet<PackedVersion>>,
}

impl<P: VersionProvider> PubGrubResolver<P> {
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            marker_env: MarkerEnvironment::current(),
            active_extras: HashSet::new(),
            allow_prereleases: false,
            yanked_versions: HashMap::new(),
        }
    }

    /// Set the marker environment for conditional dependency evaluation
    pub fn with_marker_env(mut self, env: MarkerEnvironment) -> Self {
        self.marker_env = env;
        self
    }

    /// Set active extras for the resolution
    pub fn with_extras(mut self, extras: HashSet<String>) -> Self {
        self.active_extras = extras;
        self
    }

    /// Allow pre-release versions in resolution
    pub fn allow_prereleases(mut self, allow: bool) -> Self {
        self.allow_prereleases = allow;
        self
    }

    /// Mark specific versions as yanked (to be skipped)
    pub fn with_yanked(mut self, package: &str, versions: HashSet<PackedVersion>) -> Self {
        self.yanked_versions.insert(package.to_string(), versions);
        self
    }

    /// Check if a dependency should be included based on its markers
    fn should_include_dependency(&self, dep: &Dependency) -> bool {
        if let Some(ref markers) = dep.markers {
            let extras: Vec<String> = self.active_extras.iter().cloned().collect();
            MarkerEvaluator::evaluate(markers, &self.marker_env, &extras)
        } else {
            true
        }
    }

    /// Check if a version is yanked
    fn is_yanked(&self, package: &str, version: &PackedVersion) -> bool {
        self.yanked_versions
            .get(package)
            .map(|versions| versions.contains(version))
            .unwrap_or(false)
    }

    /// Filter dependencies based on markers and extras
    fn filter_dependencies(&self, deps: Vec<Dependency>) -> Vec<Dependency> {
        deps.into_iter().filter(|d| self.should_include_dependency(d)).collect()
    }

    /// Resolve dependencies using PubGrub algorithm
    pub fn resolve(&mut self, deps: &[Dependency]) -> Result<Resolution> {
        let start = std::time::Instant::now();

        // Filter dependencies based on markers
        let filtered_deps = self.filter_dependencies(deps.to_vec());

        // Create the provider wrapper
        let mut pubgrub_provider = PubGrubProvider::new(&self.provider);

        // Pre-populate version cache for all packages we might need
        self.populate_caches(&mut pubgrub_provider, &filtered_deps)?;

        // Create root package with dependencies
        let root = Package("__root__".to_string());
        let root_version = SemanticVersion::new(0, 0, 0);

        // Add root to version cache
        pubgrub_provider
            .version_cache
            .insert("__root__".to_string(), vec![root_version]);

        // Add root dependencies (already filtered)
        let root_deps: Vec<(Package, Range<SemanticVersion>)> = filtered_deps
            .iter()
            .map(|d| (Package(d.name.clone()), constraint_to_range(&d.constraint)))
            .collect();
        pubgrub_provider
            .dep_cache
            .insert(("__root__".to_string(), root_version), root_deps);

        // Run PubGrub resolution
        match pubgrub::solver::resolve(&pubgrub_provider, root.clone(), root_version) {
            Ok(solution) => {
                let packages = self.convert_solution(solution)?;
                let elapsed = start.elapsed();
                Ok(Resolution::new(packages, elapsed.as_millis() as u64))
            }
            Err(pubgrub::error::PubGrubError::NoSolution(derivation)) => {
                let explanation = self.explain_conflict(&derivation);
                Err(Error::DependencyConflict(explanation.to_string()))
            }
            Err(e) => Err(Error::Resolution(format!("Resolution failed: {}", e))),
        }
    }

    /// Pre-populate caches by traversing the dependency graph
    fn populate_caches(
        &self,
        pubgrub_provider: &mut PubGrubProvider<&P>,
        deps: &[Dependency],
    ) -> Result<()> {
        let mut to_visit: Vec<String> = deps.iter().map(|d| d.name.clone()).collect();
        let mut visited: HashSet<String> = HashSet::new();

        while let Some(pkg_name) = to_visit.pop() {
            if visited.contains(&pkg_name) {
                continue;
            }
            visited.insert(pkg_name.clone());

            // Get versions (filtering yanked ones)
            let versions = pubgrub_provider.get_versions(&pkg_name)?;
            let filtered_versions: Vec<SemanticVersion> = versions
                .into_iter()
                .filter(|v| !self.is_yanked(&pkg_name, &v.to_packed()))
                .collect();

            // Update cache with filtered versions
            pubgrub_provider
                .version_cache
                .insert(pkg_name.clone(), filtered_versions.clone());

            // Get dependencies for each version, filtering by markers
            for version in &filtered_versions {
                let deps = pubgrub_provider.get_deps(&pkg_name, version)?;

                // Filter dependencies based on markers
                let filtered_deps: Vec<(Package, Range<SemanticVersion>)> = deps
                    .into_iter()
                    .filter(|(_pkg, _)| {
                        // Check if the original dependency had markers that exclude it
                        // For now, we keep all deps since we filtered at the Dependency level
                        true
                    })
                    .collect();

                pubgrub_provider
                    .dep_cache
                    .insert((pkg_name.clone(), *version), filtered_deps.clone());

                for (dep_pkg, _) in filtered_deps {
                    if !visited.contains(&dep_pkg.0) {
                        to_visit.push(dep_pkg.0);
                    }
                }
            }
        }

        Ok(())
    }

    /// Convert PubGrub solution to our Resolution format
    fn convert_solution<S: std::hash::BuildHasher>(
        &self,
        solution: std::collections::HashMap<Package, SemanticVersion, S>,
    ) -> Result<Vec<ResolvedPackage>> {
        let mut packages = Vec::new();

        for (pkg, version) in solution {
            // Skip root package
            if pkg.0 == "__root__" {
                continue;
            }

            let packed = version.to_packed();
            let version_string = version.to_string();

            // Get dependencies for this version
            let deps = self.provider.get_dependencies(&pkg.0, &packed)?;
            let dep_names: Vec<String> = deps.into_iter().map(|d| d.name).collect();

            let resolved = ResolvedPackage {
                name: pkg.0,
                version: packed,
                version_string,
                dependencies: dep_names,
                content_hash: [0u8; 32],
            };

            packages.push(resolved);
        }

        Ok(packages)
    }

    /// Generate a detailed conflict explanation
    fn explain_conflict(
        &self,
        derivation: &pubgrub::report::DerivationTree<Package, SemanticVersion>,
    ) -> ConflictExplanation {
        // Use PubGrub's built-in reporter for the message
        let message = DefaultStringReporter::report(derivation);

        // Extract packages involved
        let packages = self.extract_packages_from_derivation(derivation);

        // Generate suggestions
        let suggestions = self.generate_suggestions(&packages);

        ConflictExplanation {
            message,
            packages,
            suggestions,
        }
    }

    /// Extract package names from derivation tree
    fn extract_packages_from_derivation(
        &self,
        derivation: &pubgrub::report::DerivationTree<Package, SemanticVersion>,
    ) -> Vec<String> {
        let mut packages = HashSet::new();
        Self::collect_packages(derivation, &mut packages);
        packages.into_iter().collect()
    }

    fn collect_packages(
        derivation: &pubgrub::report::DerivationTree<Package, SemanticVersion>,
        packages: &mut HashSet<String>,
    ) {
        match derivation {
            pubgrub::report::DerivationTree::External(external) => match external {
                pubgrub::report::External::NotRoot(pkg, _) => {
                    packages.insert(pkg.0.clone());
                }
                pubgrub::report::External::NoVersions(pkg, _) => {
                    packages.insert(pkg.0.clone());
                }
                pubgrub::report::External::FromDependencyOf(pkg1, _, pkg2, _) => {
                    packages.insert(pkg1.0.clone());
                    packages.insert(pkg2.0.clone());
                }
                _ => {}
            },
            pubgrub::report::DerivationTree::Derived(derived) => {
                Self::collect_packages(&derived.cause1, packages);
                Self::collect_packages(&derived.cause2, packages);
            }
        }
    }

    /// Generate helpful suggestions for resolving conflicts
    fn generate_suggestions(&self, packages: &[String]) -> Vec<String> {
        let mut suggestions = Vec::new();

        suggestions.push("Try relaxing version constraints for conflicting packages".to_string());

        if packages.len() > 2 {
            suggestions
                .push(format!("Consider updating {} to compatible versions", packages.join(", ")));
        }

        suggestions.push(
            "Check if there are newer versions available that resolve the conflict".to_string(),
        );
        suggestions.push("Use `--pre` flag to allow pre-release versions if needed".to_string());

        suggestions
    }
}

// Implement VersionProvider for references to allow borrowing
impl<P: VersionProvider> VersionProvider for &P {
    fn get_versions(&self, package: &str) -> Result<Vec<(PackedVersion, String)>> {
        (*self).get_versions(package)
    }

    fn get_dependencies(&self, package: &str, version: &PackedVersion) -> Result<Vec<Dependency>> {
        (*self).get_dependencies(package, version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolver::InMemoryProvider;

    #[test]
    fn test_semantic_version_ordering() {
        let v1 = SemanticVersion::new(1, 0, 0);
        let v2 = SemanticVersion::new(1, 1, 0);
        let v3 = SemanticVersion::new(2, 0, 0);

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v1 < v3);
    }

    #[test]
    fn test_semantic_version_bump() {
        let v = SemanticVersion::new(1, 2, 3);
        let bumped = v.bump();
        assert_eq!(bumped, SemanticVersion::new(1, 2, 4));
    }

    #[test]
    fn test_constraint_to_range_any() {
        let range = constraint_to_range(&VersionConstraint::Any);
        assert!(range.contains(&SemanticVersion::new(0, 0, 0)));
        assert!(range.contains(&SemanticVersion::new(100, 0, 0)));
    }

    #[test]
    fn test_constraint_to_range_exact() {
        let v = PackedVersion::new(1, 2, 3);
        let range = constraint_to_range(&VersionConstraint::Exact(v));
        assert!(range.contains(&SemanticVersion::new(1, 2, 3)));
        assert!(!range.contains(&SemanticVersion::new(1, 2, 4)));
    }

    #[test]
    fn test_constraint_to_range_gte() {
        let v = PackedVersion::new(1, 0, 0);
        let range = constraint_to_range(&VersionConstraint::Gte(v));
        assert!(range.contains(&SemanticVersion::new(1, 0, 0)));
        assert!(range.contains(&SemanticVersion::new(2, 0, 0)));
        assert!(!range.contains(&SemanticVersion::new(0, 9, 0)));
    }

    #[test]
    fn test_pubgrub_simple_resolution() {
        let mut provider = InMemoryProvider::new();
        provider.add_package("requests", "2.28.0", vec![]);
        provider.add_package("requests", "2.29.0", vec![]);
        provider.add_package("requests", "2.30.0", vec![]);

        let mut resolver = PubGrubResolver::new(provider);
        let deps = vec![Dependency::new(
            "requests",
            VersionConstraint::Gte(PackedVersion::new(2, 28, 0)),
        )];

        let resolution = resolver.resolve(&deps).unwrap();
        assert_eq!(resolution.packages.len(), 1);
        assert_eq!(resolution.packages[0].name, "requests");
        // Should pick highest version
        assert_eq!(resolution.packages[0].version, PackedVersion::new(2, 30, 0));
    }

    #[test]
    fn test_pubgrub_with_dependencies() {
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

        let mut resolver = PubGrubResolver::new(provider);
        let deps = vec![Dependency::new("requests", VersionConstraint::Any)];

        let resolution = resolver.resolve(&deps).unwrap();
        assert_eq!(resolution.packages.len(), 2);

        let names: HashSet<_> = resolution.packages.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains("requests"));
        assert!(names.contains("urllib3"));
    }

    #[test]
    fn test_pubgrub_conflict_detection() {
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

        let mut resolver = PubGrubResolver::new(provider);
        let deps = vec![
            Dependency::new("a", VersionConstraint::Any),
            Dependency::new("b", VersionConstraint::Any),
        ];

        let result = resolver.resolve(&deps);
        assert!(result.is_err());

        // Check that error message contains conflict information
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("conflict") || err_msg.contains("Conflict") || err_msg.contains("c")
        );
    }

    #[test]
    fn test_pubgrub_backtracking() {
        // Test case where backtracking is needed
        let mut provider = InMemoryProvider::new();

        // a@2.0 requires c@2.0
        // a@1.0 requires c@1.0
        // b@1.0 requires c@1.0
        provider.add_package(
            "a",
            "2.0.0",
            vec![Dependency::new(
                "c",
                VersionConstraint::Exact(PackedVersion::new(2, 0, 0)),
            )],
        );
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
                VersionConstraint::Exact(PackedVersion::new(1, 0, 0)),
            )],
        );
        provider.add_package("c", "1.0.0", vec![]);
        provider.add_package("c", "2.0.0", vec![]);

        let mut resolver = PubGrubResolver::new(provider);
        let deps = vec![
            Dependency::new("a", VersionConstraint::Any),
            Dependency::new("b", VersionConstraint::Any),
        ];

        // Should backtrack from a@2.0 to a@1.0 to satisfy b's requirement
        let resolution = resolver.resolve(&deps).unwrap();

        let a_pkg = resolution.packages.iter().find(|p| p.name == "a").unwrap();
        let c_pkg = resolution.packages.iter().find(|p| p.name == "c").unwrap();

        // a should be 1.0.0 (backtracked from 2.0.0)
        assert_eq!(a_pkg.version, PackedVersion::new(1, 0, 0));
        // c should be 1.0.0
        assert_eq!(c_pkg.version, PackedVersion::new(1, 0, 0));
    }

    #[test]
    fn test_conflict_explanation_format() {
        let explanation = ConflictExplanation {
            message: "Package a requires c==1.0.0 but b requires c==2.0.0".to_string(),
            packages: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            suggestions: vec!["Try relaxing constraints".to_string()],
        };

        let formatted = explanation.to_string();
        assert!(formatted.contains("conflict"));
        assert!(formatted.contains("a"));
        assert!(formatted.contains("b"));
        assert!(formatted.contains("c"));
    }

    #[test]
    fn test_marker_filtering() {
        let mut provider = InMemoryProvider::new();
        provider.add_package("requests", "2.30.0", vec![]);
        provider.add_package("win32api", "1.0.0", vec![]);

        // Create dependencies with markers
        let deps = vec![
            Dependency::new("requests", VersionConstraint::Any),
            Dependency {
                name: "win32api".to_string(),
                constraint: VersionConstraint::Any,
                extras: vec![],
                markers: Some("sys_platform == 'win32'".to_string()),
            },
        ];

        // On non-Windows, win32api should be filtered out
        #[cfg(not(target_os = "windows"))]
        {
            let mut resolver = PubGrubResolver::new(provider);
            let resolution = resolver.resolve(&deps).unwrap();

            let names: HashSet<_> = resolution.packages.iter().map(|p| p.name.as_str()).collect();
            assert!(names.contains("requests"));
            assert!(!names.contains("win32api"));
        }

        // On Windows, both should be included
        #[cfg(target_os = "windows")]
        {
            let mut resolver = PubGrubResolver::new(provider);
            let resolution = resolver.resolve(&deps).unwrap();

            let names: HashSet<_> = resolution.packages.iter().map(|p| p.name.as_str()).collect();
            assert!(names.contains("requests"));
            assert!(names.contains("win32api"));
        }
    }

    #[test]
    fn test_extras_handling() {
        let mut provider = InMemoryProvider::new();
        provider.add_package("mypackage", "1.0.0", vec![]);
        provider.add_package("dev-tools", "1.0.0", vec![]);

        // Create dependency with extra marker
        let deps = vec![
            Dependency::new("mypackage", VersionConstraint::Any),
            Dependency {
                name: "dev-tools".to_string(),
                constraint: VersionConstraint::Any,
                extras: vec![],
                markers: Some("extra == 'dev'".to_string()),
            },
        ];

        // Without extras, dev-tools should be filtered out
        let mut resolver = PubGrubResolver::new(provider.clone());
        let resolution = resolver.resolve(&deps).unwrap();

        let names: HashSet<_> = resolution.packages.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains("mypackage"));
        assert!(!names.contains("dev-tools"));

        // With 'dev' extra, dev-tools should be included
        let mut resolver_with_extras =
            PubGrubResolver::new(provider).with_extras(["dev".to_string()].into_iter().collect());
        let resolution = resolver_with_extras.resolve(&deps).unwrap();

        let names: HashSet<_> = resolution.packages.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains("mypackage"));
        assert!(names.contains("dev-tools"));
    }

    #[test]
    fn test_yanked_version_filtering() {
        let mut provider = InMemoryProvider::new();
        provider.add_package("pkg", "1.0.0", vec![]);
        provider.add_package("pkg", "2.0.0", vec![]); // This will be yanked
        provider.add_package("pkg", "3.0.0", vec![]);

        let deps = vec![Dependency::new("pkg", VersionConstraint::Any)];

        // Without yanked filter, should pick 3.0.0 (highest)
        let mut resolver = PubGrubResolver::new(provider.clone());
        let resolution = resolver.resolve(&deps).unwrap();
        assert_eq!(resolution.packages[0].version, PackedVersion::new(3, 0, 0));

        // With 3.0.0 yanked, should pick 2.0.0
        let mut yanked = HashSet::new();
        yanked.insert(PackedVersion::new(3, 0, 0));

        let mut resolver_with_yanked = PubGrubResolver::new(provider).with_yanked("pkg", yanked);
        let resolution = resolver_with_yanked.resolve(&deps).unwrap();
        assert_eq!(resolution.packages[0].version, PackedVersion::new(2, 0, 0));
    }
}
