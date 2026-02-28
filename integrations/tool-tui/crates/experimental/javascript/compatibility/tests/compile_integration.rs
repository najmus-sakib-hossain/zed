//! Integration tests for dx-compat-compile module.
//!
//! These tests verify the compile module functionality including:
//! - Asset embedding and compression
//! - Bundle serialization/deserialization
//! - Cross-platform target support
//! - Runtime asset extraction

use dx_compat_compile::*;
use std::str::FromStr;
use tempfile::TempDir;

// ============================================================================
// Target Tests
// ============================================================================

#[test]
fn test_target_all_variants() {
    let targets = [
        Target::LinuxX64,
        Target::LinuxArm64,
        Target::MacosX64,
        Target::MacosArm64,
        Target::WindowsX64,
    ];

    for target in &targets {
        // Each target should have a valid triple
        let triple = target.triple();
        assert!(!triple.is_empty());
        assert!(triple.contains('-'));

        // Display should work
        let display = format!("{}", target);
        assert_eq!(display, triple);
    }
}

#[test]
fn test_target_exe_extensions() {
    // Unix targets should have no extension
    assert_eq!(Target::LinuxX64.exe_extension(), "");
    assert_eq!(Target::LinuxArm64.exe_extension(), "");
    assert_eq!(Target::MacosX64.exe_extension(), "");
    assert_eq!(Target::MacosArm64.exe_extension(), "");

    // Windows should have .exe extension
    assert_eq!(Target::WindowsX64.exe_extension(), ".exe");
}

#[test]
fn test_target_from_str_variants() {
    // Test various string formats
    let test_cases = [
        ("linux-x64", Some(Target::LinuxX64)),
        ("linux_x64", Some(Target::LinuxX64)),
        ("x86_64-unknown-linux-gnu", Some(Target::LinuxX64)),
        ("linux-arm64", Some(Target::LinuxArm64)),
        ("aarch64-unknown-linux-gnu", Some(Target::LinuxArm64)),
        ("macos-x64", Some(Target::MacosX64)),
        ("darwin-x64", Some(Target::MacosX64)),
        ("macos-arm64", Some(Target::MacosArm64)),
        ("darwin-arm64", Some(Target::MacosArm64)),
        ("windows-x64", Some(Target::WindowsX64)),
        ("win32-x64", Some(Target::WindowsX64)),
        ("invalid-target", None),
        ("", None),
    ];

    for (input, expected) in test_cases {
        assert_eq!(Target::from_str(input).ok(), expected, "Failed for input: {}", input);
    }
}

#[test]
fn test_target_current() {
    let current = Target::current();
    // Current target should be valid
    assert!(!current.triple().is_empty());
}

#[test]
fn test_target_cross_compilation_detection() {
    let current = Target::current();

    // Current target should not require cross-compilation
    assert!(!current.requires_cross_compilation());

    // At least one other target should require cross-compilation
    let all_targets = [
        Target::LinuxX64,
        Target::LinuxArm64,
        Target::MacosX64,
        Target::MacosArm64,
        Target::WindowsX64,
    ];

    let cross_compile_count = all_targets.iter().filter(|t| t.requires_cross_compilation()).count();

    // Should have at least 4 targets requiring cross-compilation (all except current)
    assert!(cross_compile_count >= 4);
}

// ============================================================================
// Compression Tests
// ============================================================================

#[test]
fn test_compression_empty_data() {
    let data = b"";
    let compressed = compress_data(data).unwrap();
    let decompressed = decompress_data(&compressed).unwrap();
    assert_eq!(data.as_slice(), decompressed.as_slice());
}

#[test]
fn test_compression_small_data() {
    let data = b"Hello, World!";
    let compressed = compress_data(data).unwrap();
    let decompressed = decompress_data(&compressed).unwrap();
    assert_eq!(data.as_slice(), decompressed.as_slice());
}

