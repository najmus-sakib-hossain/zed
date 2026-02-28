/// Fact-checking space separator solutions for text with spaces
/// Run with: cargo run --example space_separator_test -p dx-serializer --features tiktoken
use serializer::llm::tokens::{ModelType, TokenCounter};

fn main() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║   SPACE SEPARATOR: Solving Text-With-Spaces Problem          ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let counter = TokenCounter::new();

    // ═══════════════════════════════════════════════════════════════
    // THE PROBLEM: Text with spaces
    // ═══════════════════════════════════════════════════════════════
    println!("═══════════════════════════════════════════════════════════════");
    println!("THE PROBLEM: How to handle 'James Smith' with space separator?");
    println!("═══════════════════════════════════════════════════════════════\n");

    // Row: id=1, name="James Smith", email, dept, salary, years, active

    let solutions = [
        // Current: comma separator (baseline)
        (
            "1,James Smith,james@ex.com,Engineering,95000,12,true",
            "Comma separator (current)",
        ),
        // Problem: naive space separator breaks on "James Smith"
        (
            "1 James Smith james@ex.com Engineering 95000 12 true",
            "Space sep (BROKEN - 8 fields!)",
        ),
        // Solution 1: Double quotes (traditional)
        ("1 \"James Smith\" james@ex.com Engineering 95000 12 true", "Quotes \"...\""),
        // Solution 2: Single quotes
        ("1 'James Smith' james@ex.com Engineering 95000 12 true", "Single quotes '...'"),
        // Solution 3: Underscore replacement
        ("1 James_Smith james@ex.com Engineering 95000 12 true", "Underscore James_Smith"),
        // Solution 4: Dot replacement
        ("1 James.Smith james@ex.com Engineering 95000 12 true", "Dot James.Smith"),
        // Solution 5: CamelCase
        ("1 JamesSmith james@ex.com Engineering 95000 12 true", "CamelCase JamesSmith"),
        // Solution 6: Backtick (like markdown code)
        ("1 `James Smith` james@ex.com Engineering 95000 12 true", "Backtick `...`"),
        // Solution 7: Parentheses
        ("1 (James Smith) james@ex.com Engineering 95000 12 true", "Parens (...)"),
        // Solution 8: Square brackets
        ("1 [James Smith] james@ex.com Engineering 95000 12 true", "Brackets [...]"),
        // Solution 9: Angle brackets
        ("1 <James Smith> james@ex.com Engineering 95000 12 true", "Angle <...>"),
        // Solution 10: Pipe wrapper
        ("1 |James Smith| james@ex.com Engineering 95000 12 true", "Pipe |...|"),
        // Solution 11: Tilde wrapper
        ("1 ~James Smith~ james@ex.com Engineering 95000 12 true", "Tilde ~...~"),
        // Solution 12: Hash wrapper
        ("1 #James Smith# james@ex.com Engineering 95000 12 true", "Hash #...#"),
        // Solution 13: At wrapper
        ("1 @James Smith@ james@ex.com Engineering 95000 12 true", "At @...@"),
        // Solution 14: Caret wrapper
        ("1 ^James Smith^ james@ex.com Engineering 95000 12 true", "Caret ^...^"),
        // Solution 15: Non-breaking space (Unicode \u00A0)
        ("1 James\u{00A0}Smith james@ex.com Engineering 95000 12 true", "NBSP (\\u00A0)"),
        // Solution 16: Tab as field separator
        ("1\tJames Smith\tjames@ex.com\tEngineering\t95000\t12\ttrue", "Tab separator"),
        // Solution 17: Double space as separator, single space in text
        ("1  James Smith  james@ex.com  Engineering  95000  12  true", "Double space sep"),
        // Solution 18: Colon wrapper (like your :: leaf inline idea!)
        ("1 :James Smith: james@ex.com Engineering 95000 12 true", "Colon :...:"),
        // Solution 19: Plus prefix for multi-word
        ("1 +James Smith james@ex.com Engineering 95000 12 true", "Plus prefix +..."),
        // Solution 20: Count prefix (like your array count!)
        ("1 2:James Smith james@ex.com Engineering 95000 12 true", "Count 2:James Smith"),
    ];

    println!("  {:50} {:>8}", "Solution", "Tokens");
    println!("  {}", "-".repeat(60));

    let baseline = counter.count(solutions[0].0, ModelType::Gpt4o).count;

    for (text, desc) in &solutions {
        let tokens = counter.count(text, ModelType::Gpt4o);
        let diff = if tokens.count < baseline {
            format!("-{}", baseline - tokens.count)
        } else if tokens.count > baseline {
            format!("+{}", tokens.count - baseline)
        } else {
            "=".to_string()
        };
        println!("  {:50} {:>5} ({:>3})", desc, tokens.count, diff);
    }

    // ═══════════════════════════════════════════════════════════════
    // TEST 2: Multiple text fields with spaces
    // ═══════════════════════════════════════════════════════════════
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("TEST 2: Row with MULTIPLE text fields containing spaces");
    println!("═══════════════════════════════════════════════════════════════\n");

    // Row: name="James Smith", title="Senior Engineer", dept="Research and Development"

    let multi_text = [
        // Comma baseline
        ("James Smith,Senior Engineer,Research and Development", "Comma (baseline)"),
        // Quotes
        ("\"James Smith\" \"Senior Engineer\" \"Research and Development\"", "Quotes"),
        // Underscore
        ("James_Smith Senior_Engineer Research_and_Development", "Underscore"),
        // Backtick
        ("`James Smith` `Senior Engineer` `Research and Development`", "Backtick"),
        // Colon wrapper
        (":James Smith: :Senior Engineer: :Research and Development:", "Colon :...:"),
        // Tab
        ("James Smith\tSenior Engineer\tResearch and Development", "Tab"),
        // Double space
        ("James Smith  Senior Engineer  Research and Development", "Double space"),
    ];

    println!("  {:60} {:>8}", "Solution", "Tokens");
    println!("  {}", "-".repeat(70));

    let baseline2 = counter.count(multi_text[0].0, ModelType::Gpt4o).count;

    for (text, desc) in &multi_text {
        let tokens = counter.count(text, ModelType::Gpt4o);
        let diff = if tokens.count < baseline2 {
            format!("-{}", baseline2 - tokens.count)
        } else if tokens.count > baseline2 {
            format!("+{}", tokens.count - baseline2)
        } else {
            "=".to_string()
        };
        println!("  {:60} {:>5} ({:>3})", desc, tokens.count, diff);
    }

    // ═══════════════════════════════════════════════════════════════
    // TEST 3: Full table with mixed data
    // ═══════════════════════════════════════════════════════════════
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("TEST 3: Full 5-row table with names (text with spaces)");
    println!("═══════════════════════════════════════════════════════════════\n");

    // 5 employees: id, name, email, department, salary

    let comma_table = r#"employees:5(id,name,email,department,salary)[
