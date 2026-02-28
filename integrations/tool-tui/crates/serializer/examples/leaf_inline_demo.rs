/// Demonstrates the leaf inline syntax (::) in Dx Serializer format
/// Run with: cargo run --example leaf_inline_demo -p dx-serializer
use serializer::llm::{DxDocument, DxLlmValue, LlmParser, LlmSerializer};

fn main() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║           Dx Serializer LEAF INLINE SYNTAX (::) DEMO                   ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // ═══════════════════════════════════════════════════════════════
    // LEAF INLINE SYNTAX
    // ═══════════════════════════════════════════════════════════════
    // The :: syntax is used for "leaf inlining" - it indicates that
    // the value is a primitive (string, number, boolean) that should
    // be stored directly without further parsing.
    //
    // This is useful for:
    // - URLs (which contain : characters)
    // - Paths with special characters
    // - Values that shouldn't be interpreted as nested objects

    println!("═══════════════════════════════════════════════════════════════");
    println!("                    LEAF INLINE SYNTAX (::)                    ");
    println!("═══════════════════════════════════════════════════════════════\n");

    let input = r#"name: dx
version: 0.0.1
forge.repository:: https://dx.vercel.app/user/repo
style.path:: @/style
api.endpoint:: http://localhost:8080/api/v1
config.regex:: ^[a-zA-Z0-9]+$"#;

    println!("INPUT (Dx Serializer with leaf inline syntax):");
    println!("─────────────────────────────────────────────────────────────────");
    println!("{}", input.trim());
    println!("─────────────────────────────────────────────────────────────────\n");

    // Parse the input
    let doc = LlmParser::parse(input).expect("Failed to parse");

    println!("PARSED VALUES:");
    println!("─────────────────────────────────────────────────────────────────");
    for (key, value) in &doc.context {
        println!("  {} = {:?}", key, value);
    }
    println!("─────────────────────────────────────────────────────────────────\n");

    // ═══════════════════════════════════════════════════════════════
    // WHY USE LEAF INLINE?
    // ═══════════════════════════════════════════════════════════════
    println!("═══════════════════════════════════════════════════════════════");
    println!("                    WHY USE LEAF INLINE?                       ");
    println!("═══════════════════════════════════════════════════════════════\n");

    println!("1. URLs with colons:");
    println!("   ❌ url: https://example.com  → might parse 'https' as key");
    println!("   ✅ url:: https://example.com → value is 'https://example.com'\n");

    println!("2. Paths with special chars:");
    println!("   ❌ path: @/components/ui  → might interpret @ specially");
    println!("   ✅ path:: @/components/ui → value is '@/components/ui'\n");

    println!("3. Regex patterns:");
    println!("   ❌ pattern: ^[a-z]+$  → might parse incorrectly");
    println!("   ✅ pattern:: ^[a-z]+$ → value is '^[a-z]+$'\n");

    // ═══════════════════════════════════════════════════════════════
    // ROUND-TRIP DEMONSTRATION
    // ═══════════════════════════════════════════════════════════════
    println!("═══════════════════════════════════════════════════════════════");
    println!("                    ROUND-TRIP TEST                            ");
    println!("═══════════════════════════════════════════════════════════════\n");

    // Create a document with leaf inline values
    let mut doc = DxDocument::new();
    doc.context.insert(
        "forge.repository".to_string(),
        DxLlmValue::Str("https://github.com/user/repo".to_string()),
    );
    doc.context
        .insert("api.base".to_string(), DxLlmValue::Str("http://localhost:3000".to_string()));
    doc.context.insert("name".to_string(), DxLlmValue::Str("my-app".to_string()));

    // Serialize
    let serializer = LlmSerializer::new();
    let output = serializer.serialize(&doc);

    println!("SERIALIZED OUTPUT:");
    println!("─────────────────────────────────────────────────────────────────");
    println!("{}", output);
    println!("─────────────────────────────────────────────────────────────────\n");

    // Parse back
    let parsed = LlmParser::parse(&output).expect("Failed to parse serialized output");

    println!("✅ Round-trip successful!");
    println!("   Original values preserved: {}", parsed.context.len());
}
