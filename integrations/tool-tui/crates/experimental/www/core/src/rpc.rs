//! # RPC Module - The Bridge
//!
//! Connects `server/api/*.fn.dx` to client-side code via binary streaming RPC.
//!
//! ## Architecture
//! - Server: Exports WASM-callable handlers at `/rpc/{module}.{function}`
//! - Client: Generates "Ghost Functions" that call the RPC endpoint
//!
//! ## Example
//! ```dx
//! // server/api/user.fn.dx
//! export fn login(email: string, password: string) -> Result<Session> {
//!     // Server-side logic
//! }
//! ```
//! Becomes callable from client as `user.login("me@mail.com", "pass")`

use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// A parsed server function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerFunction {
    /// Module name (from filename, e.g. "user" from "user.fn.dx")
    pub module: String,
    /// Function name
    pub name: String,
    /// Full qualified name: "module.function"
    pub fqn: String,
    /// Parameter names and types
    pub params: Vec<(String, String)>,
    /// Return type (if any)
    pub return_type: Option<String>,
    /// Source file path
    pub source_path: PathBuf,
}

/// RPC Registry containing all server functions
#[derive(Debug, Clone, Default)]
pub struct RpcRegistry {
    pub functions: HashMap<String, ServerFunction>,
}

impl RpcRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up a function by fully qualified name
    pub fn get(&self, fqn: &str) -> Option<&ServerFunction> {
        self.functions.get(fqn)
    }
}

