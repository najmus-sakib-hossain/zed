//! Test REAL token efficiency - no fake abbreviations
//! Run with: cargo run --example test_real -p dx-serializer

use serializer::llm::tokens::{ModelType, TokenCounter};

fn main() {
    let counter = TokenCounter::new();

    println!("=== TESTING ABBREVIATION MYTH ===\n");

    // Test 1: Field names - full vs abbreviated
    let full_fields = "timestamp level endpoint statusCode responseTime error";
    let abbrev_fields = "ts lv ep sc rt er";

    println!(
        "Full field names:  '{}' = {} tokens",
        full_fields,
        counter.count(full_fields, ModelType::Gpt4o).count
    );
    println!(
        "Abbrev fields:     '{}' = {} tokens",
        abbrev_fields,
        counter.count(abbrev_fields, ModelType::Gpt4o).count
    );
    println!();

    // Test 2: Product codes
    let full_products = "WIDGET-A GADGET-B THING-C";
    let abbrev_products = "WA GB TC";

    println!(
        "Full products:     '{}' = {} tokens",
        full_products,
        counter.count(full_products, ModelType::Gpt4o).count
    );
    println!(
        "Abbrev products:   '{}' = {} tokens",
        abbrev_products,
        counter.count(abbrev_products, ModelType::Gpt4o).count
    );
    println!();

    // Test 3: Status codes
    let full_status = "shipped delivered processing";
    let abbrev_status = "s d p";

    println!(
        "Full status:       '{}' = {} tokens",
        full_status,
        counter.count(full_status, ModelType::Gpt4o).count
    );
    println!(
        "Abbrev status:     '{}' = {} tokens",
        abbrev_status,
        counter.count(abbrev_status, ModelType::Gpt4o).count
    );
    println!();

    // Test 4: Log levels
    let full_levels = "info error warn debug";
    let abbrev_levels = "i e w d";

    println!(
        "Full levels:       '{}' = {} tokens",
        full_levels,
        counter.count(full_levels, ModelType::Gpt4o).count
    );
    println!(
        "Abbrev levels:     '{}' = {} tokens",
        abbrev_levels,
        counter.count(abbrev_levels, ModelType::Gpt4o).count
    );
    println!();

    println!("=== REAL GAME CHANGERS ===\n");

    // Test 5: Prefix elimination - this is REAL savings
    let with_prefix = "/api/users /api/orders /api/products /api/payment /api/auth";
    let without_prefix = "users orders products payment auth";

    println!(
        "With /api/:        '{}' = {} tokens",
        with_prefix,
        counter.count(with_prefix, ModelType::Gpt4o).count
    );
    println!(
        "Without /api/:     '{}' = {} tokens",
        without_prefix,
        counter.count(without_prefix, ModelType::Gpt4o).count
    );
    println!();

    // Test 6: Date prefix elimination
    let full_dates =
        "2025-01-15T10:23:45Z 2025-01-15T10:24:12Z 2025-01-15T10:25:03Z 2025-01-15T10:26:47Z";
    let short_dates = "10:23:45Z 10:24:12Z 10:25:03Z 10:26:47Z";

    println!(
        "Full timestamps:   {} tokens",
        counter.count(full_dates, ModelType::Gpt4o).count
    );
    println!(
        "Time only:         {} tokens",
        counter.count(short_dates, ModelType::Gpt4o).count
    );
    println!();

    // Test 7: null vs ~
    let with_null = "null null null null null";
    let with_tilde = "~ ~ ~ ~ ~";

    println!(
        "With null:         '{}' = {} tokens",
        with_null,
        counter.count(with_null, ModelType::Gpt4o).count
    );
    println!(
        "With ~:            '{}' = {} tokens",
        with_tilde,
        counter.count(with_tilde, ModelType::Gpt4o).count
    );
    println!();

    // Test 8: Semicolon vs comma-space
    let comma_space = "a, b, c, d, e, f, g, h";
    let semicolon = "a;b;c;d;e;f;g;h";

    println!(
        "Comma-space:       '{}' = {} tokens",
        comma_space,
        counter.count(comma_space, ModelType::Gpt4o).count
    );
    println!(
        "Semicolon:         '{}' = {} tokens",
        semicolon,
        counter.count(semicolon, ModelType::Gpt4o).count
    );
    println!();

    // Test 9: Newline vs inline
    let with_newlines = "row1\nrow2\nrow3\nrow4";
    let inline = "row1,row2,row3,row4";

    println!(
        "With newlines:     {} tokens",
        counter.count(with_newlines, ModelType::Gpt4o).count
    );
    println!("Inline:            {} tokens", counter.count(inline, ModelType::Gpt4o).count);
    println!();

    // Test 10: Indentation cost
    let with_indent = "  value1\n  value2\n  value3\n  value4";
    let no_indent = "value1\nvalue2\nvalue3\nvalue4";

    println!(
        "With 2-space indent: {} tokens",
        counter.count(with_indent, ModelType::Gpt4o).count
    );
    println!(
        "No indent:           {} tokens",
        counter.count(no_indent, ModelType::Gpt4o).count
    );
    println!();

    println!("=== REAL COMPARISON: LOGS ===\n");

    // TOON format
    let logs_toon = r#"logs[4]{timestamp,level,endpoint,statusCode,responseTime,error}:
  2025-01-15T10:23:45Z,info,/api/users,200,45,null
  2025-01-15T10:24:12Z,error,/api/orders,500,120,Database timeout
  2025-01-15T10:25:03Z,info,/api/products,200,32,null
  2025-01-15T10:26:47Z,warn,/api/payment,429,5,Rate limit exceeded"#;

    // DX with ONLY real optimizations (no fake abbreviations)
    // Real optimizations: inline, semicolon separator, ~ for null, prefix elimination
    let logs_dx_real = r#"logs:4(timestamp level endpoint statusCode responseTime error)@/api/@2025-01-15T[10:23:45Z info users 200 45 ~;10:24:12Z error orders 500 120 Database_timeout;10:25:03Z info products 200 32 ~;10:26:47Z warn payment 429 5 Rate_limit_exceeded]"#;

    let toon_t = counter.count(logs_toon, ModelType::Gpt4o).count;
    let dx_t = counter.count(logs_dx_real, ModelType::Gpt4o).count;
    let savings = ((toon_t as f64 - dx_t as f64) / toon_t as f64) * 100.0;

    println!("TOON:     {} tokens", toon_t);
    println!("DX real:  {} tokens", dx_t);
    println!("Savings:  {:.1}%", savings);
    println!();
    println!("TOON format:");
    println!("{}", logs_toon);
    println!();
    println!("DX format (real optimizations only):");
    println!("{}", logs_dx_real);
}
