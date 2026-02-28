use std::fs;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

use crate::core::manifest::deserialize_commit;
use crate::core::repository::Repository;

fn ts_to_datetime(timestamp_ns: i64) -> DateTime<Utc> {
    let secs = timestamp_ns.div_euclid(1_000_000_000);
    let nsecs = timestamp_ns.rem_euclid(1_000_000_000) as u32;
    DateTime::<Utc>::from_timestamp(secs, nsecs).unwrap_or_else(Utc::now)
}

pub fn run(count: usize) -> Result<()> {
    let cwd = std::env::current_dir().context("get current dir")?;
    let repo = Repository::discover(&cwd)?;

    let Some(mut current) = repo.read_head()? else {
        println!("No commits yet");
        return Ok(());
    };

    for _ in 0..count {
        let hex_id = hex::encode(current);
        let path = repo.forge_dir.join("manifests").join(&hex_id);
        if !path.exists() {
            break;
        }
        let bytes = fs::read(&path).with_context(|| format!("read manifest {}", path.display()))?;
        let commit = deserialize_commit(&bytes)?;
        let dt = ts_to_datetime(commit.timestamp_ns);

        println!("\x1b[33mcommit {}\x1b[0m", hex_id);
        println!("Author: {}", commit.author);
        println!("Date:   {}", dt.format("%Y-%m-%d %H:%M:%S UTC"));
        println!("Files:  {}", commit.files.len());
        println!("\n    {}\n", commit.message);

        let Some(parent) = commit.parents.first().copied() else {
            break;
        };
        current = parent;
    }

    Ok(())
}
