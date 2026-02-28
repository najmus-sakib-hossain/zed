//! Fixture management for pytest-compatible test execution
//!
//! This crate implements:
//! - FixtureManager with dependency resolution
//! - Fixture scopes (function, class, module, session)
//! - Yield-based fixture teardown
//! - Memory-mapped fixture caching with Blake3 hashing
//! - TeardownManager for reliable fixture cleanup

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

use blake3::Hash;
use memmap2::Mmap;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub use dx_py_core::{FixtureError, FixtureId};

mod registry;
mod teardown;
pub use registry::FixtureRegistry;
pub use teardown::{
    TeardownCode, TeardownCodeType, TeardownManager, TeardownResult, TeardownSummary,
};

/// Fixture scope determines when fixtures are set up and torn down
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum FixtureScope {
    /// Created and destroyed for each test function
    #[default]
    Function,
    /// Created once per test class, destroyed after all class tests
    Class,
    /// Created once per module, destroyed after all module tests
    Module,
    /// Created once per session, destroyed at session end
    Session,
}

impl FixtureScope {
    /// Get the priority of the scope (higher = longer lived)
    pub fn priority(&self) -> u8 {
        match self {
            FixtureScope::Function => 0,
            FixtureScope::Class => 1,
            FixtureScope::Module => 2,
            FixtureScope::Session => 3,
        }
    }
}

/// Definition of a fixture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureDefinition {
    /// Unique identifier for the fixture
    pub id: FixtureId,
    /// Name of the fixture
    pub name: String,
    /// Scope of the fixture
    pub scope: FixtureScope,
    /// Whether the fixture is automatically used
    pub autouse: bool,
    /// Names of fixtures this fixture depends on
    pub dependencies: Vec<String>,
    /// Whether the fixture uses yield (has teardown)
    pub is_generator: bool,
    /// Module path where the fixture is defined
    pub module_path: PathBuf,
    /// Line number where the fixture is defined
    pub line_number: u32,
}

impl FixtureDefinition {
    /// Create a new fixture definition
    pub fn new(name: impl Into<String>, module_path: impl Into<PathBuf>, line_number: u32) -> Self {
        let name = name.into();
        let module_path = module_path.into();
        let name_hash = blake3::hash(name.as_bytes()).as_bytes()[0..8]
            .iter()
            .fold(0u64, |acc, &b| (acc << 8) | b as u64);

        Self {
            id: FixtureId::new(name_hash),
            name,
            scope: FixtureScope::Function,
            autouse: false,
            dependencies: Vec::new(),
            is_generator: false,
            module_path,
            line_number,
        }
    }

    /// Set the scope of the fixture
    pub fn with_scope(mut self, scope: FixtureScope) -> Self {
        self.scope = scope;
        self
    }

    /// Set whether the fixture is automatically used
    pub fn with_autouse(mut self, autouse: bool) -> Self {
        self.autouse = autouse;
        self
    }

    /// Add dependencies to the fixture
    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }

    /// Set whether the fixture is a generator (has teardown)
    pub fn with_generator(mut self, is_generator: bool) -> Self {
        self.is_generator = is_generator;
        self
    }
}

/// A resolved fixture ready for execution
#[derive(Debug, Clone)]
pub struct ResolvedFixture {
    /// The fixture definition
    pub definition: FixtureDefinition,
    /// Cached value if available
    pub cached_value: Option<Vec<u8>>,
    /// Whether this fixture needs setup
    pub needs_setup: bool,
}

/// Scope instance identifier for caching fixtures per scope
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScopeInstance {
    /// Function scope - no caching, always create new
    Function,
    /// Class scope - cache per (module_path, class_name)
    Class { module_path: PathBuf, class_name: String },
    /// Module scope - cache per module_path
    Module { module_path: PathBuf },
    /// Session scope - cache globally for entire session
    Session,
}

impl ScopeInstance {
    /// Create a scope instance from test context
    pub fn from_test_context(
        scope: FixtureScope,
        module_path: &PathBuf,
        class_name: Option<&String>,
    ) -> Self {
        match scope {
            FixtureScope::Function => ScopeInstance::Function,
            FixtureScope::Class => {
                let class_name = class_name
                    .cloned()
                    .unwrap_or_else(|| "<no-class>".to_string());
                ScopeInstance::Class {
                    module_path: module_path.clone(),
                    class_name,
                }
            }
            FixtureScope::Module => ScopeInstance::Module {
                module_path: module_path.clone(),
            },
            FixtureScope::Session => ScopeInstance::Session,
        }
    }

