//! Instant O(1) installer - Single symlink for entire node_modules!
//!
//! This is the breakthrough: Instead of O(n) file operations,
//! we do O(1) - just symlink to a pre-built layout.

use dx_pkg_layout::{compute_packages_hash, LayoutCache, ResolvedPackage};
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

/// Installation result
#[derive(Debug)]
pub struct InstantInstallResult {
    /// Installation time
    pub duration: Duration,
    /// Number of packages
    pub package_count: usize,
    /// Cache status
    pub cache_status: CacheStatus,
}

/// Cache status indicator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStatus {
    /// Full layout cache hit - O(1) symlink (INSTANT!)
    LayoutHit,
    /// Packages extracted, layout built (fast)
    LayoutBuilt,
    /// Some packages need extraction (medium)
    PartialHit,
    /// Full cold install needed (slow)
    ColdInstall,
}

/// Instant installer
pub struct InstantInstaller {
    layout_cache: LayoutCache,
}

impl InstantInstaller {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            layout_cache: LayoutCache::new()?,
        })
    }

    /// Try instant install (returns None if cold install needed)
    pub fn try_install(
        &mut self,
        packages: &[ResolvedPackage],
    ) -> io::Result<Option<InstantInstallResult>> {
        let start = Instant::now();

        // Compute project hash
        let project_hash = compute_packages_hash(packages);

        // Check for cached layout
        if self.layout_cache.has_layout(project_hash) {
            // O(1) INSTALL - SINGLE SYMLINK!
            let layout_path = self.layout_cache.layout_path(project_hash);
            let node_modules = std::env::current_dir()?.join("node_modules");

            // Remove existing node_modules if present
            if node_modules.exists() {
                // Check if it's already our symlink/junction
                #[cfg(unix)]
                {
                    if let Ok(target) = node_modules.read_link() {
                        if target == layout_path {
                            return Ok(Some(InstantInstallResult {
                                duration: start.elapsed(),
                                package_count: packages.len(),
                                cache_status: CacheStatus::LayoutHit,
                            }));
                        }
                    }
                }

                // Remove old node_modules
                #[cfg(windows)]
                {
                    // On Windows, try to delete junction first
                    let _ = junction::delete(&node_modules);
                    let _ = std::fs::remove_dir_all(&node_modules);
                }

                #[cfg(unix)]
                {
                    if node_modules.is_symlink() || node_modules.read_link().is_ok() {
                        std::fs::remove_file(&node_modules)?;
                    } else {
                        std::fs::remove_dir_all(&node_modules)?;
                    }
                }
            }

            // Create symlink - THIS IS THE ENTIRE INSTALL!
            self.create_symlink(&layout_path, &node_modules)?;

            return Ok(Some(InstantInstallResult {
                duration: start.elapsed(),
                package_count: packages.len(),
                cache_status: CacheStatus::LayoutHit,
            }));
        }

        // Check what's missing
        let mut all_extracted = true;
        for pkg in packages {
            if !self.layout_cache.has_extracted(&pkg.name, &pkg.version) {
                all_extracted = false;
                break;
            }
        }

        // If all packages extracted, just build layout
        if all_extracted {
            let layout_path = self.layout_cache.build_layout(project_hash, packages)?;
            let node_modules = std::env::current_dir()?.join("node_modules");

            if node_modules.exists() {
                #[cfg(windows)]
                {
                    let _ = junction::delete(&node_modules);
                    let _ = std::fs::remove_dir_all(&node_modules);
                }

                #[cfg(unix)]
                {
                    if node_modules.is_symlink() || node_modules.read_link().is_ok() {
                        std::fs::remove_file(&node_modules)?;
                    } else {
                        std::fs::remove_dir_all(&node_modules)?;
                    }
                }
            }

            self.create_symlink(&layout_path, &node_modules)?;

            return Ok(Some(InstantInstallResult {
                duration: start.elapsed(),
                package_count: packages.len(),
                cache_status: CacheStatus::LayoutBuilt,
            }));
        }

        // Need cold install
        Ok(None)
    }

    /// Extract missing packages and build layout
    pub fn extract_and_install(
        &mut self,
        packages: &[ResolvedPackage],
        tarball_cache_dir: &Path,
    ) -> io::Result<InstantInstallResult> {
        let start = Instant::now();

        // Extract missing packages
        for pkg in packages {
            if !self.layout_cache.has_extracted(&pkg.name, &pkg.version) {
                let tarball_name = format!("{}-{}.tgz", pkg.name.replace('/', "-"), pkg.version);
                let tarball_path = tarball_cache_dir.join(tarball_name);

                if tarball_path.exists() {
                    self.layout_cache.ensure_extracted(&pkg.name, &pkg.version, &tarball_path)?;
                } else {
                    // Tarball not cached - need full download
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("Tarball not found for {}-{}", pkg.name, pkg.version),
                    ));
                }
            }
        }

        // Build layout
        let project_hash = compute_packages_hash(packages);
        let layout_path = self.layout_cache.build_layout(project_hash, packages)?;

        // Install
        let node_modules = std::env::current_dir()?.join("node_modules");

        if node_modules.exists() {
            #[cfg(windows)]
            {
                let _ = junction::delete(&node_modules);
                let _ = std::fs::remove_dir_all(&node_modules);
            }

            #[cfg(unix)]
            {
                if node_modules.is_symlink() || node_modules.read_link().is_ok() {
                    std::fs::remove_file(&node_modules)?;
                } else {
                    std::fs::remove_dir_all(&node_modules)?;
                }
            }
        }

        self.create_symlink(&layout_path, &node_modules)?;

        Ok(InstantInstallResult {
            duration: start.elapsed(),
            package_count: packages.len(),
            cache_status: CacheStatus::PartialHit,
        })
    }

    /// Platform-specific symlink creation
    fn create_symlink(&self, target: &Path, link: &Path) -> io::Result<()> {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, link)?;
        }

        #[cfg(windows)]
        {
            // Use junction on Windows (no admin rights needed)
            junction::create(target, link).map_err(io::Error::other)?;
        }

        Ok(())
    }

    /// Get layout cache (for external operations)
    pub fn layout_cache_mut(&mut self) -> &mut LayoutCache {
        &mut self.layout_cache
    }

    pub fn layout_cache(&self) -> &LayoutCache {
        &self.layout_cache
    }
}

/// Format speedup ratio
pub fn format_speedup(dx_time: Duration, baseline_time: Duration) -> String {
    let speedup = baseline_time.as_secs_f64() / dx_time.as_secs_f64();
    if speedup >= 10.0 {
        format!("{:.0}x", speedup)
    } else {
        format!("{:.1}x", speedup)
    }
}
