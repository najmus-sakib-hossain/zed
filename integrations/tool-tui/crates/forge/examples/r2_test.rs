/// Test R2 storage connection by uploading and downloading a blob
use anyhow::Result;
use dx_forge::storage::blob::Blob;
use dx_forge::storage::r2::{R2Config, R2Storage};

#[tokio::main]
async fn main() -> Result<()> {
    // Load config from .env
    dotenvy::dotenv().ok();

    println!("Environment variables:");
    println!("  R2_ACCOUNT_ID: {:?}", std::env::var("R2_ACCOUNT_ID"));
    println!("  R2_BUCKET_NAME: {:?}", std::env::var("R2_BUCKET_NAME"));

    let config = R2Config::from_env()?;
    println!("✓ Loaded R2 config for account: {}", config.account_id);
    println!("✓ Bucket: {}", config.bucket_name);
    println!("✓ Endpoint: {}", config.endpoint_url());
    println!("✓ Custom domain: {:?}", config.custom_domain);

    // Create storage client
    let storage = R2Storage::new(config)?;
    println!("✓ Created R2 storage client");

    // Create a test blob
    let test_data = b"Hello from Dx Forge! This is a test blob.";
    let blob = Blob::from_content("test.txt", test_data.to_vec());
    let hash = &blob.metadata.hash;
    println!("\n📦 Created test blob with hash: {}", hash);

    // Upload blob
    println!("\n⬆️  Uploading blob to R2...");
    let key = storage.upload_blob(&blob).await?;
    println!("✓ Uploaded to key: {}", key);

    // Check if blob exists
    println!("\n🔍 Checking if blob exists...");
    let exists = storage.blob_exists(hash).await?;
    println!("✓ Blob exists: {}", exists);

    // Download blob
    println!("\n⬇️  Downloading blob from R2...");
    let downloaded = storage.download_blob(hash).await?;
    println!("✓ Downloaded blob with hash: {}", downloaded.metadata.hash);

    // Verify content
    if downloaded.content == test_data {
        println!("✓ Content matches! Upload/download successful.");
    } else {
        println!("✗ Content mismatch!");
        return Err(anyhow::anyhow!("Downloaded content doesn't match uploaded content"));
    }

    // Clean up - delete the test blob
    println!("\n🗑️  Cleaning up test blob...");
    storage.delete_blob(hash).await?;
    println!("✓ Deleted test blob");

    // Verify deletion
    let exists_after = storage.blob_exists(hash).await?;
    println!("✓ Blob exists after deletion: {}", exists_after);

    println!("\n✅ All R2 operations successful!");
    Ok(())
}
