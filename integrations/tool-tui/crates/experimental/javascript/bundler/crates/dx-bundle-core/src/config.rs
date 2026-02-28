//! Configuration for DX JS Bundler

use std::path::PathBuf;

/// Bundle configuration
#[derive(Clone, Debug)]
pub struct BundleConfig {
    /// Entry point files
    pub entries: Vec<PathBuf>,
    /// Output directory
    pub out_dir: PathBuf,
    /// Output filename (for single bundle)
    pub out_file: Option<PathBuf>,
    /// Enable minification
    pub minify: bool,
    /// Generate source maps
    pub source_maps: bool,
    /// Enable tree shaking
    pub tree_shake: bool,
    /// Target environment
    pub target: Target,
    /// Module format
    pub format: ModuleFormat,
    /// Enable code splitting
    pub code_splitting: bool,
    /// Preserve JSX (don't transform)
    pub preserve_jsx: bool,
    /// JSX factory function
    pub jsx_factory: String,
    /// JSX fragment factory
    pub jsx_fragment: String,
    /// External modules (don't bundle)
    pub externals: Vec<String>,
    /// Alias mappings
    pub aliases: Vec<(String, String)>,
    /// Enable watch mode
    pub watch: bool,
    /// Use persistent cache
    pub cache: bool,
    /// Cache directory
    pub cache_dir: PathBuf,
    /// Arena size in MB
    pub arena_size_mb: usize,
    /// Number of worker threads (0 = auto)
    pub threads: usize,
}

impl Default for BundleConfig {
    fn default() -> Self {
        Self {
            entries: vec![],
            out_dir: PathBuf::from("dist"),
            out_file: None,
            minify: false,
            source_maps: true,
            tree_shake: true,
            target: Target::ESNext,
            format: ModuleFormat::ESM,
            code_splitting: false,
            preserve_jsx: false,
            jsx_factory: "React.createElement".into(),
            jsx_fragment: "React.Fragment".into(),
            externals: vec![],
            aliases: vec![],
            watch: false,
            cache: true,
            cache_dir: PathBuf::from(".dx-cache"),
            arena_size_mb: 64,
            threads: 0,
        }
    }
}

impl BundleConfig {
    /// Create config for production build
    pub fn production() -> Self {
        Self {
            minify: true,
            source_maps: true,
            tree_shake: true,
            ..Default::default()
        }
    }

    /// Create config for development build
    pub fn development() -> Self {
        Self {
            minify: false,
            source_maps: true,
            tree_shake: false,
            watch: true,
            ..Default::default()
        }
    }

    /// Get arena size in bytes
    pub fn arena_size(&self) -> usize {
        self.arena_size_mb * 1024 * 1024
    }

    /// Get thread count (auto = num CPUs)
    pub fn thread_count(&self) -> usize {
        if self.threads == 0 {
            num_cpus::get()
        } else {
            self.threads
        }
    }
}

/// Target environment
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    ES5,
    ES2015,
    ES2016,
    ES2017,
    ES2018,
    ES2019,
    ES2020,
    ES2021,
    ES2022,
    ES2023,
    ESNext,
    Node16,
    Node18,
    Node20,
}

impl Target {
    /// Check if target supports arrow functions
    pub fn supports_arrow_functions(&self) -> bool {
        !matches!(self, Target::ES5)
    }

    /// Check if target supports const/let
    pub fn supports_const_let(&self) -> bool {
        !matches!(self, Target::ES5)
    }

    /// Check if target supports async/await
    pub fn supports_async_await(&self) -> bool {
        matches!(
            self,
            Target::ES2017
                | Target::ES2018
                | Target::ES2019
                | Target::ES2020
                | Target::ES2021
                | Target::ES2022
                | Target::ES2023
                | Target::ESNext
                | Target::Node16
                | Target::Node18
                | Target::Node20
        )
    }

    /// Check if target supports optional chaining
    pub fn supports_optional_chaining(&self) -> bool {
        matches!(
            self,
            Target::ES2020
                | Target::ES2021
                | Target::ES2022
                | Target::ES2023
                | Target::ESNext
                | Target::Node16
                | Target::Node18
                | Target::Node20
        )
    }
}

/// Module format
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleFormat {
    /// ES Modules (import/export)
    ESM,
    /// CommonJS (require/exports)
    CJS,
    /// Immediately Invoked Function Expression
    IIFE,
    /// Universal Module Definition
    UMD,
}

impl ModuleFormat {
    /// Get module wrapper prefix
    pub fn prefix(&self) -> &'static [u8] {
        match self {
            ModuleFormat::ESM => b"",
            ModuleFormat::CJS => b"",
            ModuleFormat::IIFE => b"(function(){\n\"use strict\";\n",
            ModuleFormat::UMD => b"(function(global,factory){typeof exports===\"object\"&&typeof module!==\"undefined\"?factory(exports):typeof define===\"function\"&&define.amd?define([\"exports\"],factory):(global=typeof globalThis!==\"undefined\"?globalThis:global||self,factory(global.bundle={}));})(this,function(exports){\n\"use strict\";\n",
        }
    }

    /// Get module wrapper suffix
    pub fn suffix(&self) -> &'static [u8] {
        match self {
            ModuleFormat::ESM => b"",
            ModuleFormat::CJS => b"",
            ModuleFormat::IIFE => b"\n})();\n",
            ModuleFormat::UMD => b"\n});\n",
        }
    }
}
