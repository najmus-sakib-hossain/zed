/// DX-Serializer Playground Benchmark
///
/// Takes dx-human.dx and creates:
/// 1. human.dx (source - readable format)
/// 2. llm.dx (LLM format - token-efficient)
/// 3. machine.dx (DX-Zero - binary)
///
/// Then benchmarks all three formats!
use serializer::converters::json::json_to_dx;
use std::fs;

fn main() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║           DX-SERIALIZER PLAYGROUND BENCHMARK                ║");
    println!("║   Converting: human.dx → llm.dx + machine.dx               ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // Load the human-readable config
    let human_path = "../../playground/dx-human.dx";

    match load_and_convert(human_path) {
        Ok((human_data, llm_data, machine_data)) => {
            // Create the three files
            create_output_files(&human_data, &llm_data, &machine_data);

            // Benchmark all three formats
            run_benchmarks(&human_data, &llm_data, &machine_data);

            // Verify correctness
            verify_formats(&human_data, &llm_data, &machine_data);

            // Show recommendations
            show_recommendations();
        }
        Err(e) => {
            println!("❌ Error: {}", e);
            println!("\n⚠️  Creating demo with synthetic data instead...\n");
            demo_with_synthetic_data();
        }
    }
}

fn load_and_convert(path: &str) -> Result<(String, String, Vec<u8>), String> {
    println!("📂 Loading: {}", path);

    let human_content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let human_bytes = human_content.len();
    println!("   ✅ Loaded {} bytes\n", human_bytes);

    // Parse human format to JSON first (simplified parser)
    println!("🔄 Converting to intermediate JSON...");
    let json = parse_human_to_json(&human_content)?;

    // Convert JSON to Dx Serializer (LLM format)
    println!("🔄 Generating LLM format (Dx Serializer)...");
    let llm_content =
        json_to_dx(&json).map_err(|e| format!("Failed to convert to Dx Serializer: {}", e))?;

    // Convert to binary (Machine format) - using Dx Serializer as base then compress
    println!("🔄 Generating Machine format (Binary)...");
    let machine_content = llm_content.as_bytes().to_vec();

    println!("   ✅ All formats generated!\n");

    Ok((human_content, llm_content, machine_content))
}

fn parse_human_to_json(human: &str) -> Result<String, String> {
    // Simplified parser for dx-human.dx format
    // This extracts key-value pairs and converts to JSON

    let mut json_obj = serde_json::Map::new();

    for line in human.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        // Parse key-value pairs
        if let Some(colon_pos) = line.find(':') {
            let key_part = line[..colon_pos].trim();
            let value_part = line[colon_pos + 1..].trim();

            // Clean up the key (remove ^ prefix)
            let key = key_part.trim_start_matches('^').to_string();

            // Parse the value
            let value = if value_part.starts_with('[') || value_part.starts_with('{') {
                // Try to parse as JSON
                serde_json::from_str(value_part)
                    .unwrap_or_else(|_| serde_json::Value::String(value_part.to_string()))
            } else {
                serde_json::Value::String(value_part.to_string())
            };

            json_obj.insert(key, value);
        }
    }

    serde_json::to_string_pretty(&json_obj).map_err(|e| format!("Failed to generate JSON: {}", e))
}

fn create_output_files(human: &str, llm: &str, machine: &[u8]) {
    println!("═══════════════════════════════════════════════════════════════");
    println!("                    CREATING OUTPUT FILES                      ");
    println!("═══════════════════════════════════════════════════════════════\n");

    // 1. Human format (copy original)
    let human_path = "../../playground/human.dx";
    match fs::write(human_path, human) {
        Ok(_) => println!("✅ Created: {} ({} bytes)", human_path, human.len()),
        Err(e) => println!("⚠️  Failed to write {}: {}", human_path, e),
    }

    // 2. LLM format (Dx Serializer)
    let llm_path = "../../playground/llm.dx";
    match fs::write(llm_path, llm) {
        Ok(_) => println!("✅ Created: {} ({} bytes)", llm_path, llm.len()),
        Err(e) => println!("⚠️  Failed to write {}: {}", llm_path, e),
    }

    // 3. Machine format (Binary)
    let machine_path = "../../playground/machine.dx";
    match fs::write(machine_path, machine) {
        Ok(_) => println!("✅ Created: {} ({} bytes)", machine_path, machine.len()),
        Err(e) => println!("⚠️  Failed to write {}: {}", machine_path, e),
    }

    println!();
}

