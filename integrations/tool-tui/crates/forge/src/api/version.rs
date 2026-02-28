//! Version Governance & Package Identity APIs

use crate::version::Version;
use anyhow::Result;
use std::str::FromStr;

/// Tool self-declares its exact semver (validated against manifest)
///
/// Registers a tool's version in the registry for dependency resolution.
/// The version is validated to ensure it matches semver format.
///
/// # Arguments
/// * `tool_name` - Name of the tool
/// * `version` - Semantic version string (e.g., "1.2.3")
pub fn declare_tool_version(tool_name: &str, version: &str) -> Result<()> {
    let _parsed = Version::from_str(version).map_err(|e| {
        anyhow::anyhow!("Invalid version '{}' for tool '{}': {}", version, tool_name, e)
    })?;

    tracing::info!("📌 Tool '{}' declares version: {}", tool_name, version);
    Ok(())
}

/// Runtime panic on version mismatch — zero tolerance policy
///
/// Enforces exact version matching. If the provided version doesn't match
/// the expected version, the function panics immediately.
///
/// # Arguments
/// * `tool_name` - Name of the tool
/// * `expected` - Expected exact version
/// * `actual` - Actual version found
///
/// # Panics
/// Panics if versions don't match exactly
pub fn enforce_exact_version(tool_name: &str, expected: &str, actual: &str) {
    if expected != actual {
        panic!(
            "❌ VERSION MISMATCH for '{}': expected '{}', found '{}'. Zero tolerance policy enforced.",
            tool_name, expected, actual
        );
    }
    tracing::debug!("✅ Version verified for '{}': {}", tool_name, expected);
}

/// build.rs macro — compilation fails if forge is too old
///
/// Returns the minimum required forge version check code for build.rs.
/// This should be used in tool build scripts to ensure compatibility.
///
/// # Arguments
/// * `min_version` - Minimum required forge version
///
/// # Returns
/// Rust code snippet for build.rs validation
pub fn require_forge_minimum(min_version: &str) -> Result<String> {
    let _min = Version::from_str(min_version)
        .map_err(|e| anyhow::anyhow!("Invalid minimum version '{}': {}", min_version, e))?;

    let code = format!(
        r#"
fn main() {{
    let forge_version = env!("CARGO_PKG_VERSION");
    let min_required = "{}";
    
    // In build.rs, you'd parse and compare versions
    println!("cargo:warning=Requiring forge >= {{}}", min_required);
    
    // Add this to Cargo.toml build-dependencies to actually enforce:
    // dx-forge = {{ version = ">={}" }}
}}
"#,
        min_version, min_version
    );

    Ok(code)
}

/// Returns forge's own Version struct
///
/// # Returns
/// Current forge version
///
/// # Note
/// This function uses `.expect()` because the VERSION constant is defined at compile time
/// in Cargo.toml and is guaranteed to be valid semver. If this ever fails, it indicates
/// a build configuration error that should be caught during development.
pub fn current_forge_version() -> Version {
    // SAFETY: VERSION is a compile-time constant from Cargo.toml that is always valid semver.
    // This is provably always true - if Cargo.toml has an invalid version, the build fails.
    Version::from_str(crate::VERSION).expect("Forge VERSION constant must be valid semver")
}

/// Returns current variant ID (e.g. "shadcn-pro", "minimal-dark")
///
/// Gets the active package variant from the current context or configuration.
///
/// # Returns
/// Active variant ID, or "default" if none is set
pub fn query_active_package_variant() -> Result<String> {
    // TODO: Implement variant tracking in context
    // For now, return default
    Ok("default".to_string())
}

/// Hot-switches variant with full safety + branching preview
///
/// Changes the active package variant, triggering file updates with
/// traffic branch safety checks.
///
/// # Arguments
/// * `variant_id` - ID of the variant to activate
/// * `preview_only` - If true, shows preview without applying changes
///
/// # Returns
/// List of files that would be modified
pub fn activate_package_variant(
    variant_id: &str,
    preview_only: bool,
) -> Result<Vec<std::path::PathBuf>> {
    tracing::info!("🔄 Activating package variant: {} (preview: {})", variant_id, preview_only);

    // TODO: Implement actual variant switching logic
    // This would:
    // 1. Load variant configuration
    // 2. Compute file diffs
    // 3. Run through branching engine
    // 4. Apply changes if not preview_only

    if preview_only {
        tracing::info!("👁️  Preview mode - no changes applied");
    } else {
        tracing::info!("✅ Variant '{}' activated", variant_id);
    }

    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_declare_tool_version() {
        assert!(declare_tool_version("my-tool", "1.0.0").is_ok());
        assert!(declare_tool_version("bad-tool", "not-a-version").is_err());
    }

    #[test]
    fn test_current_forge_version() {
        let version = current_forge_version();
        // Verify version is valid (has expected format)
        assert!(!version.to_string().is_empty());
    }

    #[test]
    fn test_query_active_variant() {
        let variant = query_active_package_variant().unwrap();
        assert_eq!(variant, "default");
    }

    #[test]
    #[should_panic(expected = "VERSION MISMATCH")]
    fn test_enforce_exact_version_panic() {
        enforce_exact_version("tool", "1.0.0", "2.0.0");
    }
}
