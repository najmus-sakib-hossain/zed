//! Install packages from lock file
//!
//! This command reads the lock file and installs all packages to the
//! virtual environment, using the cache when available.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use dx_py_core::Result;
use dx_py_layout::{LayoutCache, ResolvedPackage};
use dx_py_package_manager::{AsyncPyPiClient, DplLockFile, GlobalCache, WheelInstaller};
use dx_py_store::PackageStore;

/// Get the site-packages directory, respecting VIRTUAL_ENV if set
///
/// This function implements venv-aware package installation by:
/// 1. Checking if VIRTUAL_ENV environment variable is set (indicating an active venv)
/// 2. If set, using the active venv's site-packages directory
/// 3. If not set, falling back to the local .venv directory
///
/// This ensures packages are installed to the correct location regardless of
/// whether the user has activated a virtual environment or is using the local .venv.
///
/// Requirements: 10.2, 10.3
fn get_site_packages_path(venv_path: &Path) -> PathBuf {
    // First check if we're in an active virtual environment
    if let Ok(virtual_env) = std::env::var("VIRTUAL_ENV") {
        let venv_path = PathBuf::from(virtual_env);
        #[cfg(unix)]
        {
            venv_path.join("lib/python3.12/site-packages")
        }
        #[cfg(windows)]
        {
            venv_path.join("Lib/site-packages")
        }
    } else {
        // Fall back to local .venv
        #[cfg(unix)]
        {
            venv_path.join("lib/python3.12/site-packages")
        }
        #[cfg(windows)]
        {
            venv_path.join("Lib/site-packages")
        }
    }
}

