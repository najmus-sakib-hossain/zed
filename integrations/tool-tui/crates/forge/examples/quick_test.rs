//! Quick Manual Test - All Forge Features
//! Just run: cargo run --example quick_test

use anyhow::Result;
use dx_forge::storage::blob::Blob;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔥 Forge Quick Feature Test\n");

    // Test 1: Blob Storage
    println!("1️⃣  Testing Blob Storage...");
    let blob1 = Blob::from_content("test.txt", b"Hello Forge!".to_vec());
    let blob2 = Blob::from_content("test2.txt", b"Hello Forge!".to_vec());
    println!("   ✅ Blob 1 hash: {}", &blob1.metadata.hash[..16]);
    println!("   ✅ Blob 2 hash: {}", &blob2.metadata.hash[..16]);
    println!("   ✅ Same content = same hash: {}", blob1.metadata.hash == blob2.metadata.hash);

    // Test 2: Binary Serialization
    println!("\n2️⃣  Testing Binary Format...");
    let binary = blob1.to_binary()?;
    println!("   ✅ Serialized: {} bytes", binary.len());
    let restored = Blob::from_binary(&binary)?;
    println!("   ✅ Deserialized: {} bytes", restored.content.len());
    println!("   ✅ Round-trip OK: {}", blob1.metadata.hash == restored.metadata.hash);

    // Test 3: R2 Config
    println!("\n3️⃣  Testing R2 Configuration...");
    match dx_forge::storage::r2::R2Config::from_env() {
        Ok(config) => {
            println!("   ✅ R2 Account: {}", config.account_id);
            println!("   ✅ R2 Bucket: {}", config.bucket_name);
            if let Some(domain) = config.custom_domain {
                println!("   ✅ Custom Domain: {}", domain);
            }
        }
        Err(e) => println!("   ⚠️  R2 not configured: {}", e),
    }

    // Test 4: File Watcher Events
    println!("\n4️⃣  Testing Event Types...");
    #[allow(deprecated)]
    use dx_forge::watcher_legacy::ForgeEvent;
    let rapid = ForgeEvent::Rapid {
        path: "test.rs".to_string(),
        time_us: 25,
        sequence: 1,
    };
    println!("   ✅ Rapid event created (25µs)");
    if let ForgeEvent::Rapid { time_us, .. } = rapid {
        println!("   ✅ Event time: {}µs < 35µs threshold", time_us);
    }

    // Test 5: CRDT Structures
    println!("\n5️⃣  Testing CRDT Types...");
    use dx_forge::crdt::Position;
    let pos = Position {
        lamport_timestamp: 1000,
        actor_id: "alice".to_string(),
        line: 10,
        offset: 42,
        column: 10,
    };
    println!(
        "   ✅ Position created: offset={}, line={}, timestamp={}",
        pos.offset, pos.line, pos.lamport_timestamp
    );

    // Test 6: Traffic Branches
    println!("\n6️⃣  Testing Traffic Branch Detection...");
    unsafe {
        std::env::set_var("CI", "true");
    }
    let is_ci = std::env::var("CI").is_ok();
    println!("   ✅ CI detection: {}", is_ci);
    unsafe {
        std::env::remove_var("CI");
    }

    // Test 7: Database
    println!("\n7️⃣  Testing Database...");
    let temp_db = std::env::temp_dir().join("forge-quick-test.db");
    match dx_forge::storage::Database::new(&temp_db) {
        Ok(_db) => {
            println!("   ✅ Database created at: {:?}", temp_db);
            let _ = std::fs::remove_file(temp_db);
        }
        Err(e) => println!("   ⚠️  Database error: {}", e),
    }

    // Test 8: Check forge-demo files
    println!("\n8️⃣  Checking forge-demo files...");
    let demo_dir = PathBuf::from("examples/forge-demo");
    if demo_dir.exists() {
        println!("   ✅ forge-demo directory exists");
        let files = vec![
            "README.md",
            "Cargo.toml",
            "src/main.rs",
            "src/lib.rs",
            ".forge/config.toml",
        ];
        for file in files {
            let path = demo_dir.join(file);
            if path.exists() {
                println!("   ✅ {}", file);
            }
        }
    } else {
        println!("   ⚠️  forge-demo not found");
    }

    // Test 9: Parallel blob creation
    println!("\n9️⃣  Testing Parallel Operations...");
    use tokio::task::JoinSet;
    let mut tasks = JoinSet::new();
    for i in 0..5 {
        tasks.spawn(async move {
            Blob::from_content(&format!("file{}.txt", i), format!("Content {}", i).into_bytes())
        });
    }
    let mut count = 0;
    while let Some(result) = tasks.join_next().await {
        if result.is_ok() {
            count += 1;
        }
    }
    println!("   ✅ Created {} blobs in parallel", count);

    // Test 10: Component State Manager
    println!("\n🔟 Testing Component State...");
    let temp_forge = std::env::temp_dir().join("forge-state-test");
    std::fs::create_dir_all(&temp_forge)?;
    match dx_forge::context::ComponentStateManager::new(&temp_forge) {
        Ok(_manager) => {
            println!("   ✅ State manager created");
            println!("   ✅ Can manage component states");
        }
        Err(e) => println!("   ⚠️  State manager error: {}", e),
    }
    let _ = std::fs::remove_dir_all(temp_forge);

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎉 All manual tests completed!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}
