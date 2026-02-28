// Test program to verify prompt symbol rendering
// Run with: cargo run --manifest-path crates/cli/Cargo.toml --example test_symbols

use std::io;

fn main() -> io::Result<()> {
    println!("=== Terminal Environment Detection ===");
    println!("MSYSTEM: {:?}", std::env::var("MSYSTEM"));
    println!("WT_SESSION: {:?}", std::env::var("WT_SESSION"));
    println!("ConEmuPID: {:?}", std::env::var("ConEmuPID"));
    println!("Windows: {}", cfg!(windows));

    println!("\n=== Symbol Set ===");
    println!("Using Unicode symbols on all platforms");

    println!("\n=== Symbol Preview ===");
    println!("Step Active: ◆");
    println!("Step Submit: ◇");
    println!("Bar Start: ┌");
    println!("Bar: │");
    println!("Bar End: └");
    println!("Radio Active: ●");
    println!("Radio Inactive: ○");
    println!("Checkbox Active: ◻");
    println!("Checkbox Selected: ◼");
    println!("Password Mask: •");

    println!("\n=== Example Prompt (OpenClaw Style) ===");
    println!("┌  OpenClaw onboarding");
    println!("│◇  Security ──────────────────────────────────────────────────────────────────────────────╮");
    println!("│                                                                                         │");
    println!("│  Security warning — please read.                                                        │");
    println!("│                                                                                         │");
    println!("│  OpenClaw is a hobby project and still in beta. Expect sharp edges.                     │");
    println!("│                                                                                         │");
    println!("├─────────────────────────────────────────────────────────────────────────────────────────╯");
    println!("│◇  Select AI providers to configure:");
    println!("│  ● OpenAI (GPT-4, GPT-3.5) Most popular, great for general tasks");
    println!("│  ○ Anthropic (Claude) Excellent for analysis and writing");
    println!("│  ○ Google (Gemini) Fast and cost-effective");
    println!("│  ○ Ollama (Local models) Run models locally for privacy");
    println!("│◇  Selected 2 provider(s): google, ollama");

    Ok(())
}
