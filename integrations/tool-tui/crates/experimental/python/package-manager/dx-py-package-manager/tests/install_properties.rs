//! Property-Based Tests for Wheel Installation
//!
//! Feature: dx-py-production-ready
//! Tests Properties 17 and 18 from the design document

use dx_py_package_manager::cache::GlobalCache;
use dx_py_package_manager::installer::{RecordEntry, WheelInstaller};
use proptest::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::io::{Cursor, Write};
use tempfile::TempDir;
use zip::write::FileOptions;
use zip::ZipWriter;

/// Generate a valid Python package name (lowercase, underscores, alphanumeric)
fn package_name_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{2,20}".prop_map(|s| s.to_string())
}

/// Generate a valid semantic version
fn version_strategy() -> impl Strategy<Value = String> {
    (1..100u32, 0..100u32, 0..100u32).prop_map(|(major, minor, patch)| {
        format!("{}.{}.{}", major, minor, patch)
    })
}

/// Generate valid Python file content
fn python_content_strategy() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..1000)
}

/// Generate a list of package files (relative paths and content)
fn package_files_strategy() -> impl Strategy<Value = Vec<(String, Vec<u8>)>> {
    prop::collection::vec(
        (
            "[a-z][a-z0-9_]{1,10}",
            prop::option::of("[a-z][a-z0-9_]{1,10}"),
            python_content_strategy(),
        ),
        1..10,
    )
    .prop_map(|files| {
        files
            .into_iter()
            .enumerate()
            .map(|(_i, (name, subdir, content))| {
                let path = if let Some(sub) = subdir {
                    format!("{}/{}.py", sub, name)
                } else {
                    format!("{}.py", name)
                };
                (path, content)
            })
            .collect()
    })
}

/// Create a test wheel file in memory
fn create_wheel(
    name: &str,
    version: &str,
    files: Vec<(String, Vec<u8>)>,
) -> Vec<u8> {
    let buffer = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(buffer);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    let normalized_name = name.replace('-', "_");
    let dist_info = format!("{}-{}.dist-info", normalized_name, version);

    // Add package files
    for (path, content) in &files {
        let full_path = format!("{}/{}", normalized_name, path);
        zip.start_file(&full_path, options).unwrap();
        zip.write_all(content).unwrap();
    }

    // Add METADATA file
    let metadata = format!(
        "Metadata-Version: 2.1\nName: {}\nVersion: {}\n",
        name, version
    );
    zip.start_file(format!("{}/METADATA", dist_info), options)
        .unwrap();
    zip.write_all(metadata.as_bytes()).unwrap();

    // Add WHEEL file
    let wheel_content = "Wheel-Version: 1.0\nGenerator: proptest\nRoot-Is-Purelib: true\nTag: py3-none-any\n";
    zip.start_file(format!("{}/WHEEL", dist_info), options)
        .unwrap();
    zip.write_all(wheel_content.as_bytes()).unwrap();

    // Add RECORD file (empty, will be regenerated)
    zip.start_file(format!("{}/RECORD", dist_info), options)
        .unwrap();
    zip.write_all(b"").unwrap();

    zip.finish().unwrap().into_inner()
}