/// Scan server/api/ directory for .fn.dx files and extract exported functions
pub fn scan_server_api(root: &Path, verbose: bool) -> Result<RpcRegistry> {
    let api_dir = root.join("server").join("api");
    let mut registry = RpcRegistry::new();

    if !api_dir.exists() {
        if verbose {
            println!("  📡 RPC: No server/api/ directory found, skipping");
        }
        return Ok(registry);
    }

    if verbose {
        println!("  📡 RPC: Scanning server/api/...");
    }

    for entry in WalkDir::new(&api_dir).follow_links(true).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        // Only process .fn.dx files
        if !path.is_file() {
            continue;
        }

        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !filename.ends_with(".fn.dx") {
            continue;
        }

        // Extract module name: "user.fn.dx" -> "user"
        let module = filename.trim_end_matches(".fn.dx").to_string();

        if verbose {
            println!("    📄 Found RPC module: {}", module);
        }

        // Parse the file for exported functions
        let functions = parse_fn_file(path, &module, verbose)?;

        for func in functions {
            if verbose {
                println!(
                    "      + {}({})",
                    func.fqn,
                    func.params
                        .iter()
                        .map(|(n, t)| format!("{}: {}", n, t))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            registry.functions.insert(func.fqn.clone(), func);
        }
    }

    if verbose {
        println!("  📡 RPC: Indexed {} server functions", registry.functions.len());
    }

    Ok(registry)
}

/// Parse a .fn.dx file for exported functions
fn parse_fn_file(path: &Path, module: &str, _verbose: bool) -> Result<Vec<ServerFunction>> {
    let source =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;

    let mut functions = Vec::new();

    // Regex to match: export fn name(params) -> ReturnType
    // Simplified pattern - production would use proper parser
    let fn_regex = Regex::new(r"export\s+fn\s+(\w+)\s*\(([^)]*)\)(?:\s*->\s*(\S+))?").unwrap();

    for cap in fn_regex.captures_iter(&source) {
        let name = cap[1].to_string();
        let params_str = cap.get(2).map_or("", |m| m.as_str());
        let return_type = cap.get(3).map(|m| m.as_str().to_string());

        // Parse parameters
        let params = parse_params(params_str);

        let fqn = format!("{}.{}", module, name);

        functions.push(ServerFunction {
            module: module.to_string(),
            name,
            fqn,
            params,
            return_type,
            source_path: path.to_path_buf(),
        });
    }

    Ok(functions)
}

/// Parse parameter string: "email: string, password: string" -> vec of (name, type)
fn parse_params(params_str: &str) -> Vec<(String, String)> {
    if params_str.trim().is_empty() {
        return Vec::new();
    }

    params_str
        .split(',')
        .map(|p| {
            let parts: Vec<&str> = p.trim().splitn(2, ':').collect();
            if parts.len() == 2 {
                (parts[0].trim().to_string(), parts[1].trim().to_string())
            } else {
                (parts[0].trim().to_string(), "any".to_string())
            }
        })
        .collect()
}

/// Generate client-side "Ghost Functions" for RPC calls
pub fn generate_client_ghosts(registry: &RpcRegistry, verbose: bool) -> Result<String> {
    if verbose {
        println!("  👻 Generating client ghost stubs...");
    }

    let mut code = String::new();
    code.push_str("// Auto-generated RPC Ghost Functions\n");
    code.push_str("// DO NOT EDIT - Generated by dx-compiler\n\n");

    // Group functions by module
    let mut modules: HashMap<String, Vec<&ServerFunction>> = HashMap::new();
    for func in registry.functions.values() {
        modules.entry(func.module.clone()).or_default().push(func);
    }

    for (module, funcs) in &modules {
        code.push_str(&format!("pub mod {} {{\n", module));
        code.push_str("    use super::*;\n\n");

        for func in funcs {
            // Generate the ghost function
            let params_sig: String = func
                .params
                .iter()
                .map(|(name, ty)| format!("{}: {}", name, rust_type(ty)))
                .collect::<Vec<_>>()
                .join(", ");

            let return_ty = func
                .return_type
                .as_ref()
                .map(|t| rust_type(t))
                .unwrap_or_else(|| "()".to_string());

            code.push_str(&format!(
                "    pub fn {}({}) -> {} {{\n",
                func.name, params_sig, return_ty
            ));

            // Serialize arguments
            let args: String =
                func.params.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>().join(", ");

            code.push_str(&format!("        dx_rpc::call(\"{}\", ({}))\n", func.fqn, args));
            code.push_str("    }\n\n");
        }

        code.push_str("}\n\n");
    }

    Ok(code)
}

/// Generate server-side WASM handlers for RPC
pub fn generate_server_handlers(registry: &RpcRegistry, verbose: bool) -> Result<String> {
    if verbose {
        println!("  🖥️ Generating server RPC handlers...");
    }

    let mut code = String::new();
    code.push_str("// Auto-generated RPC Server Handlers\n");
    code.push_str("// DO NOT EDIT - Generated by dx-compiler\n\n");
    code.push_str("use dx_server::rpc::*;\n\n");

    code.push_str("pub fn register_rpc_handlers(router: &mut RpcRouter) {\n");

    for func in registry.functions.values() {
        code.push_str(&format!("    router.register(\"{}\", |args| {{\n", func.fqn));
        code.push_str("        // TODO: Deserialize args and call actual implementation\n");
        code.push_str(&format!("        {}::{}(args)\n", func.module, func.name));
        code.push_str("    });\n");
    }

    code.push_str("}\n");

    Ok(code)
}

/// Convert dx type to Rust type
fn rust_type(dx_type: &str) -> String {
    match dx_type.trim() {
        "string" => "String".to_string(),
        "number" => "f64".to_string(),
        "int" => "i64".to_string(),
        "bool" | "boolean" => "bool".to_string(),
        "void" | "()" => "()".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_params() {
        let params = parse_params("email: string, password: string");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], ("email".to_string(), "string".to_string()));
        assert_eq!(params[1], ("password".to_string(), "string".to_string()));
    }

    #[test]
    fn test_rust_type() {
        assert_eq!(rust_type("string"), "String");
        assert_eq!(rust_type("number"), "f64");
        assert_eq!(rust_type("bool"), "bool");
    }
}
