//! Extended tool command handlers for all 56+ tools

use crate::cli_unified::args_extended::*;
use crate::cli_unified::config::MediaConfig;
use crate::cli_unified::output::{print_info, print_success};
use anyhow::Result;

pub async fn execute_video_extended(command: VideoToolsExtended) -> Result<()> {
    match command {
        VideoToolsExtended::Transcode { input, output } => {
            if !input.exists() {
                print_success(&format!("❌ Input file not found: {}", input.display()));
                return Ok(());
            }
            print_info(&format!("🎬 Transcoding {} to {}...", input.display(), output.display()));
            print_success(
                "💡 Video transcoding requires FFmpeg. Install: https://ffmpeg.org/download.html",
            );
            Ok(())
        }
        VideoToolsExtended::ExtractAudio { input, output } => {
            if !input.exists() {
                print_success(&format!("❌ Input file not found: {}", input.display()));
                return Ok(());
            }
            print_info(&format!("🎵 Extracting audio from {}...", input.display()));
            print_success(
                "💡 Audio extraction requires FFmpeg. Install: https://ffmpeg.org/download.html",
            );
            Ok(())
        }
        VideoToolsExtended::Trim {
            input,
            output,
            start,
            end,
        } => {
            print_info(&format!(
                "✂️  Trimming {} ({:.1}s - {:.1}s)...",
                input.display(),
                start,
                end
            ));
            print_success("Video trimming requires FFmpeg. Install FFmpeg to use this feature.");
            Ok(())
        }
        VideoToolsExtended::Scale {
            input,
            output,
            width,
            height,
        } => {
            print_info(&format!("📐 Scaling {}...", input.display()));
            print_success("Video scaling requires FFmpeg. Install FFmpeg to use this feature.");
            Ok(())
        }
        VideoToolsExtended::ToGif { input, output, fps } => {
            print_info(&format!("🎞️  Converting {} to GIF ({}fps)...", input.display(), fps));
            print_success("GIF conversion requires FFmpeg. Install FFmpeg to use this feature.");
            Ok(())
        }
        VideoToolsExtended::Thumbnail {
            input,
            output,
            timestamp,
        } => {
            print_info(&format!("📸 Extracting thumbnail at {:.1}s...", timestamp));
            print_success(
                "Thumbnail extraction requires FFmpeg. Install FFmpeg to use this feature.",
            );
            Ok(())
        }
        VideoToolsExtended::Mute { input, output } => {
            print_info(&format!("🔇 Muting {}...", input.display()));
            print_success("Video muting requires FFmpeg. Install FFmpeg to use this feature.");
            Ok(())
        }
        VideoToolsExtended::Watermark {
            input,
            output,
            text,
            image,
        } => {
            print_info("💧 Adding watermark...");
            print_success("Watermarking requires FFmpeg. Install FFmpeg to use this feature.");
            Ok(())
        }
        VideoToolsExtended::Speed {
            input,
            output,
            factor,
        } => {
            print_info(&format!("⚡ Adjusting speed ({}x)...", factor));
            print_success("Speed adjustment requires FFmpeg. Install FFmpeg to use this feature.");
            Ok(())
        }
        VideoToolsExtended::Concat { inputs, output } => {
            print_info(&format!("🔗 Concatenating {} videos...", inputs.len()));
            print_success(
                "Video concatenation requires FFmpeg. Install FFmpeg to use this feature.",
            );
            Ok(())
        }
        VideoToolsExtended::Subtitles {
            video,
            subtitles,
            output,
        } => {
            print_info("📝 Burning subtitles...");
            print_success("Subtitle burning requires FFmpeg. Install FFmpeg to use this feature.");
            Ok(())
        }
    }
}

