//! Native archive processing using pure Rust crates.
//!
//! This module provides high-performance native Rust archive handling
//! as an alternative to external tools.
//!
//! Enable with the `archive-core` feature flag.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;

use crate::tools::ToolOutput;

/// Native ZIP extraction.
#[cfg(feature = "archive-core")]
pub fn extract_zip_native(
    input: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> std::io::Result<ToolOutput> {
    use zip::ZipArchive;

    let input = input.as_ref();
    let output_dir = output_dir.as_ref();

    std::fs::create_dir_all(output_dir)?;

    let file = std::fs::File::open(input)?;
    let mut archive = ZipArchive::new(file)?;

    let mut extracted = Vec::new();
    let total_files = archive.len();

    for i in 0..total_files {
        let mut file = archive.by_index(i)?;
        let outpath = output_dir.join(file.name());

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
            extracted.push(outpath);
        }
    }

    let mut metadata = HashMap::new();
    metadata.insert("total_files".to_string(), total_files.to_string());
    metadata.insert("extracted_files".to_string(), extracted.len().to_string());

    Ok(ToolOutput {
        success: true,
        message: format!(
            "Extracted {} files from {} to {}",
            extracted.len(),
            input.display(),
            output_dir.display()
        ),
        output_paths: extracted,
        metadata,
    })
}

/// Native ZIP creation.
#[cfg(feature = "archive-core")]
pub fn create_zip_native(
    inputs: &[impl AsRef<Path>],
    output: impl AsRef<Path>,
    compression_level: Option<i64>,
) -> std::io::Result<ToolOutput> {
    use walkdir::WalkDir;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let output = output.as_ref();
    let file = std::fs::File::create(output)?;
    let mut zip = ZipWriter::new(file);

    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(compression_level);

    let mut file_count = 0;
    let mut total_size = 0u64;

    for input in inputs {
        let input = input.as_ref();

        if input.is_dir() {
            for entry in WalkDir::new(input).into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();
                let relative = path.strip_prefix(input.parent().unwrap_or(input)).unwrap_or(path);

                if path.is_file() {
                    zip.start_file(relative.to_string_lossy(), options)?;
                    let mut f = std::fs::File::open(path)?;
                    let mut buffer = Vec::new();
                    f.read_to_end(&mut buffer)?;
                    zip.write_all(&buffer)?;

                    file_count += 1;
                    total_size += buffer.len() as u64;
                } else if path.is_dir() && path != input {
                    zip.add_directory(relative.to_string_lossy(), options)?;
                }
            }
        } else if input.is_file() {
            let name = input.file_name().unwrap_or_default();
            zip.start_file(name.to_string_lossy(), options)?;
            let mut f = std::fs::File::open(input)?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;

            file_count += 1;
            total_size += buffer.len() as u64;
        }
    }

    zip.finish()?;

    let compressed_size = std::fs::metadata(output)?.len();
    let compression_ratio = if total_size > 0 {
        (1.0 - (compressed_size as f64 / total_size as f64)) * 100.0
    } else {
        0.0
    };

    let mut metadata = HashMap::new();
    metadata.insert("file_count".to_string(), file_count.to_string());
    metadata.insert("original_size".to_string(), total_size.to_string());
    metadata.insert("compressed_size".to_string(), compressed_size.to_string());
    metadata.insert("compression_ratio".to_string(), format!("{:.1}%", compression_ratio));

    Ok(ToolOutput {
        success: true,
        message: format!(
            "Created {} with {} files ({:.1}% compression)",
            output.display(),
            file_count,
            compression_ratio
        ),
        output_paths: vec![output.to_path_buf()],
        metadata,
    })
}

/// Native tar.gz extraction.
#[cfg(feature = "archive-core")]
pub fn extract_tar_gz_native(
    input: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> std::io::Result<ToolOutput> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let input = input.as_ref();
    let output_dir = output_dir.as_ref();

    std::fs::create_dir_all(output_dir)?;

    let file = std::fs::File::open(input)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    let mut extracted = Vec::new();

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_path_buf();
        let outpath = output_dir.join(&path);

        if entry.header().entry_type().is_dir() {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            entry.unpack(&outpath)?;
            extracted.push(outpath);
        }
    }

    let mut metadata = HashMap::new();
    metadata.insert("extracted_files".to_string(), extracted.len().to_string());

    Ok(ToolOutput {
        success: true,
        message: format!(
            "Extracted {} files from {} to {}",
            extracted.len(),
            input.display(),
            output_dir.display()
        ),
        output_paths: extracted,
        metadata,
    })
}