#[test]
fn test_compression_large_data() {
    // Create 1MB of data
    let data: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();
    let compressed = compress_data(&data).unwrap();
    let decompressed = decompress_data(&compressed).unwrap();
    assert_eq!(data, decompressed);

    // Compressed should be smaller for repetitive data
    assert!(compressed.len() < data.len());
}

#[test]
fn test_compression_levels() {
    let data: Vec<u8> = (0..10 * 1024).map(|i| (i % 256) as u8).collect();

    let level1 = compress_data_with_level(&data, 1).unwrap();
    let level10 = compress_data_with_level(&data, 10).unwrap();
    let level22 = compress_data_with_level(&data, 22).unwrap();

    // All should decompress correctly
    assert_eq!(decompress_data(&level1).unwrap(), data);
    assert_eq!(decompress_data(&level10).unwrap(), data);
    assert_eq!(decompress_data(&level22).unwrap(), data);

    // Higher levels should generally produce smaller output (for compressible data)
    // Note: This isn't always true for all data, but should be for repetitive data
    assert!(level22.len() <= level1.len());
}

#[test]
fn test_compression_binary_data() {
    // Test with random-ish binary data
    let data: Vec<u8> = (0..1000).map(|i| ((i * 17 + 31) % 256) as u8).collect();
    let compressed = compress_data(&data).unwrap();
    let decompressed = decompress_data(&compressed).unwrap();
    assert_eq!(data, decompressed);
}

// ============================================================================
// EmbeddedAsset Tests
// ============================================================================

#[test]
fn test_embedded_asset_creation() {
    let data = b"Test content for asset";
    let asset = EmbeddedAsset::new("test.txt", data, "text/plain").unwrap();

    assert_eq!(asset.path, "test.txt");
    assert_eq!(asset.mime_type, "text/plain");
    assert_eq!(asset.original_size, data.len() as u64);
    assert!(!asset.hash.is_empty());
    assert_eq!(asset.hash.len(), 64); // SHA-256 hex is 64 chars
}

#[test]
fn test_embedded_asset_decompress() {
    let data = b"Test content for decompression";
    let asset = EmbeddedAsset::new("test.txt", data, "text/plain").unwrap();

    let decompressed = asset.decompress().unwrap();
    assert_eq!(data.as_slice(), decompressed.as_slice());
}

#[test]
fn test_embedded_asset_verify() {
    let data = b"Test content for verification";
    let asset = EmbeddedAsset::new("test.txt", data, "text/plain").unwrap();

    assert!(asset.verify().unwrap());
}

#[test]
fn test_embedded_asset_hash_consistency() {
    let data = b"Same content";

    let asset1 = EmbeddedAsset::new("file1.txt", data, "text/plain").unwrap();
    let asset2 = EmbeddedAsset::new("file2.txt", data, "text/plain").unwrap();

    // Same content should produce same hash
    assert_eq!(asset1.hash, asset2.hash);
}

#[test]
fn test_embedded_asset_different_content() {
    let asset1 = EmbeddedAsset::new("file.txt", b"Content A", "text/plain").unwrap();
    let asset2 = EmbeddedAsset::new("file.txt", b"Content B", "text/plain").unwrap();

    // Different content should produce different hash
    assert_ne!(asset1.hash, asset2.hash);
}

// ============================================================================
// AssetBundle Tests
// ============================================================================

#[test]
fn test_asset_bundle_creation() {
    let bundle = AssetBundle::new(Target::LinuxX64, "index.js", "my-app", "1.0.0");

    assert_eq!(bundle.version, 1);
    assert_eq!(bundle.target, Target::LinuxX64);
    assert_eq!(bundle.entry_point, "index.js");
    assert_eq!(bundle.metadata.name, "my-app");
    assert_eq!(bundle.metadata.version, "1.0.0");
    assert!(bundle.assets.is_empty());
}

