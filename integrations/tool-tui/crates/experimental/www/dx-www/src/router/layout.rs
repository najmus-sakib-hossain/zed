//! # Layout Resolution System
//!
//! This module handles the resolution of layout chains for pages.
//! Layouts are applied from root to leaf, wrapping pages in nested layout components.

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::project::LayoutFile;

// =============================================================================
// Layout Resolver
// =============================================================================

/// Resolves layout chains for pages based on directory hierarchy.
#[derive(Debug, Clone, Default)]
pub struct LayoutResolver {
    /// Layouts indexed by their directory path
    layouts: HashMap<PathBuf, LayoutInfo>,
}

/// Information about a layout.
#[derive(Debug, Clone)]
pub struct LayoutInfo {
    /// Path to the layout file
    pub file_path: PathBuf,

    /// Directory this layout applies to
    pub directory: PathBuf,

    /// Compiled binary path
    pub binary_path: Option<PathBuf>,
}

impl LayoutResolver {
    /// Create a new empty layout resolver.
    pub fn new() -> Self {
        Self {
            layouts: HashMap::new(),
        }
    }

    /// Add layouts from the project scan.
    pub fn add_layouts(&mut self, layouts: &[LayoutFile]) {
        for layout in layouts {
            self.layouts.insert(
                layout.directory.clone(),
                LayoutInfo {
                    file_path: layout.path.clone(),
                    directory: layout.directory.clone(),
                    binary_path: None,
                },
            );
        }
    }

    /// Add a single layout.
    pub fn add_layout(&mut self, directory: PathBuf, file_path: PathBuf) {
        self.layouts.insert(
            directory.clone(),
            LayoutInfo {
                file_path,
                directory,
                binary_path: None,
            },
        );
    }

    /// Resolve the layout chain for a given page path.
    ///
    /// Returns layouts from root to leaf (outermost to innermost).
    pub fn resolve_chain(&self, page_path: &Path) -> Vec<&LayoutInfo> {
        let mut chain = Vec::new();
        let mut current = PathBuf::new();

        // Check for root layout
        if let Some(layout) = self.layouts.get(&PathBuf::new()) {
            chain.push(layout);
        }

        // Walk the path from root to the page's directory
        let parent = page_path.parent().unwrap_or(Path::new(""));
        for component in parent.components() {
            if let std::path::Component::Normal(os_str) = component {
                current = current.join(os_str);

                if let Some(layout) = self.layouts.get(&current) {
                    chain.push(layout);
                }
            }
        }

        chain
    }

    /// Get the layout for a specific directory.
    pub fn get_layout(&self, directory: &Path) -> Option<&LayoutInfo> {
        self.layouts.get(directory)
    }

    /// Check if a directory has a layout.
    pub fn has_layout(&self, directory: &Path) -> bool {
        self.layouts.contains_key(directory)
    }

    /// Get all layouts.
    pub fn all_layouts(&self) -> impl Iterator<Item = &LayoutInfo> {
        self.layouts.values()
    }

    /// Get the number of layouts.
    pub fn layout_count(&self) -> usize {
        self.layouts.len()
    }
}

// =============================================================================
// Layout Chain
// =============================================================================

/// A resolved layout chain for a specific page.
#[derive(Debug, Clone)]
pub struct LayoutChain {
    /// Layouts from root to leaf
    pub layouts: Vec<PathBuf>,

    /// The page file
    pub page: PathBuf,
}

impl LayoutChain {
    /// Create a new layout chain.
    pub fn new(layouts: Vec<PathBuf>, page: PathBuf) -> Self {
        Self { layouts, page }
    }

    /// Check if this chain has any layouts.
    pub fn has_layouts(&self) -> bool {
        !self.layouts.is_empty()
    }

    /// Get the outermost (root) layout.
    pub fn root_layout(&self) -> Option<&PathBuf> {
        self.layouts.first()
    }

    /// Get the innermost (closest to page) layout.
    pub fn inner_layout(&self) -> Option<&PathBuf> {
        self.layouts.last()
    }

    /// Get the depth of the layout chain.
    pub fn depth(&self) -> usize {
        self.layouts.len()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_resolver() {
        let resolver = LayoutResolver::new();
        let chain = resolver.resolve_chain(Path::new("about.pg"));
        assert!(chain.is_empty());
    }

    #[test]
    fn test_root_layout() {
        let mut resolver = LayoutResolver::new();
        resolver.add_layout(PathBuf::new(), PathBuf::from("pages/_layout.pg"));

        let chain = resolver.resolve_chain(Path::new("about.pg"));
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].file_path, PathBuf::from("pages/_layout.pg"));
    }

    #[test]
    fn test_nested_layouts() {
        let mut resolver = LayoutResolver::new();
        resolver.add_layout(PathBuf::new(), PathBuf::from("pages/_layout.pg"));
        resolver.add_layout(PathBuf::from("blog"), PathBuf::from("pages/blog/_layout.pg"));

        let chain = resolver.resolve_chain(Path::new("blog/post.pg"));
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].file_path, PathBuf::from("pages/_layout.pg"));
        assert_eq!(chain[1].file_path, PathBuf::from("pages/blog/_layout.pg"));
    }

    #[test]
    fn test_deeply_nested_layouts() {
        let mut resolver = LayoutResolver::new();
        resolver.add_layout(PathBuf::new(), PathBuf::from("pages/_layout.pg"));
        resolver.add_layout(PathBuf::from("docs"), PathBuf::from("pages/docs/_layout.pg"));
        resolver.add_layout(PathBuf::from("docs/api"), PathBuf::from("pages/docs/api/_layout.pg"));

        let chain = resolver.resolve_chain(Path::new("docs/api/reference.pg"));
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].file_path, PathBuf::from("pages/_layout.pg"));
        assert_eq!(chain[1].file_path, PathBuf::from("pages/docs/_layout.pg"));
        assert_eq!(chain[2].file_path, PathBuf::from("pages/docs/api/_layout.pg"));
    }

    #[test]
    fn test_partial_layout_chain() {
        let mut resolver = LayoutResolver::new();
        resolver.add_layout(PathBuf::new(), PathBuf::from("pages/_layout.pg"));
        // No layout in blog/, but there is one in blog/posts/
        resolver
            .add_layout(PathBuf::from("blog/posts"), PathBuf::from("pages/blog/posts/_layout.pg"));

        let chain = resolver.resolve_chain(Path::new("blog/posts/article.pg"));
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].file_path, PathBuf::from("pages/_layout.pg"));
        assert_eq!(chain[1].file_path, PathBuf::from("pages/blog/posts/_layout.pg"));
    }

    #[test]
    fn test_layout_chain_struct() {
        let chain = LayoutChain::new(
            vec![PathBuf::from("layout1.pg"), PathBuf::from("layout2.pg")],
            PathBuf::from("page.pg"),
        );

        assert!(chain.has_layouts());
        assert_eq!(chain.root_layout(), Some(&PathBuf::from("layout1.pg")));
        assert_eq!(chain.inner_layout(), Some(&PathBuf::from("layout2.pg")));
        assert_eq!(chain.depth(), 2);
    }
}