/// Native tar.gz creation.
#[cfg(feature = "archive-core")]
pub fn create_tar_gz_native(
    inputs: &[impl AsRef<Path>],
    output: impl AsRef<Path>,
    compression_level: Option<u32>,
) -> std::io::Result<ToolOutput> {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use tar::Builder;

    let output = output.as_ref();
    let file = std::fs::File::create(output)?;
    let level = compression_level.unwrap_or(6);
    let encoder = GzEncoder::new(file, Compression::new(level));
    let mut tar = Builder::new(encoder);

    let mut file_count = 0;

    for input in inputs {
        let input = input.as_ref();

        if input.is_dir() {
            tar.append_dir_all(input.file_name().unwrap_or_default(), input)?;
            file_count += walkdir::WalkDir::new(input)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .count();
        } else if input.is_file() {
            tar.append_path_with_name(input, input.file_name().unwrap_or_default())?;
            file_count += 1;
        }
    }

    tar.into_inner()?.finish()?;

    let compressed_size = std::fs::metadata(output)?.len();

    let mut metadata = HashMap::new();
    metadata.insert("file_count".to_string(), file_count.to_string());
    metadata.insert("compressed_size".to_string(), compressed_size.to_string());

    Ok(ToolOutput {
        success: true,
        message: format!("Created {} with {} files", output.display(), file_count),
        output_paths: vec![output.to_path_buf()],
        metadata,
    })
}

/// List contents of a ZIP archive.
#[cfg(feature = "archive-core")]
pub fn list_zip_native(input: impl AsRef<Path>) -> std::io::Result<ToolOutput> {
    use zip::ZipArchive;

    let input = input.as_ref();
    let file = std::fs::File::open(input)?;
    let mut archive = ZipArchive::new(file)?;

    let mut files = Vec::new();
    let mut total_size = 0u64;
    let mut compressed_size = 0u64;

    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        files.push(file.name().to_string());
        total_size += file.size();
        compressed_size += file.compressed_size();
    }

    let mut metadata = HashMap::new();
    metadata.insert("file_count".to_string(), files.len().to_string());
    metadata.insert("total_size".to_string(), total_size.to_string());
    metadata.insert("compressed_size".to_string(), compressed_size.to_string());
    metadata.insert("files".to_string(), files.join(";"));

    Ok(ToolOutput {
        success: true,
        message: format!(
            "{}: {} files, {} bytes (compressed: {} bytes)",
            input.display(),
            files.len(),
            total_size,
            compressed_size
        ),
        output_paths: vec![input.to_path_buf()],
        metadata,
    })
}

/// Extract specific file from ZIP.
#[cfg(feature = "archive-core")]
pub fn extract_file_from_zip_native(
    archive_path: impl AsRef<Path>,
    file_name: &str,
    output: impl AsRef<Path>,
) -> std::io::Result<ToolOutput> {
    use zip::ZipArchive;

    let archive_path = archive_path.as_ref();
    let output = output.as_ref();

    let file = std::fs::File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;

    let mut zip_file = archive.by_name(file_name)?;

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut outfile = std::fs::File::create(output)?;
    std::io::copy(&mut zip_file, &mut outfile)?;

    let mut metadata = HashMap::new();
    metadata.insert("file_name".to_string(), file_name.to_string());
    metadata.insert("size".to_string(), zip_file.size().to_string());

    Ok(ToolOutput {
        success: true,
        message: format!(
            "Extracted {} from {} to {}",
            file_name,
            archive_path.display(),
            output.display()
        ),
        output_paths: vec![output.to_path_buf()],
        metadata,
    })
}

// Fallback implementations when archive-core is not enabled
#[cfg(not(feature = "archive-core"))]
pub fn extract_zip_native(
    _input: impl AsRef<Path>,
    _output_dir: impl AsRef<Path>,
) -> std::io::Result<ToolOutput> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Native archive processing requires the 'archive-core' feature",
    ))
}

#[cfg(not(feature = "archive-core"))]
pub fn create_zip_native(
    _inputs: &[impl AsRef<Path>],
    _output: impl AsRef<Path>,
    _compression_level: Option<i64>,
) -> std::io::Result<ToolOutput> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Native archive processing requires the 'archive-core' feature",
    ))
}

#[cfg(not(feature = "archive-core"))]
pub fn extract_tar_gz_native(
    _input: impl AsRef<Path>,
    _output_dir: impl AsRef<Path>,
) -> std::io::Result<ToolOutput> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Native archive processing requires the 'archive-core' feature",
    ))
}

#[cfg(not(feature = "archive-core"))]
pub fn create_tar_gz_native(
    _inputs: &[impl AsRef<Path>],
    _output: impl AsRef<Path>,
    _compression_level: Option<u32>,
) -> std::io::Result<ToolOutput> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Native archive processing requires the 'archive-core' feature",
    ))
}

#[cfg(not(feature = "archive-core"))]
pub fn list_zip_native(_input: impl AsRef<Path>) -> std::io::Result<ToolOutput> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Native archive processing requires the 'archive-core' feature",
    ))
}

#[cfg(not(feature = "archive-core"))]
pub fn extract_file_from_zip_native(
    _archive_path: impl AsRef<Path>,
    _file_name: &str,
    _output: impl AsRef<Path>,
) -> std::io::Result<ToolOutput> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Native archive processing requires the 'archive-core' feature",
    ))
}
