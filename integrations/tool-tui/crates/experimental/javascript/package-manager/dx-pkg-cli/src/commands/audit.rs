//! dx audit command - Security vulnerability auditing

use anyhow::Result;
use dx_pkg_audit::{print_audit_report, PackageAuditor};
use std::collections::HashMap;

/// Run the audit command
pub async fn run(verbose: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;

    if verbose {
        println!("🔍 Auditing packages for security vulnerabilities...\n");
    }

    let auditor = PackageAuditor::new();

    // Try to find lockfile first
    let lockfile_path = cwd.join("dx.lock");
    let package_json_path = cwd.join("package.json");

    let report = if lockfile_path.exists() {
        if verbose {
            println!("📄 Using lockfile: {}", lockfile_path.display());
        }
        auditor.audit_lockfile(&lockfile_path)?
    } else if package_json_path.exists() {
        if verbose {
            println!("📄 Using package.json: {}", package_json_path.display());
        }

        // Read package.json and extract dependencies
        let content = std::fs::read_to_string(&package_json_path)?;
        let pkg: serde_json::Value = serde_json::from_str(&content)?;

        let mut all_deps = HashMap::new();

        // Collect dependencies
        if let Some(deps) = pkg.get("dependencies").and_then(|d| d.as_object()) {
            for (name, version) in deps {
                if let Some(v) = version.as_str() {
                    all_deps.insert(name.clone(), v.to_string());
                }
            }
        }

        // Collect devDependencies
        if let Some(deps) = pkg.get("devDependencies").and_then(|d| d.as_object()) {
            for (name, version) in deps {
                if let Some(v) = version.as_str() {
                    all_deps.insert(name.clone(), v.to_string());
                }
            }
        }

        auditor.audit_dependencies(&all_deps)
    } else {
        anyhow::bail!("No package.json or dx.lock found in current directory");
    };

    // Print the report
    print_audit_report(&report);

    // Exit with error code if audit failed
    if !report.passed() {
        std::process::exit(1);
    }

    Ok(())
}
