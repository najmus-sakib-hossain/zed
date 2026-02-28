//! # API Routes
//!
//! This module handles API route scanning and compilation.
//!
//! API routes are defined in the `api/` directory and are automatically
//! mapped to HTTP endpoints. Files like `api/users.rs` become `/api/users`.

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::{DxConfig, ScriptLanguage};
use crate::error::{DxError, DxResult};

// =============================================================================
// API Router
// =============================================================================

/// Router for API endpoints.
#[derive(Debug)]
pub struct ApiRouter {
    /// Registered routes
    routes: Vec<ApiRoute>,
    /// Route lookup by path
    lookup: HashMap<String, usize>,
    /// API directory root
    api_root: PathBuf,
}

impl ApiRouter {
    /// Create a new API router.
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            lookup: HashMap::new(),
            api_root: PathBuf::from("api"),
        }
    }

    /// Create a router with a custom API root.
    pub fn with_root(api_root: PathBuf) -> Self {
        Self {
            routes: Vec::new(),
            lookup: HashMap::new(),
            api_root,
        }
    }

    /// Scan the API directory and register all routes.
    pub fn scan(&mut self, project_root: &Path) -> DxResult<()> {
        let api_dir = project_root.join(&self.api_root);
        if !api_dir.exists() {
            return Ok(()); // No API directory is valid
        }

        self.scan_directory(&api_dir, &api_dir)?;
        Ok(())
    }

    /// Recursively scan a directory for API routes.
    fn scan_directory(&mut self, dir: &Path, api_root: &Path) -> DxResult<()> {
        let entries = std::fs::read_dir(dir).map_err(|e| DxError::IoError {
            path: Some(dir.to_path_buf()),
            message: e.to_string(),
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| DxError::IoError {
                path: Some(dir.to_path_buf()),
                message: e.to_string(),
            })?;
            let path = entry.path();

            if path.is_dir() {
                self.scan_directory(&path, api_root)?;
            } else if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy();
                if matches!(ext_str.as_ref(), "rs" | "py" | "js" | "ts" | "go") {
                    self.register_route(&path, api_root)?;
                }
            }
        }

        Ok(())
    }

    /// Register an API route from a file.
    fn register_route(&mut self, file: &Path, api_root: &Path) -> DxResult<()> {
        let relative = file.strip_prefix(api_root).map_err(|_| DxError::IoError {
            path: Some(file.to_path_buf()),
            message: "Failed to get relative path".to_string(),
        })?;

        // Convert file path to route path
        let route_path = self.file_to_route_path(relative);
        let methods = self.detect_methods(file)?;
        let language = self.detect_language(file);

        let route = ApiRoute {
            path: route_path.clone(),
            file: file.to_path_buf(),
            methods,
            language,
            handler: None,
        };

        let index = self.routes.len();
        self.lookup.insert(route_path, index);
        self.routes.push(route);

        Ok(())
    }

    /// Convert a file path to a route path.
    fn file_to_route_path(&self, relative: &Path) -> String {
        let mut path = String::from("/api");
        let mut is_index = false;

        for component in relative.components() {
            if let std::path::Component::Normal(part) = component {
                let part_str = part.to_string_lossy();
                // Remove file extension
                let name = if let Some(idx) = part_str.rfind('.') {
                    &part_str[..idx]
                } else {
                    &part_str
                };

                // Handle index files
                if name == "index" {
                    is_index = true;
                    continue;
                }

                // Handle dynamic segments [param] -> :param
                if name.starts_with('[') && name.ends_with(']') {
                    let param = &name[1..name.len() - 1];
                    if param.starts_with("...") {
                        // Catch-all: [...slug] -> *slug
                        path.push_str(&format!("/*{}", &param[3..]));
                    } else {
                        path.push_str(&format!("/:{}", param));
                    }
                } else {
                    path.push('/');
                    path.push_str(name);
                }
            }
        }

        // Add trailing slash for index routes (e.g., users/index.rs -> /api/users/)
        if is_index && !path.ends_with('/') {
            path.push('/');
        } else if path == "/api" {
            path.push('/');
        }

        path
    }

    /// Detect HTTP methods from file content.
    fn detect_methods(&self, file: &Path) -> DxResult<Vec<HttpMethod>> {
        let content = std::fs::read_to_string(file).map_err(|e| DxError::IoError {
            path: Some(file.to_path_buf()),
            message: e.to_string(),
        })?;

        let mut methods = Vec::new();

        // Look for exported handler functions
        if content.contains("pub async fn get")
            || content.contains("export async function GET")
            || content.contains("async def get")
            || content.contains("func Get")
        {
            methods.push(HttpMethod::Get);
        }
        if content.contains("pub async fn post")
            || content.contains("export async function POST")
            || content.contains("async def post")
            || content.contains("func Post")
        {
            methods.push(HttpMethod::Post);
        }
        if content.contains("pub async fn put")
            || content.contains("export async function PUT")
            || content.contains("async def put")
            || content.contains("func Put")
        {
            methods.push(HttpMethod::Put);
        }
        if content.contains("pub async fn delete")
            || content.contains("export async function DELETE")
            || content.contains("async def delete")
            || content.contains("func Delete")
        {
            methods.push(HttpMethod::Delete);
        }
        if content.contains("pub async fn patch")
            || content.contains("export async function PATCH")
            || content.contains("async def patch")
            || content.contains("func Patch")
        {
            methods.push(HttpMethod::Patch);
        }

        // Default to GET if no methods detected
        if methods.is_empty() {
            methods.push(HttpMethod::Get);
        }

        Ok(methods)
    }

    /// Detect the script language from file extension.
    fn detect_language(&self, file: &Path) -> ScriptLanguage {
        match file.extension().and_then(|e| e.to_str()) {
            Some("rs") => ScriptLanguage::Rust,
            Some("py") => ScriptLanguage::Python,
            Some("js") | Some("ts") => ScriptLanguage::JavaScript,
            Some("go") => ScriptLanguage::Go,
            _ => ScriptLanguage::Rust,
        }
    }

    /// Get all registered routes.
    pub fn routes(&self) -> &[ApiRoute] {
        &self.routes
    }

    /// Find a route by path.
    pub fn find_route(&self, path: &str) -> Option<&ApiRoute> {
        self.lookup.get(path).map(|&idx| &self.routes[idx])
    }

    /// Match a request path to a route, extracting parameters.
    pub fn match_route(&self, path: &str) -> Option<(&ApiRoute, HashMap<String, String>)> {
        for route in &self.routes {
            if let Some(params) = self.match_pattern(&route.path, path) {
                return Some((route, params));
            }
        }
        None
    }

    /// Match a pattern against a path, extracting parameters.
    fn match_pattern(&self, pattern: &str, path: &str) -> Option<HashMap<String, String>> {
        let pattern_parts: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
        let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        let mut params = HashMap::new();
        let mut pattern_idx = 0;
        let mut path_idx = 0;

        while pattern_idx < pattern_parts.len() && path_idx < path_parts.len() {
            let pattern_part = pattern_parts[pattern_idx];
            let path_part = path_parts[path_idx];

            if pattern_part.starts_with(':') {
                // Dynamic parameter
                let param_name = &pattern_part[1..];
                params.insert(param_name.to_string(), path_part.to_string());
            } else if pattern_part.starts_with('*') {
                // Catch-all - consume rest of path
                let param_name = &pattern_part[1..];
                let rest: Vec<&str> = path_parts[path_idx..].to_vec();
                params.insert(param_name.to_string(), rest.join("/"));
                return Some(params);
            } else if pattern_part != path_part {
                return None;
            }

            pattern_idx += 1;
            path_idx += 1;
        }

        if pattern_idx == pattern_parts.len() && path_idx == path_parts.len() {
            Some(params)
        } else {
            None
        }
    }
}

