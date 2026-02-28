//! # Packer Module - The Binary Writer
//!
//! Creates the final `.dxb` artifact.
//!
//! ## Format Specification
//! - **Header:** `MAGIC_BYTES ("DX")` + `VERSION (1)`
//! - **Section 1:** Capabilities Manifest (Signed)
//! - **Section 2:** Template Dictionary (Gzipped JSON)
//! - **Section 3:** WASM Blob (Optimized)

use anyhow::{Context, Result};
use flate2::Compression;
use flate2::write::GzEncoder;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use crate::splitter::Template;

// Re-export shared types
pub use dx_www_packet::{CapabilitiesManifest, DxbArtifact};

/// Magic bytes for .dxb format
const MAGIC_BYTES: &[u8] = b"DX";

/// Format version
const FORMAT_VERSION: u8 = 1;

/// Pack templates and WASM into .dxb file
pub fn pack_dxb(
    output_dir: &Path,
    templates: Vec<Template>,
    wasm_bytes: Vec<u8>,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("  Packing .dxb artifact...");
    }

    // Create output directory
    fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    let output_path = output_dir.join("app.dxb");

    // Create capabilities manifest (default for now)
    let capabilities = CapabilitiesManifest::default();

    // Create artifact metadata
    let artifact = DxbArtifact {
        version: FORMAT_VERSION,
        capabilities,
        templates: templates.clone(),
        wasm_size: wasm_bytes.len() as u32,
    };

    // Serialize to JSON (replacing bincode)
    let artifact_bytes = serde_json::to_vec(&artifact).context("Failed to serialize artifact")?;

    // Compress templates section
    let mut compressed_templates = Vec::new();
    {
        let mut encoder = GzEncoder::new(&mut compressed_templates, Compression::best());
        encoder.write_all(&artifact_bytes).context("Failed to compress templates")?;
        encoder.finish().context("Failed to finish compression")?;
    }

    // Write final .dxb file
    let mut file = File::create(&output_path)
        .context(format!("Failed to create output file: {}", output_path.display()))?;

    // Write header
    file.write_all(MAGIC_BYTES).context("Failed to write magic bytes")?;
    file.write_all(&[FORMAT_VERSION]).context("Failed to write version")?;

    // Write compressed artifact size (4 bytes, little endian)
    let artifact_size = compressed_templates.len() as u32;
    file.write_all(&artifact_size.to_le_bytes())
        .context("Failed to write artifact size")?;

    // Write compressed artifact
    file.write_all(&compressed_templates)
        .context("Failed to write compressed artifact")?;

    // Write WASM blob
    file.write_all(&wasm_bytes).context("Failed to write WASM blob")?;

    file.flush().context("Failed to flush file")?;

    if verbose {
        println!("    Artifact size: {} bytes", artifact_size);
        println!("    WASM size: {} bytes", wasm_bytes.len());
        println!(
            "    Total .dxb size: {} bytes",
            MAGIC_BYTES.len() + 1 + 4 + artifact_size as usize + wasm_bytes.len()
        );
    }

    // Also create separate files for debugging
    if verbose {
        let templates_path = output_dir.join("templates.json");
        let templates_json = serde_json::to_string_pretty(&templates)?;
        fs::write(&templates_path, templates_json)?;
        println!("    Debug templates: {}", templates_path.display());

        let wasm_path = output_dir.join("app.wasm");
        fs::write(&wasm_path, &wasm_bytes)?;
        println!("    Debug WASM: {}", wasm_path.display());
    }

    println!("  ✓ Packed to: {}", output_path.display());

    Ok(())
}

