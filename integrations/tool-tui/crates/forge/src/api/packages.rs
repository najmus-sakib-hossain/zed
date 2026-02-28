//! Package Management — The Death of npm/cargo/pip
//!
//! This module provides APIs for managing DX packages, including installation,
//! uninstallation, updates, and registry operations.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Package information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub variant: String,
    pub installed_files: Vec<PathBuf>,
    pub description: Option<String>,
    pub dependencies: Vec<String>,
}

/// Package index stored in .dx/packages/index.json
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PackageIndex {
    packages: HashMap<String, PackageInfo>,
    pinned_versions: HashMap<String, String>,
}

impl PackageIndex {
    fn load(project_root: &Path) -> Result<Self> {
        let index_path = project_root.join(".dx").join("packages").join("index.json");
        if index_path.exists() {
            let content =
                std::fs::read_to_string(&index_path).context("Failed to read package index")?;
            serde_json::from_str(&content).context("Failed to parse package index")
        } else {
            Ok(Self::default())
        }
    }

    fn save(&self, project_root: &Path) -> Result<()> {
        let packages_dir = project_root.join(".dx").join("packages");
        std::fs::create_dir_all(&packages_dir).context("Failed to create packages directory")?;

        let index_path = packages_dir.join("index.json");
        let content =
            serde_json::to_string_pretty(self).context("Failed to serialize package index")?;
        std::fs::write(&index_path, content).context("Failed to write package index")?;

        Ok(())
    }
}

/// Get the DX package registry URL
fn get_registry_url() -> String {
    std::env::var("DX_PACKAGE_REGISTRY").unwrap_or_else(|_| "https://registry.dx.dev".to_string())
}

/// Get the project root directory
fn get_project_root() -> Result<PathBuf> {
    std::env::current_dir().context("Failed to get current directory")
}

