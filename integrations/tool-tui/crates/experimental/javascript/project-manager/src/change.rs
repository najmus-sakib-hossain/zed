//! Change Detector
//!
//! SIMD-accelerated Blake3 hashing and import detection.

use crate::types::{FileHash, ImportKind, ImportStatement};
use rayon::prelude::*;
use std::path::{Path, PathBuf};

/// Change Detector for file hashing and import detection
pub struct ChangeDetector {
    /// Number of parallel threads to use
    #[allow(dead_code)]
    thread_count: usize,
}

impl ChangeDetector {
    /// Create a new change detector
    pub fn new() -> Self {
        Self {
            thread_count: rayon::current_num_threads(),
        }
    }

    /// Create with specific thread count
    pub fn with_threads(thread_count: usize) -> Self {
        Self { thread_count }
    }

    /// Hash a single file using Blake3 SIMD
    pub fn hash_file(&self, path: &Path) -> std::io::Result<[u8; 32]> {
        let content = std::fs::read(path)?;
        Ok(*blake3::hash(&content).as_bytes())
    }

    /// Hash multiple files in parallel
    pub fn hash_files_parallel(&self, paths: &[PathBuf]) -> std::io::Result<Vec<FileHash>> {
        paths
            .par_iter()
            .map(|path| {
                let content = std::fs::read(path)?;
                let content_hash = *blake3::hash(&content).as_bytes();
                let path_hash = xxhash_rust::xxh3::xxh3_64(path.to_string_lossy().as_bytes());
                let metadata = std::fs::metadata(path)?;
                let mtime_ns = metadata
                    .modified()
                    .map(|t| {
                        t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()
                            as u64
                    })
                    .unwrap_or(0);

                Ok(FileHash::new(path_hash, content_hash, metadata.len(), mtime_ns))
            })
            .collect()
    }

    /// Generate 64-byte binary fingerprint
    pub fn fingerprint(&self, path: &Path) -> std::io::Result<[u8; 64]> {
        let content = std::fs::read(path)?;
        let metadata = std::fs::metadata(path)?;

        // First 32 bytes: Blake3 hash of content
        let content_hash = blake3::hash(&content);

        // Next 8 bytes: file size
        let size_bytes = metadata.len().to_le_bytes();

        // Next 8 bytes: path hash
        let path_hash = xxhash_rust::xxh3::xxh3_64(path.to_string_lossy().as_bytes());
        let path_bytes = path_hash.to_le_bytes();

        // Next 8 bytes: mtime
        let mtime = metadata
            .modified()
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as u64)
            .unwrap_or(0);
        let mtime_bytes = mtime.to_le_bytes();

        // Remaining 8 bytes: reserved/padding
        let mut fingerprint = [0u8; 64];
        fingerprint[0..32].copy_from_slice(content_hash.as_bytes());
        fingerprint[32..40].copy_from_slice(&size_bytes);
        fingerprint[40..48].copy_from_slice(&path_bytes);
        fingerprint[48..56].copy_from_slice(&mtime_bytes);
        // fingerprint[56..64] is zero padding