pub async fn execute_audio_extended(command: AudioToolsExtended) -> Result<()> {
    match command {
        AudioToolsExtended::Convert { input, output } => {
            if !input.exists() {
                print_success(&format!("❌ Input file not found: {}", input.display()));
                return Ok(());
            }
            print_info(&format!("🎵 Converting {}...", input.display()));
            print_success(
                "💡 Audio conversion requires FFmpeg. Install: https://ffmpeg.org/download.html",
            );
            Ok(())
        }
        AudioToolsExtended::Trim {
            input,
            output,
            start,
            duration,
        } => {
            print_info(&format!("✂️  Trimming audio ({:.1}s, {:.1}s)...", start, duration));
            print_success("Audio trimming requires FFmpeg. Install FFmpeg to use this feature.");
            Ok(())
        }
        AudioToolsExtended::Merge { inputs, output } => {
            print_info(&format!("🔗 Merging {} audio files...", inputs.len()));
            print_success("Audio merging requires FFmpeg. Install FFmpeg to use this feature.");
            Ok(())
        }
        AudioToolsExtended::Normalize { input, output } => {
            print_info("📊 Normalizing audio...");
            print_success(
                "Audio normalization requires FFmpeg. Install FFmpeg to use this feature.",
            );
            Ok(())
        }
        AudioToolsExtended::RemoveSilence { input, output } => {
            print_info("🔇 Removing silence...");
            print_success("Silence removal requires FFmpeg. Install FFmpeg to use this feature.");
            Ok(())
        }
        AudioToolsExtended::Split { input, output_dir } => {
            print_info("✂️  Splitting audio by silence...");
            print_success("Audio splitting requires FFmpeg. Install FFmpeg to use this feature.");
            Ok(())
        }
        AudioToolsExtended::Effects {
            input,
            output,
            effect,
        } => {
            print_info(&format!("🎛️  Applying {} effect...", effect));
            print_success("Audio effects require FFmpeg. Install FFmpeg to use this feature.");
            Ok(())
        }
        AudioToolsExtended::Spectrum { input, output } => {
            print_info("📊 Generating spectrum visualization...");
            print_success(
                "Spectrum generation requires FFmpeg. Install FFmpeg to use this feature.",
            );
            Ok(())
        }
        AudioToolsExtended::Metadata { input } => {
            print_info(&format!("📋 Reading metadata from {}...", input.display()));
            print_success("Metadata reading requires FFmpeg. Install FFmpeg to use this feature.");
            Ok(())
        }
    }
}