1,James Smith,james@ex.com,Engineering,95000
2,Mary Johnson,mary@ex.com,Sales,87000
3,John Williams,john@ex.com,Marketing,102000
4,Patricia Brown,patricia@ex.com,HR,76000
5,Robert Davis,robert@ex.com,Finance,91000]"#;

    let underscore_table = r#"employees:5(id name email department salary)[
1 James_Smith james@ex.com Engineering 95000
2 Mary_Johnson mary@ex.com Sales 87000
3 John_Williams john@ex.com Marketing 102000
4 Patricia_Brown patricia@ex.com HR 76000
5 Robert_Davis robert@ex.com Finance 91000]"#;

    let quote_table = r#"employees:5(id name email department salary)[
1 "James Smith" james@ex.com Engineering 95000
2 "Mary Johnson" mary@ex.com Sales 87000
3 "John Williams" john@ex.com Marketing 102000
4 "Patricia Brown" patricia@ex.com HR 76000
5 "Robert Davis" robert@ex.com Finance 91000]"#;

    let backtick_table = r#"employees:5(id name email department salary)[
1 `James Smith` james@ex.com Engineering 95000
2 `Mary Johnson` mary@ex.com Sales 87000
3 `John Williams` john@ex.com Marketing 102000
4 `Patricia Brown` patricia@ex.com HR 76000
5 `Robert Davis` robert@ex.com Finance 91000]"#;

    let tab_table = "employees:5(id\tname\temail\tdepartment\tsalary)[\n1\tJames Smith\tjames@ex.com\tEngineering\t95000\n2\tMary Johnson\tmary@ex.com\tSales\t87000\n3\tJohn Williams\tjohn@ex.com\tMarketing\t102000\n4\tPatricia Brown\tpatricia@ex.com\tHR\t76000\n5\tRobert Davis\trobert@ex.com\tFinance\t91000]";

    let colon_table = r#"employees:5(id name email department salary)[
