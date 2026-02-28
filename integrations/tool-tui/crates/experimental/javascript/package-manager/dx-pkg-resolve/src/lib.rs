//! dx-pkg-resolve: Dependency Resolution
//!
//! Now uses npm registry API directly (zero infrastructure!)
//! Still fast through: binary lock file checking, BFS resolution, parallel fetching

use anyhow::{Context, Result};
use dx_pkg_core::version::Version;
use std::collections::{HashMap, HashSet, VecDeque};

// Re-export npm client
pub use dx_pkg_npm::{AbbreviatedMetadata, NpmClient};

/// Package identifier
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct PackageId {
    pub name: String,
    pub version: Version,
}

/// Package resolution result
#[derive(Debug, Clone, serde::Serialize)]
pub struct ResolvedPackage {
    pub name: String,
    pub version: String,
    pub tarball_url: String,
    pub dependencies: HashMap<String, String>,
}

/// Complete resolved dependency graph
#[derive(Debug, Clone)]
pub struct ResolvedGraph {
    pub packages: Vec<ResolvedPackage>,
    /// Fast lookup: name -> package
    lookup: HashMap<String, ResolvedPackage>,
}

impl Default for ResolvedGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl ResolvedGraph {
    pub fn new() -> Self {
        Self {
            packages: Vec::new(),
            lookup: HashMap::new(),
        }
    }

    pub fn add(&mut self, package: ResolvedPackage) {
        self.lookup.insert(package.name.clone(), package.clone());
        self.packages.push(package);
    }

    pub fn get(&self, name: &str) -> Option<&ResolvedPackage> {
        self.lookup.get(name)
    }
}

/// Local dependency resolver
pub struct LocalResolver {
    npm: NpmClient,
}

impl LocalResolver {
    pub fn new() -> Result<Self> {
        Ok(Self {
            npm: NpmClient::new()?,
        })
    }

    /// Resolve all dependencies from package.json manifest (PARALLEL!)
    pub async fn resolve(
        &mut self,
        dependencies: &HashMap<String, String>,
    ) -> Result<ResolvedGraph> {
        use futures::stream::{self, StreamExt};

        let mut resolved = ResolvedGraph::new();
        let mut queue: VecDeque<(String, String)> = VecDeque::new();
        let mut seen: HashSet<String> = HashSet::new();

        // Start with direct dependencies
        for (name, version) in dependencies {
            queue.push_back((name.clone(), version.clone()));
        }

        // Parallel BFS resolution (process 32 at a time)
        while !queue.is_empty() {
            let batch_size = queue.len().min(32);
            let batch: Vec<_> = queue.drain(..batch_size).collect();

            // Mark as seen BEFORE fetching (prevents duplicate fetches)
            for (name, constraint) in &batch {
                let key = format!("{}@{}", name, constraint);
                seen.insert(key);
            }

            // Fetch all in parallel!
            let results: Vec<_> = stream::iter(batch)
                .map(|(name, constraint)| {
                    let npm = &self.npm;
                    let name_owned = name.clone();
                    let constraint_owned = constraint.clone();
                    async move {
                        // Fetch metadata
                        let metadata = npm.get_abbreviated(&name_owned).await
                            .with_context(|| format!("Failed to fetch metadata for {}", name_owned))?;

                        // Find best version
                        let version = Self::find_best_version(&metadata, &constraint_owned)?;
                        let version_info = metadata.versions.get(&version)
                            .ok_or_else(|| anyhow::anyhow!("Version {} not found for {}", version, name_owned))?;

                        // Return package + its deps
                        let package = ResolvedPackage {
                            name: name_owned,
                            version,
                            tarball_url: version_info.dist.tarball.clone(),
                            dependencies: version_info.dependencies.clone(),
                        };

                        Ok::<_, anyhow::Error>(package)
                    }
                })
                .buffer_unordered(32) // 32 concurrent requests!
                .collect()
                .await;

            // Process results
            for result in results {
                let package = result?;

                // Queue transitive dependencies
                for (dep_name, dep_constraint) in &package.dependencies {
                    let key = format!("{}@{}", dep_name, dep_constraint);
                    if !seen.contains(&key) {
                        queue.push_back((dep_name.clone(), dep_constraint.clone()));
                    }
                }

                resolved.add(package);
            }
        }

        Ok(resolved)
    }

    /// Find the best matching version for a semver constraint
    fn find_best_version(metadata: &AbbreviatedMetadata, constraint: &str) -> Result<String> {
        // Handle special cases
        if constraint == "latest" || constraint == "*" {
            return metadata
                .dist_tags
                .get("latest")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("No 'latest' tag found"));
        }

        // Handle exact versions
        if metadata.versions.contains_key(constraint) {
            return Ok(constraint.to_string());
        }

        // Normalize npm-style constraints to semver format
        let normalized = Self::normalize_constraint(constraint);

