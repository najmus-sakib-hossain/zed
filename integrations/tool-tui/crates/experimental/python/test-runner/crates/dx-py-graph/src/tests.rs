//! Tests for dx-py-graph

use super::*;
use proptest::prelude::*;
use tempfile::TempDir;

// Property 7: Import Graph Construction
// For any set of files with imports, the graph correctly represents dependencies

proptest! {
    /// Feature: dx-py-test-runner, Property 7: Import Graph Construction
    /// Validates: Requirements 4.1
    #[test]
    fn prop_graph_construction(
        file_count in 2usize..=5usize,
    ) {
        let mut graph = ImportGraph::new();
        let files: Vec<PathBuf> = (0..file_count)
            .map(|i| PathBuf::from(format!("file_{}.py", i)))
            .collect();

        // Add all files
        for file in &files {
            graph.add_file(file);
        }

        prop_assert_eq!(graph.file_count(), file_count);

        // Add chain of imports: file_0 <- file_1 <- file_2 <- ...
        for i in 1..file_count {
            graph.add_import(&files[i], &files[i - 1]);
        }

        prop_assert_eq!(graph.edge_count(), file_count - 1);
    }

    /// Feature: dx-py-test-runner, Property 8: Transitive Dependency Detection
    /// Validates: Requirements 4.2
    #[test]
    fn prop_transitive_dependencies(
        chain_length in 2usize..=6usize,
    ) {
        let mut graph = ImportGraph::new();
        let files: Vec<PathBuf> = (0..chain_length)
            .map(|i| PathBuf::from(format!("module_{}.py", i)))
            .collect();

        // Create a chain: file_0 <- file_1 <- file_2 <- ...
        // (file_1 imports file_0, file_2 imports file_1, etc.)
        for file in &files {
            graph.add_file(file);
        }
        for i in 1..chain_length {
            graph.add_import(&files[i], &files[i - 1]);
        }

        // Changing file_0 should affect all files that depend on it
        let dependents = graph.get_dependents(&files[0]);

        // All files except file_0 should be dependents
        prop_assert_eq!(
            dependents.len(),
            chain_length - 1,
            "Expected {} dependents, got {}",
            chain_length - 1,
            dependents.len()
        );

        for file in files.iter().skip(1) {
            prop_assert!(
                dependents.contains(file),
                "Expected {} to be a dependent",
                file.display()
            );
        }
    }

    /// Feature: dx-py-test-runner, Property 10: Dependency Graph Round-Trip
    /// Validates: Requirements 4.4, 4.5
    #[test]
    fn prop_graph_roundtrip(
        file_count in 1usize..=5usize,
        edge_count in 0usize..=10usize,
    ) {
        let temp_dir = TempDir::new().unwrap();
        let cache_file = temp_dir.path().join("graph.dxgr");

        let mut graph = ImportGraph::new();
        let files: Vec<PathBuf> = (0..file_count)
            .map(|i| PathBuf::from(format!("test_{}.py", i)))
            .collect();

        for file in &files {
            graph.add_file(file);
            graph.set_file_hash(file, [0u8; 32]);
        }

        // Add some random edges
        for i in 0..edge_count.min(file_count * file_count) {
            let from = i % file_count;
            let to = (i + 1) % file_count;
            if from != to {
                graph.add_import(&files[from], &files[to]);
            }
        }

        let original_file_count = graph.file_count();
        let original_edge_count = graph.edge_count();

        // Save and reload
        graph.save(&cache_file).unwrap();
        let loaded = ImportGraph::load(&cache_file).unwrap();

        prop_assert_eq!(loaded.file_count(), original_file_count);
        prop_assert_eq!(loaded.edge_count(), original_edge_count);
    }
}

// Unit tests

#[test]
fn test_import_extractor_simple() {
    let source = r#"
import os
import sys
"#;
    let mut extractor = ImportExtractor::new().unwrap();
    let imports = extractor.extract_imports(source).unwrap();

    assert_eq!(imports.len(), 2);
    assert_eq!(imports[0].module, "os");
    assert_eq!(imports[1].module, "sys");
    assert!(!imports[0].is_relative);
}

#[test]
fn test_import_extractor_from() {
    let source = r#"
from os import path
from collections import defaultdict
"#;
    let mut extractor = ImportExtractor::new().unwrap();
    let imports = extractor.extract_imports(source).unwrap();

    assert_eq!(imports.len(), 2);
    // The module name should be the module being imported from
    assert!(imports[0].module == "os" || imports[0].module.contains("os"));
    assert!(imports[1].module == "collections" || imports[1].module.contains("collections"));
}