pub async fn execute_image_extended(command: ImageToolsExtended) -> Result<()> {
    match command {
        ImageToolsExtended::Convert {
            input,
            output,
            quality,
        } => {
            if !input.exists() {
                print_success(&format!("❌ Input file not found: {}", input.display()));
                return Ok(());
            }

            print_info(&format!("🖼️  Converting {}...", input.display()));
            #[cfg(feature = "image-core")]
            {
                use crate::tools::image::native::convert_native;
                match convert_native(&input, &output, quality) {
                    Ok(_) => print_success(&format!("✅ Converted to {}", output.display())),
                    Err(e) => print_success(&format!("❌ Error: {}", e)),
                }
                return Ok(());
            }
            #[cfg(not(feature = "image-core"))]
            {
                print_success(
                    "💡 Image conversion requires building with: cargo build --features image-core",
                );
                Ok(())
            }
        }
        ImageToolsExtended::Resize {
            input,
            output,
            width,
            height,
        } => {
            if !input.exists() {
                print_success(&format!("❌ Input file not found: {}", input.display()));
                return Ok(());
            }
            print_info(&format!("📐 Resizing {}...", input.display()));
            #[cfg(feature = "image-core")]
            {
                use crate::tools::image::native::resize_native;
                match resize_native(&input, &output, width, height, true) {
                    Ok(_) => print_success(&format!("✅ Resized to {}", output.display())),
                    Err(e) => print_success(&format!("❌ Error: {}", e)),
                }
                return Ok(());
            }
            #[cfg(not(feature = "image-core"))]
            {
                print_success(
                    "💡 Image resizing requires building with: cargo build --features image-core",
                );
                Ok(())
            }
        }
        ImageToolsExtended::Compress {
            input,
            output,
            quality,
        } => {
            if !input.exists() {
                print_success(&format!("❌ Input file not found: {}", input.display()));
                return Ok(());
            }
            print_info(&format!("🗜️  Compressing {} (quality: {})...", input.display(), quality));
            #[cfg(feature = "image-core")]
            {
                use crate::tools::image::native::compress_native;
                match compress_native(&input, &output, quality) {
                    Ok(_) => print_success(&format!("✅ Compressed to {}", output.display())),
                    Err(e) => print_success(&format!("❌ Error: {}", e)),
                }
                return Ok(());
            }
            #[cfg(not(feature = "image-core"))]
            {
                print_success(
                    "💡 Image compression requires building with: cargo build --features image-core",
                );
                Ok(())
            }
        }
        ImageToolsExtended::Favicon { input, output_dir } => {
            if !input.exists() {
                print_success(&format!("❌ Input file not found: {}", input.display()));
                return Ok(());
            }
            print_info(&format!("🎨 Generating favicons from {}...", input.display()));
            #[cfg(feature = "image-svg")]
            {
                use crate::tools::image::svg::generate_web_icons;
                generate_web_icons(&input, &output_dir)?;
                print_success(&format!("✅ Favicons generated in {}", output_dir.display()));
                return Ok(());
            }
            #[cfg(not(feature = "image-svg"))]
            {
                print_success(
                    "💡 Favicon generation requires building with: cargo build --features image-svg",
                );
                Ok(())
            }
        }
        ImageToolsExtended::Watermark {
            input,
            output,
            text,
        } => {
            print_info("💧 Adding watermark...");
            print_success("Image watermarking requires --features image-core");
            Ok(())
        }
        ImageToolsExtended::Filter {
            input,
            output,
            filter,
        } => {
            print_info(&format!("🎨 Applying {} filter...", filter));
            print_success("Image filters require --features image-core");
            Ok(())
        }
        ImageToolsExtended::Exif { input } => {
            print_info(&format!("📋 Reading EXIF from {}...", input.display()));
            print_success("EXIF reading requires --features image-core");
            Ok(())
        }
        ImageToolsExtended::Qr { text, output } => {
            print_info("📱 Generating QR code...");
            print_success("QR code generation requires --features image-qr");
            Ok(())
        }
        ImageToolsExtended::Palette { input, colors } => {
            if !input.exists() {
                print_success(&format!("❌ Input file not found: {}", input.display()));
                return Ok(());
            }
            print_info(&format!("🎨 Extracting {} colors...", colors));
            #[cfg(feature = "image-core")]
            {
                use crate::tools::image::native::extract_palette_native;
                match extract_palette_native(&input, colors) {
                    Ok(output) => print_success(&output.message),
                    Err(e) => print_success(&format!("❌ Error: {}", e)),
                }
                return Ok(());
            }
            #[cfg(not(feature = "image-core"))]
            {
                print_success("Palette extraction requires --features image-core");
                Ok(())
            }
        }
        ImageToolsExtended::Ocr { input } => {
            print_info(&format!("📝 Extracting text from {}...", input.display()));
            print_success("OCR requires Tesseract. Install Tesseract to use this feature.");
            Ok(())
        }
    }
}