/// URL-encode a string for use in query parameters
fn url_encode(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len() * 3);
    for c in s.chars() {
        match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => encoded.push(c),
            ' ' => encoded.push('+'),
            _ => {
                for byte in c.to_string().as_bytes() {
                    encoded.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    encoded
}

/// Installs a package with a specific variant.
///
/// Attempts to fetch the package from the DX registry. If the registry is unavailable
/// or the package is not found, creates a local placeholder.
///
/// # Status
///
/// **Partially implemented** - Registry fetching is attempted but actual package
/// extraction is not yet implemented. Currently creates marker files instead of
/// extracting real package contents.
///
/// # Arguments
///
/// * `package_id` - The unique identifier of the package
/// * `variant` - The variant of the package to install (e.g., "linux-x64", "darwin-arm64")
///
/// # Returns
///
/// A list of paths to installed files (currently just marker files).
pub async fn install_package_with_variant(package_id: &str, variant: &str) -> Result<Vec<PathBuf>> {
    tracing::info!("📦 Installing package '{}' with variant '{}'", package_id, variant);

    crate::api::events::emit_package_installation_begin(package_id)?;

    let project_root = get_project_root()?;
    let mut index = PackageIndex::load(&project_root)?;

    // Check if already installed
    let full_id = format!("{}:{}", package_id, variant);
    if index.packages.contains_key(&full_id) {
        tracing::info!("📦 Package '{}' variant '{}' already installed", package_id, variant);
        crate::api::events::emit_package_installation_success(package_id)?;
        return Ok(index
            .packages
            .get(&full_id)
            .map(|p| p.installed_files.clone())
            .unwrap_or_default());
    }

    // Create package directory
    let package_dir = project_root.join(".dx").join("packages").join(package_id).join(variant);
    std::fs::create_dir_all(&package_dir).context("Failed to create package directory")?;

    // Try to fetch package from registry
    let registry_url = get_registry_url();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;

    let url = format!("{}/packages/{}/variants/{}", registry_url, package_id, variant);

    let installed_files = match client.get(&url).send().await {
        Ok(response) if response.status().is_success() => {
            // In a full implementation, we would download and extract the package
            // For now, create a marker file
            let marker_path = package_dir.join(".installed");
            std::fs::write(&marker_path, format!("{}@{}", package_id, variant))
                .context("Failed to write marker file")?;
            vec![marker_path]
        }
        Ok(response) => {
            tracing::warn!(
                "📦 Package '{}' variant '{}' not found in registry (status: {})",
                package_id,
                variant,
                response.status()
            );
            // Create local placeholder
            let marker_path = package_dir.join(".local");
            std::fs::write(&marker_path, format!("{}@{} (local)", package_id, variant))
                .context("Failed to write marker file")?;
            vec![marker_path]
        }
        Err(e) => {
            tracing::warn!("📦 Failed to fetch package from registry: {}", e);
            // Create local placeholder
            let marker_path = package_dir.join(".local");
            std::fs::write(&marker_path, format!("{}@{} (offline)", package_id, variant))
                .context("Failed to write marker file")?;
            vec![marker_path]
        }
    };

    // Update index
    let package_info = PackageInfo {
        id: full_id.clone(),
        name: package_id.to_string(),
        version: "latest".to_string(),
        variant: variant.to_string(),
        installed_files: installed_files.clone(),
        description: None,
        dependencies: Vec::new(),
    };
    index.packages.insert(full_id, package_info);
    index.save(&project_root)?;

    crate::api::events::emit_package_installation_success(package_id)?;

    Ok(installed_files)
}

/// Uninstalls a package and removes its files.
///
/// Removes all variants of the specified package from the local installation
/// and cleans up the package index.
///
/// # Arguments
///
/// * `package_id` - The unique identifier of the package to uninstall
///
/// # Returns
///
/// A list of paths to files that were removed.
pub fn uninstall_package_safely(package_id: &str) -> Result<Vec<PathBuf>> {
    tracing::info!("🗑️  Uninstalling package: {}", package_id);

    let project_root = get_project_root()?;
    let mut index = PackageIndex::load(&project_root)?;

    // Find all variants of this package
    let keys_to_remove: Vec<String> = index
        .packages
        .keys()
        .filter(|k| k.starts_with(&format!("{}:", package_id)) || *k == package_id)
        .cloned()
        .collect();

    if keys_to_remove.is_empty() {
        tracing::warn!("🗑️  Package '{}' not found in index", package_id);
        return Ok(Vec::new());
    }

    let mut removed_files = Vec::new();

    for key in &keys_to_remove {
        if let Some(package_info) = index.packages.remove(key) {
            // Remove installed files
            for file in &package_info.installed_files {
                if file.exists() {
                    if let Err(e) = std::fs::remove_file(file) {
                        tracing::warn!("Failed to remove file {:?}: {}", file, e);
                    } else {
                        removed_files.push(file.clone());
                    }
                }
            }

            // Try to remove package directory
            let package_dir = project_root
                .join(".dx")
                .join("packages")
                .join(&package_info.name)
                .join(&package_info.variant);

            if package_dir.exists() {
                if let Err(e) = std::fs::remove_dir_all(&package_dir) {
                    tracing::warn!("Failed to remove package directory {:?}: {}", package_dir, e);
                }
            }
        }
    }

    // Remove pinned version if exists
    index.pinned_versions.remove(package_id);

    index.save(&project_root)?;

    tracing::info!("🗑️  Removed {} files for package '{}'", removed_files.len(), package_id);

    Ok(removed_files)
}

/// Updates a package to the latest version.
///
/// Checks for updates to all installed variants of the package and reinstalls
/// them if newer versions are available.
///
/// # Status
///
/// **Partially implemented** - Currently performs a full reinstall rather than
/// an incremental update. Version comparison with the registry is not yet implemented.
///
/// # Arguments
///
/// * `package_id` - The unique identifier of the package to update
///
/// # Returns
///
/// A list of paths to updated files.
pub async fn update_package_intelligently(package_id: &str) -> Result<Vec<PathBuf>> {
    tracing::info!("🔄 Intelligently updating package: {}", package_id);

    let project_root = get_project_root()?;
    let index = PackageIndex::load(&project_root)?;

    // Find installed variants
    let installed_variants: Vec<&PackageInfo> =
        index.packages.values().filter(|p| p.name == package_id).collect();

    if installed_variants.is_empty() {
        return Err(anyhow::anyhow!("Package '{}' is not installed", package_id));
    }

    // Check for pinned version
    if let Some(pinned) = index.pinned_versions.get(package_id) {
        tracing::info!(
            "🔄 Package '{}' is pinned to version {}, skipping update",
            package_id,
            pinned
        );
        return Ok(Vec::new());
    }

    let mut updated_files = Vec::new();

    // Update each variant
    for package_info in installed_variants {
        tracing::info!(
            "🔄 Checking for updates to '{}' variant '{}'",
            package_id,
            package_info.variant
        );

        // In a full implementation, we would:
        // 1. Query registry for latest version
        // 2. Compare with installed version
        // 3. Download and apply update if newer
        // 4. Run branching for changed files

        // For now, just reinstall
        let variant = package_info.variant.clone();
        uninstall_package_safely(package_id)?;
        let files = install_package_with_variant(package_id, &variant).await?;
        updated_files.extend(files);
    }

    Ok(updated_files)
}

/// Lists all installed packages.
///
/// # Returns
///
/// A vector of `PackageInfo` for all packages in the local index.
pub fn list_all_installed_packages() -> Result<Vec<PackageInfo>> {
    let project_root = get_project_root()?;
    let index = PackageIndex::load(&project_root)?;

    Ok(index.packages.values().cloned().collect())
}

/// Searches the DX package registry.
///
/// Queries the registry for packages matching the search query.
///
/// # Status
///
/// **Partially implemented** - Makes HTTP requests to the registry but returns
/// an empty vector if the registry is unavailable or returns an error. The registry
/// endpoint may not be operational.
///
/// # Arguments
///
/// * `query` - The search query string
///
/// # Returns
///
/// A vector of matching `PackageInfo` results, or an empty vector if the search fails.
pub async fn search_dx_package_registry(query: &str) -> Result<Vec<PackageInfo>> {
    tracing::info!("🔍 Searching package registry: {}", query);

    let registry_url = get_registry_url();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .context("Failed to create HTTP client")?;

    let url = format!("{}/search?q={}", registry_url, url_encode(query));

    match client.get(&url).send().await {
        Ok(response) if response.status().is_success() => {
            match response.json::<Vec<PackageInfo>>().await {
                Ok(packages) => Ok(packages),
                Err(e) => {
                    tracing::warn!("Failed to parse search results: {}", e);
                    Ok(Vec::new())
                }
            }
        }
        Ok(response) => {
            tracing::warn!("Search request failed with status: {}", response.status());
            Ok(Vec::new())
        }
        Err(e) => {
            tracing::warn!("Failed to search registry: {}", e);
            Ok(Vec::new())
        }
    }
}

/// Pins a package to a specific version.
///
/// Prevents automatic updates from changing the package version.
///
/// # Arguments
///
/// * `package_id` - The unique identifier of the package
/// * `version` - The version to pin to
///
/// # Returns
///
/// `Ok(())` on successful pinning.
pub fn pin_package_to_exact_version(package_id: &str, version: &str) -> Result<()> {
    tracing::info!("📌 Pinning '{}' to version {}", package_id, version);

    let project_root = get_project_root()?;
    let mut index = PackageIndex::load(&project_root)?;

    index.pinned_versions.insert(package_id.to_string(), version.to_string());
    index.save(&project_root)?;

    tracing::info!("📌 Package '{}' pinned to version {}", package_id, version);

    Ok(())
}

/// Forks an existing package variant to create a new variant.
///
/// Copies all files from the source variant to a new variant directory,
/// allowing local customization.
///
/// # Arguments
///
/// * `package_id` - The unique identifier of the package
/// * `variant` - The source variant to fork from
/// * `new_variant_name` - The name for the new variant
///
/// # Returns
///
/// The name of the newly created variant.
pub fn fork_existing_variant(
    package_id: &str,
    variant: &str,
    new_variant_name: &str,
) -> Result<String> {
    tracing::info!(
        "🍴 Forking variant '{}' from '{}' to '{}'",
        variant,
        package_id,
        new_variant_name
    );

    let project_root = get_project_root()?;
    let mut index = PackageIndex::load(&project_root)?;

    let source_key = format!("{}:{}", package_id, variant);
    let source_info = index
        .packages
        .get(&source_key)
        .ok_or_else(|| anyhow::anyhow!("Source variant '{}:{}' not found", package_id, variant))?
        .clone();

    // Create new variant directory
    let source_dir = project_root.join(".dx").join("packages").join(package_id).join(variant);

    let target_dir = project_root
        .join(".dx")
        .join("packages")
        .join(package_id)
        .join(new_variant_name);

    if target_dir.exists() {
        return Err(anyhow::anyhow!("Variant '{}' already exists", new_variant_name));
    }

    // Copy files
    if source_dir.exists() {
        copy_dir_recursive(&source_dir, &target_dir)?;
    } else {
        std::fs::create_dir_all(&target_dir).context("Failed to create variant directory")?;
    }

    // Update index
    let new_key = format!("{}:{}", package_id, new_variant_name);
    let new_info = PackageInfo {
        id: new_key.clone(),
        name: package_id.to_string(),
        version: source_info.version.clone(),
        variant: new_variant_name.to_string(),
        installed_files: source_info
            .installed_files
            .iter()
            .map(|p| {
                let rel = p.strip_prefix(&source_dir).unwrap_or(p);
                target_dir.join(rel)
            })
            .collect(),
        description: source_info.description.clone(),
        dependencies: source_info.dependencies.clone(),
    };

    index.packages.insert(new_key, new_info);
    index.save(&project_root)?;

    Ok(new_variant_name.to_string())
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Publishes a package variant to the DX registry.
///
/// # Status
///
/// **Partially implemented** - Makes HTTP requests to the registry but the
/// registry endpoint may not be operational. Requires `DX_REGISTRY_TOKEN`
/// environment variable for authentication.
///
/// # Arguments
///
/// * `package_id` - The unique identifier of the package
/// * `variant` - The variant to publish
///
/// # Returns
///
/// The published package identifier on success.
///
/// # Errors
///
/// Returns an error if:
/// - The variant is not found locally
/// - `DX_REGISTRY_TOKEN` is not set
/// - The registry request fails
pub async fn publish_your_variant(package_id: &str, variant: &str) -> Result<String> {
    tracing::info!("📤 Publishing variant '{}' for package '{}'", variant, package_id);

    let project_root = get_project_root()?;
    let index = PackageIndex::load(&project_root)?;

    let key = format!("{}:{}", package_id, variant);
    let _package_info = index
        .packages
        .get(&key)
        .ok_or_else(|| anyhow::anyhow!("Variant '{}:{}' not found", package_id, variant))?;

    let registry_url = get_registry_url();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .context("Failed to create HTTP client")?;

    // Check for auth token
    let auth_token = std::env::var("DX_REGISTRY_TOKEN")
        .context("DX_REGISTRY_TOKEN not set. Please authenticate with the registry first.")?;

    let url = format!("{}/packages/{}/variants/{}/publish", registry_url, package_id, variant);

    match client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => {
            let published_id = format!("{}-{}", package_id, variant);
            tracing::info!("📤 Successfully published '{}'", published_id);
            Ok(published_id)
        }
        Ok(response) => Err(anyhow::anyhow!("Failed to publish: {}", response.status())),
        Err(e) => Err(anyhow::anyhow!("Failed to connect to registry: {}", e)),
    }
}
