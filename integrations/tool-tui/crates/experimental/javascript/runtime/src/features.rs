//! DX Features Detection
//!
//! This module provides the `dx.features` object that allows JavaScript code
//! to detect which ECMAScript features and DX-specific capabilities are supported.
//!
//! # Example
//!
//! ```javascript
//! if (dx.features.es2022) {
//!     // Use ES2022 features
//! }
//!
//! if (dx.features.typescript) {
//!     // TypeScript is supported
//! }
//! ```

use std::collections::HashMap;

/// Supported ECMAScript and DX features
///
/// This struct represents the features available in the DX runtime.
/// Each field indicates whether a particular feature is supported.
#[derive(Debug, Clone)]
pub struct DxFeatures {
    // ECMAScript versions
    /// ES2015 (ES6) support
    pub es2015: bool,
    /// ES2016 support
    pub es2016: bool,
    /// ES2017 support (async/await)
    pub es2017: bool,
    /// ES2018 support (rest/spread properties, async iteration)
    pub es2018: bool,
    /// ES2019 support (Array.flat, Object.fromEntries)
    pub es2019: bool,
    /// ES2020 support (BigInt, nullish coalescing, optional chaining)
    pub es2020: bool,
    /// ES2021 support (String.replaceAll, Promise.any)
    pub es2021: bool,
    /// ES2022 support (class fields, top-level await)
    pub es2022: bool,

    // Language features
    /// TypeScript support
    pub typescript: bool,
    /// JSX support
    pub jsx: bool,
    /// Decorators support (stage 3)
    pub decorators: bool,

    // Runtime features
    /// CommonJS module support
    pub commonjs: bool,
    /// ES Modules support
    pub esm: bool,
    /// Worker threads support
    pub workers: bool,
    /// WebAssembly support
    pub wasm: bool,

    // DX-specific features
    /// Hot module replacement
    pub hmr: bool,
    /// Source maps
    pub source_maps: bool,
    /// JIT compilation
    pub jit: bool,
}

impl Default for DxFeatures {
    fn default() -> Self {
        Self::current()
    }
}

impl DxFeatures {
    /// Get the current feature set supported by this DX runtime
    pub fn current() -> Self {
        Self {
            // ECMAScript versions - all supported through ES2022
            es2015: true,
            es2016: true,
            es2017: true,
            es2018: true,
            es2019: true,
            es2020: true,
            es2021: true,
            es2022: true,

            // Language features
            typescript: true,
            jsx: true,
            decorators: false, // Not yet implemented

            // Runtime features
            commonjs: true,
            esm: true,
            workers: true,
            wasm: true,

            // DX-specific features
            hmr: false, // Not yet implemented
            source_maps: true,
            jit: true,
        }
    }

    /// Convert features to a HashMap for JavaScript object creation
    pub fn to_map(&self) -> HashMap<String, bool> {
        let mut map = HashMap::new();

        // ECMAScript versions
        map.insert("es2015".to_string(), self.es2015);
        map.insert("es2016".to_string(), self.es2016);
        map.insert("es2017".to_string(), self.es2017);
        map.insert("es2018".to_string(), self.es2018);
        map.insert("es2019".to_string(), self.es2019);
        map.insert("es2020".to_string(), self.es2020);
        map.insert("es2021".to_string(), self.es2021);
        map.insert("es2022".to_string(), self.es2022);

        // Language features
        map.insert("typescript".to_string(), self.typescript);
        map.insert("jsx".to_string(), self.jsx);
        map.insert("decorators".to_string(), self.decorators);

        // Runtime features
        map.insert("commonjs".to_string(), self.commonjs);
        map.insert("esm".to_string(), self.esm);
        map.insert("workers".to_string(), self.workers);
        map.insert("wasm".to_string(), self.wasm);

        // DX-specific features
        map.insert("hmr".to_string(), self.hmr);
        map.insert("sourceMaps".to_string(), self.source_maps);
        map.insert("jit".to_string(), self.jit);

        map
    }