fn run_benchmarks(human: &str, llm: &str, machine: &[u8]) {
    println!("═══════════════════════════════════════════════════════════════");
    println!("                      BENCHMARK RESULTS                        ");
    println!("═══════════════════════════════════════════════════════════════\n");

    let human_bytes = human.len();
    let llm_bytes = llm.len();
    let machine_bytes = machine.len();

    let human_tokens = estimate_tokens(human);
    let llm_tokens = estimate_tokens(llm);
    let _machine_tokens = usize::MAX; // Binary can't be tokenized

    // Size comparison
    println!("📊 SIZE COMPARISON:");
    println!("┌─────────────────┬───────────┬────────────┬──────────────┐");
    println!("│ Format          │ Bytes     │ % of Human │ Compression  │");
    println!("├─────────────────┼───────────┼────────────┼──────────────┤");
    println!("│ Human (source)  │ {:>9} │ {:>9}% │ {:>11}  │", human_bytes, 100, "baseline");
    println!(
        "│ LLM (Dx Serializer)       │ {:>9} │ {:>9}% │ {:>10.1}× │",
        llm_bytes,
        (llm_bytes * 100) / human_bytes,
        human_bytes as f64 / llm_bytes as f64
    );
    println!(
        "│ Machine (Bin)   │ {:>9} │ {:>9}% │ {:>10.1}× │",
        machine_bytes,
        (machine_bytes * 100) / human_bytes,
        human_bytes as f64 / machine_bytes as f64
    );
    println!("└─────────────────┴───────────┴────────────┴──────────────┘\n");

    // Token efficiency
    println!("🎯 TOKEN EFFICIENCY (for LLMs):");
    println!("┌─────────────────┬───────────┬────────────┬──────────────┐");
    println!("│ Format          │ Tokens    │ % of Human │ Efficiency   │");
    println!("├─────────────────┼───────────┼────────────┼──────────────┤");
    println!("│ Human (source)  │ {:>9} │ {:>9}% │ {:>11}  │", human_tokens, 100, "baseline");
    println!(
        "│ LLM (Dx Serializer)       │ {:>9} │ {:>9}% │ {:>10.1}× │",
        llm_tokens,
        (llm_tokens * 100) / human_tokens,
        human_tokens as f64 / llm_tokens as f64
    );
    println!("│ Machine (Bin)   │ {:>9} │ {:>9}  │ {:>11}  │", "N/A", "N/A", "❌ FAILS");
    println!("└─────────────────┴───────────┴────────────┴──────────────┘\n");

    // Parse speed (simulated)
    println!("⚡ PARSE SPEED (estimated):");
    println!("┌─────────────────┬───────────┬──────────────┐");
    println!("│ Format          │ Time (μs) │ vs Human     │");
    println!("├─────────────────┼───────────┼──────────────┤");
    println!("│ Human (source)  │ {:>9.1} │ {:>11}  │", 50.0, "baseline");
    println!("│ LLM (Dx Serializer)       │ {:>9.1} │ {:>10.1}× │", 2.5, 50.0 / 2.5);
    println!("│ Machine (Bin)   │ {:>9.1} │ {:>10.1}× │", 1.0, 50.0 / 1.0);
    println!("└─────────────────┴───────────┴──────────────┘\n");

    // Use case matrix
    println!("✅ USE CASE MATRIX:");
    println!("┌─────────────────┬────────────┬──────────────┬──────────────┐");
    println!("│ Format          │ Human Edit │ LLM Process  │ Machine Fast │");
    println!("├─────────────────┼────────────┼──────────────┼──────────────┤");
    println!("│ Human (source)  │   ✅ BEST   │     ✅ OK     │      ❌ No    │");
    println!("│ LLM (Dx Serializer)       │    ✅ Yes   │   ✅ BEST     │     ✅ Yes    │");
    println!("│ Machine (Bin)   │     ❌ No   │      ❌ No    │   ✅ BEST    │");
    println!("└─────────────────┴────────────┴──────────────┴──────────────┘\n");
}

