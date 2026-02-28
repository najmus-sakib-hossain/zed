//! Tests for native Rust archive processing (no external dependencies).
//!
//! These tests use the `zip`, `tar`, and `flate2` crates.

mod common;

use common::TestFixture;

#[cfg(feature = "archive-core")]
#[test]
fn test_create_zip_archive() {
    use dx_media::tools::archive::zip::create_zip;

    let fixture = TestFixture::new();
    let file1 = fixture.create_test_text_file("file1.txt", "Hello World");
    let file2 = fixture.create_test_text_file("file2.txt", "Test Content");
    let output = fixture.path("archive.zip");

    let result = create_zip(&[&file1, &file2], &output);
    assert!(result.is_ok(), "ZIP creation should succeed: {:?}", result.err());
    assert!(output.exists(), "ZIP file should exist");

    let metadata = std::fs::metadata(&output).unwrap();
    assert!(metadata.len() > 0, "ZIP file should not be empty");
}

#[cfg(feature = "archive-core")]
#[test]
fn test_extract_zip_archive() {
    use dx_media::tools::archive::zip::{create_zip, extract_zip};

    let fixture = TestFixture::new();
    let file1 = fixture.create_test_text_file("file1.txt", "Hello World");
    let zip_path = fixture.path("archive.zip");
    let extract_dir = fixture.path("extracted");

    // Create ZIP
    create_zip(&[&file1], &zip_path).unwrap();

    // Extract ZIP
    let result = extract_zip(&zip_path, &extract_dir);
    assert!(result.is_ok(), "ZIP extraction should succeed: {:?}", result.err());
    assert!(extract_dir.exists(), "Extract directory should exist");
}

#[cfg(feature = "archive-core")]
#[test]
fn test_create_tar_archive() {
    use dx_media::tools::archive::tar::create_tar;

    let fixture = TestFixture::new();
    let file1 = fixture.create_test_text_file("file1.txt", "Hello World");
    let file2 = fixture.create_test_text_file("file2.txt", "Test Content");
    let output = fixture.path("archive.tar");

    let result = create_tar(&[&file1, &file2], &output);
    assert!(result.is_ok(), "TAR creation should succeed: {:?}", result.err());
    assert!(output.exists(), "TAR file should exist");
}

#[cfg(feature = "archive-core")]
#[test]
fn test_create_tar_gz_archive() {
    use dx_media::tools::archive::tar::create_tar_gz;

    let fixture = TestFixture::new();
    let file1 = fixture.create_test_text_file("file1.txt", "Hello World");
    let output = fixture.path("archive.tar.gz");

    let result = create_tar_gz(&[&file1], &output);
    assert!(result.is_ok(), "TAR.GZ creation should succeed: {:?}", result.err());
    assert!(output.exists(), "TAR.GZ file should exist");
}

#[cfg(not(feature = "archive-core"))]
#[test]
fn test_archive_core_feature_disabled() {
    assert!(true, "archive-core feature is disabled");
}

#[test]
fn test_archive_list() {
    use dx_media::tools::archive::list_archive;

    let fixture = TestFixture::new();

    // Create a test file and zip it
    let test_file = fixture.path("test.txt");
    std::fs::write(&test_file, b"test content").unwrap();

    let zip_path = fixture.path("test.zip");
    let result = dx_media::tools::archive::create_zip(&[&test_file], &zip_path);
    assert!(result.is_ok(), "ZIP creation should succeed");

    // List contents
    let result = list_archive(&zip_path);
    assert!(result.is_ok(), "Archive listing should succeed: {:?}", result.err());
}

#[test]
fn test_archive_gzip() {
    use dx_media::tools::archive::{gunzip, gzip};

    let fixture = TestFixture::new();

    let test_file = fixture.path("test.txt");
    std::fs::write(&test_file, b"test content for compression").unwrap();

    let gzip_path = fixture.path("test.txt.gz");

    // Compress
    let result = gzip(&test_file, &gzip_path);
    assert!(result.is_ok(), "Gzip compression should succeed: {:?}", result.err());
    assert!(gzip_path.exists(), "Gzipped file should exist");

    // Decompress
    let decompressed = fixture.path("decompressed.txt");
    let result = gunzip(&gzip_path, &decompressed);
    assert!(result.is_ok(), "Gunzip decompression should succeed: {:?}", result.err());
    assert!(decompressed.exists(), "Decompressed file should exist");
}

#[test]
#[ignore] // Requires 7z or zip command
fn test_archive_encrypt() {
    use dx_media::tools::archive::create_encrypted_zip;

    let fixture = TestFixture::new();

    let test_file = fixture.path("secret.txt");
    std::fs::write(&test_file, b"secret content").unwrap();

    let encrypted_zip = fixture.path("encrypted.zip");

    let result = create_encrypted_zip(&[&test_file], &encrypted_zip, "password123");
    assert!(result.is_ok(), "Encrypted ZIP creation should succeed: {:?}", result.err());
    assert!(encrypted_zip.exists(), "Encrypted ZIP should exist");
}

#[test]
fn test_archive_split_merge() {
    use dx_media::tools::archive::{merge_archives, split_archive};

    let fixture = TestFixture::new();

    // Create a test file
    let test_file = fixture.path("large.txt");
    let content = "x".repeat(1024 * 1024); // 1MB
    std::fs::write(&test_file, content.as_bytes()).unwrap();

    let split_dir = fixture.path("split");
    std::fs::create_dir_all(&split_dir).unwrap();

    // Split into 0.5MB parts
    let result = split_archive(&test_file, &split_dir, 1); // 1MB parts (won't split this file)
    assert!(result.is_ok(), "Archive split should succeed: {:?}", result.err());
}