impl Default for ApiRouter {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// API Route
// =============================================================================

/// An API route definition.
#[derive(Debug, Clone)]
pub struct ApiRoute {
    /// Route path (e.g., "/api/users/:id")
    pub path: String,
    /// Source file path
    pub file: PathBuf,
    /// HTTP methods supported
    pub methods: Vec<HttpMethod>,
    /// Script language
    pub language: ScriptLanguage,
    /// Compiled handler (WASM bytes)
    pub handler: Option<Vec<u8>>,
}

impl ApiRoute {
    /// Check if this route supports a given HTTP method.
    pub fn supports_method(&self, method: HttpMethod) -> bool {
        self.methods.contains(&method)
    }
}

/// HTTP methods for API routes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    /// GET request
    Get,
    /// POST request
    Post,
    /// PUT request
    Put,
    /// DELETE request
    Delete,
    /// PATCH request
    Patch,
    /// HEAD request
    Head,
    /// OPTIONS request
    Options,
}

impl HttpMethod {
    /// Convert to string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        }
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// =============================================================================
// API Handler Compiler
// =============================================================================

/// Compiles API route handlers.
pub struct ApiCompiler {
    config: DxConfig,
}

impl ApiCompiler {
    /// Create a new API compiler.
    pub fn new(config: &DxConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Compile an API route handler.
    pub fn compile(&self, route: &ApiRoute) -> DxResult<Vec<u8>> {
        let source = std::fs::read_to_string(&route.file).map_err(|e| DxError::IoError {
            path: Some(route.file.clone()),
            message: e.to_string(),
        })?;

        // Generate wrapper code for request/response handling
        let wrapped = self.generate_wrapper(&source, route)?;

        // Compile based on language
        match route.language {
            ScriptLanguage::Rust => self.compile_rust(&wrapped),
            ScriptLanguage::JavaScript | ScriptLanguage::TypeScript => {
                self.compile_javascript(&wrapped)
            }
            ScriptLanguage::Python => self.compile_python(&wrapped),
            ScriptLanguage::Go => self.compile_go(&wrapped),
        }
    }

    /// Generate request/response wrapper code.
    fn generate_wrapper(&self, source: &str, _route: &ApiRoute) -> DxResult<String> {
        // For now, return source as-is
        // In production, this would add serialization/deserialization
        Ok(source.to_string())
    }

    /// Compile Rust API handler.
    fn compile_rust(&self, _source: &str) -> DxResult<Vec<u8>> {
        // Placeholder - would use rustc/wasm-pack
        Ok(Vec::new())
    }

    /// Compile JavaScript API handler.
    fn compile_javascript(&self, _source: &str) -> DxResult<Vec<u8>> {
        // Placeholder - would use esbuild/bun
        Ok(Vec::new())
    }

    /// Compile Python API handler.
    fn compile_python(&self, _source: &str) -> DxResult<Vec<u8>> {
        // Placeholder - would use dx-python
        Ok(Vec::new())
    }

    /// Compile Go API handler.
    fn compile_go(&self, _source: &str) -> DxResult<Vec<u8>> {
        // Placeholder - would use tinygo
        Ok(Vec::new())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_to_route_path() {
        let router = ApiRouter::new();

        assert_eq!(router.file_to_route_path(Path::new("users.rs")), "/api/users");
        assert_eq!(router.file_to_route_path(Path::new("users/index.rs")), "/api/users/");
        assert_eq!(router.file_to_route_path(Path::new("users/[id].rs")), "/api/users/:id");
        assert_eq!(router.file_to_route_path(Path::new("docs/[...slug].rs")), "/api/docs/*slug");
    }

    #[test]
    fn test_match_pattern() {
        let router = ApiRouter::new();

        // Static path
        let params = router.match_pattern("/api/users", "/api/users");
        assert!(params.is_some());
        assert!(params.unwrap().is_empty());

        // Dynamic parameter
        let params = router.match_pattern("/api/users/:id", "/api/users/123");
        assert!(params.is_some());
        assert_eq!(params.unwrap().get("id"), Some(&"123".to_string()));

        // Multiple parameters
        let params =
            router.match_pattern("/api/users/:userId/posts/:postId", "/api/users/1/posts/42");
        assert!(params.is_some());
        let params = params.unwrap();
        assert_eq!(params.get("userId"), Some(&"1".to_string()));
        assert_eq!(params.get("postId"), Some(&"42".to_string()));

        // Catch-all
        let params = router.match_pattern("/api/docs/*path", "/api/docs/guide/intro/basics");
        assert!(params.is_some());
        assert_eq!(params.unwrap().get("path"), Some(&"guide/intro/basics".to_string()));

        // Non-match
        let params = router.match_pattern("/api/users", "/api/posts");
        assert!(params.is_none());
    }

    #[test]
    fn test_http_method_display() {
        assert_eq!(HttpMethod::Get.to_string(), "GET");
        assert_eq!(HttpMethod::Post.to_string(), "POST");
        assert_eq!(HttpMethod::Delete.to_string(), "DELETE");
    }

    #[test]
    fn test_api_route_supports_method() {
        let route = ApiRoute {
            path: "/api/users".to_string(),
            file: PathBuf::from("api/users.rs"),
            methods: vec![HttpMethod::Get, HttpMethod::Post],
            language: ScriptLanguage::Rust,
            handler: None,
        };

        assert!(route.supports_method(HttpMethod::Get));
        assert!(route.supports_method(HttpMethod::Post));
        assert!(!route.supports_method(HttpMethod::Delete));
    }
}
