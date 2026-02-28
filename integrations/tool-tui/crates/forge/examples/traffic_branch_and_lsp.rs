/// Example: Traffic Branch System & LSP Detection
///
/// This example demonstrates:
/// 1. Traffic Branch System (Red/Yellow/Green) for component updates
/// 2. LSP detection and fallback to file watching
use anyhow::Result;
use colored::*;
use dx_forge::context::{ComponentStateManager, TrafficBranch, apply_update};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    println!("{}", "═".repeat(80).bright_cyan());
    println!("{}", "Forge: Traffic Branch System & LSP Detection Demo".cyan().bold());
    println!("{}", "═".repeat(80).bright_cyan());

    // Create a temporary test directory
    let temp_dir = tempfile::tempdir()?;
    let forge_dir = temp_dir.path().join(".dx/forge");
    tokio::fs::create_dir_all(&forge_dir).await?;

    println!("\n{}", "Part 1: Traffic Branch System".yellow().bold());
    println!("{}", "─".repeat(80).bright_black());

    // Initialize component state manager
    let mut state_mgr = ComponentStateManager::new(&forge_dir)?;

    // Create a test component file
    let component_path = temp_dir.path().join("Button.tsx");
    let initial_content = r#"
// Button Component v1.0.0
export function Button({ children, onClick }) {
    return (
        <button onClick={onClick}>
            {children}
        </button>
    );
}
"#;

    tokio::fs::write(&component_path, initial_content).await?;
    println!("\n{} Created test component: {}", "✓".green(), "Button.tsx".cyan());

    // Register component
    state_mgr.register_component(&component_path, "dx-ui", "Button", "1.0.0", initial_content)?;

    println!("{} Registered component with state manager", "✓".green());

    // Scenario 1: 🟢 GREEN BRANCH - No local modifications
    println!("\n{}", "Scenario 1: 🟢 Green Branch (Auto-Update)".bright_green().bold());
    println!("{}", "─".repeat(40).bright_black());

    let remote_v2 = r#"
// Button Component v2.0.0
export function Button({ children, onClick, variant = "primary" }) {
    return (
        <button 
            onClick={onClick}
            className={`btn-${variant}`}
        >
            {children}
        </button>
    );
}
"#;

    let branch = state_mgr.analyze_update(&component_path, remote_v2)?;
    println!("Analysis: {:?}", branch);

    match branch {
        TrafficBranch::Green => {
            println!(
                "{} {} No local changes detected",
                "🟢".bright_green(),
                "GREEN:".green().bold()
            );
            println!("   {} Component will be auto-updated", "→".bright_black());

            // Apply update
            apply_update(&component_path, remote_v2, "2.0.0", &mut state_mgr).await?;
        }
        _ => {}
    }

    // Scenario 2: 🟡 YELLOW BRANCH - Non-conflicting local changes
    println!("\n{}", "Scenario 2: 🟡 Yellow Branch (Merge)".bright_yellow().bold());
    println!("{}", "─".repeat(40).bright_black());

    // Modify locally (add a comment)
    let local_modified = r#"
// Button Component v2.0.0
// Custom modification: Added logging
export function Button({ children, onClick, variant = "primary" }) {
    console.log("Button clicked!");
    return (
        <button 
            onClick={onClick}
            className={`btn-${variant}`}
        >
            {children}
        </button>
    );
}
"#;

    tokio::fs::write(&component_path, local_modified).await?;
    println!("{} Made local modification (added logging)", "✓".cyan());

    // Remote update adds a prop
    let remote_v3 = r#"
