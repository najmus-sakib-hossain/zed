pub mod ai_context;
pub mod annotations;
pub mod discussions;
pub mod traffic_branch;

use anyhow::Result;
use std::path::Path;

pub use annotations::Annotation;
pub use traffic_branch::{ComponentStateManager, TrafficBranch, UpdateResult, apply_update};

use crate::crdt::{Anchor, Position};
use crate::storage::Database;

pub async fn create_anchor(
    file: &Path,
    line: usize,
    column: usize,
    message: Option<String>,
) -> Result<Anchor> {
    let db = Database::open(".dx/forge")?;

    // Load config to get actor_id
    let config: serde_json::Value =
        serde_json::from_str(&tokio::fs::read_to_string(".dx/forge/config.json").await?)?;
    let actor_id = config["actor_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing 'actor_id' in config.json"))?
        .to_string();

    let position = Position::new(line, column, 0, actor_id, 0);
    let anchor = Anchor::new(file.display().to_string(), position, message);

    db.store_anchor(&anchor)?;

    Ok(anchor)
}

pub async fn annotate(file: &Path, line: usize, message: &str, is_ai: bool) -> Result<()> {
    let annotation = Annotation::new(file.display().to_string(), line, message.to_string(), is_ai);

    // Store annotation
    let db = Database::open(".dx/forge")?;
    annotations::store_annotation(&db, &annotation)?;

    Ok(())
}

pub async fn show_context(file: &Path, line: Option<usize>) -> Result<()> {
    use colored::*;

    let db = Database::open(".dx/forge")?;
    let annotations = annotations::get_annotations(&db, file, line)?;

    println!("{}", format!("Context for: {}", file.display()).cyan().bold());
    println!("{}", "═".repeat(80).bright_black());

    for ann in annotations {
        let icon = if ann.is_ai { "🤖" } else { "👤" };
        let author = if ann.is_ai {
            "AI Agent".bright_magenta()
        } else {
            ann.author.bright_cyan()
        };

        println!(
            "\n{} {} {} {}",
            icon,
            author,
            format!("(line {})", ann.line).bright_black(),
            ann.created_at.format("%Y-%m-%d %H:%M").to_string().bright_black()
        );
        println!("   {}", ann.content.bright_white());
    }

    Ok(())
}