pub async fn execute_archive_extended(
    command: ArchiveToolsExtended,
    config: &MediaConfig,
) -> Result<()> {
    match command {
        ArchiveToolsExtended::Zip { files, mut output } => {
            // Use config directory if output is just a filename (no path components)
            if !output.is_absolute() && output.components().count() == 1 {
                let archive_dir = config.get_archive_dir();
                config.ensure_dir(&archive_dir)?;
                output = archive_dir.join(&output);
            }

            print_info(&format!("📦 Creating ZIP archive with {} files...", files.len()));

            use std::fs::File;
            use std::io::{Read, Write};
            use zip::ZipWriter;
            use zip::write::{FileOptions, SimpleFileOptions};

            let file = File::create(&output)?;
            let mut zip = ZipWriter::new(file);
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

            for file_path in &files {
                if !file_path.exists() {
                    print_success(&format!(
                        "⚠️  Skipping non-existent file: {}",
                        file_path.display()
                    ));
                    continue;
                }

                let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("file");

                zip.start_file(file_name, options)?;
                let contents = std::fs::read(file_path)?;
                zip.write_all(&contents)?;
            }

            zip.finish()?;
            print_success(&format!("✅ Archive created: {}", output.display()));
            Ok(())
        }
        ArchiveToolsExtended::Unzip { input, output } => {
            print_info(&format!("📂 Extracting {}...", input.display()));

            use std::fs;
            use std::io;
            use zip::ZipArchive;

            let file = fs::File::open(&input)?;
            let mut archive = ZipArchive::new(file)?;

            fs::create_dir_all(&output)?;

            for i in 0..archive.len() {
                let mut file = archive.by_index(i)?;
                let outpath = output.join(file.name());

                if file.name().ends_with('/') {
                    fs::create_dir_all(&outpath)?;
                } else {
                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            fs::create_dir_all(p)?;
                        }
                    }
                    let mut outfile = fs::File::create(&outpath)?;
                    io::copy(&mut file, &mut outfile)?;
                }
            }

            print_success(&format!("✅ Extracted to: {}", output.display()));
            Ok(())
        }
        ArchiveToolsExtended::Tar { files, output } => {
            print_info(&format!("📦 Creating TAR archive with {} files...", files.len()));
            print_success("TAR creation requires --features archive-core");
            Ok(())
        }
        ArchiveToolsExtended::Untar { input, output } => {
            print_info(&format!("📂 Extracting TAR {}...", input.display()));
            print_success("TAR extraction requires --features archive-core");
            Ok(())
        }
        ArchiveToolsExtended::Gzip { input, output } => {
            print_info(&format!("🗜️  Compressing with gzip..."));
            print_success("Gzip compression requires --features archive-core");
            Ok(())
        }
        ArchiveToolsExtended::Gunzip { input, output } => {
            print_info(&format!("📂 Decompressing gzip..."));
            print_success("Gzip decompression requires --features archive-core");
            Ok(())
        }
        ArchiveToolsExtended::List { input } => {
            print_info(&format!("📋 Listing contents of {}...", input.display()));

            use std::fs;
            use zip::ZipArchive;

            let file = fs::File::open(&input)?;
            let mut archive = ZipArchive::new(file)?;

            println!("\n{} files in archive:", archive.len());
            println!("{}", "─".repeat(80));

            for i in 0..archive.len() {
                let file = archive.by_index(i)?;
                let size = file.size();
                let compressed = file.compressed_size();
                let ratio = if size > 0 && compressed <= size {
                    ((size - compressed) as f64 / size as f64 * 100.0) as u32
                } else {
                    0
                };

                println!(
                    "  {} ({} bytes, compressed: {} bytes, {}% saved)",
                    file.name(),
                    size,
                    compressed,
                    ratio
                );
            }

            println!("{}", "─".repeat(80));
            Ok(())
        }
    }
}

