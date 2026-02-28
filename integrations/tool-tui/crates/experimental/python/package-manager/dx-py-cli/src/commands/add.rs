//! Add dependencies to the project
//!
//! This module implements the `dx-py add` command which adds packages
//! to pyproject.toml while preserving formatting and comments.

use std::fs;
use std::path::Path;

use dx_py_core::Result;
use toml_edit::{Array, DocumentMut, Item, Table, Value};

/// Validate a package name according to PEP 508
fn validate_package_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_alphanumeric() {
        return false;
    }

    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

/// Extract the package name from a dependency string
fn extract_package_name(dep: &str) -> &str {
    let specifier_chars = ['>', '<', '=', '!', '~', '[', ';', '@'];
    
    for (i, c) in dep.char_indices() {
        if specifier_chars.contains(&c) {
            return dep[..i].trim();
        }
    }
    
    dep.trim()
}

/// Parse a package specification into name and optional version constraint
fn parse_package_spec(spec: &str) -> (&str, Option<&str>) {
    let name = extract_package_name(spec);
    let version = if spec.len() > name.len() {
        Some(spec[name.len()..].trim())
    } else {
        None
    };
    (name, version)
}

/// Result of adding a package
enum AddResult {
    Added,
    Updated,
    AlreadyExists,
}

/// Run the add command
pub fn run(packages: &[String], dev: bool, optional: Option<&str>) -> Result<()> {
    let pyproject_path = Path::new("pyproject.toml");

    if !pyproject_path.exists() {
        eprintln!("Error: No pyproject.toml found. Run 'dx-py init' first.");
        return Err(dx_py_core::Error::Cache(
            "No pyproject.toml found. Run 'dx-py init' first.".to_string(),
        ));
    }

    let content = fs::read_to_string(pyproject_path)?;
    
    let mut doc: DocumentMut = content.parse().map_err(|e| {
        eprintln!("Error: Failed to parse pyproject.toml: {}", e);
        dx_py_core::Error::Cache(format!("Failed to parse pyproject.toml: {}", e))
    })?;

    if doc.get("project").is_none() {
        eprintln!("Error: No [project] section in pyproject.toml");
        return Err(dx_py_core::Error::Cache(
            "No [project] section in pyproject.toml".to_string(),
        ));
    }

    let mut added_count = 0;
    let mut updated_count = 0;
    let mut skipped_count = 0;

    for package in packages {
        let (pkg_name, _version) = parse_package_spec(package);
        
        if !validate_package_name(pkg_name) {
            eprintln!(
                "Error: Invalid package name '{}'. Names must start with a letter/digit.",
                pkg_name
            );
            return Err(dx_py_core::Error::Cache(format!(
                "Invalid package name: {}",
                pkg_name
            )));
        }

        if dev || optional.is_some() {
            let group = optional.unwrap_or("dev");
            let result = add_to_optional_dependencies(&mut doc, package, group)?;
            match result {
                AddResult::Added => {
                    println!("Added {} to [project.optional-dependencies.{}]", package, group);
                    added_count += 1;
                }
                AddResult::Updated => {
                    println!("Updated {} in [project.optional-dependencies.{}]", package, group);
                    updated_count += 1;
                }
                AddResult::AlreadyExists => {
                    println!("{} already in [project.optional-dependencies.{}]", package, group);
                    skipped_count += 1;
                }
            }
        } else {
            let result = add_to_dependencies(&mut doc, package)?;
            match result {
                AddResult::Added => {
                    println!("Added {} to [project.dependencies]", package);
                    added_count += 1;
                }
                AddResult::Updated => {
                    println!("Updated {} in [project.dependencies]", package);
                    updated_count += 1;
                }
                AddResult::AlreadyExists => {
                    println!("{} already in [project.dependencies]", package);
                    skipped_count += 1;
                }
            }
        }
    }

    fs::write(pyproject_path, doc.to_string())?;

    if added_count > 0 || updated_count > 0 {
        println!();
        if added_count > 0 {
            println!("Successfully added {} package(s).", added_count);
        }
        if updated_count > 0 {
            println!("Successfully updated {} package(s).", updated_count);
        }
        println!("\nRun 'dx-py install' to install the new dependencies.");
    } else if skipped_count > 0 {
        println!("\nNo changes made. All packages already present.");
    }

    Ok(())
}


/// Add a package to [project.dependencies]
fn add_to_dependencies(doc: &mut DocumentMut, package: &str) -> Result<AddResult> {
    let project = doc["project"].as_table_mut().ok_or_else(|| {
        dx_py_core::Error::Cache("No [project] section in pyproject.toml".to_string())
    })?;

    if project.get("dependencies").is_none() {
        let mut arr = Array::new();
        arr.set_trailing_comma(true);
        project.insert("dependencies", Item::Value(Value::Array(arr)));
    }

    let deps = project["dependencies"].as_array_mut().ok_or_else(|| {
        dx_py_core::Error::Cache("dependencies is not an array".to_string())
    })?;

    let pkg_name = extract_package_name(package);

    // First pass: find if package exists and get its index
    let mut found_idx: Option<usize> = None;
    let mut is_same = false;
    
    for i in 0..deps.len() {
        if let Some(existing) = deps.get(i).and_then(|v| v.as_str()) {
            let existing_name = extract_package_name(existing);
            if existing_name.eq_ignore_ascii_case(pkg_name) {
                if existing == package {
                    is_same = true;
                }
                found_idx = Some(i);
                break;
            }
        }
    }

    if let Some(idx) = found_idx {
        if is_same {
            return Ok(AddResult::AlreadyExists);
        }
        deps.replace(idx, package);
        return Ok(AddResult::Updated);
    }

    deps.push(package);
    Ok(AddResult::Added)
}

