use anyhow::Result;
use std::path::Path;

pub async fn sync_with_git(path: &Path) -> Result<()> {
    // Check if Forge is already initialized
    if path.join(".dx").exists() {
        println!("✓ Forge repository already exists.");
        return Ok(());
    }

    println!("🔄 Initializing Forge repository...");

    // Initialize Forge repository
    crate::storage::init(path).await?;

    println!("✓ Forge repository initialized successfully.");
    println!("💡 You can now use Forge for operation-level version control.");

    Ok(())
}
