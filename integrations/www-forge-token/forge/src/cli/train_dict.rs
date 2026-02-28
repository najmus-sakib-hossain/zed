use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use walkdir::WalkDir;

use crate::store::compression;

pub fn run(file_type: &str, samples_dir: &str, output: &str) -> Result<()> {
    let ext = file_type.trim_start_matches('.').to_ascii_lowercase();
    let mut samples = Vec::new();

    for entry in WalkDir::new(samples_dir)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        if samples.len() >= 1000 {
            break;
        }
        let path = entry.path();
        let matches = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case(&ext))
            .unwrap_or(false);
        if !matches {
            continue;
        }

        let bytes = fs::read(path).with_context(|| format!("read sample {}", path.display()))?;
        samples.push(bytes.into_iter().take(128 * 1024).collect::<Vec<u8>>());
    }

    let dict = compression::train_dictionary(&samples, 112_640)?;
    let out_path = Path::new(output);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    fs::write(out_path, &dict).with_context(|| format!("write dict {}", out_path.display()))?;
    println!(
        "Trained dictionary from {} samples -> {} bytes",
        samples.len(),
        dict.len()
    );
    Ok(())
}