/// Property 17: Wheel Installation Completeness
///
/// **Validates: Requirements 9.1, 9.2, 9.4, 9.6**
///
/// *For any* installed wheel, all files listed in the wheel's RECORD SHALL exist
/// in the installation directory with matching hashes.
#[test]
fn property_17_wheel_installation_completeness() {
    proptest!(|(
        name in package_name_strategy(),
        version in version_strategy(),
        files in package_files_strategy()
    )| {
        // Setup
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();
        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        // Create and install wheel
        let wheel_data = create_wheel(&name, &version, files.clone());
        let result = installer.install_wheel(&wheel_data).unwrap();

        // Read RECORD file
        let record_path = result.dist_info.join("RECORD");
        prop_assert!(record_path.exists(), "RECORD file must exist");

        let record_content = fs::read_to_string(&record_path).unwrap();
        let record_entries: Vec<RecordEntry> = record_content
            .lines()
            .filter_map(RecordEntry::parse)
            .collect();

        // Verify all files in RECORD exist
        for entry in &record_entries {
            let file_path = site_packages.path().join(&entry.path);
            
            // Skip RECORD file itself (it has no hash)
            if entry.path.ends_with("/RECORD") {
                prop_assert!(entry.hash.is_none(), "RECORD file should not have hash");
                continue;
            }

            prop_assert!(
                file_path.exists(),
                "File from RECORD must exist: {}",
                entry.path
            );

            // Verify hash matches if present
            if let Some(ref hash_str) = entry.hash {
                if hash_str.starts_with("sha256=") {
                    let file_content = fs::read(&file_path).unwrap();
                    let computed_entry = RecordEntry::new(entry.path.clone(), &file_content);
                    prop_assert_eq!(
                        &computed_entry.hash,
                        &entry.hash,
                        "Hash mismatch for file: {}",
                        entry.path
                    );
                }
            }

            // Verify size matches if present
            if let Some(expected_size) = entry.size {
                let actual_size = fs::metadata(&file_path).unwrap().len();
                prop_assert_eq!(
                    actual_size,
                    expected_size,
                    "Size mismatch for file: {}",
                    entry.path
                );
            }
        }

        // Verify all package files are in RECORD
        let normalized_name = name.replace('-', "_");
        let record_paths: HashSet<String> = record_entries
            .iter()
            .map(|e| e.path.clone())
            .collect();

        for (file_path, _) in &files {
            let full_path = format!("{}/{}", normalized_name, file_path);
            prop_assert!(
                record_paths.iter().any(|p| p.contains(&full_path)),
                "Package file must be in RECORD: {}",
                full_path
            );
        }

        // Verify dist-info files are in RECORD
        let dist_info_name = format!("{}-{}.dist-info", normalized_name, version);
        prop_assert!(
            record_paths.iter().any(|p| p.contains(&format!("{}/METADATA", dist_info_name))),
            "METADATA must be in RECORD"
        );
        prop_assert!(
            record_paths.iter().any(|p| p.contains(&format!("{}/WHEEL", dist_info_name))),
            "WHEEL must be in RECORD"
        );
        prop_assert!(
            record_paths.iter().any(|p| p.contains(&format!("{}/INSTALLER", dist_info_name))),
            "INSTALLER must be in RECORD"
        );
    });
}

/// Property 18: Uninstall Completeness
///
/// **Validates: Requirements 9.5**
///
/// *For any* uninstalled package, all files that were installed (per RECORD)
/// SHALL be removed from the filesystem.
#[test]
fn property_18_uninstall_completeness() {
    proptest!(|(
        name in package_name_strategy(),
        version in version_strategy(),
        files in package_files_strategy()
    )| {
        // Setup
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();
        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        // Create and install wheel
        let wheel_data = create_wheel(&name, &version, files.clone());
        let result = installer.install_wheel(&wheel_data).unwrap();

        // Collect all installed file paths before uninstall
        let record_path = result.dist_info.join("RECORD");
        let record_content = fs::read_to_string(&record_path).unwrap();
        let installed_files: Vec<String> = record_content
            .lines()
            .filter_map(RecordEntry::parse)
            .map(|e| e.path)
            .collect();

        // Verify files exist before uninstall
        for file_path in &installed_files {
            let full_path = site_packages.path().join(file_path);
            if !file_path.ends_with("/RECORD") {
                prop_assert!(
                    full_path.exists() || full_path.parent().map(|p| !p.exists()).unwrap_or(false),
                    "File should exist before uninstall: {}",
                    file_path
                );
            }
        }

        // Uninstall the package
        let removed_count = installer.uninstall(&name).unwrap();
        prop_assert!(removed_count > 0, "Uninstall should remove at least one file");

        // Verify all files are removed
        let normalized_name = name.replace('-', "_");
        let pkg_dir = site_packages.path().join(&normalized_name);
        prop_assert!(
            !pkg_dir.exists(),
            "Package directory should be removed: {}",
            normalized_name
        );

        // Verify dist-info directory is removed
        let dist_info_name = format!("{}-{}.dist-info", normalized_name, version);
        let dist_info_path = site_packages.path().join(&dist_info_name);
        prop_assert!(
            !dist_info_path.exists(),
            "Dist-info directory should be removed: {}",
            dist_info_name
        );

        // Verify no files from RECORD remain
        for file_path in &installed_files {
            let full_path = site_packages.path().join(file_path);
            prop_assert!(
                !full_path.exists(),
                "File should be removed after uninstall: {}",
                file_path
            );
        }
    });
}

