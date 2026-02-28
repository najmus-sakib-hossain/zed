use std::path::Path;

use anyhow::Result;

use crate::core::repository::Repository;

pub fn run(path: &str) -> Result<()> {
    let repo = Repository::init(Path::new(path))?;
    println!("Initialized Forge repository at {}", repo.root.display());
    Ok(())
}
