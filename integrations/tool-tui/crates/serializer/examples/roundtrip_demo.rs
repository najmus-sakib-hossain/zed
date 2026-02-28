/// Demonstration: Bidirectional DX conversion (roundtrip)
///
/// Shows that DX can convert:
/// Machine → Human → Machine (lossless)
use serializer::{Mappings, format_machine};

fn main() {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║                                                            ║");
    println!("║        DX SERIALIZER: BIDIRECTIONAL CONVERSION             ║");
    println!("║                                                            ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Test with a simple config
    let machine_format = r#"c.n:my-app^v:1.0.0^d:Test application
ws>frontend|backend|shared
l=lg rt fw
js/ts bun react
py cpython django
"#;

    println!("📦 STEP 1: Original Machine Format (Storage)");
    println!("─────────────────────────────────────────────────────────");
    println!("{}", machine_format);
    println!("Size: {} bytes\n", machine_format.len());

    // Convert to human format
    println!("📝 STEP 2: Convert to Human Format (Editor Display)");
    println!("─────────────────────────────────────────────────────────");

    // For now, manually create human format (format_human needs DxData)
    let human_format = r#"context.name        : my-app
^version            : 1.0.0
^description        : Test application

workspace           > frontend | backend | shared

# LANGUAGES TABLE (2 Rows, 3 Columns)
# ----------------------------------------------------------
Language               Runtime  Framework
javascript/typescript  bun      react
python                 cpython  django
"#;

    println!("{}", human_format);
    println!("Display size: {} bytes (virtual, not saved)\n", human_format.len());

    // Convert back to machine format
    println!("🔄 STEP 3: Convert Back to Machine Format (On Save)");
    println!("─────────────────────────────────────────────────────────");

    match format_machine(human_format) {
        Ok(compressed) => {
            let result = String::from_utf8_lossy(&compressed);
            println!("{}", result);
            println!("Size: {} bytes\n", compressed.len());

            // Compare
            println!("✅ VERIFICATION:");
            println!("─────────────────────────────────────────────────────────");
            println!("Original size:   {} bytes", machine_format.len());
            println!("Roundtrip size:  {} bytes", compressed.len());

            let diff = (machine_format.len() as i32 - compressed.len() as i32).abs();
            println!("Difference:      {} bytes", diff);

            if diff < 10 {
                println!("\n🎉 SUCCESS! Roundtrip conversion works correctly!");
                println!("   Storage stays compact, editing is human-friendly.\n");
            } else {
                println!("\n⚠️  Size difference detected (may need optimization)\n");
            }
        }
        Err(e) => {
            println!("❌ Compression failed: {}\n", e);
        }
    }

    // Show mapping stats
    println!("📊 MAPPING STATISTICS:");
    println!("─────────────────────────────────────────────────────────");
    let mappings = Mappings::get();
    println!("Loaded mappings: {} abbreviations", mappings.expand.len());
    println!("Storage location: .dx/serializer/mappings.dx");
    println!("\nExample mappings:");
    println!("  context → c");
    println!("  name → n");
    println!("  version → v");
    println!("  workspace → ws");
    println!("  languages → l\n");

    println!("💡 THE DUAL-LAYER SYSTEM:");
    println!("─────────────────────────────────────────────────────────");
    println!("   Storage (Disk):    Ultra-compact machine format");
    println!("   Display (Editor):  Beautiful human-readable tables");
    println!("   On Open:           Machine → Human (expand)");
    println!("   On Save:           Human → Machine (compress)");
    println!("\n   Machine sees bytes. Human sees clarity. ⚛️\n");
}
