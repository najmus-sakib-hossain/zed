//! Test game-changing ideas for token efficiency
//! Run with: cargo run --example test_ideas -p dx-serializer

use serializer::llm::tokens::{ModelType, TokenCounter};

fn main() {
    let counter = TokenCounter::new();

    println!("=== GAME-CHANGING IDEAS FOR TOKEN EFFICIENCY ===\n");

    // Current TOON logs
    let logs_toon = r#"logs[24]{timestamp,level,endpoint,statusCode,responseTime,error}:
  2025-01-15T10:23:45Z,info,/api/users,200,45,null
  2025-01-15T10:24:12Z,error,/api/orders,500,120,Database timeout
  2025-01-15T10:25:03Z,info,/api/products,200,32,null
  2025-01-15T10:26:47Z,warn,/api/payment,429,5,Rate limit exceeded"#;

    // Current DX logs
    let logs_dx_current = r#"logs:24(timestamp level endpoint statusCode responseTime error)[2025-01-15T10:23:45Z info /api/users 200 45 null, 2025-01-15T10:24:12Z error /api/orders 500 120 Database_timeout, 2025-01-15T10:25:03Z info /api/products 200 32 null, 2025-01-15T10:26:47Z warn /api/payment 429 5 Rate_limit_exceeded]"#;

    println!("=== IDEA 1: Abbreviated field names in header ===");
    // Use short field names: ts, lv, ep, sc, rt, er
    let logs_dx_abbrev = r#"logs:24(ts lv ep sc rt er)[2025-01-15T10:23:45Z info /api/users 200 45 null, 2025-01-15T10:24:12Z error /api/orders 500 120 Database_timeout, 2025-01-15T10:25:03Z info /api/products 200 32 null, 2025-01-15T10:26:47Z warn /api/payment 429 5 Rate_limit_exceeded]"#;

    let toon_t = counter.count(logs_toon, ModelType::Gpt4o).count;
    let dx_curr_t = counter.count(logs_dx_current, ModelType::Gpt4o).count;
    let dx_abbrev_t = counter.count(logs_dx_abbrev, ModelType::Gpt4o).count;

    println!("TOON:        {} tokens", toon_t);
    println!("DX current:  {} tokens", dx_curr_t);
    println!("DX abbrev:   {} tokens (saved {})", dx_abbrev_t, dx_curr_t - dx_abbrev_t);
    println!();

    println!("=== IDEA 2: Omit null values with ~ ===");
    // Use ~ for null instead of "null"
    let logs_dx_tilde = r#"logs:24(ts lv ep sc rt er)[2025-01-15T10:23:45Z info /api/users 200 45 ~, 2025-01-15T10:24:12Z error /api/orders 500 120 Database_timeout, 2025-01-15T10:25:03Z info /api/products 200 32 ~, 2025-01-15T10:26:47Z warn /api/payment 429 5 Rate_limit_exceeded]"#;

    let dx_tilde_t = counter.count(logs_dx_tilde, ModelType::Gpt4o).count;
    println!("DX with ~:   {} tokens (saved {})", dx_tilde_t, dx_abbrev_t - dx_tilde_t);
    println!();

    println!("=== IDEA 3: Single char level codes ===");
    // i=info, e=error, w=warn, d=debug
    let logs_dx_codes = r#"logs:24(ts lv ep sc rt er)[2025-01-15T10:23:45Z i /api/users 200 45 ~, 2025-01-15T10:24:12Z e /api/orders 500 120 Database_timeout, 2025-01-15T10:25:03Z i /api/products 200 32 ~, 2025-01-15T10:26:47Z w /api/payment 429 5 Rate_limit_exceeded]"#;

    let dx_codes_t = counter.count(logs_dx_codes, ModelType::Gpt4o).count;
    println!("DX codes:    {} tokens (saved {})", dx_codes_t, dx_tilde_t - dx_codes_t);
    println!();

    println!("=== IDEA 4: Omit /api/ prefix (common prefix elimination) ===");
    // Define prefix once, omit in data
    let logs_dx_prefix = r#"logs:24(ts lv ep sc rt er)@/api/[2025-01-15T10:23:45Z i users 200 45 ~, 2025-01-15T10:24:12Z e orders 500 120 Database_timeout, 2025-01-15T10:25:03Z i products 200 32 ~, 2025-01-15T10:26:47Z w payment 429 5 Rate_limit_exceeded]"#;

    let dx_prefix_t = counter.count(logs_dx_prefix, ModelType::Gpt4o).count;
    println!("DX prefix:   {} tokens (saved {})", dx_prefix_t, dx_codes_t - dx_prefix_t);
    println!();

    println!("=== IDEA 5: Date prefix elimination ===");
    // Define date prefix once
    let logs_dx_date = r#"logs:24(ts lv ep sc rt er)@/api/@2025-01-15T[10:23:45Z i users 200 45 ~, 10:24:12Z e orders 500 120 Database_timeout, 10:25:03Z i products 200 32 ~, 10:26:47Z w payment 429 5 Rate_limit_exceeded]"#;

    let dx_date_t = counter.count(logs_dx_date, ModelType::Gpt4o).count;
    println!("DX date:     {} tokens (saved {})", dx_date_t, dx_prefix_t - dx_date_t);
    println!();

    println!("=== IDEA 6: Semicolon row separator (fewer tokens than comma-space) ===");
    let logs_dx_semi = r#"logs:24(ts lv ep sc rt er)[2025-01-15T10:23:45Z i /api/users 200 45 ~;2025-01-15T10:24:12Z e /api/orders 500 120 Database_timeout;2025-01-15T10:25:03Z i /api/products 200 32 ~;2025-01-15T10:26:47Z w /api/payment 429 5 Rate_limit_exceeded]"#;

    let dx_semi_t = counter.count(logs_dx_semi, ModelType::Gpt4o).count;
    println!("DX semicolon: {} tokens", dx_semi_t);
    println!();

    println!("=== COMBINED: All optimizations ===");
    let logs_dx_all = r#"logs:24(ts lv ep sc rt er)@/api/@2025-01-15T[10:23:45Z i users 200 45 ~;10:24:12Z e orders 500 120 Db_timeout;10:25:03Z i products 200 32 ~;10:26:47Z w payment 429 5 Rate_exceeded]"#;

    let dx_all_t = counter.count(logs_dx_all, ModelType::Gpt4o).count;
    let savings = ((toon_t as f64 - dx_all_t as f64) / toon_t as f64) * 100.0;
    println!("TOON:        {} tokens", toon_t);
    println!("DX combined: {} tokens", dx_all_t);
    println!("Savings:     {:.1}%", savings);
    println!();

    println!("=== ORDERS TEST ===\n");

    let orders_toon = r#"orders[4]{orderId,customerName,customerEmail,items,total,status,date}:
  ORD-001,Alice Chen,alice@example.com,WIDGET-A:2:29.99|GADGET-B:1:49.99,109.97,shipped,2025-01-10
  ORD-002,Bob Smith,bob@example.com,THING-C:3:15,45,delivered,2025-01-11
  ORD-003,Carol Davis,carol@example.com,WIDGET-A:1:29.99|THING-C:2:15,159.97,processing,2025-01-12
  ORD-004,David Wilson,david@example.com,GADGET-B:3:49.99,149.97,shipped,2025-01-12"#;

    let orders_dx_current = r#"orders:4(orderId customerName customerEmail items total status date)[ORD-001 Alice_Chen alice@example.com WIDGET-A:2:29.99|GADGET-B:1:49.99 109.97 shipped 2025-01-10, ORD-002 Bob_Smith bob@example.com THING-C:3:15 45 delivered 2025-01-11, ORD-003 Carol_Davis carol@example.com WIDGET-A:1:29.99|THING-C:2:15 159.97 processing 2025-01-12, ORD-004 David_Wilson david@example.com GADGET-B:3:49.99 149.97 shipped 2025-01-12]"#;

    // Optimized: short headers, ORD- prefix, status codes (s=shipped, d=delivered, p=processing)
    let orders_dx_opt = r#"orders:4(id name email items $ st dt)@ORD-@2025-01-[001 Alice_Chen alice@example.com WA:2:29.99|GB:1:49.99 109.97 s 10;002 Bob_Smith bob@example.com TC:3:15 45 d 11;003 Carol_Davis carol@example.com WA:1:29.99|TC:2:15 159.97 p 12;004 David_Wilson david@example.com GB:3:49.99 149.97 s 12]"#;

    let toon_o = counter.count(orders_toon, ModelType::Gpt4o).count;
    let dx_curr_o = counter.count(orders_dx_current, ModelType::Gpt4o).count;
    let dx_opt_o = counter.count(orders_dx_opt, ModelType::Gpt4o).count;

    let savings_o = ((toon_o as f64 - dx_opt_o as f64) / toon_o as f64) * 100.0;
    println!("TOON:        {} tokens", toon_o);
    println!("DX current:  {} tokens", dx_curr_o);
    println!("DX optimized: {} tokens", dx_opt_o);
    println!("Savings:     {:.1}%", savings_o);
}