#[test]
fn test_asset_bundle_add_assets() {
    let mut bundle = AssetBundle::new(Target::MacosArm64, "main.ts", "test-app", "2.0.0");

    let asset1 = EmbeddedAsset::new("file1.txt", b"Content 1", "text/plain").unwrap();
    let asset2 = EmbeddedAsset::new("file2.json", b"{}", "application/json").unwrap();

    bundle.add_asset(asset1);
    bundle.add_asset(asset2);

    assert_eq!(bundle.assets.len(), 2);
    assert!(bundle.get_asset("file1.txt").is_some());
    assert!(bundle.get_asset("file2.json").is_some());
    assert!(bundle.get_asset("nonexistent").is_none());
}

#[test]
fn test_asset_bundle_metadata_tracking() {
    let mut bundle = AssetBundle::new(Target::WindowsX64, "app.js", "app", "1.0.0");

    let data1 = b"First file content";
    let data2 = b"Second file content with more data";

    let asset1 = EmbeddedAsset::new("file1.txt", data1, "text/plain").unwrap();
    let asset2 = EmbeddedAsset::new("file2.txt", data2, "text/plain").unwrap();

    bundle.add_asset(asset1);
    bundle.add_asset(asset2);

    // Total original size should be sum of both files
    assert_eq!(bundle.metadata.total_original_size, (data1.len() + data2.len()) as u64);
}

#[test]
fn test_asset_bundle_serialization_roundtrip() {
    let mut bundle =
        AssetBundle::new(Target::LinuxArm64, "index.ts", "serialization-test", "3.0.0");

    let asset =
        EmbeddedAsset::new("data.json", b"{\"key\": \"value\"}", "application/json").unwrap();
    bundle.add_asset(asset);

    // Serialize
    let bytes = bundle.to_bytes().unwrap();
    assert!(!bytes.is_empty());

    // Deserialize
    let restored = AssetBundle::from_bytes(&bytes).unwrap();

    assert_eq!(restored.version, bundle.version);
    assert_eq!(restored.target, bundle.target);
    assert_eq!(restored.entry_point, bundle.entry_point);
    assert_eq!(restored.metadata.name, bundle.metadata.name);
    assert_eq!(restored.metadata.version, bundle.metadata.version);
    assert_eq!(restored.assets.len(), bundle.assets.len());

    // Verify asset content
    let original_asset = bundle.get_asset("data.json").unwrap();
    let restored_asset = restored.get_asset("data.json").unwrap();
    assert_eq!(original_asset.hash, restored_asset.hash);
}

#[test]
fn test_asset_bundle_compression_ratio() {
    let mut bundle = AssetBundle::new(Target::LinuxX64, "app.js", "ratio-test", "1.0.0");

    // Empty bundle should have ratio of 1.0
    assert_eq!(bundle.compression_ratio(), 1.0);

    // Add compressible content
    let repetitive_data: Vec<u8> = vec![b'A'; 10000];
    let asset = EmbeddedAsset::new("repetitive.txt", &repetitive_data, "text/plain").unwrap();
    bundle.add_asset(asset);

    // Compression ratio should be less than 1 for compressible data
    assert!(bundle.compression_ratio() < 1.0);
}

// ============================================================================
// CompileOptions Tests
// ============================================================================

#[test]
fn test_compile_options_defaults() {
    let options = CompileOptions::new("./src/index.ts");

    assert_eq!(options.entry_point.to_string_lossy(), "./src/index.ts");
    assert_eq!(options.target, Target::current());
    assert!(options.minify);
    assert!(!options.sourcemap);
    assert_eq!(options.compression_level, 3);
}

#[test]
fn test_compile_options_builder() {
    let options = CompileOptions::new("./app.js")
        .target(Target::MacosArm64)
        .output("./dist/myapp")
        .name("my-application")
        .version("2.1.0")
        .minify(false)
        .sourcemap(true)
        .compression_level(15);

    assert_eq!(options.target, Target::MacosArm64);
    assert_eq!(options.output.to_string_lossy(), "./dist/myapp");
    assert_eq!(options.name, "my-application");
    assert_eq!(options.version, "2.1.0");
    assert!(!options.minify);
    assert!(options.sourcemap);
    assert_eq!(options.compression_level, 15);
}

