//! # CLI Commands
//!
//! This module provides CLI command handlers for the DX WWW Framework.
//!
//! Commands:
//! - `dx new <name>` - Create a new project  
//! - `dx create <name>` - Alias for `dx new`
//! - `dx dev` - Start development server
//! - `dx build` - Build for production
//! - `dx generate <type> <name>` - Generate new files
//! - `dx add <component>` - Add a component from the library

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::DxConfig;
use crate::error::{DxError, DxResult};

// Component library
use dx_compiler::components::{get_all_components, get_component, ComponentDef};

// =============================================================================
// CLI
// =============================================================================

/// CLI command runner.
#[derive(Debug)]
pub struct Cli {
    /// Current working directory
    cwd: PathBuf,
}

impl Cli {
    /// Create a new CLI instance.
    pub fn new() -> Self {
        Self {
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Create with a specific working directory.
    pub fn with_cwd(cwd: PathBuf) -> Self {
        Self { cwd }
    }

    /// Run the CLI.
    ///
    /// Parses command-line arguments and executes the appropriate command.
    pub fn run() -> DxResult<()> {
        let args: Vec<String> = std::env::args().collect();

        if args.len() < 2 {
            Self::print_help();
            return Ok(());
        }

        let cli = Cli::new();

        match args[1].as_str() {
            "new" | "create" => {
                if args.len() < 3 {
                    eprintln!("Error: Project name required");
                    eprintln!("Usage: dx new <name>");
                    return Err(DxError::ConfigValidationError {
                        message: "Project name required".to_string(),
                        field: Some("name".to_string()),
                    });
                }
                cli.cmd_new(&args[2])?;
            }
            "dev" => {
                cli.cmd_dev()?;
            }
            "build" => {
                cli.cmd_build()?;
            }
            "generate" | "g" => {
                if args.len() < 4 {
                    eprintln!("Error: Type and name required");
                    eprintln!("Usage: dx generate <type> <name>");
                    return Err(DxError::ConfigValidationError {
                        message: "Type and name required".to_string(),
                        field: Some("type/name".to_string()),
                    });
                }
                cli.cmd_generate(&args[2], &args[3])?;
            }
            "add" => {
                if args.len() < 3 {
                    eprintln!("Error: Component name required");
                    eprintln!("Usage: dx add <component>");
                    eprintln!("       dx add --all");
                    eprintln!("       dx add --list");
                    return Err(DxError::ConfigValidationError {
                        message: "Component name required".to_string(),
                        field: Some("component".to_string()),
                    });
                }
                let components: Vec<&str> = args[2..].iter().map(|s| s.as_str()).collect();
                cli.cmd_add(&components)?;
            }
            "help" | "--help" | "-h" => {
                Self::print_help();
            }
            "version" | "--version" | "-v" => {
                Self::print_version();
            }
            cmd => {
                eprintln!("Unknown command: {}", cmd);
                Self::print_help();
                return Err(DxError::ConfigValidationError {
                    message: format!("Unknown command: {}", cmd),
                    field: None,
                });
            }
        }

        Ok(())
    }

    /// Print help message.
    fn print_help() {
        eprintln!("dx-www: Binary-first web framework");
        eprintln!();
        eprintln!("USAGE:");
        eprintln!("    dx <COMMAND> [OPTIONS]");
        eprintln!();
        eprintln!("COMMANDS:");
        eprintln!("    new, create <name>      Create a new project");
        eprintln!("    dev                     Start development server");
        eprintln!("    build                   Build for production");
        eprintln!("    generate <type> <name>  Generate new files (alias: g)");
        eprintln!("    add <component>         Add component to project");
        eprintln!("    help                    Show this help message");
        eprintln!("    version                 Show version information");
        eprintln!();
        eprintln!("GENERATE TYPES:");
        eprintln!("    page       Generate a new page (.pg)");
        eprintln!("    component  Generate a new component (.cp)");
        eprintln!("    api        Generate a new API route");
        eprintln!("    layout     Generate a new layout (.lyt)");
        eprintln!();
        eprintln!("ADD OPTIONS:");
        eprintln!("    dx add button           Add Button component");
        eprintln!("    dx add button card      Add multiple components");
        eprintln!("    dx add --all            Add all components");
        eprintln!("    dx add --list           List available components");
        eprintln!();
        eprintln!("OPTIONS:");
        eprintln!("    -h, --help     Show help");
        eprintln!("    -v, --version  Show version");
        eprintln!();
        eprintln!("Run `dx <COMMAND> --help` for more information.");
    }

    /// Print version.
    fn print_version() {
        eprintln!("dx-www {}", env!("CARGO_PKG_VERSION"));
    }

    /// Create a new project.
    pub fn cmd_new(&self, name: &str) -> DxResult<()> {
        let project_dir = self.cwd.join(name);

        if project_dir.exists() {
            return Err(DxError::IoError {
                path: Some(project_dir),
                message: "Directory already exists".to_string(),
            });
        }

        eprintln!("Creating new project: {}", name);

        // Create directory structure
        std::fs::create_dir_all(&project_dir).map_err(|e| DxError::IoError {
            path: Some(project_dir.clone()),
            message: e.to_string(),
        })?;

        let dirs = ["pages", "components", "api", "styles", "public"];
        for dir in dirs {
            std::fs::create_dir_all(project_dir.join(dir)).map_err(|e| DxError::IoError {
                path: Some(project_dir.join(dir)),
                message: e.to_string(),
            })?;
        }

        // Create config file
        let config_content = format!(
            r#"[project]
name = "{}"
version = "0.1.0"

[build]
output_dir = ".dx/build"
optimization_level = "release"

[dev]
port = 3000
hot_reload = true
"#,
            name
        );
        std::fs::write(project_dir.join("dx.config.toml"), config_content).map_err(|e| {
            DxError::IoError {
                path: Some(project_dir.join("dx.config.toml")),
                message: e.to_string(),
            }
        })?;

        // Create index page
        let index_content = r#"<script lang="rust">
pub struct Props {
    title: String,
}

pub async fn load() -> Props {
    Props {
        title: "Welcome to DX".to_string(),
    }
}
</script>

<page>
    <main class="max-w-4xl mx-auto px-8 py-8 font-sans">
        <h1 class="text-3xl font-bold text-gray-900">{title}</h1>
        <p class="mt-4 text-gray-600">Edit pages/index.pg to get started.</p>
    </main>
</page>
"#;
        std::fs::write(project_dir.join("pages/index.pg"), index_content).map_err(|e| {
            DxError::IoError {
                path: Some(project_dir.join("pages/index.pg")),
                message: e.to_string(),
            }
        })?;

        // Create root layout
        let layout_content = r#"<script lang="rust">
pub struct Props {
    children: Children,
}
</script>

<page>
    <html lang="en">
        <head>
            <meta charset="UTF-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1.0" />
            <title>DX App</title>
        </head>
        <body class="m-0 leading-normal">
            <slot />
        </body>
    </html>
</page>
"#;
        std::fs::write(project_dir.join("pages/_layout.pg"), layout_content).map_err(|e| {
            DxError::IoError {
                path: Some(project_dir.join("pages/_layout.pg")),
                message: e.to_string(),
            }
        })?;

        // Create global styles
        let styles_content = r#"/* Global styles */
*, *::before, *::after {
    box-sizing: border-box;
}

body {
    margin: 0;
    line-height: 1.5;
}
"#;
        std::fs::write(project_dir.join("styles/global.css"), styles_content).map_err(|e| {
            DxError::IoError {
                path: Some(project_dir.join("styles/global.css")),
                message: e.to_string(),
            }
        })?;

        eprintln!("✓ Created project structure");
        eprintln!("✓ Created dx.config.toml");
        eprintln!("✓ Created pages/index.pg");
        eprintln!("✓ Created pages/_layout.pg");
        eprintln!("✓ Created styles/global.css");
        eprintln!();
        eprintln!("Next steps:");
        eprintln!("  cd {}", name);
        eprintln!("  dx dev");

        Ok(())
    }

    /// Start development server.
    pub fn cmd_dev(&self) -> DxResult<()> {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        eprintln!("Starting development server...");

        // Load config
        let config_path = self.cwd.join("dx.config.toml");
        let config = if config_path.exists() {
            DxConfig::load(&config_path)?
        } else {
            DxConfig::default()
        };

        let port = config.dev.port;
        // Use 127.0.0.1 instead of localhost for better Windows compatibility
        let addr = format!("127.0.0.1:{}", port);

        // Load translations first
        let translations = self.load_translations()?;
        
        // Start HTTP server - bind first to catch errors
        let listener = TcpListener::bind(&addr).map_err(|e| DxError::IoError {
            path: None,
            message: format!("Failed to bind to {}: {}", addr, e),
        })?;

        eprintln!();
        eprintln!("🚀 Development server running at http://localhost:{}", port);
        eprintln!("   Hot reload: {}", if config.dev.hot_reload { "enabled" } else { "disabled" });
        eprintln!("   Project: {}", config.project.name);
        eprintln!();
        eprintln!("Press Ctrl+C to stop");
        eprintln!();

        let cwd = self.cwd.clone();
        
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let cwd = cwd.clone();
                    let translations = translations.clone();
                    
                    std::thread::spawn(move || {
                        let mut buffer = [0; 8192];
                        if let Ok(n) = stream.read(&mut buffer) {
                            let request = String::from_utf8_lossy(&buffer[..n]);
                            let path = Self::extract_request_path(&request);
                            
                            let (status, content_type, body) = Self::handle_request(&cwd, &path, &translations);
                            
                            let response = format!(
                                "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                status, content_type, body.len(), body
                            );
                            
                            let _ = stream.write_all(response.as_bytes());
                            let _ = stream.flush();
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Connection error: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Extract path from HTTP request
    fn extract_request_path(request: &str) -> String {
        request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("/")
            .to_string()
    }

    /// Handle HTTP request
    fn handle_request(cwd: &PathBuf, path: &str, translations: &HashMap<String, String>) -> (String, String, String) {
        // Serve static files
        if path.starts_with("/styles/") || path.starts_with("/public/") {
            let file_path = cwd.join(path.trim_start_matches('/'));
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                let content_type = if path.ends_with(".css") {
                    "text/css; charset=utf-8"
                } else if path.ends_with(".js") {
                    "application/javascript; charset=utf-8"
                } else if path.ends_with(".json") {
                    "application/json; charset=utf-8"
                } else if path.ends_with(".svg") {
                    "image/svg+xml"
                } else if path.ends_with(".png") {
                    "image/png"
                } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
                    "image/jpeg"
                } else {
                    "text/plain; charset=utf-8"
                };
                return ("200 OK".to_string(), content_type.to_string(), content);
            }
        }

        // Render page
        let page_name = if path == "/" { "index" } else { path.trim_start_matches('/').trim_end_matches('/') };
        let page_path = cwd.join("pages").join(format!("{}.pg", page_name));

        if page_path.exists() {
            match Self::render_page(cwd, &page_path, translations) {
                Ok(html) => ("200 OK".to_string(), "text/html; charset=utf-8".to_string(), html),
                Err(e) => ("500 Internal Server Error".to_string(), "text/html; charset=utf-8".to_string(), 
                    Self::render_error_page("500", &format!("Render Error: {}", e))),
            }
        } else {
            ("404 Not Found".to_string(), "text/html; charset=utf-8".to_string(), 
                Self::render_error_page("404", &format!("Page not found: {}", path)))
        }
    }

    /// Render a .pg page file to HTML
    fn render_page(cwd: &PathBuf, page_path: &PathBuf, translations: &HashMap<String, String>) -> Result<String, String> {
        let content = std::fs::read_to_string(page_path)
            .map_err(|e| format!("Failed to read page: {}", e))?;

        // Extract template content - supports <page>, <template>, or raw content
        let template = if let Some(start) = content.find("<page>") {
            if let Some(end) = content.rfind("</page>") {
                content[start + 6..end].to_string()
            } else {
                return Err("Missing </page> tag".to_string());
            }
        } else if let Some(start) = content.find("<template>") {
            if let Some(end) = content.rfind("</template>") {
                content[start + 10..end].to_string()
            } else {
                return Err("Missing </template> tag".to_string());
            }
        } else {
            // Try to extract just the content between body or main tags or use everything
            content.clone()
        };

        // Load layout
        let layout_path = cwd.join("pages/_layout.pg");
        let layout_content = if layout_path.exists() {
            let layout_raw = std::fs::read_to_string(&layout_path)
                .map_err(|e| format!("Failed to read layout: {}", e))?;
            if let Some(start) = layout_raw.find("<page>") {
                if let Some(end) = layout_raw.rfind("</page>") {
                    layout_raw[start + 6..end].to_string()
                } else {
                    Self::default_layout()
                }
            } else if let Some(start) = layout_raw.find("<template>") {
                if let Some(end) = layout_raw.rfind("</template>") {
                    layout_raw[start + 10..end].to_string()
                } else {
                    Self::default_layout()
                }
            } else {
                Self::default_layout()
            }
        } else {
            Self::default_layout()
        };

        // Process template
        let mut html = template;
        
        // Replace i18n calls: {t!("key")}
        let re = regex::Regex::new(r#"\{t!\("([^"]+)"\)\}"#).unwrap();
        html = re.replace_all(&html, |caps: &regex::Captures| {
            let key = &caps[1];
            translations.get(key).cloned().unwrap_or_else(|| format!("[{}]", key))
        }).to_string();

        // Replace variables with defaults
        html = html.replace("{title}", translations.get("home.title").unwrap_or(&"DX WWW".to_string()));
        html = html.replace("{description}", translations.get("home.description").unwrap_or(&"Binary-first web framework".to_string()));
        html = html.replace("{total_users}", "10,000+");
        html = html.replace("{github_stars}", "5,000");

        // Process components - render them inline
        html = Self::process_components(cwd, &html, translations)?;

        // Process dx-icon elements
        html = Self::process_icons(&html);

        // Remove dx-animate attributes for static rendering
        let attr_re = regex::Regex::new(r#"\s*dx-animate[^=]*="[^"]*""#).unwrap();
        html = attr_re.replace_all(&html, "").to_string();

        // Insert into layout
        let final_html = layout_content.replace("{children}", &html);

        // Process layout i18n
        let final_html = re.replace_all(&final_html, |caps: &regex::Captures| {
            let key = &caps[1];
            translations.get(key).cloned().unwrap_or_else(|| format!("[{}]", key))
        }).to_string();

        Ok(Self::wrap_with_html_shell(&final_html))
    }

    /// Process component references in template
    fn process_components(cwd: &PathBuf, html: &str, translations: &HashMap<String, String>) -> Result<String, String> {
        let mut result = html.to_string();
        
        // List of known components to process
        let components = [
            ("Button", "ui/Button"),
            ("Card", "ui/Card"),
            ("Badge", "ui/Badge"),
            ("Input", "ui/Input"),
            ("FeatureCard", "FeatureCard"),
            ("StatCard", "StatCard"),
            ("GlowButton", "ui/GlowButton"),
            ("AnimatedCard", "ui/AnimatedCard"),
            ("ParticleBackground", "ui/ParticleBackground"),
            ("Textarea", "ui/Textarea"),
        ];

        for (name, path) in components {
            let component_path = cwd.join("components").join(format!("{}.cp", path));
            if component_path.exists() {
                result = Self::expand_component(&result, name, &component_path, translations)?;
            }
        }

        Ok(result)
    }

    /// Expand a component tag into its template
    fn expand_component(html: &str, name: &str, _component_path: &PathBuf, _translations: &HashMap<String, String>) -> Result<String, String> {
        let mut result = html.to_string();
        
        // Simple component expansion - replace self-closing and regular tags
        // For a production system, this would be much more sophisticated
        
        match name {
            "Button" => {
                // Expand Button component
                let re = regex::Regex::new(r#"<Button\s+([^>]*)>([^<]*)</Button>"#).unwrap();
                result = re.replace_all(&result, |caps: &regex::Captures| {
                    let attrs = &caps[1];
                    let content = &caps[2];
                    let href = Self::extract_attr(attrs, "href");
                    let variant = Self::extract_attr(attrs, "variant").unwrap_or("primary".to_string());
                    let size = Self::extract_attr(attrs, "size").unwrap_or("md".to_string());
                    
                    let variant_class = match variant.as_str() {
                        "primary" => "bg-emerald-600 text-white hover:bg-emerald-700",
                        "secondary" => "bg-slate-800 text-white hover:bg-slate-700",
                        "white" => "bg-white text-slate-900 hover:bg-slate-100",
                        _ => "bg-emerald-600 text-white hover:bg-emerald-700",
                    };
                    let size_class = match size.as_str() {
                        "sm" => "px-3 py-1.5 text-sm",
                        "md" => "px-4 py-2 text-base",
                        "lg" => "px-6 py-3 text-lg",
                        _ => "px-4 py-2 text-base",
                    };

                    if let Some(href) = href {
                        format!(r#"<a href="{}" class="inline-flex items-center justify-center font-medium rounded-lg transition-all {} {}">{}</a>"#, 
                            href, variant_class, size_class, content)
                    } else {
                        format!(r#"<button class="inline-flex items-center justify-center font-medium rounded-lg transition-all {} {}">{}</button>"#, 
                            variant_class, size_class, content)
                    }
                }).to_string();
            }
            "Card" => {
                let re = regex::Regex::new(r#"<Card\s*([^>]*)>([\s\S]*?)</Card>"#).unwrap();
                result = re.replace_all(&result, |caps: &regex::Captures| {
                    let attrs = &caps[1];
                    let content = &caps[2];
                    let class = Self::extract_attr(attrs, "class").unwrap_or_default();
                    format!(r#"<div class="bg-slate-900 border border-slate-800 rounded-xl shadow-xl {}">{}</div>"#, class, content)
                }).to_string();
            }
            "Badge" => {
                let re = regex::Regex::new(r#"<Badge\s*([^>]*)>([\s\S]*?)</Badge>"#).unwrap();
                result = re.replace_all(&result, |caps: &regex::Captures| {
                    let attrs = &caps[1];
                    let content = &caps[2];
                    let variant = Self::extract_attr(attrs, "variant").unwrap_or("primary".to_string());
                    let class = match variant.as_str() {
                        "emerald" => "bg-emerald-500/20 text-emerald-400 border border-emerald-500/30",
                        "teal" => "bg-teal-500/20 text-teal-400 border border-teal-500/30",
                        _ => "bg-emerald-600 text-white",
                    };
                    format!(r#"<span class="inline-flex items-center px-3 py-1 rounded-full text-sm font-medium {}">{}</span>"#, class, content)
                }).to_string();
            }
            "FeatureCard" => {
                let re = regex::Regex::new(r#"<FeatureCard\s+([^/]*)/?>"#).unwrap();
                result = re.replace_all(&result, |caps: &regex::Captures| {
                    let attrs = &caps[1];
                    let icon = Self::extract_attr(attrs, "icon").unwrap_or("zap".to_string());
                    let title = Self::extract_attr(attrs, "title").unwrap_or("Feature".to_string());
                    let description = Self::extract_attr(attrs, "description").unwrap_or_default();
                    format!(r#"<div class="bg-slate-900 border border-slate-800 rounded-xl p-6 hover:border-emerald-500 transition-all group">
                        <svg class="w-12 h-12 mb-4 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><title>{}</title><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/></svg>
                        <h3 class="text-xl font-bold text-white mb-2">{}</h3>
                        <p class="text-slate-300">{}</p>
                    </div>"#, icon, title, description)
                }).to_string();
            }
            "StatCard" => {
                let re = regex::Regex::new(r#"<StatCard\s+([^/]*)/?>"#).unwrap();
                result = re.replace_all(&result, |caps: &regex::Captures| {
                    let attrs = &caps[1];
                    let number = Self::extract_attr(attrs, "number").unwrap_or("0".to_string());
                    let label = Self::extract_attr(attrs, "label").unwrap_or("Label".to_string());
                    format!(r#"<div class="text-center p-8">
                        <div class="text-5xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-emerald-400 to-teal-400 mb-2">{}</div>
                        <div class="text-slate-400 text-sm uppercase tracking-wider">{}</div>
                    </div>"#, number, label)
                }).to_string();
            }
            "GlowButton" => {
                let re = regex::Regex::new(r#"<GlowButton\s+([^>]*)>([\s\S]*?)</GlowButton>"#).unwrap();
                result = re.replace_all(&result, |caps: &regex::Captures| {
                    let attrs = &caps[1];
                    let content = &caps[2];
                    let href = Self::extract_attr(attrs, "href").unwrap_or("#".to_string());
                    format!(r#"<a href="{}" class="group relative inline-flex items-center justify-center overflow-hidden rounded-lg p-0.5 text-sm font-medium text-white">
                        <span class="absolute h-full w-full bg-gradient-to-br from-emerald-600 to-emerald-400 opacity-70 group-hover:opacity-100 transition-opacity"></span>
                        <span class="relative flex items-center gap-2 rounded-md bg-slate-950 px-6 py-3 transition-all group-hover:bg-transparent">{}</span>
                    </a>"#, href, content)
                }).to_string();
            }
            "AnimatedCard" => {
                let re = regex::Regex::new(r#"<AnimatedCard\s*([^>]*)>([\s\S]*?)</AnimatedCard>"#).unwrap();
                result = re.replace_all(&result, |caps: &regex::Captures| {
                    let content = &caps[2];
                    format!(r#"<div class="animate-fade-in">{}</div>"#, content)
                }).to_string();
            }
            "ParticleBackground" => {
                let re = regex::Regex::new(r#"<ParticleBackground\s*[^/]*/>"#).unwrap();
                result = re.replace_all(&result, r#"<div class="absolute inset-0 overflow-hidden pointer-events-none">
                    <div class="absolute inset-0 bg-gradient-to-t from-slate-950 via-transparent to-transparent"></div>
                </div>"#).to_string();
            }
            _ => {}
        }

        Ok(result)
    }

    /// Extract attribute value from attribute string
    fn extract_attr(attrs: &str, name: &str) -> Option<String> {
        let pattern = format!(r#"{}=\{{([^}}]+)\}}"#, name);
        if let Ok(re) = regex::Regex::new(&pattern) {
            if let Some(caps) = re.captures(attrs) {
                return Some(caps[1].to_string());
            }
        }
        let pattern2 = format!(r#"{}="([^"]*)""#, name);
        if let Ok(re) = regex::Regex::new(&pattern2) {
            if let Some(caps) = re.captures(attrs) {
                return Some(caps[1].to_string());
            }
        }
        None
    }

    /// Process dx-icon elements into SVG
    fn process_icons(html: &str) -> String {
        let re = regex::Regex::new(r#"<dx-icon\s+([^>]*)/?>"#).unwrap();
        re.replace_all(html, |caps: &regex::Captures| {
            let attrs = &caps[1];
            let name = Self::extract_attr(attrs, "name").unwrap_or("zap".to_string());
            let class = Self::extract_attr(attrs, "class").unwrap_or("w-6 h-6".to_string());
            
            let path = match name.as_str() {
                "zap" => "M13 10V3L4 14h7v7l9-11h-7z",
                "book-open" => "M12 6.042A8.967 8.967 0 0 0 6 3.75c-1.052 0-2.062.18-3 .512v14.25A8.987 8.987 0 0 1 6 18c2.305 0 4.408.867 6 2.292m0-14.25a8.966 8.966 0 0 1 6-2.292c1.052 0 2.062.18 3 .512v14.25A8.987 8.987 0 0 0 18 18a8.967 8.967 0 0 0-6 2.292m0-14.25v14.25",
                "github" => "M12 2C6.477 2 2 6.477 2 12c0 4.42 2.87 8.17 6.84 9.5.5.08.66-.23.66-.5v-1.69c-2.77.6-3.36-1.34-3.36-1.34-.46-1.16-1.11-1.47-1.11-1.47-.91-.62.07-.6.07-.6 1 .07 1.53 1.03 1.53 1.03.87 1.52 2.34 1.07 2.91.83.09-.65.35-1.09.63-1.34-2.22-.25-4.55-1.11-4.55-4.92 0-1.11.38-2 1.03-2.71-.1-.25-.45-1.29.1-2.64 0 0 .84-.27 2.75 1.02.79-.22 1.65-.33 2.5-.33.85 0 1.71.11 2.5.33 1.91-1.29 2.75-1.02 2.75-1.02.55 1.35.2 2.39.1 2.64.65.71 1.03 1.6 1.03 2.71 0 3.82-2.34 4.66-4.57 4.91.36.31.69.92.69 1.85V21c0 .27.16.59.67.5C19.14 20.16 22 16.42 22 12A10 10 0 0 0 12 2Z",
                "star" => "M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z",
                "users" => "M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8zm14 10v-2a4 4 0 0 0-3-3.87m-4-12a4 4 0 0 1 0 7.75",
                "chevron-down" => "M6 9l6 6 6-6",
                "binary" => "M10 4H6v16h4V4zm8 6h-4v10h4V10z",
                "shield" => "M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z",
                "globe" => "M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10z M2 12h20 M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z",
                "database" => "M12 2C6.48 2 2 4.02 2 6.5V17.5C2 19.98 6.48 22 12 22S22 19.98 22 17.5V6.5C22 4.02 17.52 2 12 2zM12 4C16.42 4 20 5.35 20 6.5S16.42 9 12 9 4 7.65 4 6.5 7.58 4 12 4zM4 17.5V14.87C5.68 15.94 8.63 16.5 12 16.5S18.32 15.94 20 14.87V17.5C20 18.65 16.42 20 12 20S4 18.65 4 17.5z",
                "palette" => "M12 2C6.49 2 2 6.49 2 12s4.49 10 10 10c1.38 0 2.5-1.12 2.5-2.5 0-.61-.23-1.2-.64-1.67-.08-.1-.13-.21-.13-.33 0-.28.22-.5.5-.5H16c3.31 0 6-2.69 6-6 0-4.96-4.49-9-10-9zm-5.5 9c-.83 0-1.5-.67-1.5-1.5S5.67 8 6.5 8 8 8.67 8 9.5 7.33 11 6.5 11zm3-4C8.67 7 8 6.33 8 5.5S8.67 4 9.5 4s1.5.67 1.5 1.5S10.33 7 9.5 7zm5 0c-.83 0-1.5-.67-1.5-1.5S13.67 4 14.5 4s1.5.67 1.5 1.5S15.33 7 14.5 7zm3 4c-.83 0-1.5-.67-1.5-1.5S16.67 8 17.5 8s1.5.67 1.5 1.5-.67 1.5-1.5 1.5z",
                "rocket" => "M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 0 0-2.91-.09zM12 15l-3-3a22 22 0 0 1 2-3.95A12.88 12.88 0 0 1 22 2c0 2.72-.78 7.5-6 11a22.35 22.35 0 0 1-4 2z",
                "message-circle" => "M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z",
                "send" => "M22 2L11 13M22 2l-7 20-4-9-9-4 20-7z",
                _ => "M13 10V3L4 14h7v7l9-11h-7z",
            };
            
            format!(r#"<svg class="{}" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="{}"/></svg>"#, class, path)
        }).to_string()
    }

    /// Default layout HTML
    fn default_layout() -> String {
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>DX WWW</title>
</head>
<body class="bg-slate-950 text-slate-100">
    {children}
</body>
</html>"#.to_string()
    }

    /// Wrap content with full HTML document
    fn wrap_with_html_shell(content: &str) -> String {
        if content.contains("<!DOCTYPE html>") || content.contains("<html") {
            return content.to_string();
        }

        format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>DX WWW - Binary-First Web Framework</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <link rel="stylesheet" href="/styles/global.css">
    <style>
        body {{ font-family: 'Inter', system-ui, sans-serif; }}
        @keyframes fadeIn {{ from {{ opacity: 0; }} to {{ opacity: 1; }} }}
        .animate-fade-in {{ animation: fadeIn 0.5s ease-in; }}
    </style>
</head>
<body class="bg-slate-950 text-slate-100 antialiased">
    <nav class="fixed top-0 left-0 right-0 z-50 bg-slate-950/80 backdrop-blur-xl border-b border-slate-800">
        <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
            <div class="flex items-center justify-between h-16">
                <a href="/" class="flex items-center gap-2">
                    <svg class="w-8 h-8 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
                    </svg>
                    <span class="text-xl font-bold text-white">DX WWW</span>
                </a>
                <div class="hidden md:flex items-center gap-8">
                    <a href="/" class="text-slate-300 hover:text-white transition-colors">Home</a>
                    <a href="/docs" class="text-slate-300 hover:text-white transition-colors">Docs</a>
                    <a href="/pricing" class="text-slate-300 hover:text-white transition-colors">Pricing</a>
                    <a href="/blog" class="text-slate-300 hover:text-white transition-colors">Blog</a>
                    <a href="/contact" class="text-slate-300 hover:text-white transition-colors">Contact</a>
                </div>
            </div>
        </div>
    </nav>
    <main class="pt-16">
        {}
    </main>
    <footer class="bg-slate-900 border-t border-slate-800 py-12">
        <div class="max-w-7xl mx-auto px-4 text-center">
            <p class="text-slate-400">&copy; 2026 DX WWW. Built with binary-first architecture.</p>
        </div>
    </footer>
</body>
</html>"#, content)
    }

    /// Render error page
    fn render_error_page(code: &str, message: &str) -> String {
        format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - DX WWW</title>
    <script src="https://cdn.tailwindcss.com"></script>
</head>
<body class="bg-slate-950 text-slate-100 min-h-screen flex items-center justify-center">
    <div class="text-center">
        <h1 class="text-6xl font-bold text-emerald-400 mb-4">{}</h1>
        <p class="text-xl text-slate-400 mb-8">{}</p>
        <a href="/" class="text-emerald-400 hover:text-emerald-300 underline">Go Home</a>
    </div>
</body>
</html>"#, code, code, message)
    }

    /// Load translations from locale files
    fn load_translations(&self) -> DxResult<HashMap<String, String>> {
        let mut translations = HashMap::new();
        
        let locale_path = self.cwd.join("locales/en-US.sr");
        if locale_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&locale_path) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
                        continue;
                    }
                    if let Some((key, value)) = line.split_once('=') {
                        translations.insert(
                            key.trim().to_string(),
                            value.trim().trim_matches('"').to_string()
                        );
                    }
                }
            }
        }
        
        // Provide defaults
        if translations.is_empty() {
            translations.insert("home.title".to_string(), "Binary-First Web Framework".to_string());
            translations.insert("home.description".to_string(), "Build blazing-fast web applications.".to_string());
        }
        
        Ok(translations)
    }

    /// Build for production.
    pub fn cmd_build(&self) -> DxResult<()> {
        eprintln!("Building for production...");

        // Load config
        let config_path = self.cwd.join("dx.config.toml");
        let config = if config_path.exists() {
            DxConfig::load(&config_path)?
        } else {
            DxConfig::default()
        };

        let output_dir = self.cwd.join(&config.build.output_dir);
        std::fs::create_dir_all(&output_dir).map_err(|e| DxError::IoError {
            path: Some(output_dir.clone()),
            message: e.to_string(),
        })?;

        // Create subdirectories
        std::fs::create_dir_all(output_dir.join("pages")).ok();
        std::fs::create_dir_all(output_dir.join("components")).ok();
        std::fs::create_dir_all(output_dir.join("styles")).ok();
        std::fs::create_dir_all(output_dir.join("public")).ok();

        let translations = self.load_translations()?;
        let mut compiled_count = 0;
        let mut total_size = 0usize;

        // Compile pages to .dxob binary format
        let pages_dir = self.cwd.join("pages");
        if pages_dir.exists() {
            for entry in walkdir::WalkDir::new(&pages_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map(|ext| ext == "pg").unwrap_or(false))
            {
                let page_path = entry.path();
                let rel_path = page_path.strip_prefix(&pages_dir).unwrap();
                let output_path = output_dir.join("pages").join(rel_path).with_extension("dxob");

                if let Some(parent) = output_path.parent() {
                    std::fs::create_dir_all(parent).ok();
                }

                match self.compile_to_binary(page_path, &translations) {
                    Ok(binary) => {
                        total_size += binary.len();
                        std::fs::write(&output_path, &binary).map_err(|e| DxError::IoError {
                            path: Some(output_path.clone()),
                            message: e.to_string(),
                        })?;
                        compiled_count += 1;
                        eprintln!("  ✓ Compiled {} ({} bytes)", rel_path.display(), binary.len());
                    }
                    Err(e) => {
                        eprintln!("  ✗ Failed to compile {}: {}", rel_path.display(), e);
                    }
                }
            }
        }

        // Copy styles
        let styles_dir = self.cwd.join("styles");
        if styles_dir.exists() {
            for entry in walkdir::WalkDir::new(&styles_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let src = entry.path();
                let rel = src.strip_prefix(&styles_dir).unwrap();
                let dst = output_dir.join("styles").join(rel);
                if let Some(parent) = dst.parent() {
                    std::fs::create_dir_all(parent).ok();
                }
                std::fs::copy(src, &dst).ok();
            }
        }

        // Copy public assets
        let public_dir = self.cwd.join("public");
        if public_dir.exists() {
            for entry in walkdir::WalkDir::new(&public_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let src = entry.path();
                let rel = src.strip_prefix(&public_dir).unwrap();
                let dst = output_dir.join("public").join(rel);
                if let Some(parent) = dst.parent() {
                    std::fs::create_dir_all(parent).ok();
                }
                std::fs::copy(src, &dst).ok();
            }
        }

        // Generate manifest
        let manifest = serde_json::json!({
            "version": "1.0.0",
            "generated": chrono::Utc::now().to_rfc3339(),
            "files_compiled": compiled_count,
            "total_size": total_size,
        });
        std::fs::write(
            output_dir.join("manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap()
        ).ok();

        eprintln!();
        eprintln!("✓ Build complete");
        eprintln!("  Files compiled: {}", compiled_count);
        eprintln!("  Total size: {} bytes", total_size);
        eprintln!("  Output: {}", config.build.output_dir.display());

        Ok(())
    }

    /// Compile a .pg or .cp file to DXOB binary format
    fn compile_to_binary(&self, path: &Path, _translations: &HashMap<String, String>) -> Result<Vec<u8>, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // DXOB Binary Format:
        // Magic: "DXOB" (4 bytes)
        // Version: u16 (2 bytes)
        // Flags: u16 (2 bytes)  
        // Template offset: u32 (4 bytes)
        // Template length: u32 (4 bytes)
        // Script offset: u32 (4 bytes)
        // Script length: u32 (4 bytes)
        // Style offset: u32 (4 bytes)
        // Style length: u32 (4 bytes)
        // String table offset: u32 (4 bytes)
        // String table length: u32 (4 bytes)
        // [Data sections...]

        let mut binary = Vec::new();

        // Magic
        binary.extend_from_slice(b"DXOB");
        // Version
        binary.extend_from_slice(&1u16.to_le_bytes());
        // Flags (0 = page, 1 = component, 2 = layout)
        let flags: u16 = if path.extension().map(|e| e == "pg").unwrap_or(false) { 0 } else { 1 };
        binary.extend_from_slice(&flags.to_le_bytes());

        // Extract sections
        let template = Self::extract_section(&content, "<page>", "</page>")
            .or_else(|| Self::extract_section(&content, "<component>", "</component>"))
            .unwrap_or_default();
        let script = Self::extract_section(&content, "<script", "</script>").unwrap_or_default();
        let style = Self::extract_section(&content, "<style>", "</style>").unwrap_or_default();

        // Header size = 4 + 2 + 2 + 4*8 = 40 bytes
        let header_size = 40u32;

        let template_bytes = template.as_bytes();
        let script_bytes = script.as_bytes();
        let style_bytes = style.as_bytes();

        // Calculate offsets
        let template_offset = header_size;
        let script_offset = template_offset + template_bytes.len() as u32;
        let style_offset = script_offset + script_bytes.len() as u32;
        let string_table_offset = style_offset + style_bytes.len() as u32;

        // Build string table (simple: just collect all string literals)
        let string_table = Self::build_string_table(&template);
        let string_table_bytes = string_table.join("\0").into_bytes();

        // Write offsets and lengths
        binary.extend_from_slice(&template_offset.to_le_bytes());
        binary.extend_from_slice(&(template_bytes.len() as u32).to_le_bytes());
        binary.extend_from_slice(&script_offset.to_le_bytes());
        binary.extend_from_slice(&(script_bytes.len() as u32).to_le_bytes());
        binary.extend_from_slice(&style_offset.to_le_bytes());
        binary.extend_from_slice(&(style_bytes.len() as u32).to_le_bytes());
        binary.extend_from_slice(&string_table_offset.to_le_bytes());
        binary.extend_from_slice(&(string_table_bytes.len() as u32).to_le_bytes());

        // Write data sections
        binary.extend_from_slice(template_bytes);
        binary.extend_from_slice(script_bytes);
        binary.extend_from_slice(style_bytes);
        binary.extend_from_slice(&string_table_bytes);

        // Add content hash at the end
        let hash = blake3::hash(&binary);
        binary.extend_from_slice(hash.as_bytes());

        Ok(binary)
    }

    /// Extract section content from source
    fn extract_section(content: &str, start_tag: &str, end_tag: &str) -> Option<String> {
        let start = content.find(start_tag)?;
        let end = content.rfind(end_tag)?;
        
        // Find the end of the start tag
        let content_start = if start_tag.ends_with('>') {
            start + start_tag.len()
        } else {
            content[start..].find('>')? + start + 1
        };

        if content_start < end {
            Some(content[content_start..end].to_string())
        } else {
            None
        }
    }

    /// Build string table from template
    fn build_string_table(template: &str) -> Vec<String> {
        let mut strings = Vec::new();
        
        // Extract class names
        let class_re = regex::Regex::new(r#"class="([^"]*)""#).unwrap();
        for caps in class_re.captures_iter(template) {
            for class in caps[1].split_whitespace() {
                if !strings.contains(&class.to_string()) {
                    strings.push(class.to_string());
                }
            }
        }

        strings
    }

    /// Generate a new file.
    pub fn cmd_generate(&self, gen_type: &str, name: &str) -> DxResult<()> {
        match gen_type {
            "page" | "p" => self.generate_page(name),
            "component" | "c" => self.generate_component(name),
            "api" | "a" => self.generate_api(name),
            "layout" | "l" => self.generate_layout(name),
            _ => Err(DxError::ConfigValidationError {
                message: format!("Unknown generator type: {}", gen_type),
                field: Some("type".to_string()),
            }),
        }
    }

    /// Generate a new page.
    fn generate_page(&self, name: &str) -> DxResult<()> {
        let path = self.cwd.join("pages").join(format!("{}.pg", name));

        if path.exists() {
            return Err(DxError::IoError {
                path: Some(path.clone()),
                message: "File already exists".to_string(),
            });
        }

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DxError::IoError {
                path: Some(parent.to_path_buf()),
                message: e.to_string(),
            })?;
        }

        let content = format!(
            r#"<script lang="rust">
pub struct Props {{
    // Define props here
}}

pub async fn load() -> Props {{
    Props {{}}
}}
</script>

<page>
    <div class="p-8">
        <h1 class="text-3xl font-bold">{}</h1>
    </div>
</page>
"#,
            name
        );

        std::fs::write(&path, content).map_err(|e| DxError::IoError {
            path: Some(path.clone()),
            message: e.to_string(),
        })?;

        eprintln!("✓ Created pages/{}.pg", name);
        Ok(())
    }

    /// Generate a new component.
    fn generate_component(&self, name: &str) -> DxResult<()> {
        let pascal_name = to_pascal_case(name);
        let path = self.cwd.join("components").join(format!("{}.cp", pascal_name));

        if path.exists() {
            return Err(DxError::IoError {
                path: Some(path.clone()),
                message: "File already exists".to_string(),
            });
        }

        let content = format!(
            r#"<script lang="rust">
pub struct Props {{
    // Define props here
}}
</script>

<component>
    <div class="p-4">
        <!-- Component content -->
    </div>
</component>
"#
        );

        std::fs::write(&path, content).map_err(|e| DxError::IoError {
            path: Some(path.clone()),
            message: e.to_string(),
        })?;

        eprintln!("✓ Created components/{}.cp", pascal_name);
        Ok(())
    }

    /// Generate a new API route.
    fn generate_api(&self, name: &str) -> DxResult<()> {
        let path = self.cwd.join("api").join(format!("{}.rs", name));

        if path.exists() {
            return Err(DxError::IoError {
                path: Some(path.clone()),
                message: "File already exists".to_string(),
            });
        }

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DxError::IoError {
                path: Some(parent.to_path_buf()),
                message: e.to_string(),
            })?;
        }

        let content = r#"use dx_www::prelude::*;

/// Handle GET requests.
pub async fn get(req: Request) -> Response {
    Response::json(&serde_json::json!({
        "message": "Hello from API"
    }))
}

/// Handle POST requests.
pub async fn post(req: Request) -> Response {
    let body: serde_json::Value = req.json().await?;
    Response::json(&body)
}
"#;

        std::fs::write(&path, content).map_err(|e| DxError::IoError {
            path: Some(path.clone()),
            message: e.to_string(),
        })?;

        eprintln!("✓ Created api/{}.rs", name);
        Ok(())
    }

    /// Generate a new layout.
    fn generate_layout(&self, name: &str) -> DxResult<()> {
        let dir = self.cwd.join("pages").join(name);
        let path = dir.join("_layout.pg");

        if path.exists() {
            return Err(DxError::IoError {
                path: Some(path.clone()),
                message: "File already exists".to_string(),
            });
        }

        std::fs::create_dir_all(&dir).map_err(|e| DxError::IoError {
            path: Some(dir.clone()),
            message: e.to_string(),
        })?;

        let content = r#"<script lang="rust">
pub struct Props {
    children: Children,
}
</script>

<page>
    <div class="min-h-screen">
        <slot />
    </div>
</page>
"#;

        std::fs::write(&path, content).map_err(|e| DxError::IoError {
            path: Some(path.clone()),
            message: e.to_string(),
        })?;

        eprintln!("✓ Created pages/{}/_layout.pg", name);
        Ok(())
    }

    // =========================================================================
    // ADD COMMAND - shadcn-style component installation
    // =========================================================================

    /// Add components to the project.
    pub fn cmd_add(&self, components: &[&str]) -> DxResult<()> {
        // Handle flags
        if components.contains(&"--list") || components.contains(&"-l") {
            return self.list_components();
        }

        if components.contains(&"--all") || components.contains(&"-a") {
            return self.add_all_components();
        }

        // Add specific components
        for component_name in components {
            if component_name.starts_with('-') {
                continue; // Skip flags
            }
            self.add_component(component_name)?;
        }

        Ok(())
    }

    /// List all available components.
    fn list_components(&self) -> DxResult<()> {
        let components = get_all_components();
        
        eprintln!("Available components ({}):", components.len());
        eprintln!();
        
        // Group by category
        let categories = [
            ("Primitives", "primitive"),
            ("Layout", "layout"),
            ("Navigation", "navigation"),
            ("Feedback", "feedback"),
            ("Overlay", "overlay"),
            ("Data Display", "data-display"),
            ("Form", "form"),
        ];
        
        for (category_name, category_slug) in categories {
            let category_components: Vec<_> = components
                .iter()
                .filter(|c| c.category.as_str() == category_slug)
                .collect();
            
            if !category_components.is_empty() {
                eprintln!("  {}:", category_name);
                for comp in category_components {
                    eprintln!("    {} - {}", comp.name, comp.description);
                }
                eprintln!();
            }
        }
        
        eprintln!("Usage: dx add <component-name>");
        eprintln!("       dx add button card modal");
        eprintln!("       dx add --all");
        
        Ok(())
    }

    /// Add all components to the project.
    fn add_all_components(&self) -> DxResult<()> {
        let components = get_all_components();
        eprintln!("Adding all {} components...", components.len());
        eprintln!();
        
        for component in &components {
            self.add_component_def(component)?;
        }
        
        eprintln!();
        eprintln!("✓ Added {} components to components/", components.len());
        Ok(())
    }

    /// Add a single component by name.
    fn add_component(&self, name: &str) -> DxResult<()> {
        match get_component(name) {
            Some(component) => self.add_component_def(&component),
            None => {
                eprintln!("Component '{}' not found.", name);
                eprintln!();
                eprintln!("Run `dx add --list` to see available components.");
                Err(DxError::ConfigValidationError {
                    message: format!("Unknown component: {}", name),
                    field: Some("component".to_string()),
                })
            }
        }
    }

    /// Add a component definition to the project.
    fn add_component_def(&self, component: &ComponentDef) -> DxResult<()> {
        let components_dir = self.cwd.join("components");
        std::fs::create_dir_all(&components_dir).map_err(|e| DxError::IoError {
            path: Some(components_dir.clone()),
            message: e.to_string(),
        })?;

        let path = components_dir.join(format!("{}.cp", component.name));
        
        // Check if already exists
        if path.exists() {
            eprintln!("  ⊘ {} already exists, skipping", component.name);
            return Ok(());
        }

        // Add dependencies first
        for dep_name in &component.dependencies {
            if let Some(dep) = get_component(dep_name) {
                let dep_path = components_dir.join(format!("{}.cp", dep.name));
                if !dep_path.exists() {
                    self.add_component_def(&dep)?;
                }
            }
        }

        // Write the component file
        std::fs::write(&path, &component.source).map_err(|e| DxError::IoError {
            path: Some(path.clone()),
            message: e.to_string(),
        })?;

        eprintln!("  ✓ Added {}", component.name);
        Ok(())
    }
}

impl Default for Cli {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Helpers
// =============================================================================

/// Convert string to PascalCase.
fn to_pascal_case(s: &str) -> String {
    s.split(|c: char| c == '-' || c == '_' || c.is_whitespace())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_new() {
        let cli = Cli::new();
        assert!(cli.cwd.exists() || cli.cwd == PathBuf::from("."));
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("my-component"), "MyComponent");
        assert_eq!(to_pascal_case("my_component"), "MyComponent");
        assert_eq!(to_pascal_case("mycomponent"), "Mycomponent");
        assert_eq!(to_pascal_case("my component"), "MyComponent");
    }
}
