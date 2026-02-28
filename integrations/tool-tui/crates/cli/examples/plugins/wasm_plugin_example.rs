//! Example WASM Plugin for DX
//!
//! This demonstrates how to create a WASM plugin that can be loaded
//! by the DX plugin system.
//!
//! # Building
//!
//! ```bash
//! cargo component build --release
//! ```
//!
//! # Plugin Structure
//!
//! ```
//! hello-plugin/
//! ├── Cargo.toml
//! ├── plugin.sr
//! ├── wit/
//! │   └── world.wit
//! └── src/
//!     └── lib.rs (this file)
//! ```

// This example shows how a WASM plugin would be structured.
// In practice, you'd use `cargo component` to build this.

use std::collections::HashMap;

/// Plugin metadata
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub capabilities: Vec<String>,
}

/// Plugin result
pub enum PluginResult {
    Success(String),
    Error(String),
    Json(String),
    Binary(Vec<u8>),
}

/// Plugin context passed to each invocation
pub struct PluginContext {
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_dir: String,
}

/// Example: Hello World WASM Plugin
/// 
/// This plugin demonstrates the basic structure of a DX WASM plugin.
/// 
/// # Usage (after installing)
/// 
/// ```bash
/// dx hello              # prints "Hello, World!"
/// dx hello --name Alice # prints "Hello, Alice!"
/// ```
pub mod hello_plugin {
    use super::*;

    /// Returns plugin metadata
    pub fn metadata() -> PluginMetadata {
        PluginMetadata {
            name: "hello".to_string(),
            version: "1.0.0".to_string(),
            description: "A simple hello world plugin".to_string(),
            author: "DX Team".to_string(),
            capabilities: vec!["logging".to_string()],
        }
    }

    /// Main entry point
    pub fn execute(ctx: PluginContext) -> PluginResult {
        // Parse arguments
        let name = parse_name(&ctx.args);
        
        // Generate greeting
        let greeting = format!("Hello, {}!", name);
        
        // Log via host function (if capability granted)
        log_info(&greeting);
        
        PluginResult::Success(greeting)
    }

    fn parse_name(args: &[String]) -> String {
        for (i, arg) in args.iter().enumerate() {
            if arg == "--name" || arg == "-n" {
                if let Some(name) = args.get(i + 1) {
                    return name.clone();
                }
            }
        }
        "World".to_string()
    }

    fn log_info(message: &str) {
        // This would call a host function in actual WASM
        // For demo purposes, we just print
        println!("[INFO] {}", message);
    }
}

/// Example: Weather WASM Plugin
/// 
/// This plugin demonstrates HTTP capability usage.
/// 
/// # Usage
/// 
/// ```bash
/// dx weather Tokyo
/// dx weather --json London
/// ```
pub mod weather_plugin {
    use super::*;

    pub fn metadata() -> PluginMetadata {
        PluginMetadata {
            name: "weather".to_string(),
            version: "1.0.0".to_string(),
            description: "Get weather information".to_string(),
            author: "DX Team".to_string(),
            capabilities: vec!["http".to_string(), "logging".to_string()],
        }
    }

    pub fn execute(ctx: PluginContext) -> PluginResult {
        let location = ctx.args.get(0).map(|s| s.as_str()).unwrap_or("London");
        let json_output = ctx.args.iter().any(|a| a == "--json");

        // In actual WASM, this would call the host's http_request capability
        let weather = fetch_weather(location);

        if json_output {
            PluginResult::Json(format!(
                r#"{{"location":"{}","temperature":{},"condition":"{}"}}"#,
                weather.location, weather.temperature, weather.condition
            ))
        } else {
            PluginResult::Success(format!(
                "Weather in {}: {}°C, {}",
                weather.location, weather.temperature, weather.condition
            ))
        }
    }

    struct Weather {
        location: String,
        temperature: i32,
        condition: String,
    }

    fn fetch_weather(location: &str) -> Weather {
        // Simulated - in real plugin, this calls host HTTP capability
        Weather {
            location: location.to_string(),
            temperature: 22,
            condition: "Partly Cloudy".to_string(),
        }
    }
}

/// Example: Calculator WASM Plugin
/// 
/// This plugin demonstrates pure computation (no capabilities needed).
/// 
/// # Usage
/// 
/// ```bash
/// dx calc "2 + 2"
/// dx calc "sin(45)"
/// dx calc "sqrt(16)"
/// ```
pub mod calc_plugin {
    use super::*;

    pub fn metadata() -> PluginMetadata {
        PluginMetadata {
            name: "calc".to_string(),
            version: "1.0.0".to_string(),
            description: "Simple calculator".to_string(),
            author: "DX Team".to_string(),
            capabilities: vec![], // No capabilities needed!
        }
    }

    pub fn execute(ctx: PluginContext) -> PluginResult {
        let expr = ctx.args.join(" ");
        
        match evaluate(&expr) {
            Ok(result) => PluginResult::Success(format!("{} = {}", expr, result)),
            Err(e) => PluginResult::Error(format!("Error: {}", e)),
        }
    }

