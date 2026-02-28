/// Fact-checking the "natural language is more token-efficient" theory
/// Run with: cargo run --example token_theory_test -p dx-serializer
///
/// HYPOTHESIS: Using common English words instead of structured syntax
/// will result in fewer tokens because LLMs are trained on natural language.
use serializer::llm::tokens::{ModelType, TokenCounter};

fn main() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║     FACT-CHECKING: Natural Language vs Structured Formats    ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let counter = TokenCounter::new();

    // ═══════════════════════════════════════════════════════════════
    // TEST 1: Boolean values
    // ═══════════════════════════════════════════════════════════════
    println!("═══════════════════════════════════════════════════════════════");
    println!("TEST 1: Boolean Representations");
    println!("═══════════════════════════════════════════════════════════════\n");

    let booleans = [
        ("true", "JSON/TOON style"),
        ("false", "JSON/TOON style"),
        ("yes", "Natural English"),
        ("no", "Natural English"),
        ("+", "Dx Serializer compact"),
        ("-", "Dx Serializer compact"),
        ("1", "Numeric"),
        ("0", "Numeric"),
        ("active", "Semantic word"),
        ("inactive", "Semantic word"),
    ];

    for (val, desc) in &booleans {
        let tokens = counter.count(val, ModelType::Gpt4o);
        println!("  {:12} ({:20}) = {} tokens", val, desc, tokens.count);
    }

    // ═══════════════════════════════════════════════════════════════
    // TEST 2: Number formats
    // ═══════════════════════════════════════════════════════════════
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("TEST 2: Number Representations");
    println!("═══════════════════════════════════════════════════════════════\n");

    let numbers = [
        ("95000", "Raw number"),
        ("95,000", "With comma"),
        ("95k", "Abbreviated"),
        ("ninety five thousand", "Words"),
        ("$95,000", "Currency"),
        ("95000.00", "With decimals"),
    ];

    for (val, desc) in &numbers {
        let tokens = counter.count(val, ModelType::Gpt4o);
        println!("  {:25} ({:20}) = {} tokens", val, desc, tokens.count);
    }

    // ═══════════════════════════════════════════════════════════════
    // TEST 3: Date formats
    // ═══════════════════════════════════════════════════════════════
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("TEST 3: Date Representations");
    println!("═══════════════════════════════════════════════════════════════\n");

    let dates = [
        ("2025-01-15", "ISO format"),
        ("Jan 15 2025", "Natural format"),
        ("January 15, 2025", "Full natural"),
        ("15/01/2025", "EU format"),
        ("01/15/2025", "US format"),
        ("250115", "Compact YYMMDD"),
        ("jan15", "Ultra compact"),
    ];

    for (val, desc) in &dates {
        let tokens = counter.count(val, ModelType::Gpt4o);
        println!("  {:20} ({:20}) = {} tokens", val, desc, tokens.count);
    }

    // ═══════════════════════════════════════════════════════════════
    // TEST 4: Field names / Keys
    // ═══════════════════════════════════════════════════════════════
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("TEST 4: Field Name Representations");
    println!("═══════════════════════════════════════════════════════════════\n");

    let keys = [
        ("firstName", "camelCase"),
        ("first_name", "snake_case"),
        ("first-name", "kebab-case"),
        ("firstname", "lowercase"),
        ("FirstName", "PascalCase"),
        ("name", "Simple word"),
        ("n", "Single char"),
        ("bounceRate", "camelCase compound"),
        ("bounce_rate", "snake_case compound"),
        ("bounce rate", "Space separated"),
        ("bounce", "Single word"),
    ];

    for (val, desc) in &keys {
        let tokens = counter.count(val, ModelType::Gpt4o);
        println!("  {:20} ({:20}) = {} tokens", val, desc, tokens.count);
    }

    // ═══════════════════════════════════════════════════════════════
    // TEST 5: Separators and Syntax
    // ═══════════════════════════════════════════════════════════════
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("TEST 5: Separators and Syntax Characters");
    println!("═══════════════════════════════════════════════════════════════\n");

    let syntax = [
        (",", "Comma"),
        (":", "Colon"),
        (";", "Semicolon"),
        (" ", "Space"),
        ("\n", "Newline"),
        ("\t", "Tab"),
        ("=", "Equals"),
        ("|", "Pipe"),
        ("[", "Open bracket"),
        ("]", "Close bracket"),
        ("{", "Open brace"),
        ("}", "Close brace"),
        ("(", "Open paren"),
        (")", "Close paren"),
    ];

    for (val, desc) in &syntax {
        let tokens = counter.count(val, ModelType::Gpt4o);
        let display = val.replace('\n', "\\n").replace('\t', "\\t");
        println!("  {:5} ({:20}) = {} tokens", display, desc, tokens.count);
    }

    // ═══════════════════════════════════════════════════════════════
    // TEST 6: Full Row Comparison (THE REAL TEST)
    // ═══════════════════════════════════════════════════════════════
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("TEST 6: Full Data Row Comparison (Employee Record)");
    println!("═══════════════════════════════════════════════════════════════\n");

    let rows = [
        // JSON
        (
            r#"{"id":1,"name":"James Smith","email":"james.smith@example.com","department":"Engineering","salary":95000,"years":12,"active":true}"#,
            "JSON",
        ),
        // JSON compact
        (
            r#"{"id":1,"name":"James Smith","email":"james.smith@example.com","department":"Engineering","salary":95000,"years":12,"active":true}"#,
            "JSON compact",
        ),
        // TOON row (with schema assumed)
        ("1,James Smith,james.smith@example.com,Engineering,95000,12,true", "TOON row"),
        // Current Dx Serializer
        (
            "1,James Smith,james.smith@example.com,Engineering,95000,12,true",
            "Dx Serializer row",
        ),
        // Space-separated
        ("1 James Smith james.smith@example.com Engineering 95000 12 true", "Space-sep"),
        // Natural language style
        (
            "James Smith (Engineering, 95k, 12yr) james.smith@example.com active",
            "Natural style",
        ),
        // Prose style
        (
            "James Smith works in Engineering earning 95k with 12 years experience, active",
            "Prose",
        ),
        // Ultra compact
        ("1|James Smith|james.smith@example.com|Engineering|95000|12|+", "Pipe-sep"),
        // With common abbreviations
        ("1,James Smith,james.smith@ex.com,Eng,95k,12y,yes", "Abbreviated"),
    ];

    println!("  {:80} {:>8}", "Format", "Tokens");
    println!("  {}", "-".repeat(90));

    let mut results: Vec<(&str, usize)> = Vec::new();
    for (row, desc) in &rows {
        let tokens = counter.count(row, ModelType::Gpt4o);
        results.push((desc, tokens.count));
        println!("  {:80} {:>8}", desc, tokens.count);
    }

    // ═══════════════════════════════════════════════════════════════
    // TEST 7: Full Table Comparison (60 rows analytics)
    // ═══════════════════════════════════════════════════════════════
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("TEST 7: Full Table Comparison (5 rows of analytics data)");
    println!("═══════════════════════════════════════════════════════════════\n");

    // TOON format
    let toon = r#"metrics[5]{date,views,clicks,conversions,revenue,bounceRate}:
  2025-01-01,6621,265,29,8669.84,0.57
  2025-01-02,6693,201,28,6350.68,0.63
  2025-01-03,6958,278,36,3529.44,0.53
  2025-01-04,3435,103,13,3481.66,0.64
  2025-01-05,2945,147,18,2546.82,0.41"#;

    // Current Dx Serializer format
    let dsr = r#"metrics:5(date,views,clicks,conversions,revenue,bounceRate)[