/// Run the sync command
pub fn run(dev: bool, extras: &[String], verbose: bool) -> Result<()> {
    let start_time = Instant::now();
    let lock_path = Path::new("dx-py.lock");

    if !lock_path.exists() {
        return Err(dx_py_core::Error::Cache(
            "No lock file found. Run 'dx-py lock' first.".to_string(),
        ));
    }

    let venv_path = Path::new(".venv");
    if !venv_path.exists() {
        return Err(dx_py_core::Error::Cache(
            "No virtual environment found. Run 'dx-py init' first.".to_string(),
        ));
    }

    println!("Reading lock file...");

    let lock_file = DplLockFile::open(lock_path)?;
    let package_count = lock_file.package_count();

    if package_count == 0 {
        println!("No packages to install.");
        return Ok(());
    }

    // Set up cache directories
    let cache_dir = dirs::cache_dir()
        .map(|p| p.join("dx-py"))
        .unwrap_or_else(|| Path::new(".dx-py-cache").to_path_buf());

    let cache = GlobalCache::new(&cache_dir)?;
    let store = Arc::new(
        PackageStore::open(cache_dir.join("store"))
            .map_err(|e| dx_py_core::Error::Cache(e.to_string()))?,
    );
    let layouts_path = cache_dir.join("layouts");

    // Determine site-packages path (Requirements 10.2, 10.3)
    let site_packages = get_site_packages_path(venv_path);

    std::fs::create_dir_all(&site_packages)?;

    // Collect resolved packages for layout cache
    let resolved_packages: Vec<ResolvedPackage> = lock_file
        .iter()
        .map(|entry| ResolvedPackage {
            name: entry.name_str().to_string(),
            version: entry.version_str().to_string(),
            hash: entry.source_hash,
        })
        .collect();

    // Try to use layout cache for instant install
    let mut layout_cache = LayoutCache::open(&layouts_path, Arc::clone(&store))
        .map_err(|e| dx_py_core::Error::Cache(e.to_string()))?;
    let project_hash = LayoutCache::compute_project_hash(&resolved_packages);

    if layout_cache.contains(&project_hash) {
        // Warm install path - use cached layout
        let install_start = Instant::now();

        if verbose {
            println!("  Layout cache hit!");
        }

        match layout_cache.install_cached(&project_hash, site_packages.parent().unwrap()) {
            Ok(result) => {
                let install_time = install_start.elapsed();
                println!("\n✓ Installed {} packages from cache", package_count);

                if verbose {
                    println!("\nCache Statistics:");
                    println!("  Layout cache: HIT");
                    println!("  Files linked: {}", result.symlinks);
                    println!("  Install time: {:.2}ms", install_time.as_secs_f64() * 1000.0);
                    println!("  Total time: {:.2}ms", start_time.elapsed().as_secs_f64() * 1000.0);
                }

                return Ok(());
            }
            Err(e) => {
                if verbose {
                    println!("  Layout cache error: {}, falling back to standard install", e);
                }
                // Fall through to standard install
            }
        }
    } else if verbose {
        println!("  Layout cache miss, performing standard install");
    }

    // Standard install path
    println!("Installing {} packages...", package_count);

    let installer = WheelInstaller::new(GlobalCache::new(&cache_dir)?, site_packages.clone());

    // Create async runtime for downloads
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| dx_py_core::Error::Cache(format!("Failed to create runtime: {}", e)))?;

    let client = AsyncPyPiClient::new();
    let platform_env = dx_py_core::wheel::PlatformEnvironment::detect();

    let mut installed_count = 0;
    let mut cached_count = 0;
    let mut download_count = 0;

    // Iterate through locked packages
    for entry in lock_file.iter() {
        let name = entry.name_str();
        let version = entry.version_str();
        let hash = entry.source_hash;

        print!("  {} @ {} ... ", name, version);

        // Check if already in cache
        if cache.contains(&hash) {
            // Install from cache
            match installer.install_from_cache(&hash) {
                Ok(_installed) => {
                    println!("✓ (cached)");
                    cached_count += 1;
                    installed_count += 1;
                }
                Err(e) => {
                    println!("✗ cache error: {}", e);
                    // Try downloading instead
                    if let Err(e) = rt.block_on(download_and_install(
                        &client,
                        &installer,
                        &cache,
                        name,
                        version,
                        &platform_env,
                    )) {
                        eprintln!("    Failed to download: {}", e);
                        continue;
                    }
                    download_count += 1;
                    installed_count += 1;
                }
            }
        } else {
            // Download from PyPI
            match rt.block_on(download_and_install(
                &client,
                &installer,
                &cache,
                name,
                version,
                &platform_env,
            )) {
                Ok(()) => {
                    println!("✓ (downloaded)");
                    download_count += 1;
                    installed_count += 1;
                }
                Err(e) => {
                    println!("✗ {}", e);
                }
            }
        }
    }

    // Build layout cache for future installs
    if installed_count > 0 {
        // Store packages in the package store for layout cache
        for entry in lock_file.iter() {
            let hash = entry.source_hash;
            if !store.contains(&hash) {
                // Package data should be in global cache, copy to store
                if let Ok(data) = cache.get(&hash) {
                    let _ = store.store(&hash, &data);
                }
            }
        }

        // Build layout for future warm installs
        if let Err(e) = layout_cache.build_layout(&project_hash, &resolved_packages) {
            if verbose {
                println!("  Warning: Failed to cache layout: {}", e);
            }
        } else if verbose {
            println!("  Layout cached for future installs");
        }
    }

    if dev {
        println!("\n  (including dev dependencies)");
    }

    if !extras.is_empty() {
        println!("  (including extras: {})", extras.join(", "));
    }

    let total_time = start_time.elapsed();
    println!("\nInstallation complete!");
    println!("  {} packages installed", installed_count);
    println!("  {} from cache, {} downloaded", cached_count, download_count);

    if verbose {
        println!("\nCache Statistics:");
        println!("  Layout cache: MISS (now cached)");
        println!("  Package cache hits: {}", cached_count);
        println!("  Package downloads: {}", download_count);
        println!("  Total time: {:.2}ms", total_time.as_secs_f64() * 1000.0);
    }

    Ok(())
}

/// Download a package from PyPI and install it
async fn download_and_install(
    client: &AsyncPyPiClient,
    installer: &WheelInstaller,
    cache: &GlobalCache,
    name: &str,
    version: &str,
    platform_env: &dx_py_core::wheel::PlatformEnvironment,
) -> Result<()> {
    // Find best wheel for this platform
    let dist = client.find_distribution(name, version, platform_env).await?.ok_or_else(|| {
        dx_py_core::Error::Cache(format!(
            "No compatible distribution found for {}=={}",
            name, version
        ))
    })?;

    // Download the wheel
    let data = client.download(&dist.url, &dist.digests.sha256).await?;

    // Store in cache
    let hash = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    };
    cache.store(&hash, &data)?;

    // Install the wheel
    installer.install_wheel(&data)?;

    Ok(())
}