#[test]
fn test_compile_options_compression_level_clamping() {
    let options_low = CompileOptions::new("app.js").compression_level(-5);
    assert_eq!(options_low.compression_level, 1);

    let options_high = CompileOptions::new("app.js").compression_level(100);
    assert_eq!(options_high.compression_level, 22);
}

// ============================================================================
// Compiler Tests
// ============================================================================

#[test]
fn test_compiler_creation() {
    let compiler = Compiler::new();
    // Should create without error
    let _ = compiler;
}

#[test]
fn test_compiler_with_compression_level() {
    let compiler = Compiler::new().with_compression_level(10);
    let _ = compiler;
}

#[test]
fn test_compiler_compile_with_temp_file() {
    let temp_dir = TempDir::new().unwrap();
    let entry_path = temp_dir.path().join("index.js");

    // Create a test entry file
    std::fs::write(&entry_path, b"console.log('Hello, World!');").unwrap();

    let options = CompileOptions::new(&entry_path)
        .target(Target::LinuxX64)
        .output(temp_dir.path().join("output"))
        .name("test-app")
        .version("1.0.0");

    let compiler = Compiler::new();
    let output = compiler.compile(options).unwrap();

    assert_eq!(output.target, Target::LinuxX64);
    assert!(!output.bundle_data.is_empty());
    assert!(!output.bundle.assets.is_empty());
}

#[test]
fn test_compiler_embed_files() {
    let temp_dir = TempDir::new().unwrap();

    // Create entry file
    let entry_path = temp_dir.path().join("main.js");
    std::fs::write(&entry_path, b"// Main entry").unwrap();

    // Create additional files to embed
    let asset1_path = temp_dir.path().join("config.json");
    std::fs::write(&asset1_path, b"{\"debug\": true}").unwrap();

    let asset2_path = temp_dir.path().join("data.txt");
    std::fs::write(&asset2_path, b"Some data").unwrap();

    let options = CompileOptions::new(&entry_path).embed_files(&[&asset1_path, &asset2_path]);

    let compiler = Compiler::new();
    let output = compiler.compile(options).unwrap();

    // Should have entry + 2 embedded files
    assert!(output.bundle.assets.len() >= 3);
}

#[test]
fn test_compiler_embed_directory() {
    let temp_dir = TempDir::new().unwrap();

    // Create entry file
    let entry_path = temp_dir.path().join("app.js");
    std::fs::write(&entry_path, b"// App").unwrap();

    // Create assets directory with files
    let assets_dir = temp_dir.path().join("assets");
    std::fs::create_dir(&assets_dir).unwrap();
    std::fs::write(assets_dir.join("style.css"), b"body {}").unwrap();
    std::fs::write(assets_dir.join("script.js"), b"// JS").unwrap();

    let options = CompileOptions::new(&entry_path).embed_assets(&[&assets_dir]);

    let compiler = Compiler::new();
    let output = compiler.compile(options).unwrap();

    // Should have entry + directory contents
    assert!(output.bundle.assets.len() >= 3);
}

#[test]
fn test_compiler_missing_entry_file() {
    let options = CompileOptions::new("./nonexistent/file.js");
    let compiler = Compiler::new();
    let result = compiler.compile(options);

    assert!(result.is_err());
}

// ============================================================================
// CompiledOutput Tests
// ============================================================================

#[test]
fn test_compiled_output_stats() {
    let temp_dir = TempDir::new().unwrap();
    let entry_path = temp_dir.path().join("index.js");
    std::fs::write(&entry_path, b"console.log('test');").unwrap();

    let options = CompileOptions::new(&entry_path);
    let compiler = Compiler::new();
    let output = compiler.compile(options).unwrap();

    let stats = output.stats();
    assert!(stats.asset_count >= 1);
    assert!(stats.original_size > 0);
}