        // Parse semver constraint
        let req = semver::VersionReq::parse(&normalized)
            .with_context(|| format!("Invalid version constraint: {}", constraint))?;

        // Find all matching versions
        let mut matching: Vec<semver::Version> = metadata
            .versions
            .keys()
            .filter_map(|v| semver::Version::parse(v).ok())
            .filter(|v| req.matches(v))
            .collect();

        // Sort descending (prefer newest)
        matching.sort_by(|a, b| b.cmp(a));

        matching.first().map(|v| v.to_string()).ok_or_else(|| {
            anyhow::anyhow!("No matching version found for constraint: {}", constraint)
        })
    }

    /// Normalize npm-style version constraints to semver format
    ///
    /// npm uses formats like ">= 2.1.2 < 3" which semver crate doesn't support.
    /// This converts them to semver-compatible format like ">=2.1.2, <3.0.0"
    fn normalize_constraint(constraint: &str) -> String {
        let constraint = constraint.trim();

        // Handle OR syntax - semver uses || too, so this is fine
        if constraint.contains("||") {
            return constraint
                .split("||")
                .map(|part| Self::normalize_constraint(part.trim()))
                .collect::<Vec<_>>()
                .join(" || ");
        }

        // Handle space-separated compound constraints like ">= 2.1.2 < 3"
        // Convert to comma-separated format: ">=2.1.2, <3.0.0"
        let parts: Vec<&str> = constraint.split_whitespace().collect();
        if parts.len() >= 2 {
            let mut normalized_parts = Vec::new();
            let mut i = 0;

            while i < parts.len() {
                let part = parts[i];

                // Check if this is an operator that might be separated from its version
                if (part == ">=" || part == "<=" || part == ">" || part == "<")
                    && i + 1 < parts.len()
                {
                    // Combine operator with next part (the version)
                    let version = parts[i + 1];
                    // Ensure version has all three parts for semver
                    let normalized_version = Self::normalize_version(version);
                    normalized_parts.push(format!("{}{}", part, normalized_version));
                    i += 2;
                } else if part.starts_with(">=")
                    || part.starts_with("<=")
                    || part.starts_with('>')
                    || part.starts_with('<')
                {
                    // Operator is attached to version
                    let (op, version) = if let Some(stripped) = part.strip_prefix(">=") {
                        (">=", stripped)
                    } else if let Some(stripped) = part.strip_prefix("<=") {
                        ("<=", stripped)
                    } else if let Some(stripped) = part.strip_prefix('>') {
                        (">", stripped)
                    } else {
                        ("<", &part[1..])
                    };
                    let normalized_version = Self::normalize_version(version);
                    normalized_parts.push(format!("{}{}", op, normalized_version));
                    i += 1;
                } else if part.starts_with('^') || part.starts_with('~') {
                    // Caret and tilde constraints - pass through
                    normalized_parts.push(part.to_string());
                    i += 1;
                } else {
                    // Plain version or other - pass through
                    normalized_parts.push(part.to_string());
                    i += 1;
                }
            }

            if normalized_parts.len() > 1 {
                return normalized_parts.join(", ");
            } else if normalized_parts.len() == 1 {
                return normalized_parts[0].clone();
            }
        }

        // Single constraint - just return as-is
        constraint.to_string()
    }

    /// Normalize a version string to have all three parts (major.minor.patch)
    fn normalize_version(version: &str) -> String {
        let parts: Vec<&str> = version.split('.').collect();
        match parts.len() {
            1 => format!("{}.0.0", parts[0]),
            2 => format!("{}.{}.0", parts[0], parts[1]),
            _ => version.to_string(),
        }
    }
}

// Note: Default implementation removed because LocalResolver::new() returns Result
// Use LocalResolver::new()? instead

// Keep old dependency graph types for compatibility
/// Dependency constraint
#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub constraint: VersionConstraint,
}

/// Version constraint types
#[derive(Debug, Clone)]
pub enum VersionConstraint {
    Exact(Version),
    Range { min: Version, max: Version },
    Caret(Version), // ^1.2.3 (>=1.2.3 <2.0.0)
    Tilde(Version), // ~1.2.3 (>=1.2.3 <1.3.0)
    Latest,
}

/// Version conflict information
#[derive(Debug, Clone)]
pub struct VersionConflict {
    pub package: String,
    pub required_by: Vec<(String, String)>, // (requirer, constraint)
    pub resolved_versions: Vec<String>,
}

/// Peer dependency validation result
#[derive(Debug, Clone)]
pub struct PeerValidation {
    pub package: String,
    pub peer_dep: String,
    pub required: String,
    pub installed: Option<String>,
    pub satisfied: bool,
}

