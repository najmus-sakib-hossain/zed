//! # DX Package Manager v3.0
//!
//! ## Optimizations
//!
//! 1. **CPRI** (Registry Index) - Local resolution, reduced network calls
//! 2. **Speculative Pipeline** - Overlap resolution + downloads
//! 3. **Parallel HTTP/2** - Concurrent connections
//! 4. **SIMD Extraction** - AVX2 gzip + parallel writes
//! 5. **Reflink Install** - Copy-on-write linking

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::fs;

// Import the 5 innovations
use dx_pkg_extract::FastExtractor;
use dx_pkg_link::ReflinkLinker;
use dx_pkg_pipeline::{ManifestDep, SpeculativePipeline};
use dx_pkg_registry_index::RegistryIndex;

#[derive(Debug, serde::Deserialize)]
struct PackageJson {
    #[serde(default)]
    dependencies: HashMap<String, String>,
    #[serde(rename = "devDependencies")]
    #[serde(default)]
    dev_dependencies: HashMap<String, String>,
}

/// Optimized cold install (v3.0)
pub async fn install_v3(_frozen: bool, production: bool) -> Result<()> {
    let total_start = Instant::now();

    println!("⚡ DX Package Manager v3.0");
    println!();

    // ═══════════════════════════════════════════════════════════════
    // PHASE 1: Registry Index
    // ═══════════════════════════════════════════════════════════════

    println!("📦 Phase 1: Loading registry index...");
    let phase1_start = Instant::now();

    let index = Arc::new(
        RegistryIndex::open_or_download()
            .await
            .context("Failed to load registry index")?,
    );

    let phase1_time = phase1_start.elapsed();
    println!("   ✓ Index ready in {:.2}ms", phase1_time.as_secs_f64() * 1000.0);

    // Read package.json
    let package_json_str = fs::read_to_string("package.json")
        .await
        .context("Failed to read package.json")?;
    let package_json: PackageJson =
        serde_json::from_str(&package_json_str).context("Failed to parse package.json")?;

    // Extract dependencies
    let mut dependencies = package_json.dependencies;
    if !production {
        dependencies.extend(package_json.dev_dependencies);
    }

    if dependencies.is_empty() {
        println!("✨ No dependencies to install");
        return Ok(());
    }

    let manifest_deps: Vec<ManifestDep> = dependencies
        .into_iter()
        .map(|(name, constraint)| ManifestDep { name, constraint })
        .collect();

    println!("   📋 {} dependencies declared", manifest_deps.len());

    // ═══════════════════════════════════════════════════════════════
    // PHASE 2: Speculative Pipeline
    // - Resolution + Download overlap
    // - Parallel HTTP/2 connections
    // ═══════════════════════════════════════════════════════════════

    println!();
    println!("🌐 Phase 2: Resolution + parallel download...");
    let phase2_start = Instant::now();

    let pipeline = SpeculativePipeline::new(index)?;
    let downloaded = pipeline.run(manifest_deps).await.context("Pipeline failed")?;

    let phase2_time = phase2_start.elapsed();
    println!(
        "   ✓ Downloaded {} packages in {:.2}ms",
        downloaded.len(),
        phase2_time.as_secs_f64() * 1000.0
    );

    // ═══════════════════════════════════════════════════════════════
    // PHASE 3: SIMD Extraction
    // - AVX2 gzip decompression
    // - Parallel file writes
    // ═══════════════════════════════════════════════════════════════

    println!();
    println!("📂 Phase 3: Extraction (SIMD + parallel)...");
    let phase3_start = Instant::now();

    let nm = std::env::current_dir()?.join("node_modules");
    std::fs::create_dir_all(&nm)?;

    // Prepare extraction jobs
    let jobs: Vec<(Vec<u8>, PathBuf)> =
        downloaded.iter().map(|pkg| (pkg.data.clone(), nm.join(&pkg.name))).collect();

    // Extract all in parallel with SIMD
    FastExtractor::extract_many(&jobs).context("Extraction failed")?;

    let phase3_time = phase3_start.elapsed();
    println!(
        "   ✓ Extracted in {:.2}ms (SIMD accelerated)",
        phase3_time.as_secs_f64() * 1000.0
    );

    // ═══════════════════════════════════════════════════════════════
    // PHASE 4: Reflink Install
    // - Copy-on-write linking
    // ═══════════════════════════════════════════════════════════════

    // Note: For npm packages, we extract directly to node_modules,
    // so reflink phase is skipped. Reflinks are used for .dxp cache
    // when we read from binary format.

    let linker = ReflinkLinker::new();
    if linker.supports_reflinks() {
        println!("   💚 Reflinks supported (COW enabled)");
    } else {
        println!("   💛 Reflinks not supported (using fallback)");
    }

    // ═══════════════════════════════════════════════════════════════
    // PHASE 5: Create Lock File
    // ═══════════════════════════════════════════════════════════════

    let phase5_start = Instant::now();
    write_lock_file(&downloaded).await?;
    let phase5_time = phase5_start.elapsed();

    // ═══════════════════════════════════════════════════════════════
    // SUMMARY
    // ═══════════════════════════════════════════════════════════════

    let total_time = total_start.elapsed();
    let total_ms = total_time.as_secs_f64() * 1000.0;

    println!();
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║                   INSTALLATION COMPLETE                   ║");
    println!("╠══════════════════════════════════════════════════════════╣");
    println!("║                                                           ║");
    println!("║  Total Time:      {:.2} ms                              ║", total_ms);
    println!("║                                                           ║");
    println!("║  Phase Breakdown:                                         ║");
    println!(
        "║  ├─ Registry Index:   {:.2} ms                          ║",
        phase1_time.as_secs_f64() * 1000.0
    );
    println!(
        "║  ├─ Download:         {:.2} ms                          ║",
        phase2_time.as_secs_f64() * 1000.0
    );
    println!(
        "║  ├─ Extract (SIMD):   {:.2} ms                          ║",
        phase3_time.as_secs_f64() * 1000.0
    );
    println!(
        "║  └─ Lock file:        {:.2} ms                          ║",
        phase5_time.as_secs_f64() * 1000.0
    );
    println!("║                                                           ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    Ok(())
}

async fn write_lock_file(packages: &[dx_pkg_pipeline::DownloadedPackage]) -> Result<()> {
    use std::collections::BTreeMap;

    let mut lock = BTreeMap::new();
    for pkg in packages {
        lock.insert(&pkg.name, &pkg.version);
    }

    let lock_json = serde_json::to_string_pretty(&lock)?;
    fs::write("dx-lock.json", lock_json).await?;

    Ok(())
}

/// Simple benchmark mode (for testing)
pub async fn benchmark_v3(runs: usize) -> Result<()> {
    let mut times = Vec::new();

    println!("🔬 Running {} benchmark runs...", runs);
    println!();

    for i in 1..=runs {
        println!("═══════════════════════════════════════════════════════════");
        println!("  Run {}/{}", i, runs);
        println!("═══════════════════════════════════════════════════════════");

        // Clean node_modules
        let _ = std::fs::remove_dir_all("node_modules");

        let start = Instant::now();
        install_v3(false, false).await?;
        let elapsed = start.elapsed();

        times.push(elapsed.as_secs_f64() * 1000.0);

        println!();
    }

    // Calculate statistics
    let avg = times.iter().sum::<f64>() / times.len() as f64;
    let min = times.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║                    BENCHMARK RESULTS                      ║");
    println!("╠══════════════════════════════════════════════════════════╣");
    println!("║                                                           ║");
    println!("║  Runs:    {}                                             ║", runs);
    println!("║  Average: {:.2} ms                                      ║", avg);
    println!("║  Min:     {:.2} ms                                      ║", min);
    println!("║  Max:     {:.2} ms                                      ║", max);
    println!("║                                                           ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    Ok(())
}
