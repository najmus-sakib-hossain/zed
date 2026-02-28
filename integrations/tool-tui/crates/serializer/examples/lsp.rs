//! LSP Integration Example
//!
//! Shows how to integrate the human formatter for IDE/LSP usage.

use serializer::formatter::FormatterConfig;
use serializer::{format_human, parse};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== DX Serializer: LSP Integration ===\n");

    // Simulate receiving DX format from a file
    let dx_content = b"app.name:DX Runtime^version:0.1.0^env:production
db.host:localhost^port:5432^name:dxdb^pool:20
features>auth|cache|logging|metrics|analytics
users=id%i name%s role%s active%b
1 Alice admin +
2 Bob user +
3 Charlie user -
limits.requests:1000^timeout:30^max_size:10485760
";

    println!("1. Parse DX Format (Machine-Optimized)");
    println!("   Size: {} bytes\n", dx_content.len());
    println!("{}", String::from_utf8_lossy(dx_content));

    // Parse the data
    let data = parse(dx_content)?;

    // Format for human display (what LSP would show)
    println!("\n2. Format for LSP Display (Human-Readable)");
    let human = format_human(&data)?;
    println!("{}", human);

    // Custom formatting configuration
    println!("\n3. Custom Formatting (ASCII-only, no Unicode)");
    let _config = FormatterConfig {
        column_padding: 2,
        use_unicode: false, // For terminals without Unicode support
        add_dividers: false,
        use_colors: false,
    };

    // Note: Would need to implement custom formatter with config
    // This shows the API design
    println!("   (Custom formatter would go here)");

    // Show the use case
    println!("\n4. LSP Use Case:");
    println!("   - User opens file.dx in VS Code");
    println!("   - Extension reads DX format ({}B)", dx_content.len());
    println!("   - Calls format_human() for display");
    println!("   - User sees formatted view");
    println!("   - On save, writes back DX format");
    println!("   - Round-trip preserves data perfectly");

    Ok(())
}