    /// Get a cache key for this scope instance
    pub fn cache_key(&self, fixture_id: FixtureId) -> String {
        match self {
            ScopeInstance::Function => {
                // Function scope never caches - use unique key
                // This shouldn't actually be used since function scope doesn't cache
                format!("func_{:016x}_nocache", fixture_id.0)
            }
            ScopeInstance::Class { module_path, class_name } => {
                format!(
                    "class_{:016x}_{}::{}",
                    fixture_id.0,
                    module_path.display(),
                    class_name
                )
            }
            ScopeInstance::Module { module_path } => {
                format!("module_{:016x}_{}", fixture_id.0, module_path.display())
            }
            ScopeInstance::Session => {
                format!("session_{:016x}", fixture_id.0)
            }
        }
    }
}

/// Manages fixture definitions, resolution, and lifecycle
pub struct FixtureManager {
    /// All registered fixtures by name
    fixtures: HashMap<String, FixtureDefinition>,
    /// Fixture cache for scoped fixtures
    cache: FixtureCache,
    /// Currently active fixtures by scope
    active_fixtures: HashMap<FixtureScope, HashSet<String>>,
    /// Cached fixture values per scope instance
    /// Maps (fixture_name, scope_instance) -> cached_bytes
    scope_cache: HashMap<(String, ScopeInstance), Vec<u8>>,
}

impl FixtureManager {
    /// Create a new fixture manager
    pub fn new(cache_dir: impl Into<PathBuf>) -> Result<Self, FixtureError> {
        Ok(Self {
            fixtures: HashMap::new(),
            cache: FixtureCache::new(cache_dir)?,
            active_fixtures: HashMap::new(),
            scope_cache: HashMap::new(),
        })
    }

    /// Register a fixture definition
    pub fn register(&mut self, fixture: FixtureDefinition) {
        self.fixtures.insert(fixture.name.clone(), fixture);
    }

    /// Get a fixture by name
    pub fn get(&self, name: &str) -> Option<&FixtureDefinition> {
        self.fixtures.get(name)
    }

    /// Get all autouse fixtures for a given scope
    pub fn get_autouse_fixtures(&self, scope: FixtureScope) -> Vec<&FixtureDefinition> {
        self.fixtures
            .values()
            .filter(|f| f.autouse && f.scope.priority() >= scope.priority())
            .collect()
    }

    /// Resolve fixtures for a test, returning them in dependency order
    pub fn resolve_fixtures(
        &self,
        fixture_names: &[String],
    ) -> Result<Vec<ResolvedFixture>, FixtureError> {
        let mut resolved = Vec::new();
        let mut to_resolve: VecDeque<String> = fixture_names.iter().cloned().collect();
        let mut seen = HashSet::new();
        let mut resolution_order = Vec::new();

        // First pass: collect all fixtures and their dependencies
        while let Some(name) = to_resolve.pop_front() {
            if seen.contains(&name) {
                continue;
            }
            seen.insert(name.clone());

            let fixture = self
                .fixtures
                .get(&name)
                .ok_or_else(|| FixtureError::NotFound(format!("Fixture '{}' not found", name)))?;

            // Add dependencies to the queue
            for dep in &fixture.dependencies {
                if !seen.contains(dep) {
                    to_resolve.push_back(dep.clone());
                }
            }

            resolution_order.push(name);
        }

        // Second pass: topological sort
        let sorted = self.topological_sort(&resolution_order)?;

        // Third pass: create resolved fixtures
        for name in sorted {
            let fixture = self.fixtures.get(&name).unwrap();
            let cached_value = self.cache.get_cached_bytes(fixture.id);
            let needs_setup = cached_value.is_none() || fixture.scope == FixtureScope::Function;

            resolved.push(ResolvedFixture {
                definition: fixture.clone(),
                cached_value,
                needs_setup,
            });
        }

        Ok(resolved)
    }

    /// Match test parameters to fixture names and resolve their dependencies
    /// 
    /// This is the core fixture injection mechanism. It takes test parameters
    /// and returns the fixtures that need to be injected, in dependency order.
    /// 
    /// Note: This method doesn't have scope context, so it applies all autouse fixtures.
    /// For proper scope boundary handling, use resolve_fixtures_for_test_with_context.
    /// 
    /// Requirements: 11.1, 11.5, 11.6
    pub fn resolve_fixtures_for_test(
        &self,
        test_parameters: &[String],
    ) -> Result<Vec<ResolvedFixture>, FixtureError> {
        // Match test parameters to fixture names
        let mut fixture_names = Vec::new();
        
        for param in test_parameters {
            // Check if this parameter matches a fixture name
            if self.fixtures.contains_key(param) {
                fixture_names.push(param.clone());
            }
        }
        
        // Add all autouse fixtures (no scope context available)
        // Requirements: 11.6 - automatically use fixtures for all tests in scope
        for fixture in self.get_autouse_fixtures(FixtureScope::Function) {
            if !fixture_names.contains(&fixture.name) {
                fixture_names.push(fixture.name.clone());
            }
        }
        
        // Resolve all fixtures and their dependencies
        self.resolve_fixtures(&fixture_names)
    }