    fn evaluate(expr: &str) -> Result<f64, String> {
        // Simple expression parser (demo only)
        let expr = expr.trim();
        
        // Handle basic operations
        if let Some(pos) = expr.find('+') {
            let left: f64 = expr[..pos].trim().parse().map_err(|_| "Invalid number")?;
            let right: f64 = expr[pos+1..].trim().parse().map_err(|_| "Invalid number")?;
            return Ok(left + right);
        }
        if let Some(pos) = expr.find('-') {
            if pos > 0 {
                let left: f64 = expr[..pos].trim().parse().map_err(|_| "Invalid number")?;
                let right: f64 = expr[pos+1..].trim().parse().map_err(|_| "Invalid number")?;
                return Ok(left - right);
            }
        }
        if let Some(pos) = expr.find('*') {
            let left: f64 = expr[..pos].trim().parse().map_err(|_| "Invalid number")?;
            let right: f64 = expr[pos+1..].trim().parse().map_err(|_| "Invalid number")?;
            return Ok(left * right);
        }
        if let Some(pos) = expr.find('/') {
            let left: f64 = expr[..pos].trim().parse().map_err(|_| "Invalid number")?;
            let right: f64 = expr[pos+1..].trim().parse().map_err(|_| "Invalid number")?;
            if right == 0.0 {
                return Err("Division by zero".to_string());
            }
            return Ok(left / right);
        }

        // Handle functions
        if expr.starts_with("sqrt(") && expr.ends_with(")") {
            let num: f64 = expr[5..expr.len()-1].trim().parse().map_err(|_| "Invalid number")?;
            return Ok(num.sqrt());
        }
        if expr.starts_with("sin(") && expr.ends_with(")") {
            let num: f64 = expr[4..expr.len()-1].trim().parse().map_err(|_| "Invalid number")?;
            return Ok(num.to_radians().sin());
        }
        if expr.starts_with("cos(") && expr.ends_with(")") {
            let num: f64 = expr[4..expr.len()-1].trim().parse().map_err(|_| "Invalid number")?;
            return Ok(num.to_radians().cos());
        }

        // Try parsing as a plain number
        expr.parse().map_err(|_| "Unknown expression".to_string())
    }
}

// Plugin.sr manifest example
// 
// ```sr
// [plugin]
// name = "hello"
// version = "1.0.0"
// description = "A simple hello world plugin"
// author = "Your Name"
// license = "MIT"
// 
// [plugin.capabilities]
// required = ["logging"]
// optional = ["http"]
// 
// [plugin.commands]
// hello = { description = "Say hello", args = "[--name NAME]" }
// 
// [plugin.build]
// entry_point = "target/wasm32-wasip2/release/hello_plugin.wasm"
// min_dx_version = "0.1.0"
// ```

// WIT interface definition (wit/world.wit)
//
// ```wit
// package dx:plugin;
// 
// interface host {
//     // Logging
//     log-info: func(message: string);
//     log-warn: func(message: string);
//     log-error: func(message: string);
//     
//     // HTTP (requires capability)
//     record http-request {
//         method: string,
//         url: string,
//         headers: list<tuple<string, string>>,
//         body: option<list<u8>>,
//     }
//     
//     record http-response {
//         status: u16,
//         headers: list<tuple<string, string>>,
//         body: list<u8>,
//     }
//     
//     http-request: func(req: http-request) -> result<http-response, string>;
//     
//     // Key-Value (requires capability)
//     kv-get: func(key: string) -> option<list<u8>>;
//     kv-set: func(key: string, value: list<u8>) -> result<_, string>;
//     kv-delete: func(key: string) -> bool;
// }
// 
// interface plugin {
//     record metadata {
//         name: string,
//         version: string,
//         description: string,
//         author: string,
//         capabilities: list<string>,
//     }
//     
//     record context {
//         args: list<string>,
//         env: list<tuple<string, string>>,
//         working-dir: string,
//     }
//     
//     variant result {
//         success(string),
//         error(string),
//         json(string),
//         binary(list<u8>),
//     }
//     
//     metadata: func() -> metadata;
//     execute: func(ctx: context) -> result;
// }
// 
// world dx-plugin {
//     import host;
//     export plugin;
// }
// ```

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_metadata() {
        let meta = hello_plugin::metadata();
        assert_eq!(meta.name, "hello");
        assert_eq!(meta.capabilities, vec!["logging"]);
    }

    #[test]
    fn test_hello_execute() {
        let ctx = PluginContext {
            args: vec!["--name".to_string(), "Alice".to_string()],
            env: HashMap::new(),
            working_dir: ".".to_string(),
        };
        
        match hello_plugin::execute(ctx) {
            PluginResult::Success(msg) => assert_eq!(msg, "Hello, Alice!"),
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn test_calc_add() {
        let ctx = PluginContext {
            args: vec!["2".to_string(), "+".to_string(), "3".to_string()],
            env: HashMap::new(),
            working_dir: ".".to_string(),
        };
        
        match calc_plugin::execute(ctx) {
            PluginResult::Success(msg) => assert!(msg.contains("= 5")),
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn test_calc_sqrt() {
        let ctx = PluginContext {
            args: vec!["sqrt(16)".to_string()],
            env: HashMap::new(),
            working_dir: ".".to_string(),
        };
        
        match calc_plugin::execute(ctx) {
            PluginResult::Success(msg) => assert!(msg.contains("= 4")),
            _ => panic!("Expected Success"),
        }
    }
}