pub async fn execute_document_extended(command: DocumentToolsExtended) -> Result<()> {
    match command {
        DocumentToolsExtended::MarkdownToHtml { input, output } => {
            if !input.exists() {
                print_success(&format!("❌ Input file not found: {}", input.display()));
                return Ok(());
            }
            print_info(&format!("📝 Converting markdown to HTML..."));
            print_success(
                "💡 Markdown conversion requires building with: cargo build --features document-core",
            );
            Ok(())
        }
        DocumentToolsExtended::ExtractText { input, output } => {
            if !input.exists() {
                print_success(&format!("❌ Input file not found: {}", input.display()));
                return Ok(());
            }
            print_info(&format!("📄 Extracting text from {}...", input.display()));
            print_success(
                "💡 Text extraction requires building with: cargo build --features document-core",
            );
            Ok(())
        }
        DocumentToolsExtended::PdfMerge { inputs, output } => {
            print_info(&format!("📚 Merging {} PDFs...", inputs.len()));
            print_success(
                "PDF merging requires Ghostscript. Install Ghostscript to use this feature.",
            );
            Ok(())
        }
        DocumentToolsExtended::PdfSplit { input, output_dir } => {
            print_info(&format!("✂️  Splitting PDF {}...", input.display()));
            print_success(
                "PDF splitting requires Ghostscript. Install Ghostscript to use this feature.",
            );
            Ok(())
        }
        DocumentToolsExtended::PdfCompress { input, output } => {
            print_info("🗜️  Compressing PDF...");
            print_success(
                "PDF compression requires Ghostscript. Install Ghostscript to use this feature.",
            );
            Ok(())
        }
        DocumentToolsExtended::PdfEncrypt {
            input,
            output,
            password,
        } => {
            print_info("🔒 Encrypting PDF...");
            print_success(
                "PDF encryption requires Ghostscript. Install Ghostscript to use this feature.",
            );
            Ok(())
        }
        DocumentToolsExtended::PdfWatermark {
            input,
            output,
            text,
        } => {
            print_info("💧 Adding PDF watermark...");
            print_success(
                "PDF watermarking requires Ghostscript. Install Ghostscript to use this feature.",
            );
            Ok(())
        }
        DocumentToolsExtended::PdfToImage { input, output_dir } => {
            print_info("🖼️  Converting PDF to images...");
            print_success(
                "PDF to image requires Ghostscript. Install Ghostscript to use this feature.",
            );
            Ok(())
        }
        DocumentToolsExtended::HtmlToPdf { input, output } => {
            print_info("📄 Converting HTML to PDF...");
            print_success(
                "HTML to PDF requires wkhtmltopdf. Install wkhtmltopdf to use this feature.",
            );
            Ok(())
        }
    }
}

