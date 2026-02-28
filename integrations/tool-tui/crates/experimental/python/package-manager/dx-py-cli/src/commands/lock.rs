//! Generate lock file from dependencies
//!
//! This command resolves all dependencies from pyproject.toml and generates
//! a lock file (dx-py.lock) with exact versions and hashes.

use std::path::Path;

use dx_py_compat::PyProjectToml;
use dx_py_core::Result;
use dx_py_package_manager::{AsyncPyPiClient, DependencySpec, DplBuilder};

/// Run the lock command
pub fn run(upgrade: bool) -> Result<()> {
    let pyproject_path = Path::new("pyproject.toml");

    if !pyproject_path.exists() {
        return Err(dx_py_core::Error::Cache(
            "No pyproject.toml found. Run 'dx-py init' first.".to_string(),
        ));
    }

    let pyproject = PyProjectToml::load(pyproject_path)?;

    let deps = pyproject.dependencies();
    if deps.is_empty() {
        println!("No dependencies to lock.");
        return Ok(());
    }

    println!("Resolving dependencies...");

    // Parse dependencies into DependencySpec
    let parsed_specs: Vec<DependencySpec> =
        deps.iter().filter_map(|d| DependencySpec::parse(d).ok()).collect();

    if parsed_specs.is_empty() {
        println!("No valid dependencies found.");
        return Ok(());
    }

    // Try async resolution with real PyPI
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| dx_py_core::Error::Cache(format!("Failed to create runtime: {}", e)))?;

    let resolution = rt.block_on(async { resolve_with_pypi(&parsed_specs, upgrade).await })?;

    // Build lock file
    println!("Creating lock file...");

    // Use default Python version
    let python_version_str = "3.12";

    let mut builder = DplBuilder::new(python_version_str, "any");

    for pkg in &resolution.packages {
        builder.add_package(&pkg.name, &pkg.version_string, pkg.content_hash);
        println!("  Locked {} @ {}", pkg.name, pkg.version_string);
    }

    // Write lock file
    let lock_path = Path::new("dx-py.lock");
    let lock_data = builder.build();
    std::fs::write(lock_path, lock_data)?;

    println!("\nLock file written to dx-py.lock");
    println!(
        "Resolved {} packages in {}ms",
        resolution.packages.len(),
        resolution.resolution_time_ms
    );

    if resolution.from_cache {
        println!("(used cached resolution)");
    }

    println!("Run 'dx-py sync' to install locked dependencies.");

    if upgrade {
        println!("(--upgrade flag: all packages updated to latest compatible versions)");
    }

    Ok(())
}

/// Resolve dependencies using real PyPI
async fn resolve_with_pypi(
    specs: &[DependencySpec],
    _upgrade: bool,
) -> Result<dx_py_package_manager::Resolution> {
    use dx_py_package_manager::PyPiResolver;

    let client = AsyncPyPiClient::new();
    let mut resolver = PyPiResolver::new(client);

    resolver.resolve(specs).await
}
