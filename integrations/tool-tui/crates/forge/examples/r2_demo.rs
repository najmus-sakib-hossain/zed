//! Forge Demo with R2 Integration
//!
//! This example demonstrates:
//! 1. Creating a Forge repository (not Git!)
//! 2. Storing files as binary blobs
//! 3. Uploading blobs to Cloudflare R2
//! 4. Verifying uploads by downloading
//!
//! Run with: cargo run --example r2_demo

use anyhow::{Context, Result};
use dx_forge::storage::blob::Blob;
use dx_forge::storage::r2::{R2Config, R2Storage};
use std::fs;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Forge R2 Demo - Complete Version Control System Test       ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // Step 1: Load R2 configuration
    println!("📋 Step 1: Loading R2 Configuration...");
    let r2_config = R2Config::from_env()?;
    println!("   ✓ Account ID: {}", r2_config.account_id);
    println!("   ✓ Bucket: {}", r2_config.bucket_name);
    println!("   ✓ Endpoint: {}", r2_config.endpoint_url());

    let r2_storage = R2Storage::new(r2_config.clone())?;
    println!("   ✓ R2 Storage client initialized\n");

    // Step 2: Process demo repository files
    println!("📁 Step 2: Processing Forge Demo Repository Files...");
    let demo_path = Path::new("examples/forge-demo");

    if !demo_path.exists() {
        anyhow::bail!("Demo directory not found at {}", demo_path.display());
    }

    let files_to_process = vec![
        "README.md",
        "Cargo.toml",
        "src/main.rs",
        "src/lib.rs",
        ".forge/config.toml",
    ];

    let mut blobs = Vec::new();
    let mut total_size = 0usize;
    let mut compressed_size = 0usize;

    for file_path in files_to_process {
        let full_path = demo_path.join(file_path);

        if !full_path.exists() {
            println!("   ⚠ Skipping {} (not found)", file_path);
            continue;
        }

        let content =
            fs::read(&full_path).with_context(|| format!("Failed to read {}", file_path))?;

        total_size += content.len();

        // Create blob from content
        let blob = Blob::from_content(file_path, content);
        let binary = blob.to_binary()?;
        compressed_size += binary.len();

        println!(
            "   ✓ Processed: {} ({} bytes → {} bytes)",
            file_path,
            blob.metadata.size,
            binary.len()
        );

        blobs.push(blob);
    }

    let compression_ratio = if total_size > 0 {
        100.0 * (1.0 - compressed_size as f64 / total_size as f64)
    } else {
        0.0
    };

    println!("\n   📊 Statistics:");
    println!("      Total files: {}", blobs.len());
    println!("      Original size: {} bytes", total_size);
    println!("      Binary size: {} bytes", compressed_size);
    println!("      Space savings: {:.1}%\n", compression_ratio);

    // Step 3: Upload blobs to R2
    println!("☁️  Step 3: Uploading Blobs to Cloudflare R2...");
    println!("   Endpoint: {}", r2_config.endpoint_url());
    println!("   Bucket: {}", r2_config.bucket_name);
    println!();

    let mut upload_count = 0;
    let mut upload_errors = Vec::new();

    for blob in &blobs {
        let hash = &blob.metadata.hash;
        let short_hash = &hash[..8];

        print!("   📤 Uploading {} ({}...)... ", blob.metadata.path, short_hash);

        match r2_storage.upload_blob(blob).await {
            Ok(_) => {
                println!("✓");
                upload_count += 1;
            }
            Err(e) => {
                println!("✗");
                upload_errors.push(format!("{}: {}", blob.metadata.path, e));
            }
        }
    }

    println!("\n   📊 Upload Statistics:");
    println!("      Successfully uploaded: {}/{}", upload_count, blobs.len());

    if !upload_errors.is_empty() {
        println!("      Errors:");
        for error in &upload_errors {
            println!("        • {}", error);
        }
    }
    println!();

    // Step 4: Verify uploads by downloading
    if upload_count > 0 {
        println!("🔄 Step 4: Verifying Uploads (Download Test)...");

        let mut verify_count = 0;
        for blob in &blobs {
            let hash = &blob.metadata.hash;
            let short_hash = &hash[..8];

            print!("   📥 Downloading {}... ", short_hash);

            match r2_storage.download_blob(hash).await {
                Ok(downloaded_blob) => {
                    if downloaded_blob.metadata.hash == blob.metadata.hash
                        && downloaded_blob.content == blob.content
                    {
                        println!("✓ Verified");
                        verify_count += 1;
                    } else {
                        println!("✗ Hash mismatch!");
                    }
                }
                Err(e) => {
                    println!("✗ Error: {}", e);
                }
            }
        }

        println!("\n   📊 Verification Statistics:");
        println!("      Successfully verified: {}/{}", verify_count, upload_count);
        println!();
    }

    // Step 5: Display R2 URLs for verification
    println!("🌐 Step 5: R2 Storage URLs...");
    println!("\n   You can verify the uploads in Cloudflare Dashboard:");
    println!(
        "   URL: https://dash.cloudflare.com/?to=/:account/r2/overview/buckets/{}",
        r2_config.bucket_name
    );
    println!("\n   Blob paths in R2:");

    for blob in &blobs {
        let hash = &blob.metadata.hash;
        let path = format!("blobs/{}/{}", &hash[..2], &hash[2..]);
        println!("   • {} → {}", blob.metadata.path, path);
    }
    println!();

    // Final summary
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Demo Complete! Summary:                                     ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!(
        "║  ✓ Files processed: {:2}                                       ║",
        blobs.len()
    );
    println!(
        "║  ✓ Blobs uploaded to R2: {:2}                                 ║",
        upload_count
    );
    println!(
        "║  ✓ Space savings: {:>4.1}%                                 ║",
        compression_ratio
    );
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  🎉 Forge is fully operational with R2 storage!             ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("💡 Next Steps:");
    println!("   1. Check your R2 bucket in Cloudflare Dashboard");
    println!("   2. Try modifying files in examples/forge-demo/");
    println!("   3. Run this demo again to see new blobs uploaded");
    println!("   4. All blobs are content-addressed (same content = same hash)\n");

    Ok(())
}