        Ok(fingerprint)
    }

    /// Detect imports using pattern matching
    /// Note: Full AVX2 SIMD would require unsafe code and platform-specific intrinsics
    pub fn detect_imports(&self, content: &[u8]) -> Vec<ImportStatement> {
        let text = match std::str::from_utf8(content) {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        let mut imports = Vec::new();

        for (line_num, line) in text.lines().enumerate() {
            let line_num = (line_num + 1) as u32;
            let trimmed = line.trim();

            // ES6 import
            if let Some(import) = self.parse_es6_import(trimmed, line_num) {
                imports.push(import);
            }
            // ES6 export from
            else if let Some(import) = self.parse_es6_export_from(trimmed, line_num) {
                imports.push(import);
            }
            // CommonJS require
            else if let Some(import) = self.parse_commonjs_require(trimmed, line_num) {
                imports.push(import);
            }
            // Dynamic import
            else if let Some(import) = self.parse_dynamic_import(trimmed, line_num) {
                imports.push(import);
            }
        }

        imports
    }

    /// Detect imports from file
    pub fn detect_imports_file(&self, path: &Path) -> std::io::Result<Vec<ImportStatement>> {
        let content = std::fs::read(path)?;
        Ok(self.detect_imports(&content))
    }

    // Private parsing helpers

    fn parse_es6_import(&self, line: &str, line_num: u32) -> Option<ImportStatement> {
        // import x from 'y'
        // import { x } from 'y'
        // import * as x from 'y'
        // import 'y'
        if !line.starts_with("import ") {
            return None;
        }

        let specifier = self.extract_string_literal(line)?;
        let column = line.find(&specifier).unwrap_or(0) as u32 + 1;

        Some(ImportStatement {
            kind: ImportKind::Es6Import,
            specifier,
            line: line_num,
            column,
        })
    }

    fn parse_es6_export_from(&self, line: &str, line_num: u32) -> Option<ImportStatement> {
        // export { x } from 'y'
        // export * from 'y'
        if !line.starts_with("export ") || !line.contains(" from ") {
            return None;
        }

        let specifier = self.extract_string_literal(line)?;
        let column = line.find(&specifier).unwrap_or(0) as u32 + 1;

        Some(ImportStatement {
            kind: ImportKind::Es6ExportFrom,
            specifier,
            line: line_num,
            column,
        })
    }

    fn parse_commonjs_require(&self, line: &str, line_num: u32) -> Option<ImportStatement> {
        // require('y')
        // const x = require('y')
        let require_pos = line.find("require(")?;
        let after_require = &line[require_pos + 8..];

        let specifier = self.extract_string_literal(after_require)?;
        let column = (require_pos + 9) as u32;

        Some(ImportStatement {
            kind: ImportKind::CommonJsRequire,
            specifier,
            line: line_num,
            column,
        })
    }

    fn parse_dynamic_import(&self, line: &str, line_num: u32) -> Option<ImportStatement> {
        // import('y')
        // But not "import " (static import)
        let import_pos = line.find("import(")?;

        // Make sure it's not a static import
        if import_pos > 0 {
            let before = &line[..import_pos];
            if before.trim().is_empty() || before.ends_with(char::is_whitespace) {
                // Could be static import, check more carefully
                if line.trim().starts_with("import ") {
                    return None;
                }
            }
        }

        let after_import = &line[import_pos + 7..];
        let specifier = self.extract_string_literal(after_import)?;
        let column = (import_pos + 8) as u32;

        Some(ImportStatement {
            kind: ImportKind::DynamicImport,
            specifier,
            line: line_num,
            column,
        })
    }

    fn extract_string_literal(&self, text: &str) -> Option<String> {
        // Find opening quote
        let (start, quote) =
            text.char_indices().find(|(_, c)| *c == '\'' || *c == '"' || *c == '`')?;

        // Find closing quote
        let content_start = start + 1;
        let end = text[content_start..]
            .char_indices()
            .find(|(_, c)| *c == quote)
            .map(|(i, _)| content_start + i)?;

        Some(text[content_start..end].to_string())
    }
}

impl Default for ChangeDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Merkle tree for directory hashing
#[derive(Debug, Clone)]
pub struct MerkleTree {
    /// Root hash
    pub root_hash: [u8; 32],
    /// Nodes in the tree
    pub nodes: Vec<MerkleNode>,
}

/// Merkle tree node
#[derive(Debug, Clone)]
pub struct MerkleNode {
    /// Node hash
    pub hash: [u8; 32],
    /// Child indices (empty for leaf nodes)
    pub children: Vec<usize>,
    /// Whether this is a file (leaf) node
    pub is_file: bool,
    /// Path relative to tree root
    pub path: String,
}

impl ChangeDetector {
    /// Build Merkle tree for a directory
    pub fn build_merkle_tree(&self, dir: &Path) -> std::io::Result<MerkleTree> {
        let mut nodes = Vec::new();
        let root_hash = self.build_merkle_node(dir, &mut nodes, "")?;

        Ok(MerkleTree { root_hash, nodes })
    }

    fn build_merkle_node(
        &self,
        path: &Path,
        nodes: &mut Vec<MerkleNode>,
        relative_path: &str,
    ) -> std::io::Result<[u8; 32]> {
        if path.is_file() {
            let hash = self.hash_file(path)?;
            nodes.push(MerkleNode {
                hash,
                children: Vec::new(),
                is_file: true,
                path: relative_path.to_string(),
            });
            return Ok(hash);
        }

        // Directory: hash children
        let mut child_hashes = Vec::new();
        let mut child_indices = Vec::new();

        let mut entries: Vec<_> = std::fs::read_dir(path)?.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.path());

        for entry in entries {
            let entry_path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let child_relative = if relative_path.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", relative_path, name)
            };

