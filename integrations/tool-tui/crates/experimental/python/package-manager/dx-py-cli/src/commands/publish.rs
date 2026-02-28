//! Publish package to PyPI
//!
//! This command uploads wheel and sdist packages to PyPI or compatible registries.

use std::path::Path;

use dx_py_core::Result;
use dx_py_package_manager::{PublishClient, DEFAULT_REPOSITORY_URL, TEST_PYPI_URL};

/// Run the publish command
pub fn run(repository: Option<&str>, token: Option<&str>, files: &str) -> Result<()> {
    // Determine repository URL
    let (repo_url, repo_name) = match repository {
        Some("testpypi") => (TEST_PYPI_URL, "TestPyPI"),
        Some("pypi") => (DEFAULT_REPOSITORY_URL, "PyPI"),
        Some(url) => (url, "custom repository"),
        None => (DEFAULT_REPOSITORY_URL, "PyPI"),
    };

    // Get API token
    let api_token = match token {
        Some(t) => t.to_string(),
        None => {
            // Try environment variables in order of preference
            std::env::var("DX_PY_TOKEN")
                .or_else(|_| std::env::var("PYPI_TOKEN"))
                .or_else(|_| std::env::var("TWINE_PASSWORD"))
                .map_err(|_| {
                    dx_py_core::Error::Cache(
                        "No API token provided. Use --token or set DX_PY_TOKEN environment variable.\n\
                         Get a token from https://pypi.org/manage/account/token/"
                            .to_string(),
                    )
                })?
        }
    };

    // Parse file paths
    let file_paths: Vec<&Path> = files.split(',').map(|s| Path::new(s.trim())).collect();

    // Validate files exist
    for path in &file_paths {
        if !path.exists() {
            return Err(dx_py_core::Error::Cache(format!("File not found: {}", path.display())));
        }

        // Validate file type
        let filename =
            path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();

        if !filename.ends_with(".whl") && !filename.ends_with(".tar.gz") {
            return Err(dx_py_core::Error::Cache(format!(
                "Invalid file type: {}. Expected .whl or .tar.gz",
                filename
            )));
        }
    }

    println!("Publishing to {}...", repo_name);
    println!("Repository: {}", repo_url);
    println!("Files: {}", file_paths.len());

    // Create publish client
    let client = PublishClient::with_repository(repo_url);

    let mut success_count = 0;
    let mut error_count = 0;

    // Upload each file
    for path in &file_paths {
        let filename =
            path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();

        print!("\n  Uploading {}... ", filename);

        match client.upload(path, &api_token) {
            Ok(result) => {
                println!("✓");
                println!("    Package: {} v{}", result.name, result.version);
                println!("    SHA256: {}...", &result.sha256[..16]);
                success_count += 1;
            }
            Err(e) => {
                println!("✗");
                eprintln!("    Error: {}", e);
                error_count += 1;
            }
        }
    }

    println!();

    if error_count == 0 {
        println!("✓ Publish complete! {} file(s) uploaded.", success_count);

        // Show package URL
        if repo_url == DEFAULT_REPOSITORY_URL {
            // Try to extract package name from first file
            if let Some(first_file) = file_paths.first() {
                if let Some(name) = extract_package_name(first_file) {
                    println!("\nView at: https://pypi.org/project/{}/", name);
                }
            }
        } else if repo_url == TEST_PYPI_URL {
            if let Some(first_file) = file_paths.first() {
                if let Some(name) = extract_package_name(first_file) {
                    println!("\nView at: https://test.pypi.org/project/{}/", name);
                }
            }
        }
    } else if success_count > 0 {
        println!("⚠ Partial success: {} uploaded, {} failed", success_count, error_count);
    } else {
        println!("✗ Publish failed: all {} file(s) failed to upload", error_count);
    }

    if error_count > 0 {
        Err(dx_py_core::Error::Cache(format!("{} file(s) failed to upload", error_count)))
    } else {
        Ok(())
    }
}

/// Extract package name from wheel or sdist filename
fn extract_package_name(path: &Path) -> Option<String> {
    let filename = path.file_name()?.to_string_lossy();

    if filename.ends_with(".whl") {
        // Wheel format: {name}-{version}(-{build})?-{python}-{abi}-{platform}.whl
        let parts: Vec<&str> = filename.split('-').collect();
        if !parts.is_empty() {
            return Some(parts[0].replace('_', "-"));
        }
    } else if filename.ends_with(".tar.gz") {
        // Sdist format: {name}-{version}.tar.gz
        let without_ext = filename.strip_suffix(".tar.gz")?;
        let parts: Vec<&str> = without_ext.rsplitn(2, '-').collect();
        if parts.len() == 2 {
            return Some(parts[1].replace('_', "-"));
        }
    }

    None
}