#[test]
fn test_compiled_output_write() {
    let temp_dir = TempDir::new().unwrap();
    let entry_path = temp_dir.path().join("index.js");
    std::fs::write(&entry_path, b"console.log('test');").unwrap();

    let output_path = temp_dir.path().join("dist").join("app");

    let options = CompileOptions::new(&entry_path).output(&output_path);

    let compiler = Compiler::new();
    let output = compiler.compile(options).unwrap();

    output.write().unwrap();

    // Output file should exist
    assert!(output.output_path.exists());
}

// ============================================================================
// Runtime Tests
// ============================================================================

#[test]
fn test_runtime_from_bundle() {
    let mut bundle = AssetBundle::new(Target::LinuxX64, "index.js", "runtime-test", "1.0.0");

    let entry_asset =
        EmbeddedAsset::new("index.js", b"console.log('Hello');", "application/javascript").unwrap();
    let data_asset =
        EmbeddedAsset::new("data.json", b"{\"key\": \"value\"}", "application/json").unwrap();

    bundle.add_asset(entry_asset);
    bundle.add_asset(data_asset);

    let bytes = bundle.to_bytes().unwrap();
    let runtime = Runtime::from_bytes(&bytes).unwrap();

    assert_eq!(runtime.entry_point(), "index.js");
    assert_eq!(runtime.target(), Target::LinuxX64);
    assert_eq!(runtime.metadata().name, "runtime-test");
}

#[test]
fn test_runtime_list_assets() {
    let mut bundle = AssetBundle::new(Target::MacosX64, "main.ts", "list-test", "1.0.0");

    bundle.add_asset(EmbeddedAsset::new("file1.txt", b"1", "text/plain").unwrap());
    bundle.add_asset(EmbeddedAsset::new("file2.txt", b"2", "text/plain").unwrap());
    bundle.add_asset(EmbeddedAsset::new("file3.txt", b"3", "text/plain").unwrap());

    let bytes = bundle.to_bytes().unwrap();
    let runtime = Runtime::from_bytes(&bytes).unwrap();

    let assets = runtime.list_assets();
    assert_eq!(assets.len(), 3);
}

#[test]
fn test_runtime_read_asset() {
    let mut bundle = AssetBundle::new(Target::WindowsX64, "app.js", "read-test", "1.0.0");

    let content = b"This is the file content";
    bundle.add_asset(EmbeddedAsset::new("readme.txt", content, "text/plain").unwrap());

    let bytes = bundle.to_bytes().unwrap();
    let runtime = Runtime::from_bytes(&bytes).unwrap();

    let read_content = runtime.read_asset("readme.txt").unwrap();
    assert_eq!(read_content, content);
}

#[test]
fn test_runtime_read_asset_string() {
    let mut bundle = AssetBundle::new(Target::LinuxArm64, "app.js", "string-test", "1.0.0");

    let content = "Hello, UTF-8 World! 🌍";
    bundle.add_asset(EmbeddedAsset::new("greeting.txt", content.as_bytes(), "text/plain").unwrap());

    let bytes = bundle.to_bytes().unwrap();
    let runtime = Runtime::from_bytes(&bytes).unwrap();

    let read_string = runtime.read_asset_string("greeting.txt").unwrap();
    assert_eq!(read_string, content);
}

#[test]
fn test_runtime_asset_not_found() {
    let bundle = AssetBundle::new(Target::LinuxX64, "app.js", "notfound-test", "1.0.0");

    let bytes = bundle.to_bytes().unwrap();
    let runtime = Runtime::from_bytes(&bytes).unwrap();

    let result = runtime.read_asset("nonexistent.txt");
    assert!(result.is_err());
}