/// Add a package to [project.optional-dependencies.<group>]
fn add_to_optional_dependencies(
    doc: &mut DocumentMut,
    package: &str,
    group: &str,
) -> Result<AddResult> {
    let project = doc["project"].as_table_mut().ok_or_else(|| {
        dx_py_core::Error::Cache("No [project] section in pyproject.toml".to_string())
    })?;

    if project.get("optional-dependencies").is_none() {
        project.insert("optional-dependencies", Item::Table(Table::new()));
    }

    let optional_deps = project["optional-dependencies"].as_table_mut().ok_or_else(|| {
        dx_py_core::Error::Cache("optional-dependencies is not a table".to_string())
    })?;

    if optional_deps.get(group).is_none() {
        let mut arr = Array::new();
        arr.set_trailing_comma(true);
        optional_deps.insert(group, Item::Value(Value::Array(arr)));
    }

    let group_deps = optional_deps[group].as_array_mut().ok_or_else(|| {
        dx_py_core::Error::Cache(format!("optional-dependencies.{} is not an array", group))
    })?;

    let pkg_name = extract_package_name(package);

    // First pass: find if package exists and get its index
    let mut found_idx: Option<usize> = None;
    let mut is_same = false;
    
    for i in 0..group_deps.len() {
        if let Some(existing) = group_deps.get(i).and_then(|v| v.as_str()) {
            let existing_name = extract_package_name(existing);
            if existing_name.eq_ignore_ascii_case(pkg_name) {
                if existing == package {
                    is_same = true;
                }
                found_idx = Some(i);
                break;
            }
        }
    }

    if let Some(idx) = found_idx {
        if is_same {
            return Ok(AddResult::AlreadyExists);
        }
        group_deps.replace(idx, package);
        return Ok(AddResult::Updated);
    }

    group_deps.push(package);
    Ok(AddResult::Added)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_package_name() {
        assert!(validate_package_name("requests"));
        assert!(validate_package_name("flask"));
        assert!(validate_package_name("my-package"));
        assert!(validate_package_name("my_package"));
        assert!(validate_package_name("my.package"));
        assert!(validate_package_name("package123"));
        assert!(validate_package_name("123package"));

        assert!(!validate_package_name(""));
        assert!(!validate_package_name("-package"));
        assert!(!validate_package_name("_package"));
        assert!(!validate_package_name(".package"));
        assert!(!validate_package_name("package!"));
        assert!(!validate_package_name("package@version"));
    }

    #[test]
    fn test_extract_package_name() {
        assert_eq!(extract_package_name("requests"), "requests");
        assert_eq!(extract_package_name("requests>=2.0"), "requests");
        assert_eq!(extract_package_name("requests==2.28.0"), "requests");
        assert_eq!(extract_package_name("requests~=2.28"), "requests");
        assert_eq!(extract_package_name("requests[security]"), "requests");
        assert_eq!(extract_package_name("requests>=2.0,<3.0"), "requests");
    }

    #[test]
    fn test_parse_package_spec() {
        let (name, version) = parse_package_spec("requests");
        assert_eq!(name, "requests");
        assert_eq!(version, None);

        let (name, version) = parse_package_spec("requests>=2.0");
        assert_eq!(name, "requests");
        assert_eq!(version, Some(">=2.0"));

        let (name, version) = parse_package_spec("requests==2.28.0");
        assert_eq!(name, "requests");
        assert_eq!(version, Some("==2.28.0"));
    }

    #[test]
    fn test_add_to_dependencies() {
        let toml = r#"
[project]
name = "test-package"
version = "1.0.0"
"#;
        let mut doc: DocumentMut = toml.parse().unwrap();
        
        let result = add_to_dependencies(&mut doc, "requests>=2.0").unwrap();
        assert!(matches!(result, AddResult::Added));
        
        let result = add_to_dependencies(&mut doc, "requests>=2.0").unwrap();
        assert!(matches!(result, AddResult::AlreadyExists));
        
        let result = add_to_dependencies(&mut doc, "requests>=3.0").unwrap();
        assert!(matches!(result, AddResult::Updated));
        
        let result = add_to_dependencies(&mut doc, "flask").unwrap();
        assert!(matches!(result, AddResult::Added));
    }

    #[test]
    fn test_add_to_optional_dependencies() {
        let toml = r#"
[project]
name = "test-package"
version = "1.0.0"
"#;
        let mut doc: DocumentMut = toml.parse().unwrap();
        
        let result = add_to_optional_dependencies(&mut doc, "pytest", "dev").unwrap();
        assert!(matches!(result, AddResult::Added));
        
        let result = add_to_optional_dependencies(&mut doc, "pytest", "dev").unwrap();
        assert!(matches!(result, AddResult::AlreadyExists));
        
        let result = add_to_optional_dependencies(&mut doc, "sphinx", "docs").unwrap();
        assert!(matches!(result, AddResult::Added));
    }

    #[test]
    fn test_preserves_formatting() {
        let toml = r#"# My project configuration
[project]
name = "test-package"
version = "1.0.0"

# Existing dependencies
dependencies = [
    "existing-package>=1.0",
]
"#;
        let mut doc: DocumentMut = toml.parse().unwrap();
        
        add_to_dependencies(&mut doc, "new-package").unwrap();
        
        let output = doc.to_string();
        assert!(output.contains("# My project configuration"));
        assert!(output.contains("# Existing dependencies"));
    }
}
