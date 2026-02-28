//! Brutal token testing - no assumptions
//! Run with: cargo run --example test_brutal -p dx-serializer --features tiktoken

use serializer::llm::tokens::{ModelType, TokenCounter};

fn main() {
    let counter = TokenCounter::new();

    println!("=== GAME CHANGER DISCOVERY ===\n");

    // ============================================
    // TEST 1: Underscores vs Spaces in multi-word
    // ============================================
    println!("=== UNDERSCORES vs SPACES ===");
    println!(
        "'Blue_Lake_Trail' = {} tokens",
        counter.count("Blue_Lake_Trail", ModelType::Gpt4o).count
    );
    println!(
        "'Blue Lake Trail' = {} tokens",
        counter.count("Blue Lake Trail", ModelType::Gpt4o).count
    );
    println!(
        "'Our_favorite_hikes_together' = {} tokens",
        counter.count("Our_favorite_hikes_together", ModelType::Gpt4o).count
    );
    println!(
        "'Our favorite hikes together' = {} tokens",
        counter.count("Our favorite hikes together", ModelType::Gpt4o).count
    );
    println!();

    // ============================================
    // TEST 2: Quotes for multi-word values
    // ============================================
    println!("=== QUOTES FOR MULTI-WORD ===");
    println!(
        "'task=Our_favorite_hikes' = {} tokens",
        counter.count("task=Our_favorite_hikes", ModelType::Gpt4o).count
    );
    println!(
        "'task=\"Our favorite hikes\"' = {} tokens",
        counter.count("task=\"Our favorite hikes\"", ModelType::Gpt4o).count
    );
    println!(
        "'task='Our favorite hikes'' = {} tokens",
        counter.count("task='Our favorite hikes'", ModelType::Gpt4o).count
    );
    println!();

    // ============================================
    // TEST 3: Different bracket types
    // ============================================
    println!("=== BRACKET TYPES ===");
    println!("'[a b c]' = {} tokens", counter.count("[a b c]", ModelType::Gpt4o).count);
    println!("'(a b c)' = {} tokens", counter.count("(a b c)", ModelType::Gpt4o).count);
    println!("'{{a b c}}' = {} tokens", counter.count("{a b c}", ModelType::Gpt4o).count);
    println!();

    // ============================================
    // TEST 4: Row separators in context
    // ============================================
    println!("=== ROW SEPARATORS (full rows) ===");
    let row1 = "1 Blue Lake Trail 7.5 320 ana true 4.5";
    let row2 = "2 Ridge Overlook 9.2 540 luis false 4.2";
    println!(
        "comma-space: '{}' = {} tokens",
        format!("{}, {}", row1, row2),
        counter.count(&format!("{}, {}", row1, row2), ModelType::Gpt4o).count
    );
    println!(
        "semicolon:   '{}' = {} tokens",
        format!("{};{}", row1, row2),
        counter.count(&format!("{};{}", row1, row2), ModelType::Gpt4o).count
    );
    println!(
        "newline:     = {} tokens",
        counter.count(&format!("{}\n{}", row1, row2), ModelType::Gpt4o).count
    );
    println!(
        "pipe:        '{}' = {} tokens",
        format!("{}|{}", row1, row2),
        counter.count(&format!("{}|{}", row1, row2), ModelType::Gpt4o).count
    );
    println!();

    // ============================================
    // TEST 5: Header declaration styles
    // ============================================
    println!("=== HEADER STYLES ===");
    println!(
        "'hikes:20(id name dist)' = {} tokens",
        counter.count("hikes:20(id name dist)", ModelType::Gpt4o).count
    );
    println!(
        "'hikes[20]{{id,name,dist}}:' = {} tokens",
        counter.count("hikes[20]{id,name,dist}:", ModelType::Gpt4o).count
    );
    println!(
        "'hikes|20|id name dist|' = {} tokens",
        counter.count("hikes|20|id name dist|", ModelType::Gpt4o).count
    );
    println!();

    // ============================================
    // TEST 6: Prefix elimination syntax
    // ============================================
    println!("=== PREFIX SYNTAX ===");
    println!("'@/api/' = {} tokens", counter.count("@/api/", ModelType::Gpt4o).count);
    println!(
        "'@2025-01-15T' = {} tokens",
        counter.count("@2025-01-15T", ModelType::Gpt4o).count
    );
    println!("'@@' (double) = {} tokens", counter.count("@@", ModelType::Gpt4o).count);
    println!();

    // ============================================
    // TEST 7: Newlines vs inline
    // ============================================
    println!("=== NEWLINES vs INLINE ===");
    let with_newlines = "context:\n  task: hikes\n  location: Boulder";
    let inline = "context:[task=hikes location=Boulder]";
    println!("with newlines: {} tokens", counter.count(with_newlines, ModelType::Gpt4o).count);
    println!("inline:        {} tokens", counter.count(inline, ModelType::Gpt4o).count);
    println!();

    // ============================================
    // TEST 8: Indentation cost
    // ============================================
    println!("=== INDENTATION COST ===");
    println!("'  ' (2 spaces) = {} tokens", counter.count("  ", ModelType::Gpt4o).count);
    println!("'    ' (4 spaces) = {} tokens", counter.count("    ", ModelType::Gpt4o).count);
    println!("'\\n  row' = {} tokens", counter.count("\n  row", ModelType::Gpt4o).count);
    println!("',row' = {} tokens", counter.count(",row", ModelType::Gpt4o).count);
    println!();

    // ============================================
    // TEST 9: Number formats
    // ============================================
    println!("=== NUMBER FORMATS ===");
    println!("'2025-01-15' = {} tokens", counter.count("2025-01-15", ModelType::Gpt4o).count);
    println!("'20250115' = {} tokens", counter.count("20250115", ModelType::Gpt4o).count);
    println!("'2025/01/15' = {} tokens", counter.count("2025/01/15", ModelType::Gpt4o).count);
    println!(
        "'Jan 15 2025' = {} tokens",
        counter.count("Jan 15 2025", ModelType::Gpt4o).count
    );
    println!();

    // ============================================
    // TEST 10: Common words vs symbols
    // ============================================
    println!("=== WORDS vs SYMBOLS ===");
    println!("'true' = {} tokens", counter.count("true", ModelType::Gpt4o).count);
    println!("'false' = {} tokens", counter.count("false", ModelType::Gpt4o).count);
    println!("'null' = {} tokens", counter.count("null", ModelType::Gpt4o).count);
    println!("'none' = {} tokens", counter.count("none", ModelType::Gpt4o).count);
    println!("'~' = {} tokens", counter.count("~", ModelType::Gpt4o).count);
    println!();

    // ============================================
    // TEST 11: Full comparison - current DX vs optimized
    // ============================================
    println!("=== FULL MIXED COMPARISON ===\n");

    let toon = r#"context:
  task: Our favorite hikes together
  location: Boulder
  season: spring_2025
friends[3]: ana,luis,sam
hikes[3]{id,name,distanceKm,companion,wasSunny}:
  1,Blue Lake Trail,7.5,ana,true
  2,Ridge Overlook,9.2,luis,false
  3,Wildflower Loop,5.1,sam,true"#;

    // Current DX with underscores
    let dx_underscore = r#"context:4[task=Our_favorite_hikes_together location=Boulder season=spring_2025]
friends:3=ana luis sam
hikes:3(id name distanceKm companion wasSunny)[1 Blue_Lake_Trail 7.5 ana true, 2 Ridge_Overlook 9.2 luis false, 3 Wildflower_Loop 5.1 sam true]"#;

    // DX with quotes for multi-word (spaces preserved)
    let dx_quotes = r#"context:4[task="Our favorite hikes together" location=Boulder season=spring_2025]
friends:3=ana luis sam
hikes:3(id name distanceKm companion wasSunny)[1 "Blue Lake Trail" 7.5 ana true, 2 "Ridge Overlook" 9.2 luis false, 3 "Wildflower Loop" 5.1 sam true]"#;

    println!("TOON:           {} tokens", counter.count(toon, ModelType::Gpt4o).count);
    println!(
        "DX underscore:  {} tokens",
        counter.count(dx_underscore, ModelType::Gpt4o).count
    );
    println!("DX quotes:      {} tokens", counter.count(dx_quotes, ModelType::Gpt4o).count);
    println!();

    // ============================================
    // TEST 12: Logs with prefix elimination
    // ============================================
    println!("=== LOGS PREFIX ELIMINATION ===\n");

    let toon_logs = r#"logs[4]{timestamp,level,endpoint,statusCode}:
  2025-01-15T10:23:45Z,info,/api/users,200
  2025-01-15T10:24:12Z,error,/api/orders,500
  2025-01-15T10:25:03Z,info,/api/products,200
  2025-01-15T10:26:47Z,warn,/api/payment,429"#;

    let dx_logs_current = r#"logs:4(timestamp level endpoint statusCode)@/api/ @2025-01-15T[10:23:45Z info users 200, 10:24:12Z error orders 500, 10:25:03Z info products 200, 10:26:47Z warn payment 429]"#;

    println!("TOON logs:    {} tokens", counter.count(toon_logs, ModelType::Gpt4o).count);
    println!(
        "DX logs:      {} tokens",
        counter.count(dx_logs_current, ModelType::Gpt4o).count
    );

    let toon_t = counter.count(toon_logs, ModelType::Gpt4o).count;
    let dx_t = counter.count(dx_logs_current, ModelType::Gpt4o).count;
    println!("Savings:      {:.1}%", ((toon_t as f64 - dx_t as f64) / toon_t as f64) * 100.0);
    println!();

    println!("=== BRUTAL TOKEN TESTING - MIXED EXAMPLE ===\n");

    // Current DX mixed
    let dx_current = r#"context:5[task=Our_favorite_hikes_together location=Boulder season=spring_2025 year=2025 group=hiking_club]
friends:8=ana luis sam maria carlos emma david sophia
hikes:20(id name distanceKm elevationGain companion wasSunny rating)[1 Blue_Lake_Trail 7.5 320 ana true 4.5, 2 Ridge_Overlook 9.2 540 luis false 4.2, 3 Wildflower_Loop 5.1 180 sam true 4.8, 4 Summit_Peak 12.3 890 maria true 4.9, 5 Canyon_View 8.7 420 carlos false 4.1, 6 Forest_Path 4.2 150 ana true 4.6, 7 Mountain_Ridge 15.1 1100 luis true 4.7, 8 Valley_Trail 6.8 280 sam false 4.3, 9 Lakeside_Walk 3.5 90 maria true 4.4, 10 Alpine_Meadow 11.2 650 carlos true 4.8, 11 Sunset_Point 5.8 220 emma true 4.5, 12 Eagle_Nest 14.2 980 david false 4.6, 13 River_Bend 4.5 120 sophia true 4.2, 14 Pine_Grove 7.1 340 ana false 4.3, 15 Rocky_Summit 16.5 1250 luis true 4.9, 16 Meadow_Loop 3.2 80 sam true 4.1, 17 Cliff_Edge 9.8 560 maria false 4.7, 18 Waterfall_Trail 8.3 410 carlos true 4.8, 19 Hidden_Lake 10.5 720 emma true 4.6, 20 Thunder_Pass 18.2 1400 david true 5.0]"#;

    // TOON mixed
    let toon = r#"context:
  task: Our favorite hikes together
  location: Boulder
  season: spring_2025
  year: 2025
  group: hiking_club
friends[8]: ana,luis,sam,maria,carlos,emma,david,sophia
hikes[20]{id,name,distanceKm,elevationGain,companion,wasSunny,rating}:
  1,Blue Lake Trail,7.5,320,ana,true,4.5
  2,Ridge Overlook,9.2,540,luis,false,4.2
  3,Wildflower Loop,5.1,180,sam,true,4.8
  4,Summit Peak,12.3,890,maria,true,4.9
  5,Canyon View,8.7,420,carlos,false,4.1
  6,Forest Path,4.2,150,ana,true,4.6
  7,Mountain Ridge,15.1,1100,luis,true,4.7
  8,Valley Trail,6.8,280,sam,false,4.3
  9,Lakeside Walk,3.5,90,maria,true,4.4
  10,Alpine Meadow,11.2,650,carlos,true,4.8
  11,Sunset Point,5.8,220,emma,true,4.5
  12,Eagle Nest,14.2,980,david,false,4.6
  13,River Bend,4.5,120,sophia,true,4.2
  14,Pine Grove,7.1,340,ana,false,4.3
  15,Rocky Summit,16.5,1250,luis,true,4.9
  16,Meadow Loop,3.2,80,sam,true,4.1
  17,Cliff Edge,9.8,560,maria,false,4.7
  18,Waterfall Trail,8.3,410,carlos,true,4.8
  19,Hidden Lake,10.5,720,emma,true,4.6
  20,Thunder Pass,18.2,1400,david,true,5.0"#;

    println!("TOON:       {} tokens", counter.count(toon, ModelType::Gpt4o).count);
    println!("DX current: {} tokens", counter.count(dx_current, ModelType::Gpt4o).count);
    println!();

    // Test individual parts
    println!("=== PART BY PART ANALYSIS ===\n");

    // Context part
    let toon_context = r#"context:
  task: Our favorite hikes together
  location: Boulder
  season: spring_2025
  year: 2025
  group: hiking_club"#;
    let dx_context = r#"context:5[task=Our_favorite_hikes_together location=Boulder season=spring_2025 year=2025 group=hiking_club]"#;
    println!("Context - TOON: {} tokens", counter.count(toon_context, ModelType::Gpt4o).count);
    println!("Context - DX:   {} tokens", counter.count(dx_context, ModelType::Gpt4o).count);
    println!();

    // Friends part
    let toon_friends = "friends[8]: ana,luis,sam,maria,carlos,emma,david,sophia";
    let dx_friends = "friends:8=ana luis sam maria carlos emma david sophia";
    println!("Friends - TOON: {} tokens", counter.count(toon_friends, ModelType::Gpt4o).count);
    println!("Friends - DX:   {} tokens", counter.count(dx_friends, ModelType::Gpt4o).count);
    println!();

    // Test true/false vs 1/0
    println!("=== TRUE/FALSE vs 1/0 ===");
    println!("'true' = {} tokens", counter.count("true", ModelType::Gpt4o).count);
    println!("'false' = {} tokens", counter.count("false", ModelType::Gpt4o).count);
    println!("'1' = {} tokens", counter.count("1", ModelType::Gpt4o).count);
    println!("'0' = {} tokens", counter.count("0", ModelType::Gpt4o).count);

    // In context
    println!(
        "'ana true 4.5' = {} tokens",
        counter.count("ana true 4.5", ModelType::Gpt4o).count
    );
    println!("'ana 1 4.5' = {} tokens", counter.count("ana 1 4.5", ModelType::Gpt4o).count);
    println!(
        "'ana false 4.5' = {} tokens",
        counter.count("ana false 4.5", ModelType::Gpt4o).count
    );
    println!("'ana 0 4.5' = {} tokens", counter.count("ana 0 4.5", ModelType::Gpt4o).count);
    println!();

    // Test underscore vs space in trail names
    println!("=== TRAIL NAMES ===");
    println!(
        "'Blue_Lake_Trail' = {} tokens",
        counter.count("Blue_Lake_Trail", ModelType::Gpt4o).count
    );
    println!(
        "'Blue Lake Trail' = {} tokens",
        counter.count("Blue Lake Trail", ModelType::Gpt4o).count
    );
    println!(
        "'BlueLakeTrail' = {} tokens",
        counter.count("BlueLakeTrail", ModelType::Gpt4o).count
    );
    println!();

    // Test row with different separators
    println!("=== ROW SEPARATORS ===");
    let row_comma =
        "1 Blue_Lake_Trail 7.5 320 ana true 4.5, 2 Ridge_Overlook 9.2 540 luis false 4.2";
    let row_semi = "1 Blue_Lake_Trail 7.5 320 ana true 4.5;2 Ridge_Overlook 9.2 540 luis false 4.2";
    let row_colon =
        "1 Blue_Lake_Trail 7.5 320 ana true 4.5:2 Ridge_Overlook 9.2 540 luis false 4.2";
    println!("comma-space: {} tokens", counter.count(row_comma, ModelType::Gpt4o).count);
    println!("semicolon:   {} tokens", counter.count(row_semi, ModelType::Gpt4o).count);
    println!("colon:       {} tokens", counter.count(row_colon, ModelType::Gpt4o).count);
    println!();

    // Test with 1/0 instead of true/false
    println!("=== FULL HIKES WITH 1/0 vs true/false ===");
    let hikes_bool = "[1 Blue_Lake_Trail 7.5 320 ana true 4.5, 2 Ridge_Overlook 9.2 540 luis false 4.2, 3 Wildflower_Loop 5.1 180 sam true 4.8]";
    let hikes_num = "[1 Blue_Lake_Trail 7.5 320 ana 1 4.5, 2 Ridge_Overlook 9.2 540 luis 0 4.2, 3 Wildflower_Loop 5.1 180 sam 1 4.8]";
    println!("true/false: {} tokens", counter.count(hikes_bool, ModelType::Gpt4o).count);
    println!("1/0:        {} tokens", counter.count(hikes_num, ModelType::Gpt4o).count);
    println!();

    // Test full DX with 1/0
    let dx_with_10 = r#"context:5[task=Our_favorite_hikes_together location=Boulder season=spring_2025 year=2025 group=hiking_club]
friends:8=ana luis sam maria carlos emma david sophia
hikes:20(id name distanceKm elevationGain companion wasSunny rating)[1 Blue_Lake_Trail 7.5 320 ana 1 4.5, 2 Ridge_Overlook 9.2 540 luis 0 4.2, 3 Wildflower_Loop 5.1 180 sam 1 4.8, 4 Summit_Peak 12.3 890 maria 1 4.9, 5 Canyon_View 8.7 420 carlos 0 4.1, 6 Forest_Path 4.2 150 ana 1 4.6, 7 Mountain_Ridge 15.1 1100 luis 1 4.7, 8 Valley_Trail 6.8 280 sam 0 4.3, 9 Lakeside_Walk 3.5 90 maria 1 4.4, 10 Alpine_Meadow 11.2 650 carlos 1 4.8, 11 Sunset_Point 5.8 220 emma 1 4.5, 12 Eagle_Nest 14.2 980 david 0 4.6, 13 River_Bend 4.5 120 sophia 1 4.2, 14 Pine_Grove 7.1 340 ana 0 4.3, 15 Rocky_Summit 16.5 1250 luis 1 4.9, 16 Meadow_Loop 3.2 80 sam 1 4.1, 17 Cliff_Edge 9.8 560 maria 0 4.7, 18 Waterfall_Trail 8.3 410 carlos 1 4.8, 19 Hidden_Lake 10.5 720 emma 1 4.6, 20 Thunder_Pass 18.2 1400 david 1 5.0]"#;

    println!("=== FINAL COMPARISON ===");
    println!("TOON:          {} tokens", counter.count(toon, ModelType::Gpt4o).count);
    println!("DX current:    {} tokens", counter.count(dx_current, ModelType::Gpt4o).count);
    println!("DX with 1/0:   {} tokens", counter.count(dx_with_10, ModelType::Gpt4o).count);

    let toon_t = counter.count(toon, ModelType::Gpt4o).count;
    let dx_t = counter.count(dx_current, ModelType::Gpt4o).count;
    let dx_10_t = counter.count(dx_with_10, ModelType::Gpt4o).count;
    println!();
    println!(
        "DX current savings: {:.1}%",
        ((toon_t as f64 - dx_t as f64) / toon_t as f64) * 100.0
    );
    println!(
        "DX with 1/0 savings: {:.1}%",
        ((toon_t as f64 - dx_10_t as f64) / toon_t as f64) * 100.0
    );

    println!("\n=== NULL vs ~ ===");
    println!("'null' = {} tokens", counter.count("null", ModelType::Gpt4o).count);
    println!("'~' = {} tokens", counter.count("~", ModelType::Gpt4o).count);
    println!();

    // Test separators
    println!("=== SEPARATORS ===");
    println!(
        "'a b c d e' (space) = {} tokens",
        counter.count("a b c d e", ModelType::Gpt4o).count
    );
    println!(
        "'a,b,c,d,e' (comma) = {} tokens",
        counter.count("a,b,c,d,e", ModelType::Gpt4o).count
    );
    println!(
        "'a;b;c;d;e' (semi) = {} tokens",
        counter.count("a;b;c;d;e", ModelType::Gpt4o).count
    );
    println!(
        "'a:b:c:d:e' (colon) = {} tokens",
        counter.count("a:b:c:d:e", ModelType::Gpt4o).count
    );
    println!(
        "'a|b|c|d|e' (pipe) = {} tokens",
        counter.count("a|b|c|d|e", ModelType::Gpt4o).count
    );
    println!();

    // Test row separators
    println!("=== ROW SEPARATORS ===");
    println!(
        "'row1, row2, row3' = {} tokens",
        counter.count("row1, row2, row3", ModelType::Gpt4o).count
    );
    println!(
        "'row1;row2;row3' = {} tokens",
        counter.count("row1;row2;row3", ModelType::Gpt4o).count
    );
    println!(
        "'row1:row2:row3' = {} tokens",
        counter.count("row1:row2:row3", ModelType::Gpt4o).count
    );
    println!(
        "'row1|row2|row3' = {} tokens",
        counter.count("row1|row2|row3", ModelType::Gpt4o).count
    );
    println!();

    // Test prefix syntax - word vs symbol
    println!("=== PREFIX SYNTAX ===");
    println!("'@/api/' = {} tokens", counter.count("@/api/", ModelType::Gpt4o).count);
    println!("'pre /api/' = {} tokens", counter.count("pre /api/", ModelType::Gpt4o).count);
    println!("'p /api/' = {} tokens", counter.count("p /api/", ModelType::Gpt4o).count);
    println!();

    // Test with actual data
    println!("=== ACTUAL DATA COMPARISON ===\n");

    // TOON baseline
    let toon = r#"logs[4]{timestamp,level,endpoint,statusCode,responseTime,error}:
  2025-01-15T10:23:45Z,info,/api/users,200,45,null
  2025-01-15T10:24:12Z,error,/api/orders,500,120,Database timeout
  2025-01-15T10:25:03Z,info,/api/products,200,32,null
  2025-01-15T10:26:47Z,warn,/api/payment,429,5,Rate limit exceeded"#;

    // DX current (comma-space separator)
    let dx_comma = r#"logs:4(timestamp level endpoint statusCode responseTime error)[2025-01-15T10:23:45Z info /api/users 200 45 null, 2025-01-15T10:24:12Z error /api/orders 500 120 Database_timeout, 2025-01-15T10:25:03Z info /api/products 200 32 null, 2025-01-15T10:26:47Z warn /api/payment 429 5 Rate_limit_exceeded]"#;

    // DX with colon separator
    let dx_colon = r#"logs:4(timestamp level endpoint statusCode responseTime error)[2025-01-15T10:23:45Z info /api/users 200 45 null:2025-01-15T10:24:12Z error /api/orders 500 120 Database_timeout:2025-01-15T10:25:03Z info /api/products 200 32 null:2025-01-15T10:26:47Z warn /api/payment 429 5 Rate_limit_exceeded]"#;

    // DX with prefix elimination using word "pre"
    let dx_prefix_word = r#"logs:4(timestamp level endpoint statusCode responseTime error)pre /api/ pre 2025-01-15T[10:23:45Z info users 200 45 null:10:24:12Z error orders 500 120 Database_timeout:10:25:03Z info products 200 32 null:10:26:47Z warn payment 429 5 Rate_limit_exceeded]"#;

    // DX with prefix elimination using "p"
    let dx_prefix_p = r#"logs:4(timestamp level endpoint statusCode responseTime error)p /api/ p 2025-01-15T[10:23:45Z info users 200 45 null:10:24:12Z error orders 500 120 Database_timeout:10:25:03Z info products 200 32 null:10:26:47Z warn payment 429 5 Rate_limit_exceeded]"#;

    println!("TOON:              {} tokens", counter.count(toon, ModelType::Gpt4o).count);
    println!("DX comma-space:    {} tokens", counter.count(dx_comma, ModelType::Gpt4o).count);
    println!("DX colon sep:      {} tokens", counter.count(dx_colon, ModelType::Gpt4o).count);
    println!(
        "DX prefix 'pre':   {} tokens",
        counter.count(dx_prefix_word, ModelType::Gpt4o).count
    );
    println!(
        "DX prefix 'p':     {} tokens",
        counter.count(dx_prefix_p, ModelType::Gpt4o).count
    );
    println!();

    // Test different prefix words
    println!("=== PREFIX WORD OPTIONS ===");
    let prefixes = ["@", "p", "pre", "px", "pfx", "base", "root"];
    for pfx in prefixes {
        let test = format!("{} /api/", pfx);
        println!("'{}' = {} tokens", test, counter.count(&test, ModelType::Gpt4o).count);
    }
    println!();

    // Best format test
    println!("=== BEST FORMAT TEST ===\n");

    // Using colon as row separator and "p" for prefix
    let dx_best = r#"logs:4(timestamp level endpoint statusCode responseTime error)p /api/ p 2025-01-15T[10:23:45Z info users 200 45 null:10:24:12Z error orders 500 120 Database_timeout:10:25:03Z info products 200 32 null:10:26:47Z warn payment 429 5 Rate_limit_exceeded]"#;

    let toon_t = counter.count(toon, ModelType::Gpt4o).count;
    let dx_t = counter.count(dx_best, ModelType::Gpt4o).count;
    let savings = ((toon_t as f64 - dx_t as f64) / toon_t as f64) * 100.0;

    println!("TOON: {} tokens", toon_t);
    println!("DX:   {} tokens", dx_t);
    println!("Savings: {:.1}%", savings);
}
