//! Demo command to showcase serializer and markdown format conversion

use anyhow::Result;
use console::style;

/// Demo the 3-format system (human, llm, machine)
pub async fn demo_formats() -> Result<()> {
    println!("\n  ╔══════════════════════════════════════════════════════════════╗");
    println!(
        "  ║   {} DX Format System Demo                                ║",
        style("[*]").cyan().bold()
    );
    println!("  ╚══════════════════════════════════════════════════════════════╝\n");

    // Serializer demo
    println!("  {} Serializer (DX ∞) - 73% smaller than JSON", style("[1]").cyan().bold());
    println!("    {} Human format:   Beautiful .sr files on real disk", style("→").dim());
    println!(
        "    {} LLM format:     Token-optimized in .dx/serializer/*.llm",
        style("→").dim()
    );
    println!(
        "    {} Machine format: Binary rkyv in .dx/serializer/*.machine",
        style("→").dim()
    );
    println!("    {} Usage: dx serializer <file.sr>", style("→").dim());
    println!();

    // Markdown demo
    println!("  {} Markdown Compiler - 10-80% token savings", style("[2]").cyan().bold());
    println!("    {} Human format:   Beautified .md files on real disk", style("→").dim());
    println!("    {} LLM format:     Compact in .dx/markdown/*.llm", style("→").dim());
    println!("    {} Machine format: Binary in .dx/markdown/*.machine", style("→").dim());
    println!("    {} Usage: dx markdown <file.md>", style("→").dim());
    println!();

    // Example workflow
    println!("  {} Example Workflow:", style("[>]").green().bold());
    println!("    {} Create config.sr with your data", style("1.").dim());
    println!("    {} Run: dx serializer config.sr", style("2.").dim());
    println!("    {} Outputs:", style("3.").dim());
    println!("       {} config.sr (human-readable, stays on disk)", style("•").dim());
    println!("       {} .dx/serializer/config.llm (LLM-optimized)", style("•").dim());
    println!("       {} .dx/serializer/config.machine (binary)", style("•").dim());
    println!();

    println!("  {} File Locations:", style("[i]").yellow().bold());
    println!("    {} Human files: Stay on real disk (you edit these)", style("→").dim());
    println!("    {} LLM/Machine: Generated in .dx/ folder (gitignored)", style("→").dim());
    println!();

    println!("  {} Try it now:", style("[!]").yellow().bold());
    println!("    {} dx serializer .", style("→").cyan());
    println!("    {} dx markdown .", style("→").cyan());
    println!();

    Ok(())
}