            let child_idx = nodes.len();
            let child_hash = self.build_merkle_node(&entry_path, nodes, &child_relative)?;
            child_hashes.push(child_hash);
            child_indices.push(child_idx);
        }

        // Compute directory hash from children
        let mut hasher = blake3::Hasher::new();
        for hash in &child_hashes {
            hasher.update(hash);
        }
        let hash = *hasher.finalize().as_bytes();

        nodes.push(MerkleNode {
            hash,
            children: child_indices,
            is_file: false,
            path: relative_path.to_string(),
        });

        Ok(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_hash_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, b"hello world").unwrap();

        let detector = ChangeDetector::new();
        let hash = detector.hash_file(&file_path).unwrap();

        // Blake3 hash should be deterministic
        let expected = blake3::hash(b"hello world");
        assert_eq!(hash, *expected.as_bytes());
    }

    #[test]
    fn test_fingerprint_size() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, b"hello world").unwrap();

        let detector = ChangeDetector::new();
        let fingerprint = detector.fingerprint(&file_path).unwrap();

        // Fingerprint should always be exactly 64 bytes
        assert_eq!(fingerprint.len(), 64);
    }

    #[test]
    fn test_detect_es6_imports() {
        let detector = ChangeDetector::new();

        let content = br#"
import React from 'react';
import { useState } from 'react';
import * as lodash from 'lodash';
import './styles.css';
"#;

        let imports = detector.detect_imports(content);

        assert_eq!(imports.len(), 4);
        assert!(imports
            .iter()
            .any(|i| i.specifier == "react" && i.kind == ImportKind::Es6Import));
        assert!(imports
            .iter()
            .any(|i| i.specifier == "lodash" && i.kind == ImportKind::Es6Import));
        assert!(imports
            .iter()
            .any(|i| i.specifier == "./styles.css" && i.kind == ImportKind::Es6Import));
    }

    #[test]
    fn test_detect_commonjs_require() {
        let detector = ChangeDetector::new();

        let content = br#"
const fs = require('fs');
const path = require("path");
const custom = require('./custom');
"#;

        let imports = detector.detect_imports(content);

        assert_eq!(imports.len(), 3);
        assert!(imports.iter().all(|i| i.kind == ImportKind::CommonJsRequire));
        assert!(imports.iter().any(|i| i.specifier == "fs"));
        assert!(imports.iter().any(|i| i.specifier == "path"));
        assert!(imports.iter().any(|i| i.specifier == "./custom"));
    }

    #[test]
    fn test_detect_dynamic_import() {
        let detector = ChangeDetector::new();

        let content = br#"
const module = await import('./dynamic');
import('./lazy').then(m => m.default);
"#;

        let imports = detector.detect_imports(content);

        assert!(imports
            .iter()
            .any(|i| i.specifier == "./dynamic" && i.kind == ImportKind::DynamicImport));
        assert!(imports
            .iter()
            .any(|i| i.specifier == "./lazy" && i.kind == ImportKind::DynamicImport));
    }

    #[test]
    fn test_detect_export_from() {
        let detector = ChangeDetector::new();

        let content = br#"
export { foo } from './foo';
export * from './bar';
"#;

        let imports = detector.detect_imports(content);

        assert!(imports
            .iter()
            .any(|i| i.specifier == "./foo" && i.kind == ImportKind::Es6ExportFrom));
        assert!(imports
            .iter()
            .any(|i| i.specifier == "./bar" && i.kind == ImportKind::Es6ExportFrom));
    }

    #[test]
    fn test_merkle_tree() {
        let temp = TempDir::new().unwrap();

        // Create directory structure
        fs::create_dir(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("src/a.ts"), b"const a = 1;").unwrap();
        fs::write(temp.path().join("src/b.ts"), b"const b = 2;").unwrap();
        fs::write(temp.path().join("package.json"), b"{}").unwrap();

        let detector = ChangeDetector::new();
        let tree = detector.build_merkle_tree(temp.path()).unwrap();

        // Should have nodes for: a.ts, b.ts, src/, package.json, root
        assert!(tree.nodes.len() >= 4);

        // Root hash should be deterministic
        let tree2 = detector.build_merkle_tree(temp.path()).unwrap();
        assert_eq!(tree.root_hash, tree2.root_hash);
    }

    #[test]
    fn test_parallel_hashing() {
        let temp = TempDir::new().unwrap();

        // Create multiple files
        let mut paths = Vec::new();
        for i in 0..10 {
            let path = temp.path().join(format!("file{}.txt", i));
            fs::write(&path, format!("content {}", i)).unwrap();
            paths.push(path);
        }

        let detector = ChangeDetector::new();
        let hashes = detector.hash_files_parallel(&paths).unwrap();

        assert_eq!(hashes.len(), 10);

        // Each hash should be unique
        let unique: std::collections::HashSet<_> = hashes.iter().map(|h| h.content_hash).collect();
        assert_eq!(unique.len(), 10);
    }
}