/// Property: Install-Uninstall Idempotence
///
/// Installing and then uninstalling a package should leave the site-packages
/// directory in the same state as before (minus any pre-existing files).
#[test]
fn property_install_uninstall_idempotence() {
    proptest!(|(
        name in package_name_strategy(),
        version in version_strategy(),
        files in package_files_strategy()
    )| {
        // Setup
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();
        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        // Record initial state
        let initial_entries: HashSet<String> = fs::read_dir(site_packages.path())
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default();

        // Install wheel
        let wheel_data = create_wheel(&name, &version, files.clone());
        let _result = installer.install_wheel(&wheel_data).unwrap();
        
        // Verify installation added files
        let after_install_entries: HashSet<String> = fs::read_dir(site_packages.path())
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default();
        
        prop_assert!(
            after_install_entries.len() > initial_entries.len(),
            "Installation should add files"
        );

        // Uninstall
        installer.uninstall(&name).unwrap();

        // Verify final state matches initial state
        let final_entries: HashSet<String> = fs::read_dir(site_packages.path())
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default();

        prop_assert_eq!(
            &final_entries,
            &initial_entries,
            "After uninstall, site-packages should return to initial state"
        );
    });
}

/// Property: RECORD File Integrity
///
/// The RECORD file must always be valid CSV format and contain entries for
/// all installed files except itself.
#[test]
fn property_record_file_integrity() {
    proptest!(|(
        name in package_name_strategy(),
        version in version_strategy(),
        files in package_files_strategy()
    )| {
        // Setup
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();
        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        // Install wheel
        let wheel_data = create_wheel(&name, &version, files.clone());
        let result = installer.install_wheel(&wheel_data).unwrap();

        // Read and parse RECORD file
        let record_path = result.dist_info.join("RECORD");
        let record_content = fs::read_to_string(&record_path).unwrap();

        // Verify RECORD is valid CSV-like format
        for (line_num, line) in record_content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split(',').collect();
            prop_assert!(
                parts.len() == 3,
                "Line {} in RECORD must have 3 comma-separated fields: {}",
                line_num + 1,
                line
            );

            // First field (path) must not be empty
            prop_assert!(
                !parts[0].is_empty(),
                "Path field must not be empty in line {}: {}",
                line_num + 1,
                line
            );

            // If hash is present, it should start with algorithm name
            if !parts[1].is_empty() {
                prop_assert!(
                    parts[1].contains('='),
                    "Hash field must contain '=' in line {}: {}",
                    line_num + 1,
                    line
                );
            }

            // If size is present, it should be a valid number
            if !parts[2].is_empty() {
                prop_assert!(
                    parts[2].parse::<u64>().is_ok(),
                    "Size field must be a valid number in line {}: {}",
                    line_num + 1,
                    line
                );
            }
        }

        // Verify RECORD file itself is listed without hash
        let record_entry_found = record_content
            .lines()
            .any(|line| line.contains("/RECORD") && line.ends_with(",,"));
        
        prop_assert!(
            record_entry_found,
            "RECORD file must list itself without hash"
        );
    });
}

/// Property: Multiple Install-Uninstall Cycles
///
/// Multiple cycles of installing and uninstalling the same package should
/// work correctly without leaving artifacts.
#[test]
fn property_multiple_install_uninstall_cycles() {
    proptest!(|(
        name in package_name_strategy(),
        version in version_strategy(),
        files in package_files_strategy(),
        cycles in 1..5usize
    )| {
        // Setup
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();
        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        let wheel_data = create_wheel(&name, &version, files.clone());

        for cycle in 0..cycles {
            // Install
            let result = installer.install_wheel(&wheel_data).unwrap();
            
            prop_assert!(
                result.dist_info.exists(),
                "Dist-info should exist after install in cycle {}",
                cycle
            );

            // Verify package directory exists
            let normalized_name = name.replace('-', "_");
            let pkg_dir = site_packages.path().join(&normalized_name);
            prop_assert!(
                pkg_dir.exists(),
                "Package directory should exist after install in cycle {}",
                cycle
            );

            // Uninstall
            let removed = installer.uninstall(&name).unwrap();
            prop_assert!(
                removed > 0,
                "Should remove files in cycle {}",
                cycle
            );

            // Verify clean state
            prop_assert!(
                !pkg_dir.exists(),
                "Package directory should not exist after uninstall in cycle {}",
                cycle
            );
            prop_assert!(
                !result.dist_info.exists(),
                "Dist-info should not exist after uninstall in cycle {}",
                cycle
            );
        }
    });
}