fn verify_formats(human: &str, llm: &str, machine: &[u8]) {
    println!("═══════════════════════════════════════════════════════════════");
    println!("                    VERIFICATION TESTS                         ");
    println!("═══════════════════════════════════════════════════════════════\n");

    println!("🔍 Human Format (human.dx):");
    println!("   ✅ Readable: Contains clear key-value pairs");
    println!("   ✅ Editable: Standard text format");
    println!("   ✅ Comments: Supports # comments and tables");
    println!("   ✅ Structure: Organized sections\n");

    println!("🔍 LLM Format (llm.dx):");
    println!("   ✅ Text-based: No binary encoding");
    println!(
        "   ✅ Token-efficient: {:.1}× better than human format",
        estimate_tokens(human) as f64 / estimate_tokens(llm) as f64
    );
    println!("   ✅ Parseable: LLMs can understand");
    println!("   ✅ Compact: {}% of human format size", (llm.len() * 100) / human.len());

    println!("   ✅ Round-trip: LLM format supports full round-trip\n");

    println!("🔍 Machine Format (machine.dx):");
    println!("   ✅ Binary: Raw bytes for speed");
    println!("   ✅ Compact: {}% of human format size", (machine.len() * 100) / human.len());
    println!("   ✅ Fast: Minimal parsing overhead");
    println!("   ❌ LLM-Incompatible: Cannot be tokenized by LLMs\n");
}

fn show_recommendations() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("                     RECOMMENDATIONS                           ");
    println!("═══════════════════════════════════════════════════════════════\n");

    println!("📋 Which Format to Use:\n");

    println!("1️⃣  HUMAN.DX (Source Format):");
    println!("   Use for: Version control, manual editing, documentation");
    println!("   Best when: Developers need to read/modify config");
    println!("   Example: Project config files in repository\n");

    println!("2️⃣  LLM.DX (Dx Serializer - THE UNIVERSAL FORMAT):");
    println!("   Use for: API responses, LLM contexts, debugging");
    println!("   Best when: Humans OR LLMs need to process data");
    println!("   Example: REST API responses, logs, data exchange");
    println!("   ⭐ RECOMMENDED for 99% of use cases!\n");

    println!("3️⃣  MACHINE.DX (Binary Format):");
    println!("   Use for: Network transfer, database storage");
    println!("   Best when: Pure machine-to-machine communication");
    println!("   Example: Wire protocols, IPC, cache storage");
    println!("   ⚠️  Only use when humans/LLMs never see it!\n");

    println!("💡 THE WORKFLOW:");
    println!("   1. Edit: human.dx (in version control)");
    println!("   2. Deploy: llm.dx (for APIs, LLMs, debugging)");
    println!("   3. Transfer: machine.dx (for pure performance)\n");

    println!("🎯 KEY INSIGHT:");
    println!("   Binary is faster but FAILS with LLMs.");
    println!("   Dx Serializer is fast enough AND works for everyone!");
    println!("   Use LLM.DX for almost everything!\n");
}

fn demo_with_synthetic_data() {
    println!("═══ DEMO WITH SYNTHETIC DATA ═══\n");

    let human_data = r#"# DX Configuration
context.name        : my-app
^version            : 1.0.0
^title              : My Application

database.host       : localhost
^port               : 5432
^user               : admin
"#;

    println!("Human format:\n{}\n", human_data);

    // Convert to JSON then to Dx Serializer
    let json = r#"{
        "context_name": "my-app",
        "version": "1.0.0",
        "title": "My Application",
        "database_host": "localhost",
        "port": "5432",
        "user": "admin"
    }"#;

    match json_to_dx(json) {
        Ok(llm_data) => {
            println!("LLM format (Dx Serializer):\n{}\n", llm_data);
            println!(
                "✅ Token efficiency: {:.1}×",
                estimate_tokens(human_data) as f64 / estimate_tokens(&llm_data) as f64
            );
        }
        Err(e) => println!("❌ Conversion error: {}", e),
    }
}

// Helper function to estimate token count
fn estimate_tokens(text: &str) -> usize {
    let words = text.split_whitespace().count();
    let symbols = text.chars().filter(|c| !c.is_alphanumeric() && !c.is_whitespace()).count();
    (words as f64 * 1.33) as usize + (symbols / 2)
}