1 :James Smith: james@ex.com Engineering 95000
2 :Mary Johnson: mary@ex.com Sales 87000
3 :John Williams: john@ex.com Marketing 102000
4 :Patricia Brown: patricia@ex.com HR 76000
5 :Robert Davis: robert@ex.com Finance 91000]"#;

    let tables = [
        (comma_table, "Comma separator (current)"),
        (underscore_table, "Space + underscore names"),
        (quote_table, "Space + quoted names"),
        (backtick_table, "Space + backtick names"),
        (tab_table, "Tab separator"),
        (colon_table, "Space + colon wrapper"),
    ];

    println!("  {:40} {:>8} {:>10}", "Solution", "Tokens", "vs Comma");
    println!("  {}", "-".repeat(60));

    let comma_tokens = counter.count(comma_table, ModelType::Gpt4o).count;

    for (table, desc) in &tables {
        let tokens = counter.count(table, ModelType::Gpt4o);
        let pct = ((tokens.count as f64 / comma_tokens as f64) - 1.0) * 100.0;
        let diff = if pct < 0.0 {
            format!("{:.1}%", pct)
        } else if pct > 0.0 {
            format!("+{:.1}%", pct)
        } else {
            "0%".to_string()
        };
        println!("  {:40} {:>8} {:>10}", desc, tokens.count, diff);
    }

    // ═══════════════════════════════════════════════════════════════
    // TEST 4: The REAL question - does space separator actually help?
    // ═══════════════════════════════════════════════════════════════
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("TEST 4: PURE NUMERIC DATA (no text with spaces)");
    println!("═══════════════════════════════════════════════════════════════\n");

    // Analytics data - no text with spaces!
    let comma_analytics = r#"metrics:5(date,views,clicks,conversions,revenue)[
2025-01-01,6621,265,29,8669.84
2025-01-02,6693,201,28,6350.68
2025-01-03,6958,278,36,3529.44
2025-01-04,3435,103,13,3481.66
2025-01-05,2945,147,18,2546.82]"#;

    let space_analytics = r#"metrics:5(date views clicks conversions revenue)[
2025-01-01 6621 265 29 8669.84
2025-01-02 6693 201 28 6350.68
2025-01-03 6958 278 36 3529.44
2025-01-04 3435 103 13 3481.66
2025-01-05 2945 147 18 2546.82]"#;

    let comma_t = counter.count(comma_analytics, ModelType::Gpt4o).count;
    let space_t = counter.count(space_analytics, ModelType::Gpt4o).count;

    println!("  Comma separator: {} tokens", comma_t);
    println!("  Space separator: {} tokens", space_t);
    println!(
        "  Difference: {} tokens ({:.1}%)",
        space_t as i32 - comma_t as i32,
        ((space_t as f64 / comma_t as f64) - 1.0) * 100.0
    );

    // ═══════════════════════════════════════════════════════════════
    // CONCLUSION
    // ═══════════════════════════════════════════════════════════════
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("CONCLUSION");
    println!("═══════════════════════════════════════════════════════════════\n");
}