    /// Topologically sort fixtures by dependencies
    fn topological_sort(&self, names: &[String]) -> Result<Vec<String>, FixtureError> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();

        // Initialize - preserve original order for stable sorting
        for name in names {
            in_degree.entry(name.as_str()).or_insert(0);
            graph.entry(name.as_str()).or_default();
        }

        // Build graph
        for name in names {
            if let Some(fixture) = self.fixtures.get(name) {
                for dep in &fixture.dependencies {
                    if names.iter().any(|n| n == dep) {
                        graph.entry(dep.as_str()).or_default().push(name.as_str());
                        *in_degree.entry(name.as_str()).or_insert(0) += 1;
                    }
                }
            }
        }

        // Kahn's algorithm with stable ordering
        // Initialize queue with nodes that have no dependencies, in original order
        let mut queue: VecDeque<&str> = names
            .iter()
            .filter(|name| in_degree.get(name.as_str()) == Some(&0))
            .map(|s| s.as_str())
            .collect();

        let mut result = Vec::new();

        while let Some(name) = queue.pop_front() {
            result.push(name.to_string());

            if let Some(dependents) = graph.get(name) {
                // Process dependents in original order for stability
                let mut sorted_dependents: Vec<&str> = dependents.clone();
                sorted_dependents.sort_by_key(|dep| {
                    names.iter().position(|n| n.as_str() == *dep).unwrap_or(usize::MAX)
                });

                for dependent in sorted_dependents {
                    if let Some(deg) = in_degree.get_mut(dependent) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(dependent);
                        }
                    }
                }
            }
        }

        if result.len() != names.len() {
            return Err(FixtureError::NotFound(
                "Circular dependency detected in fixtures".to_string(),
            ));
        }

        Ok(result)
    }

    /// Get fixtures that need teardown in reverse order
    pub fn get_teardown_order<'a>(
        &self,
        fixtures: &'a [ResolvedFixture],
    ) -> Vec<&'a ResolvedFixture> {
        fixtures.iter().filter(|f| f.definition.is_generator).rev().collect()
    }

    /// Mark a fixture as active for its scope
    pub fn activate_fixture(&mut self, name: &str, scope: FixtureScope) {
        self.active_fixtures.entry(scope).or_default().insert(name.to_string());
    }

    /// Deactivate all fixtures for a scope
    pub fn deactivate_scope(&mut self, scope: FixtureScope) {
        self.active_fixtures.remove(&scope);
    }

    /// Check if a fixture is currently active
    pub fn is_active(&self, name: &str) -> bool {
        self.active_fixtures.values().any(|fixtures| fixtures.contains(name))
    }

    /// Get the fixture cache
    pub fn cache(&self) -> &FixtureCache {
        &self.cache
    }

    /// Get mutable access to the fixture cache
    pub fn cache_mut(&mut self) -> &mut FixtureCache {
        &mut self.cache
    }

    /// Get the number of registered fixtures
    pub fn len(&self) -> usize {
        self.fixtures.len()
    }

    /// Check if no fixtures are registered
    pub fn is_empty(&self) -> bool {
        self.fixtures.is_empty()
    }

    /// Get cached fixture value for a specific scope instance
    /// 
    /// Requirements: 11.2, 11.3
    pub fn get_cached_for_scope(
        &self,
        fixture_name: &str,
        scope_instance: &ScopeInstance,
    ) -> Option<Vec<u8>> {
        self.scope_cache
            .get(&(fixture_name.to_string(), scope_instance.clone()))
            .cloned()
    }

    /// Cache a fixture value for a specific scope instance
    /// 
    /// Requirements: 11.2, 11.3
    pub fn cache_for_scope(
        &mut self,
        fixture_name: &str,
        scope_instance: ScopeInstance,
        value: Vec<u8>,
    ) {
        // Function scope should never be cached (always create new)
        if matches!(scope_instance, ScopeInstance::Function) {
            return;
        }

        self.scope_cache
            .insert((fixture_name.to_string(), scope_instance), value);
    }

    /// Clear cached fixtures for a specific scope instance
    /// 
    /// This should be called when a scope ends (e.g., end of module, end of class)
    /// 
    /// Requirements: 11.2, 11.3
    pub fn clear_scope_cache(&mut self, scope: FixtureScope, module_path: &PathBuf, class_name: Option<&String>) {
        let scope_instance = ScopeInstance::from_test_context(scope, module_path, class_name);
        
        // Remove all cached fixtures for this scope instance
        self.scope_cache.retain(|(_, instance), _| instance != &scope_instance);
    }

    /// Resolve fixtures for a test with scope context
    /// 
    /// This is the enhanced version that properly caches fixtures per scope instance
    /// and respects scope boundaries for autouse fixtures.
    /// 
    /// Requirements: 11.1, 11.2, 11.3, 11.5, 11.6
    pub fn resolve_fixtures_for_test_with_context(
        &self,
        test_parameters: &[String],
        module_path: &PathBuf,
        class_name: Option<&String>,
    ) -> Result<Vec<ResolvedFixture>, FixtureError> {
        // Match test parameters to fixture names
        let mut fixture_names = Vec::new();
        
        for param in test_parameters {
            // Check if this parameter matches a fixture name
            if self.fixtures.contains_key(param) {
                fixture_names.push(param.clone());
            }
        }
        
        // Add autouse fixtures respecting scope boundaries
        // Requirements: 11.6 - automatically use fixtures for all tests in scope
        for fixture in self.get_autouse_fixtures(FixtureScope::Function) {
            // Check if this autouse fixture applies to this test based on scope
            let applies = match fixture.scope {
                FixtureScope::Function => {
                    // Function-scoped autouse fixtures apply to all tests
                    true
                }
                FixtureScope::Class => {
                    // Class-scoped autouse fixtures only apply to tests in the same class
                    // For now, we check if the fixture is defined in the same module
                    // and if the test has a class context
                    class_name.is_some() && &fixture.module_path == module_path
                }
                FixtureScope::Module => {
                    // Module-scoped autouse fixtures only apply to tests in the same module
                    &fixture.module_path == module_path
                }
                FixtureScope::Session => {
                    // Session-scoped autouse fixtures apply to all tests
                    true
                }
            };
            
            if applies && !fixture_names.contains(&fixture.name) {
                fixture_names.push(fixture.name.clone());
            }
        }
        
        // Resolve all fixtures and their dependencies
        let mut resolved = self.resolve_fixtures(&fixture_names)?;

        // Update cached_value and needs_setup based on scope instance
        for fixture in &mut resolved {
            let scope_instance = ScopeInstance::from_test_context(
                fixture.definition.scope,
                module_path,
                class_name,
            );

            // Check if we have a cached value for this scope instance
            let cached = self.get_cached_for_scope(&fixture.definition.name, &scope_instance);
            
            // Determine if setup is needed
            let needs_setup = match fixture.definition.scope {
                FixtureScope::Function => true, // Always setup for function scope
                _ => cached.is_none(), // Setup only if not cached for other scopes
            };

            fixture.cached_value = cached;
            fixture.needs_setup = needs_setup;
        }

        Ok(resolved)
    }
}

