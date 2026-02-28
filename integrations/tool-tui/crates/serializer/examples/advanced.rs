//! Example: Advanced features

use serializer::*;

fn main() -> serializer::error::Result<()> {
    println!("=== Advanced DX Features ===\n");

    // Feature 1: Vacuum parsing (spaces in strings without quotes)
    println!("1️⃣  VACUUM PARSING (No Quotes Needed):");
    let vacuum = b"users=id%i full_name%s location%s
1 Alice Johnson San Francisco
2 Bob Smith New York City";

    let parsed = parse(vacuum)?;
    let human = format_human(&parsed)?;
    println!("{}\n", human);

    // Feature 2: Prefix inheritance
    println!("2️⃣  PREFIX INHERITANCE (^):");
    let prefix = b"app.name:DX Runtime
^version:0.1.0
^author:Team DX
^status:active";

    let parsed = parse(prefix)?;
    println!("Parsed with prefixes: {:?}\n", parsed);

    // Feature 3: Alias system
    println!("3️⃣  ALIAS SYSTEM ($):");
    let alias = b"$cfg=configuration
$cfg.database.host:localhost
$cfg.database.port:5432
$cfg.cache.enabled:+
$cfg.cache.ttl:3600";

    let parsed = parse(alias)?;
    println!("Parsed with aliases: {:?}\n", parsed);

    // Feature 4: Implicit flags
    println!("4️⃣  IMPLICIT FLAGS (! and ?):");
    let flags = b"admin!
debug!
production!
error?
warning?";

    let parsed = parse(flags)?;
    println!("Flags: {:?}\n", parsed);

    // Feature 5: Ditto compression
    println!("5️⃣  DITTO COMPRESSION (_):");
    let ditto = b"events=timestamp%i event%s user%s status%s
1000 login alice success
1001 login bob success
1002 logout alice success
_ _ bob _";

    let parsed = parse(ditto)?;
    let human = format_human(&parsed)?;
    println!("{}\n", human);

    // Feature 6: Mixed types in tables
    println!("6️⃣  MIXED TYPE TABLES:");
    let mixed = b"metrics=name%s value%i change%f active%b notes%s
CPU 85 +2.5 + Running smoothly
Memory 65 -1.2 + All good
Disk 40 +0.0 + Plenty of space";

    let parsed = parse(mixed)?;
    let human = format_human(&parsed)?;
    println!("{}\n", human);

    // Feature 7: Encoding with automatic optimization
    println!("7️⃣  AUTO-OPTIMIZED ENCODING:");
    let original = b"status:active
status:active
status:active
priority:high
priority:high
priority:high";

    let parsed = parse(original)?;
    let encoded = encode(&parsed)?;

    println!("Original: {} bytes", original.len());
    println!("Encoded:  {} bytes", encoded.len());
    println!(
        "Savings:  {:.1}%\n",
        (1.0 - encoded.len() as f64 / original.len() as f64) * 100.0
    );

    // Feature 8: Stream arrays for compact lists
    println!("8️⃣  STREAM ARRAYS (>):");
    let streams = b"languages>Rust|TypeScript|Python|Go
frameworks>React|Vue|Svelte|Angular
databases>PostgreSQL|Redis|MongoDB";

    let parsed = parse(streams)?;
    let human = format_human(&parsed)?;
    println!("{}\n", human);

    Ok(())
}
