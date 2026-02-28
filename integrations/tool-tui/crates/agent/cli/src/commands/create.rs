//! Create integrations, skills, and plugins

use crate::CreateCommands;
use colored::Colorize;

pub async fn run(what: CreateCommands) -> anyhow::Result<()> {
    match what {
        CreateCommands::Integration {
            name,
            language,
            source: _,
        } => {
            println!(
                "{} Creating integration: {} ({})",
                "🔧".bright_cyan(),
                name.bright_yellow(),
                language
            );

            // Generate template based on language
            let template = match language.as_str() {
                "python" => {
                    format!(
                        r#"# DX Integration: {}
# Language: Python
# This will be compiled to WASM and injected into DX

def init():
    """Initialize the integration"""
    pass

def handle(message: str) -> str:
    """Handle incoming messages"""
    return f"Processed: {{message}}"

def cleanup():
    """Cleanup when integration is unloaded"""
    pass
"#,
                        name
                    )
                }
                "javascript" | "js" => {
                    format!(
                        r#"// DX Integration: {}
// Language: JavaScript
// This will be compiled to WASM and injected into DX

export function init() {{
    // Initialize the integration
}}

export function handle(message) {{
    // Handle incoming messages
    return `Processed: ${{message}}`;
}}

export function cleanup() {{
    // Cleanup when integration is unloaded
}}
"#,
                        name
                    )
                }
                "rust" | "rs" => {
                    format!(
                        r#"//! DX Integration: {}
//! Language: Rust
//! This will be compiled to WASM and injected into DX

#[no_mangle]
pub extern "C" fn init() {{
    // Initialize the integration
}}

#[no_mangle]
pub extern "C" fn handle(message: *const u8, len: usize) -> *mut u8 {{
    // Handle incoming messages
    std::ptr::null_mut()
}}

#[no_mangle]
pub extern "C" fn cleanup() {{
    // Cleanup when integration is unloaded
}}
"#,
                        name
                    )
                }
                "go" => {
                    format!(
                        r#"// DX Integration: {}
// Language: Go
// This will be compiled to WASM via TinyGo

package main

//export init
func init() {{
    // Initialize the integration
}}

//export handle
func handle(message string) string {{
    // Handle incoming messages
    return "Processed: " + message
}}

//export cleanup
func cleanup() {{
    // Cleanup when integration is unloaded
}}

func main() {{}}
"#,
                        name
                    )
                }
                _ => {
                    println!("{} Unsupported language: {}", "❌".bright_red(), language);
                    println!("  Supported: python, javascript, rust, go");
                    return Ok(());
                }
            };

            // Create the integration file
            let ext = match language.as_str() {
                "python" => "py",
                "javascript" | "js" => "js",
                "rust" | "rs" => "rs",
                "go" => "go",
                _ => "txt",
            };

            let path = format!(".dx/integrations/{}.{}", name, ext);
            std::fs::create_dir_all(".dx/integrations")?;
            std::fs::write(&path, template)?;

            println!(
                "{} Integration created: {}",
                "✅".bright_green(),
                path.bright_blue()
            );
            println!();
            println!("  Next steps:");
            println!("    1. Edit the integration: {}", path.bright_blue());
            println!(
                "    2. Compile to WASM: {} dx create plugin {} --source {}",
                "→".bright_cyan(),
                name,
                path
            );
            println!("    3. The integration will be auto-loaded!");

            // Also create the DX Serializer manifest
            let manifest = format!(
                "name={}\nversion=0.0.1\nlanguage={}\ntype=integration\nenabled=true\n",
                name, language
            );
            let manifest_path = format!(".dx/integrations/{}.sr", name);
            std::fs::write(&manifest_path, manifest)?;

            println!();
            println!(
                "  {} Manifest created: {}",
                "📝".bright_cyan(),
                manifest_path.bright_blue()
            );
        }

        CreateCommands::Skill { name, description } => {
            let desc = description.unwrap_or_else(|| format!("Custom skill: {}", name));

            println!(
                "{} Creating skill: {}",
                "🎯".bright_cyan(),
                name.bright_yellow()
            );

            // Create skill definition in DX Serializer format
            let skill_def = format!(
                r#"# DX Skill: {}
# Description: {}

name = {}
description = {}
output = dx_llm

[inputs.message]
type = string
required = true
description = The input message

[action]
type = llm
prompt = Process this request: {{{{message}}}}
"#,
                name, desc, name, desc
            );

            let path = format!(".dx/skills/{}.sr", name);
            std::fs::create_dir_all(".dx/skills")?;
            std::fs::write(&path, skill_def)?;

            println!(
                "{} Skill created: {}",
                "✅".bright_green(),
                path.bright_blue()
            );
            println!();
            println!("  To use: {} dx run \"{}\"", "→".bright_cyan(), name);
        }

        CreateCommands::Plugin { name } => {
            println!(
                "{} Creating plugin: {}",
                "🔌".bright_cyan(),
                name.bright_yellow()
            );

            // Create plugin directory
            let dir = format!(".dx/plugins/{}", name);
            std::fs::create_dir_all(&dir)?;

            // Create manifest
            let manifest = format!("name={}\nversion=0.0.1\ntype=plugin\n", name);
            std::fs::write(format!("{}/manifest.sr", dir), manifest)?;

            println!(
                "{} Plugin directory created: {}",
                "✅".bright_green(),
                dir.bright_blue()
            );
        }
    }

    Ok(())
}