/// Cached fixture entry with hash for invalidation
#[derive(Debug)]
struct CacheEntry {
    /// Hash of the fixture function source
    source_hash: Hash,
    /// Memory-mapped file containing serialized fixture
    mmap: Option<Mmap>,
    /// Path to the cache file
    #[allow(dead_code)]
    path: PathBuf,
    /// Cached bytes for quick access
    bytes: Option<Vec<u8>>,
}

/// Memory-mapped fixture cache
///
/// Stores serialized fixture values on disk with Blake3 hash-based
/// invalidation. When a fixture's source code changes, the cache
/// is automatically invalidated.
pub struct FixtureCache {
    /// Cache directory
    cache_dir: PathBuf,
    /// In-memory index of cached fixtures
    entries: HashMap<FixtureId, CacheEntry>,
}

impl FixtureCache {
    /// Create a new fixture cache in the given directory
    pub fn new(cache_dir: impl Into<PathBuf>) -> Result<Self, FixtureError> {
        let cache_dir = cache_dir.into();
        fs::create_dir_all(&cache_dir)?;

        Ok(Self {
            cache_dir,
            entries: HashMap::new(),
        })
    }

    /// Get the cache file path for a fixture
    fn cache_path(&self, id: FixtureId) -> PathBuf {
        self.cache_dir.join(format!("{:016x}.fixture", id.0))
    }

    /// Get the hash file path for a fixture
    fn hash_path(&self, id: FixtureId) -> PathBuf {
        self.cache_dir.join(format!("{:016x}.hash", id.0))
    }