// Button Component v3.0.0
export function Button({ children, onClick, variant = "primary", disabled = false }) {
    return (
        <button 
            onClick={onClick}
            className={`btn-${variant}`}
            disabled={disabled}
        >
            {children}
        </button>
    );
}
"#;

    let branch = state_mgr.analyze_update(&component_path, remote_v3)?;
    println!("Analysis: {:?}", branch);

    match branch {
        TrafficBranch::Yellow { .. } => {
            println!(
                "{} {} Non-conflicting changes detected",
                "🟡".bright_yellow(),
                "YELLOW:".yellow().bold()
            );
            println!("   {} Local changes will be preserved", "→".bright_black());
            println!("   {} Remote updates will be merged", "→".bright_black());
        }
        _ => {}
    }

    // Scenario 3: 🔴 RED BRANCH - Conflicting changes
    println!("\n{}", "Scenario 3: 🔴 Red Branch (Conflict)".bright_red().bold());
    println!("{}", "─".repeat(40).bright_black());

    // Both modify the same line
    let remote_conflict = r#"
// Button Component v2.0.0
export function Button({ children, onClick, variant = "primary" }) {
    // REMOTE CHANGE: Different logging
    console.error("Button interaction!");
    return (
        <button 
            onClick={onClick}
            className={`btn-${variant}`}
        >
            {children}
        </button>
    );
}
"#;

    let branch = state_mgr.analyze_update(&component_path, remote_conflict)?;
    println!("Analysis: {:?}", branch);

    match branch {
        TrafficBranch::Red { conflicts } => {
            println!("{} {} Conflicting changes detected", "🔴".bright_red(), "RED:".red().bold());
            println!("   {} Manual resolution required", "→".bright_black());
            println!("   {} Conflicts: {}", "→".bright_black(), conflicts.join(", "));
        }
        _ => {}
    }

    // Part 2: LSP Detection
    println!("\n\n{}", "Part 2: LSP Detection".yellow().bold());
    println!("{}", "─".repeat(80).bright_black());

    #[allow(deprecated)]
    use dx_forge::watcher_legacy::lsp_detector;

    println!("\n{} Checking for DX editor extension...", "→".bright_black());
    let lsp_available = lsp_detector::detect_lsp_support().await?;

    if lsp_available {
        println!("{} {} detected!", "✓".bright_green(), "DX LSP extension".bright_cyan());
        println!("   {} Using LSP-based change detection", "→".bright_black());
        println!("   {} Benefits:", "→".bright_black());
        println!("     • Lower latency (no file system polling)");
        println!("     • More accurate change tracking");
        println!("     • Better editor integration");
        println!("     • Reduced CPU usage");
    } else {
        println!("{} {} not found", "⚠".yellow(), "DX LSP extension".bright_cyan());
        println!("   {} Falling back to file watching", "→".bright_black());
        println!("   {} To enable LSP mode:", "→".bright_black());
        println!("     1. Install DX editor extension");
        println!("     2. Restart your editor");
        println!("     3. Run forge watch again");
    }

    // Summary
    println!("\n{}", "═".repeat(80).bright_cyan());
    println!("{}", "Demo Complete!".cyan().bold());
    println!("{}", "═".repeat(80).bright_cyan());

    println!("\n{}", "Key Takeaways:".yellow().bold());
    println!(
        "  {}",
        "1. Traffic Branch System provides intelligent component updates".bright_black()
    );
    println!("     {} No local changes = Auto-update", "🟢".bright_green());
    println!("     {} Non-conflicting = Smart merge", "🟡".bright_yellow());
    println!("     {} Conflicting = Manual resolution", "🔴".bright_red());
    println!("\n  {}", "2. LSP Detection optimizes change tracking".bright_black());
    println!("     {} Editor integration = Faster, more accurate", "📡".bright_blue());
    println!("     {} No extension = File watching fallback", "👁️".bright_yellow());

    println!("\n{}", "Usage:".yellow().bold());
    println!(
        "  {} - Register a component",
        "forge register <path> --source dx-ui --name Button --version 1.0.0".bright_white()
    );
    println!("  {} - List components", "forge components".bright_white());
    println!("  {} - Update component", "forge update Button".bright_white());
    println!("  {} - Start watching", "forge watch".bright_white());

    Ok(())
}