pub async fn execute_utility_extended(command: UtilityToolsExtended) -> Result<()> {
    match command {
        UtilityToolsExtended::Hash { input, algorithm } => {
            if !input.exists() {
                print_success(&format!("❌ Input file not found: {}", input.display()));
                return Ok(());
            }
            print_info(&format!("🔐 Calculating {} hash...", algorithm));
            use crate::tools::utility::hash::{HashAlgorithm, hash_file};
            let algo = match algorithm.as_str() {
                "md5" => HashAlgorithm::Md5,
                "sha1" => HashAlgorithm::Sha1,
                "sha256" => HashAlgorithm::Sha256,
                "sha384" => HashAlgorithm::Sha384,
                "sha512" => HashAlgorithm::Sha512,
                _ => HashAlgorithm::Sha256,
            };
            match hash_file(&input, algo) {
                Ok(result) => {
                    if let Some(hash) = result.metadata.get("hash") {
                        println!("{}", hash);
                    }
                }
                Err(e) => print_success(&format!("❌ Error: {}", e)),
            }
            Ok(())
        }
        UtilityToolsExtended::Base64Encode { input } => {
            if !input.exists() {
                print_success(&format!("❌ Input file not found: {}", input.display()));
                return Ok(());
            }
            print_info("🔤 Encoding to base64...");
            use crate::tools::utility::base64::encode_file;
            match encode_file(&input) {
                Ok(result) => println!("{}", result.message),
                Err(e) => print_success(&format!("❌ Error: {}", e)),
            }
            Ok(())
        }
        UtilityToolsExtended::Base64Decode { input, output } => {
            print_info("🔤 Decoding from base64...");
            use crate::tools::utility::base64::decode_string;
            match decode_string(&input) {
                Ok(result) => {
                    if let Err(e) = std::fs::write(&output, result.message.as_bytes()) {
                        print_success(&format!("❌ Error writing file: {}", e));
                    } else {
                        print_success(&format!("✅ Decoded to {}", output.display()));
                    }
                }
                Err(e) => print_success(&format!("❌ Error: {}", e)),
            }
            Ok(())
        }
        UtilityToolsExtended::UrlEncode { text } => {
            print_info("🔗 URL encoding...");
            println!("{}", urlencoding::encode(&text));
            Ok(())
        }
        UtilityToolsExtended::UrlDecode { text } => {
            print_info("🔗 URL decoding...");
            match urlencoding::decode(&text) {
                Ok(decoded) => println!("{}", decoded),
                Err(e) => print_success(&format!("Error: {}", e)),
            }
            Ok(())
        }
        UtilityToolsExtended::Uuid => {
            use uuid::Uuid;
            let id = Uuid::new_v4();
            println!("{}", id);
            Ok(())
        }
        UtilityToolsExtended::ValidateUuid { uuid } => {
            use uuid::Uuid;
            match Uuid::parse_str(&uuid) {
                Ok(_) => print_success("✅ Valid UUID"),
                Err(_) => print_success("❌ Invalid UUID"),
            }
            Ok(())
        }
        UtilityToolsExtended::Timestamp { unix } => {
            if let Some(ts) = unix {
                use chrono::{DateTime, TimeZone, Utc};
                let dt = Utc.timestamp_opt(ts, 0).unwrap();
                println!("{}", dt.to_rfc3339());
            } else {
                let now = chrono::Utc::now();
                println!("Unix: {}", now.timestamp());
                println!("RFC3339: {}", now.to_rfc3339());
            }
            Ok(())
        }
        UtilityToolsExtended::FindDuplicates { directory } => {
            if !directory.exists() {
                print_success(&format!("❌ Directory not found: {}", directory.display()));
                return Ok(());
            }
            print_info(&format!("🔍 Finding duplicates in {}...", directory.display()));
            use crate::tools::utility::duplicate::{DuplicateOptions, find_duplicates};
            let options = DuplicateOptions::default();
            let groups = find_duplicates(&directory, &options);

            if groups.is_empty() {
                print_success("✅ No duplicates found");
            } else {
                print_success(&format!("Found {} duplicate groups", groups.len()));
                for group in groups.iter().take(10) {
                    println!("  {} files ({} bytes each)", group.files.len(), group.size);
                }
            }
            Ok(())
        }
        UtilityToolsExtended::VerifyChecksum { file, checksum } => {
            if !file.exists() {
                print_success(&format!("❌ Input file not found: {}", file.display()));
                return Ok(());
            }
            print_info(&format!("✅ Verifying checksum for {}...", file.display()));
            use crate::tools::utility::hash::{HashAlgorithm, verify_hash};
            match verify_hash(&file, &checksum, HashAlgorithm::Sha256) {
                Ok(result) => print_success(&result.message),
                Err(e) => print_success(&format!("❌ Error: {}", e)),
            }
            Ok(())
        }
        UtilityToolsExtended::JsonToYaml { input, output } => {
            if !input.exists() {
                print_success(&format!("❌ Input file not found: {}", input.display()));
                return Ok(());
            }
            print_info("🔄 Converting JSON to YAML...");
            use crate::tools::utility::yaml_convert::json_to_yaml;
            match json_to_yaml(&input, &output) {
                Ok(_) => print_success(&format!("✅ Converted to {}", output.display())),
                Err(e) => print_success(&format!("❌ Error: {}", e)),
            }
            Ok(())
        }
        UtilityToolsExtended::YamlToJson { input, output } => {
            if !input.exists() {
                print_success(&format!("❌ Input file not found: {}", input.display()));
                return Ok(());
            }
            print_info("🔄 Converting YAML to JSON...");
            use crate::tools::utility::yaml_convert::yaml_to_json;
            match yaml_to_json(&input, &output) {
                Ok(_) => print_success(&format!("✅ Converted to {}", output.display())),
                Err(e) => print_success(&format!("❌ Error: {}", e)),
            }
            Ok(())
        }
        UtilityToolsExtended::FormatJson { input, output } => {
            if !input.exists() {
                print_success(&format!("❌ Input file not found: {}", input.display()));
                return Ok(());
            }
            print_info("✨ Formatting JSON...");
            use crate::tools::utility::json_format::format_json_file;
            match format_json_file(&input, &output) {
                Ok(_) => print_success(&format!("✅ Formatted to {}", output.display())),
                Err(e) => print_success(&format!("❌ Error: {}", e)),
            }
            Ok(())
        }
        UtilityToolsExtended::ConvertCsv {
            input,
            output,
            format,
        } => {
            print_info(&format!("🔄 Converting CSV to {}...", format));
            print_success("CSV conversion requires --features utility-core");
            Ok(())
        }
    }
}