impl LocalResolver {
    /// Detect version conflicts in resolved graph
    pub fn detect_conflicts(&self, graph: &ResolvedGraph) -> Vec<VersionConflict> {
        let mut requirements: HashMap<String, Vec<(String, String)>> = HashMap::new();
        let mut resolved_versions: HashMap<String, HashSet<String>> = HashMap::new();

        // Collect all requirements
        for pkg in &graph.packages {
            resolved_versions
                .entry(pkg.name.clone())
                .or_default()
                .insert(pkg.version.clone());

            for (dep_name, constraint) in &pkg.dependencies {
                requirements
                    .entry(dep_name.clone())
                    .or_default()
                    .push((pkg.name.clone(), constraint.clone()));
            }
        }

        // Find conflicts (packages with multiple resolved versions)
        let mut conflicts = Vec::new();
        for (name, versions) in &resolved_versions {
            if versions.len() > 1 {
                conflicts.push(VersionConflict {
                    package: name.clone(),
                    required_by: requirements.get(name).cloned().unwrap_or_default(),
                    resolved_versions: versions.iter().cloned().collect(),
                });
            }
        }

        conflicts
    }

    /// Validate peer dependencies
    pub fn validate_peer_deps(
        &self,
        graph: &ResolvedGraph,
        peer_deps: &HashMap<String, HashMap<String, String>>,
    ) -> Vec<PeerValidation> {
        let mut validations = Vec::new();

        for (pkg_name, peers) in peer_deps {
            for (peer_name, required_constraint) in peers {
                let installed = graph.get(peer_name).map(|p| p.version.clone());
                let satisfied = if let Some(ref ver) = installed {
                    Self::version_satisfies(ver, required_constraint)
                } else {
                    false
                };

                validations.push(PeerValidation {
                    package: pkg_name.clone(),
                    peer_dep: peer_name.clone(),
                    required: required_constraint.clone(),
                    installed,
                    satisfied,
                });
            }
        }

        validations
    }

    /// Check if a version satisfies a constraint
    fn version_satisfies(version: &str, constraint: &str) -> bool {
        if constraint == "*" || constraint == "latest" {
            return true;
        }

        let req = match semver::VersionReq::parse(constraint) {
            Ok(r) => r,
            Err(_) => return false,
        };

        let ver = match semver::Version::parse(version) {
            Ok(v) => v,
            Err(_) => return false,
        };

        req.matches(&ver)
    }

    /// Resolve with optional dependency handling
    pub async fn resolve_with_optional(
        &mut self,
        dependencies: &HashMap<String, String>,
        optional_deps: &HashMap<String, String>,
    ) -> Result<(ResolvedGraph, Vec<String>)> {
        // First resolve required dependencies
        let mut graph = self.resolve(dependencies).await?;

        // Try to resolve optional dependencies, collecting failures
        let mut failed_optional = Vec::new();

        for (name, constraint) in optional_deps {
            match self.resolve_single(name, constraint).await {
                Ok(pkg) => {
                    // Queue transitive deps
                    let mut transitive = HashMap::new();
                    for (dep_name, dep_constraint) in &pkg.dependencies {
                        if graph.get(dep_name).is_none() {
                            transitive.insert(dep_name.clone(), dep_constraint.clone());
                        }
                    }
                    graph.add(pkg);

                    // Resolve transitive deps (ignore failures for optional)
                    if let Ok(trans_graph) = self.resolve(&transitive).await {
                        for trans_pkg in trans_graph.packages {
                            if graph.get(&trans_pkg.name).is_none() {
                                graph.add(trans_pkg);
                            }
                        }
                    }
                }
                Err(_) => {
                    failed_optional.push(name.clone());
                }
            }
        }

        Ok((graph, failed_optional))
    }

    /// Resolve a single package
    async fn resolve_single(&mut self, name: &str, constraint: &str) -> Result<ResolvedPackage> {
        let metadata = self
            .npm
            .get_abbreviated(name)
            .await
            .with_context(|| format!("Failed to fetch metadata for {}", name))?;

        let version = Self::find_best_version(&metadata, constraint)?;
        let version_info = metadata
            .versions
            .get(&version)
            .ok_or_else(|| anyhow::anyhow!("Version {} not found for {}", version, name))?;

        Ok(ResolvedPackage {
            name: name.to_string(),
            version,
            tarball_url: version_info.dist.tarball.clone(),
            dependencies: version_info.dependencies.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resolve_lodash() {
        let mut resolver = LocalResolver::new().unwrap();
        let mut deps = HashMap::new();
        deps.insert("lodash".to_string(), "^4.17.0".to_string());

        let graph = resolver.resolve(&deps).await.unwrap();

        assert_eq!(graph.packages.len(), 1);
        assert_eq!(graph.packages[0].name, "lodash");
        assert!(graph.packages[0].version.starts_with("4.17"));
    }

    #[tokio::test]
    async fn test_resolve_with_deps() {
        let mut resolver = LocalResolver::new().unwrap();
        let mut deps = HashMap::new();
        deps.insert("express".to_string(), "^4.18.0".to_string());

        let graph = resolver.resolve(&deps).await.unwrap();

        // Express has many dependencies
        assert!(graph.packages.len() > 10);
        assert!(graph.get("express").is_some());
    }
}