    /// Get a list of all feature names
    pub fn feature_names() -> &'static [&'static str] {
        &[
            "es2015",
            "es2016",
            "es2017",
            "es2018",
            "es2019",
            "es2020",
            "es2021",
            "es2022",
            "typescript",
            "jsx",
            "decorators",
            "commonjs",
            "esm",
            "workers",
            "wasm",
            "hmr",
            "sourceMaps",
            "jit",
        ]
    }

    /// Get the required feature names (from Property 16)
    pub fn required_feature_names() -> &'static [&'static str] {
        &[
            "es2015",
            "es2016",
            "es2017",
            "es2018",
            "es2019",
            "es2020",
            "es2021",
            "es2022",
            "typescript",
        ]
    }

    /// Check if a specific feature is supported
    pub fn is_supported(&self, feature: &str) -> Option<bool> {
        match feature {
            "es2015" => Some(self.es2015),
            "es2016" => Some(self.es2016),
            "es2017" => Some(self.es2017),
            "es2018" => Some(self.es2018),
            "es2019" => Some(self.es2019),
            "es2020" => Some(self.es2020),
            "es2021" => Some(self.es2021),
            "es2022" => Some(self.es2022),
            "typescript" => Some(self.typescript),
            "jsx" => Some(self.jsx),
            "decorators" => Some(self.decorators),
            "commonjs" => Some(self.commonjs),
            "esm" => Some(self.esm),
            "workers" => Some(self.workers),
            "wasm" => Some(self.wasm),
            "hmr" => Some(self.hmr),
            "sourceMaps" | "source_maps" => Some(self.source_maps),
            "jit" => Some(self.jit),
            _ => None,
        }
    }

    /// Get a list of unsupported features
    pub fn unsupported_features(&self) -> Vec<&'static str> {
        let mut unsupported = Vec::new();

        if !self.es2015 {
            unsupported.push("es2015");
        }
        if !self.es2016 {
            unsupported.push("es2016");
        }
        if !self.es2017 {
            unsupported.push("es2017");
        }
        if !self.es2018 {
            unsupported.push("es2018");
        }
        if !self.es2019 {
            unsupported.push("es2019");
        }
        if !self.es2020 {
            unsupported.push("es2020");
        }
        if !self.es2021 {
            unsupported.push("es2021");
        }
        if !self.es2022 {
            unsupported.push("es2022");
        }
        if !self.typescript {
            unsupported.push("typescript");
        }
        if !self.jsx {
            unsupported.push("jsx");
        }
        if !self.decorators {
            unsupported.push("decorators");
        }
        if !self.commonjs {
            unsupported.push("commonjs");
        }
        if !self.esm {
            unsupported.push("esm");
        }
        if !self.workers {
            unsupported.push("workers");
        }
        if !self.wasm {
            unsupported.push("wasm");
        }
        if !self.hmr {
            unsupported.push("hmr");
        }
        if !self.source_maps {
            unsupported.push("sourceMaps");
        }
        if !self.jit {
            unsupported.push("jit");
        }

        unsupported
    }
}

/// The global `dx` object that provides DX-specific APIs
#[derive(Debug, Clone)]
pub struct DxGlobal {
    /// Feature detection object
    pub features: DxFeatures,
    /// DX runtime version
    pub version: &'static str,
}

impl Default for DxGlobal {
    fn default() -> Self {
        Self::new()
    }
}

impl DxGlobal {
    /// Create a new dx global object
    pub fn new() -> Self {
        Self {
            features: DxFeatures::current(),
            version: env!("CARGO_PKG_VERSION"),
        }
    }

    /// Get the features object
    pub fn get_features(&self) -> &DxFeatures {
        &self.features
    }

    /// Get the version string
    pub fn get_version(&self) -> &'static str {
        self.version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_features_current() {
        let features = DxFeatures::current();

        // All ES versions should be supported
        assert!(features.es2015);
        assert!(features.es2016);
        assert!(features.es2017);
        assert!(features.es2018);
        assert!(features.es2019);
        assert!(features.es2020);
        assert!(features.es2021);
        assert!(features.es2022);

        // TypeScript should be supported
        assert!(features.typescript);
    }

    #[test]
    fn test_features_to_map() {
        let features = DxFeatures::current();
        let map = features.to_map();

        // Check required keys exist
        for key in DxFeatures::required_feature_names() {
            assert!(map.contains_key(*key), "Missing required key: {}", key);
        }

        // All values should be booleans (they are, by type)
        assert_eq!(map.get("es2015"), Some(&true));
        assert_eq!(map.get("typescript"), Some(&true));
    }

    #[test]
    fn test_is_supported() {
        let features = DxFeatures::current();

        assert_eq!(features.is_supported("es2022"), Some(true));
        assert_eq!(features.is_supported("typescript"), Some(true));
        assert_eq!(features.is_supported("decorators"), Some(false));
        assert_eq!(features.is_supported("unknown_feature"), None);
    }

    #[test]
    fn test_dx_global() {
        let dx = DxGlobal::new();

        assert!(!dx.version.is_empty());
        assert!(dx.features.es2022);
    }

    #[test]
    fn test_required_features_present() {
        let features = DxFeatures::current();
        let map = features.to_map();

        // Property 16 requires these keys
        let required = [
            "es2015",
            "es2016",
            "es2017",
            "es2018",
            "es2019",
            "es2020",
            "es2021",
            "es2022",
            "typescript",
        ];

        for key in required {
            assert!(map.contains_key(key), "Missing required feature: {}", key);
            // All values must be boolean (enforced by type system)
        }
    }
}
