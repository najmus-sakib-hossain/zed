//! Table Parsing Example
//!
//! Demonstrates schema-guided tabular data parsing.

use serializer::{DxValue, format_human, parse};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== DX Serializer: Table Examples ===\n");

    // Example 1: Basic table with type hints
    println!("1. User Table");
    let input = b"users=id%i name%s age%i email%s active%b
1 Alice 30 alice@example.com +
2 Bob 25 bob@example.com +
3 Charlie 35 charlie@example.com -
";

    let data = parse(input)?;

    if let DxValue::Object(obj) = &data {
        if let Some(DxValue::Table(table)) = obj.get("users") {
            println!("   Found {} users:", table.rows.len());
            for row in &table.rows {
                println!(
                    "   - {} ({}): {} - Active: {}",
                    row[1].as_str().unwrap(),
                    row[0].as_int().unwrap(),
                    row[2].as_int().unwrap(),
                    if row[4].as_bool().unwrap() {
                        "✓"
                    } else {
                        "✗"
                    }
                );
            }
        }
    }

    // Example 2: Product catalog
    println!("\n2. Product Catalog");
    let input = b"products=id%i name%s price%f stock%i sale%b
101 Laptop 999.99 15 -
102 Mouse 29.99 50 +
103 Keyboard 79.99 30 +
104 Monitor 299.99 8 -
";

    let data = parse(input)?;
    let human = format_human(&data)?;
    println!("{}", human);

    // Example 3: Shortened headers (ultra-compact)
    println!("\n3. Compact Hikes Table");
    let input = b"h=i n%s k%f g%i w%s s%b
1 Blue Lake Trail 7.5 320 ana +
2 Ridge Overlook 9.2 540 luis -
3 Wildflower Loop 5.1 180 sam +
";

    let data = parse(input)?;

    if let DxValue::Object(obj) = &data {
        if let Some(DxValue::Table(table)) = obj.get("h") {
            println!("   Hike ID | Name              | Distance | Elevation | Guide | Sunny");
            println!("   --------|-------------------|----------|-----------|-------|------");
            for row in &table.rows {
                println!(
                    "   {:7} | {:17} | {:6.1} km | {:7} m | {:5} | {}",
                    row[0].as_int().unwrap(),
                    row[1].as_str().unwrap(),
                    row[2].as_float().unwrap(),
                    row[3].as_int().unwrap(),
                    row[4].as_str().unwrap(),
                    if row[5].as_bool().unwrap() {
                        "✓"
                    } else {
                        "✗"
                    }
                );
            }
        }
    }

    Ok(())
}