/// Pack templates and HTIP stream into .dxb file (NO WASM!)
///
/// This is the new lightweight packer. The output is pure data that
/// the dx-client runtime interprets. No per-app WASM overhead.
pub fn pack_dxb_htip(
    output_dir: &Path,
    templates: &[Template],
    htip_stream: &[u8],
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("  Packing .dxb (HTIP mode)...");
    }

    // Create output directory
    fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    let output_path = output_dir.join("app.dxb");

    // Write final .dxb file
    let mut file = File::create(&output_path)
        .context(format!("Failed to create output file: {}", output_path.display()))?;

    // Write header (simplified for HTIP-only format)
    file.write_all(MAGIC_BYTES).context("Failed to write magic bytes")?;
    file.write_all(&[FORMAT_VERSION]).context("Failed to write version")?;

    // Write mode flag: 0x01 = HTIP-only (no WASM)
    file.write_all(&[0x01]).context("Failed to write mode flag")?;

    // Write HTIP stream size (4 bytes, little endian)
    let htip_size = htip_stream.len() as u32;
    file.write_all(&htip_size.to_le_bytes()).context("Failed to write HTIP size")?;

    // Write HTIP stream (already includes header, strings, templates, opcodes)
    file.write_all(htip_stream).context("Failed to write HTIP stream")?;

    file.flush().context("Failed to flush file")?;

    let total_size = MAGIC_BYTES.len() + 1 + 1 + 4 + htip_stream.len();

    if verbose {
        println!("    HTIP stream size: {} bytes", htip_size);
        println!("    Total .dxb size: {} bytes", total_size);

        // Debug: write separate files
        let htip_path = output_dir.join("app.htip");
        fs::write(&htip_path, htip_stream)?;
        println!("    Debug HTIP: {}", htip_path.display());

        let templates_path = output_dir.join("templates.json");
        let templates_json = serde_json::to_string_pretty(templates)?;
        fs::write(&templates_path, templates_json)?;
        println!("    Debug templates: {}", templates_path.display());
    }

    println!("  ✓ Packed to: {} ({} bytes - TINY!)", output_path.display(), total_size);

    Ok(())
}

/// Unpack .dxb file (for runtime loading)
pub fn unpack_dxb(dxb_path: &Path) -> Result<(DxbArtifact, Vec<u8>)> {
    let bytes = fs::read(dxb_path).context("Failed to read .dxb file")?;

    // Verify magic bytes
    if &bytes[0..2] != MAGIC_BYTES {
        return Err(anyhow::anyhow!("Invalid .dxb file: wrong magic bytes"));
    }

    // Check version
    let version = bytes[2];
    if version != FORMAT_VERSION {
        return Err(anyhow::anyhow!("Unsupported .dxb version: {}", version));
    }

    // Read artifact size
    let artifact_size = u32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]) as usize;

    // Decompress artifact
    let compressed_artifact = &bytes[7..7 + artifact_size];
    let mut decompressed = Vec::new();
    {
        use flate2::read::GzDecoder;
        use std::io::Read;
        let mut decoder = GzDecoder::new(compressed_artifact);
        decoder
            .read_to_end(&mut decompressed)
            .context("Failed to decompress artifact")?;
    }

    // Deserialize artifact from JSON
    let artifact: DxbArtifact =
        serde_json::from_slice(&decompressed).context("Failed to deserialize artifact")?;

    // Extract WASM
    let wasm_bytes = bytes[7 + artifact_size..].to_vec();

    // Verify WASM size
    if wasm_bytes.len() != artifact.wasm_size as usize {
        return Err(anyhow::anyhow!(
            "WASM size mismatch: expected {}, got {}",
            artifact.wasm_size,
            wasm_bytes.len()
        ));
    }

    Ok((artifact, wasm_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_pack_unpack_roundtrip() {
        let temp_dir = tempdir().unwrap();

        let templates = vec![Template {
            id: 0,
            html: "<div>Test</div>".to_string(),
            slots: Vec::new(),
            hash: "test".to_string(),
        }];

        let wasm_bytes = vec![0x00, 0x61, 0x73, 0x6d]; // WASM magic

        // Pack
        pack_dxb(temp_dir.path(), templates.clone(), wasm_bytes.clone(), false).unwrap();

        // Unpack
        let dxb_path = temp_dir.path().join("app.dxb");
        let (artifact, unpacked_wasm) = unpack_dxb(&dxb_path).unwrap();

        assert_eq!(artifact.version, FORMAT_VERSION);
        assert_eq!(artifact.templates.len(), 1);
        assert_eq!(artifact.templates[0].id, 0);
        assert_eq!(unpacked_wasm, wasm_bytes);
    }
}