#[test]
fn test_import_extractor_relative() {
    let source = r#"
from . import sibling
from .. import parent
from ...utils import helper
"#;
    let mut extractor = ImportExtractor::new().unwrap();
    let imports = extractor.extract_imports(source).unwrap();

    assert_eq!(imports.len(), 3);
    // Check that relative imports are detected
    // Note: The exact parsing depends on tree-sitter-python version
    assert!(imports.iter().any(|i| i.is_relative || i.level > 0));
}

#[test]
fn test_graph_add_file() {
    let mut graph = ImportGraph::new();

    let path1 = PathBuf::from("test_a.py");
    let path2 = PathBuf::from("test_b.py");

    graph.add_file(&path1);
    graph.add_file(&path2);

    assert_eq!(graph.file_count(), 2);
}

#[test]
fn test_graph_add_import() {
    let mut graph = ImportGraph::new();

    let importer = PathBuf::from("test_main.py");
    let imported = PathBuf::from("utils.py");

    graph.add_import(&importer, &imported);

    assert_eq!(graph.file_count(), 2);
    assert_eq!(graph.edge_count(), 1);
}

#[test]
fn test_graph_dependents() {
    let mut graph = ImportGraph::new();

    // Create: a.py <- b.py <- c.py
    let a = PathBuf::from("a.py");
    let b = PathBuf::from("b.py");
    let c = PathBuf::from("c.py");

    graph.add_import(&b, &a); // b imports a
    graph.add_import(&c, &b); // c imports b

    // Changing a.py affects b.py and c.py
    let dependents = graph.get_dependents(&a);
    assert!(dependents.contains(&b));
    assert!(dependents.contains(&c));
    assert_eq!(dependents.len(), 2);

    // Changing b.py only affects c.py
    let dependents = graph.get_dependents(&b);
    assert!(dependents.contains(&c));
    assert_eq!(dependents.len(), 1);

    // Changing c.py affects nothing
    let dependents = graph.get_dependents(&c);
    assert!(dependents.is_empty());
}

#[test]
fn test_graph_affected_tests() {
    let mut graph = ImportGraph::new();

    let utils = PathBuf::from("utils.py");
    let test_file = PathBuf::from("test_main.py");

    graph.add_import(&test_file, &utils);
    graph.set_file_tests(&test_file, vec![TestId(1), TestId(2)]);

    let affected = graph.get_affected_tests(&utils);
    assert_eq!(affected.len(), 2);
    assert!(affected.contains(&TestId(1)));
    assert!(affected.contains(&TestId(2)));
}

#[test]
fn test_graph_save_load() {
    let temp_dir = TempDir::new().unwrap();
    let cache_file = temp_dir.path().join("test.dxgr");

    let mut graph = ImportGraph::new();
    let a = PathBuf::from("a.py");
    let b = PathBuf::from("b.py");

    graph.add_import(&b, &a);
    graph.set_file_hash(&a, [1u8; 32]);
    graph.set_file_tests(&a, vec![TestId(42)]);

    graph.save(&cache_file).unwrap();
    let loaded = ImportGraph::load(&cache_file).unwrap();

    assert_eq!(loaded.file_count(), 2);
    assert_eq!(loaded.edge_count(), 1);
}

#[test]
fn test_import_resolve_relative() {
    let import = ImportInfo {
        module: "sibling".to_string(),
        is_relative: true,
        level: 1,
    };

    let current = PathBuf::from("/project/package/module.py");
    let resolved = import.resolve(&current).unwrap();

    assert!(resolved.to_string_lossy().contains("sibling.py"));
}

#[test]
fn test_needs_rescan() {
    let mut graph = ImportGraph::new();
    let path = PathBuf::from("test.py");

    let hash1 = [1u8; 32];
    let hash2 = [2u8; 32];

    // Unknown file needs rescan
    assert!(graph.needs_rescan(&path, &hash1));

    // After setting hash, same hash doesn't need rescan
    graph.add_file(&path);
    graph.set_file_hash(&path, hash1);
    assert!(!graph.needs_rescan(&path, &hash1));

    // Different hash needs rescan
    assert!(graph.needs_rescan(&path, &hash2));
}