2025-01-01,6621,265,29,8669.84,0.57
2025-01-02,6693,201,28,6350.68,0.63
2025-01-03,6958,278,36,3529.44,0.53
2025-01-04,3435,103,13,3481.66,0.64
2025-01-05,2945,147,18,2546.82,0.41]"#;

    // Natural language header
    let natural_header = r#"5 metrics with date views clicks conversions revenue bounce
2025-01-01,6621,265,29,8669.84,0.57
2025-01-02,6693,201,28,6350.68,0.63
2025-01-03,6958,278,36,3529.44,0.53
2025-01-04,3435,103,13,3481.66,0.64
2025-01-05,2945,147,18,2546.82,0.41"#;

    // Space-separated with natural header
    let space_sep = r#"5 metrics with date views clicks conversions revenue bounce
2025-01-01 6621 265 29 8669.84 0.57
2025-01-02 6693 201 28 6350.68 0.63
2025-01-03 6958 278 36 3529.44 0.53
2025-01-04 3435 103 13 3481.66 0.64
2025-01-05 2945 147 18 2546.82 0.41"#;

    // Abbreviated dates
    let abbrev_dates = r#"5 metrics with date views clicks conversions revenue bounce
Jan1 6621 265 29 8669.84 0.57
Jan2 6693 201 28 6350.68 0.63
Jan3 6958 278 36 3529.44 0.53
Jan4 3435 103 13 3481.66 0.64
Jan5 2945 147 18 2546.82 0.41"#;

    // Full prose (extreme test)
    let prose = r#"Five daily metrics starting January 2025:
Day 1: 6621 views, 265 clicks, 29 conversions, $8669.84 revenue, 57% bounce
Day 2: 6693 views, 201 clicks, 28 conversions, $6350.68 revenue, 63% bounce
Day 3: 6958 views, 278 clicks, 36 conversions, $3529.44 revenue, 53% bounce
Day 4: 3435 views, 103 clicks, 13 conversions, $3481.66 revenue, 64% bounce
Day 5: 2945 views, 147 clicks, 18 conversions, $2546.82 revenue, 41% bounce"#;

    let tables = [
        (toon, "TOON"),
        (dsr, "Current Dx Serializer"),
        (natural_header, "Natural header + comma"),
        (space_sep, "Natural header + space"),
        (abbrev_dates, "Abbreviated dates"),
        (prose, "Full prose"),
    ];

    println!("  {:30} {:>10} {:>10}", "Format", "Tokens", "vs TOON");
    println!("  {}", "-".repeat(55));

    let toon_tokens = counter.count(toon, ModelType::Gpt4o).count;
    for (table, desc) in &tables {
        let tokens = counter.count(table, ModelType::Gpt4o);
        let vs_toon = if tokens.count < toon_tokens {
            format!("-{:.1}%", (1.0 - tokens.count as f64 / toon_tokens as f64) * 100.0)
        } else if tokens.count > toon_tokens {
            format!("+{:.1}%", (tokens.count as f64 / toon_tokens as f64 - 1.0) * 100.0)
        } else {
            "0%".to_string()
        };
        println!("  {:30} {:>10} {:>10}", desc, tokens.count, vs_toon);
    }

    // ═══════════════════════════════════════════════════════════════
    // CONCLUSION
    // ═══════════════════════════════════════════════════════════════
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("CONCLUSION");
    println!("═══════════════════════════════════════════════════════════════\n");

    println!("Note: Token counts use approximation (~4 chars/token for GPT-4o)");
    println!("For accurate results, enable tiktoken feature and run with real tokenizer.");
    println!("\nKey findings will depend on actual tokenizer behavior.");
}
