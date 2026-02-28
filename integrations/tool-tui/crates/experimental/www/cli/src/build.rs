//! Build command implementation

use anyhow::Result;
use console::style;
use std::fs;
use std::path::Path;
use std::time::Instant;

#[derive(Default)]
pub struct BuildStats {
    pub pages: usize,
    pub pages_size: usize,
    pub components: usize,
    pub components_size: usize,
    pub assets: usize,
    pub assets_size: usize,
    pub styles: usize,
    pub styles_size: usize,
}

impl BuildStats {
    pub fn total_size(&self) -> usize {
        self.pages_size + self.components_size + self.assets_size + self.styles_size
    }
}

pub fn compile_pages(output_dir: &Path, stats: &mut BuildStats) -> Result<()> {
    if !Path::new("pages").exists() {
        return Ok(());
    }

    for entry in fs::read_dir("pages")? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("pg") {
            let content = fs::read(&path)?;
            let file_name = path.file_stem().unwrap().to_str().unwrap();
            let output_path = output_dir.join(format!("{}.dxob", file_name));

            fs::write(&output_path, &content)?;

            stats.pages += 1;
            stats.pages_size += content.len();
        }
    }

    Ok(())
}

pub fn compile_components(output_dir: &Path, stats: &mut BuildStats) -> Result<()> {
    if !Path::new("components").exists() {
        return Ok(());
    }

    for entry in fs::read_dir("components")? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("cp") {
            let content = fs::read(&path)?;
            let file_name = path.file_stem().unwrap().to_str().unwrap();
            let output_path = output_dir.join(format!("{}.dxob", file_name));

            fs::write(&output_path, &content)?;

            stats.components += 1;
            stats.components_size += content.len();
        }
    }

    Ok(())
}

pub fn copy_dir_recursive(src: &str, dst: &Path, stats: &mut BuildStats) -> Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().unwrap();
        let dst_path = dst.join(file_name);

        if path.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_dir_recursive(path.to_str().unwrap(), &dst_path, stats)?;
        } else {
            let content = fs::read(&path)?;
            fs::write(&dst_path, &content)?;

            if src.starts_with("styles") {
                stats.styles += 1;
                stats.styles_size += content.len();
            } else {
                stats.assets += 1;
                stats.assets_size += content.len();
            }
        }
    }

    Ok(())
}

pub fn generate_manifest(stats: &BuildStats, optimization: &str) -> serde_json::Value {
    use serde_json::json;

    json!({
        "version": "1.0.0",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "optimization": optimization,
        "stats": {
            "pages": stats.pages,
            "pages_size": stats.pages_size,
            "components": stats.components,
            "components_size": stats.components_size,
            "assets": stats.assets,
            "assets_size": stats.assets_size,
            "styles": stats.styles,
            "styles_size": stats.styles_size,
            "total_size": stats.total_size(),
        }
    })
}

pub async fn cmd_build(output: Option<&Path>, optimization: &str) -> Result<()> {
    let start = Instant::now();

    println!("{}", style("Building for production...").cyan().bold());
    println!();

    if !Path::new("dx").exists() {
        anyhow::bail!("No dx config file found. Are you in a DX WWW project?");
    }

    let output_dir = output.unwrap_or(Path::new(".dx/build"));
    let cache_dir = Path::new(".dx/cache");

    fs::create_dir_all(output_dir)?;
    fs::create_dir_all(cache_dir)?;
    fs::create_dir_all(output_dir.join("pages"))?;
    fs::create_dir_all(output_dir.join("components"))?;
    fs::create_dir_all(output_dir.join("public"))?;
    fs::create_dir_all(output_dir.join("styles"))?;

    let mut stats = BuildStats::default();

    if Path::new("pages").exists() {
        println!("{}", style("→ Compiling pages...").cyan());
        compile_pages(&output_dir.join("pages"), &mut stats)?;
    }

    if Path::new("components").exists() {
        println!("{}", style("→ Compiling components...").cyan());
        compile_components(&output_dir.join("components"), &mut stats)?;
    }

    if Path::new("public").exists() {
        println!("{}", style("→ Copying public assets...").cyan());
        copy_dir_recursive("public", &output_dir.join("public"), &mut stats)?;
    }

    if Path::new("styles").exists() {
        println!("{}", style("→ Processing styles...").cyan());
        copy_dir_recursive("styles", &output_dir.join("styles"), &mut stats)?;
    }

    let manifest = generate_manifest(&stats, optimization);
    fs::write(output_dir.join("manifest.json"), serde_json::to_string_pretty(&manifest)?)?;

    let total_time = start.elapsed();

    crate::utils::print_build_summary(&stats, total_time, output_dir);

    Ok(())
}