    /// Compute Blake3 hash of fixture source code
    pub fn hash_source(source: &str) -> Hash {
        blake3::hash(source.as_bytes())
    }

    /// Check if a cached fixture is valid (source hash matches)
    pub fn is_valid(&self, id: FixtureId, source_hash: Hash) -> bool {
        if let Some(entry) = self.entries.get(&id) {
            return entry.source_hash == source_hash;
        }

        // Check on-disk hash
        if let Ok(stored_hash) = self.load_hash(id) {
            return stored_hash == source_hash;
        }

        false
    }

    /// Load stored hash from disk
    fn load_hash(&self, id: FixtureId) -> Result<Hash, FixtureError> {
        let hash_path = self.hash_path(id);
        let mut file = File::open(&hash_path)?;
        let mut bytes = [0u8; 32];
        file.read_exact(&mut bytes)?;
        Ok(Hash::from_bytes(bytes))
    }

    /// Store hash to disk
    fn store_hash(&self, id: FixtureId, hash: Hash) -> Result<(), FixtureError> {
        let hash_path = self.hash_path(id);
        let mut file = File::create(&hash_path)?;
        file.write_all(hash.as_bytes())?;
        Ok(())
    }

    /// Get cached bytes for a fixture
    pub fn get_cached_bytes(&self, id: FixtureId) -> Option<Vec<u8>> {
        self.entries.get(&id).and_then(|e| e.bytes.clone())
    }

    /// Get a cached fixture value, or create it if not cached/invalid
    ///
    /// The `source` parameter should be the fixture function's source code,
    /// used for cache invalidation.
    pub fn get_or_create<T, F>(
        &mut self,
        id: FixtureId,
        source: &str,
        create: F,
    ) -> Result<T, FixtureError>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> T,
    {
        let source_hash = Self::hash_source(source);

        // Check if we have a valid cached value
        if self.is_valid(id, source_hash) {
            if let Ok(value) = self.load::<T>(id) {
                return Ok(value);
            }
        }

        // Create new value and cache it
        let value = create();
        self.store(id, source_hash, &value)?;
        Ok(value)
    }

    /// Store a fixture value in the cache
    pub fn store<T: Serialize>(
        &mut self,
        id: FixtureId,
        source_hash: Hash,
        value: &T,
    ) -> Result<(), FixtureError> {
        let cache_path = self.cache_path(id);

        // Serialize value
        let bytes = bincode::serialize(value)
            .map_err(|e| FixtureError::SerializationFailed(e.to_string()))?;

        // Write to file
        let mut file = File::create(&cache_path)?;
        file.write_all(&bytes)?;

        // Store hash
        self.store_hash(id, source_hash)?;

        // Update in-memory entry
        self.entries.insert(
            id,
            CacheEntry {
                source_hash,
                mmap: None,
                path: cache_path,
                bytes: Some(bytes),
            },
        );

        Ok(())
    }

    /// Load a fixture value from the cache using memory mapping
    pub fn load<T: DeserializeOwned>(&mut self, id: FixtureId) -> Result<T, FixtureError> {
        let cache_path = self.cache_path(id);

        if !cache_path.exists() {
            return Err(FixtureError::NotFound(format!("Fixture {:?}", id)));
        }

        // Memory-map the file
        let file = File::open(&cache_path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        // Deserialize from mapped memory
        let value: T = bincode::deserialize(&mmap)
            .map_err(|e| FixtureError::DeserializationFailed(e.to_string()))?;

        // Update entry with mmap and bytes
        let bytes = mmap.to_vec();
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.mmap = Some(mmap);
            entry.bytes = Some(bytes);
        }

        Ok(value)
    }

    /// Invalidate a fixture cache entry
    pub fn invalidate(&mut self, id: FixtureId) -> Result<(), FixtureError> {
        // Remove from memory
        self.entries.remove(&id);

        // Remove from disk
        let cache_path = self.cache_path(id);
        let hash_path = self.hash_path(id);

        if cache_path.exists() {
            fs::remove_file(&cache_path)?;
        }
        if hash_path.exists() {
            fs::remove_file(&hash_path)?;
        }

        Ok(())
    }

    /// Clear all cached fixtures
    pub fn clear(&mut self) -> Result<(), FixtureError> {
        self.entries.clear();

        // Remove all files in cache directory
        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                fs::remove_file(path)?;
            }
        }

        Ok(())
    }

    /// Get the number of cached fixtures
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Check if a fixture exists in the cache
    pub fn contains(&self, id: FixtureId) -> bool {
        self.entries.contains_key(&id) || self.cache_path(id).exists()
    }
}

#[cfg(test)]
mod tests;
