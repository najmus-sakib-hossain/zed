//! Build package for distribution
//!
//! This command builds wheel and/or sdist packages using PEP 517 build backends.

use std::path::Path;

use dx_py_core::Result;
use dx_py_package_manager::BuildFrontend;

/// Run the build command
pub fn run(output: &str, wheel_only: bool, sdist_only: bool) -> Result<()> {
    let project_dir = std::env::current_dir()
        .map_err(|e| dx_py_core::Error::Cache(format!("Failed to get current directory: {}", e)))?;

    let build_frontend = BuildFrontend::new(&project_dir)?;

    let name = build_frontend
        .name()
        .ok_or_else(|| dx_py_core::Error::Cache("No project name in pyproject.toml".to_string()))?;

    let version = build_frontend.version().unwrap_or("0.0.0");

    println!("Building {} v{}...", name, version);
    println!("  Build backend: {}", build_frontend.build_backend());

    let requires = build_frontend.build_requires();
    if !requires.is_empty() {
        println!("  Build requires: {}", requires.join(", "));
    }

    // Create output directory
    let output_dir = Path::new(output);
    if !output_dir.exists() {
        std::fs::create_dir_all(output_dir)?;
    }

    let build_wheel = !sdist_only;
    let build_sdist = !wheel_only;

    let mut built_files = Vec::new();

    if build_wheel {
        println!("\nBuilding wheel...");
        match build_frontend.build_wheel(output_dir) {
            Ok(wheel_path) => {
                println!("  ✓ Created: {}", wheel_path.display());
                built_files.push(wheel_path);
            }
            Err(e) => {
                eprintln!("  ✗ Failed to build wheel: {}", e);
                if !build_sdist {
                    return Err(e);
                }
            }
        }
    }

    if build_sdist {
        println!("\nBuilding sdist...");
        match build_frontend.build_sdist(output_dir) {
            Ok(sdist_path) => {
                println!("  ✓ Created: {}", sdist_path.display());
                built_files.push(sdist_path);
            }
            Err(e) => {
                eprintln!("  ✗ Failed to build sdist: {}", e);
                if built_files.is_empty() {
                    return Err(e);
                }
            }
        }
    }

    println!("\nBuild complete!");
    println!("Output directory: {}", output_dir.display());

    if !built_files.is_empty() {
        println!("\nBuilt files:");
        for file in &built_files {
            if let Ok(metadata) = std::fs::metadata(file) {
                let size = metadata.len();
                let size_str = if size > 1024 * 1024 {
                    format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
                } else if size > 1024 {
                    format!("{:.1} KB", size as f64 / 1024.0)
                } else {
                    format!("{} bytes", size)
                };
                println!(
                    "  {} ({})",
                    file.file_name().unwrap_or_default().to_string_lossy(),
                    size_str
                );
            } else {
                println!("  {}", file.display());
            }
        }

        println!("\nTo publish:");
        let files_arg = built_files
            .iter()
            .map(|f| f.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(",");
        println!("  dx-py publish --files {}", files_arg);
    }

    Ok(())
}
