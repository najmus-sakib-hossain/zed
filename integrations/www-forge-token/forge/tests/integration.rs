// use std::fs;

// use assert_cmd::Command;
// use forge::chunking::cdc::ChunkConfig;
// use forge::chunking::structure_aware::uasset::chunk_uasset;
// use forge::core::repository::Repository;
// use forge::db::metadata::MetadataDb;
// use forge::store::cas::ChunkStore;
// use forge::store::compression;
// use forge::util::ignore::ForgeIgnore;
// use rand::{RngCore, SeedableRng};
// use tempfile::tempdir;

// fn write_random_file(path: &std::path::Path, size: usize, seed: u64) {
//     let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
//     let mut buf = vec![0u8; size];
//     rng.fill_bytes(&mut buf);
//     fs::write(path, buf).unwrap();
// }

// fn read_head_commit(repo_path: &std::path::Path) -> String {
//     let head_ref = fs::read_to_string(repo_path.join(".forge/HEAD")).unwrap();
//     let rel = head_ref.strip_prefix("ref: ").unwrap().trim();
//     fs::read_to_string(repo_path.join(".forge").join(rel))
//         .unwrap()
//         .trim()
//         .to_string()
// }

// #[test]
// fn test_init_creates_forge_dir() {
//     let dir = tempdir().unwrap();
//     let repo = Repository::init(dir.path()).unwrap();

//     assert!(repo.forge_dir.exists());
//     for rel in [
//         "objects/chunks",
//         "objects/packs",
//         "refs/heads",
//         "refs/remotes",
//         "manifests",
//         "dictionaries",
//     ] {
//         assert!(repo.forge_dir.join(rel).exists());
//     }

//     let head = fs::read_to_string(repo.forge_dir.join("HEAD")).unwrap();
//     assert_eq!(head, "ref: refs/heads/main\n");

//     let db = MetadataDb::open(&repo.forge_dir.join("metadata.redb"));
//     assert!(db.is_ok());
// }

// #[test]
// fn test_add_and_commit_single_file() {
//     let dir = tempdir().unwrap();
//     Repository::init(dir.path()).unwrap();

//     let file = dir.path().join("a.bin");
//     write_random_file(&file, 1024 * 1024, 42);

//     Command::cargo_bin("forge")
//         .unwrap()
//         .arg("--repo-dir")
//         .arg(dir.path())
//         .arg("add")
//         .arg("a.bin")
//         .assert()
//         .success();

//     let db = MetadataDb::open(&dir.path().join(".forge/metadata.redb")).unwrap();
//     assert_eq!(db.get_staged_files().unwrap().len(), 1);
//     drop(db);

//     Command::cargo_bin("forge")
//         .unwrap()
//         .arg("--repo-dir")
//         .arg(dir.path())
//         .arg("commit")
//         .arg("-m")
//         .arg("first")
//         .assert()
//         .success();

//     let db = MetadataDb::open(&dir.path().join(".forge/metadata.redb")).unwrap();
//     assert!(db.get_staged_files().unwrap().is_empty());
//     assert!(dir.path().join(".forge/manifests").read_dir().unwrap().next().is_some());
// }

// #[test]
// fn test_deduplication() {
//     let dir = tempdir().unwrap();
//     Repository::init(dir.path()).unwrap();

//     let data = vec![7u8; 1024 * 1024];
//     fs::write(dir.path().join("a.bin"), &data).unwrap();
//     fs::write(dir.path().join("b.bin"), &data).unwrap();

//     Command::cargo_bin("forge")
//         .unwrap()
//         .args(["--repo-dir", dir.path().to_str().unwrap(), "add", "a.bin", "b.bin"])
//         .assert()
//         .success();

//     Command::cargo_bin("forge")
//         .unwrap()
//         .args(["--repo-dir", dir.path().to_str().unwrap(), "commit", "-m", "dedup"])
//         .assert()
//         .success();

//     let store = ChunkStore::new(dir.path().join(".forge/objects/chunks"));
//     let count = store.chunk_count().unwrap();
//     assert!(count > 0);
//     assert!(count < 16);
// }

// #[test]
// fn test_checkout_restores_files() {
//     let dir = tempdir().unwrap();
//     Repository::init(dir.path()).unwrap();

//     let files = [
//         ("small.bin", 100 * 1024, 1u64),
//         ("mid.bin", 1024 * 1024, 2u64),
//         ("big.bin", 5 * 1024 * 1024, 3u64),
//     ];

//     let mut before = Vec::new();
//     for (name, size, seed) in files {
//         let p = dir.path().join(name);
//         write_random_file(&p, size, seed);
//         before.push((name.to_string(), blake3::hash(&fs::read(&p).unwrap())));
//     }

//     Command::cargo_bin("forge")
//         .unwrap()
//         .args(["--repo-dir", dir.path().to_str().unwrap(), "add", "."])
//         .assert()
//         .success();
//     Command::cargo_bin("forge")
//         .unwrap()
//         .args(["--repo-dir", dir.path().to_str().unwrap(), "commit", "-m", "snapshot"])
//         .assert()
//         .success();

//     let commit = read_head_commit(dir.path());

//     for (name, _, _) in files {
//         fs::remove_file(dir.path().join(name)).unwrap();
//     }

//     Command::cargo_bin("forge")
//         .unwrap()
//         .args(["--repo-dir", dir.path().to_str().unwrap(), "checkout", &commit])
//         .assert()
//         .success();

//     for (name, hash) in before {
//         let restored = blake3::hash(&fs::read(dir.path().join(name)).unwrap());
//         assert_eq!(restored, hash);
//     }
// }

// #[test]
// fn test_incremental_commit() {
//     let dir = tempdir().unwrap();
//     Repository::init(dir.path()).unwrap();

//     let path = dir.path().join("a.bin");
//     write_random_file(&path, 1024 * 1024, 77);

//     Command::cargo_bin("forge")
//         .unwrap()
//         .args(["--repo-dir", dir.path().to_str().unwrap(), "add", "a.bin"])
//         .assert()
//         .success();
//     Command::cargo_bin("forge")
//         .unwrap()
//         .args(["--repo-dir", dir.path().to_str().unwrap(), "commit", "-m", "c1"])
//         .assert()
//         .success();

//     let c1 = read_head_commit(dir.path());
//     let store = ChunkStore::new(dir.path().join(".forge/objects/chunks"));
//     let count1 = store.chunk_count().unwrap();

//     let mut bytes = fs::read(&path).unwrap();
//     bytes[500_000] ^= 0x42;
//     fs::write(&path, bytes).unwrap();

//     Command::cargo_bin("forge")
//         .unwrap()
//         .args(["--repo-dir", dir.path().to_str().unwrap(), "add", "a.bin"])
//         .assert()
//         .success();
//     Command::cargo_bin("forge")
//         .unwrap()
//         .args(["--repo-dir", dir.path().to_str().unwrap(), "commit", "-m", "c2"])
//         .assert()
//         .success();

//     let c2 = read_head_commit(dir.path());
//     let count2 = store.chunk_count().unwrap();
//     assert!(count2 >= count1);
//     assert!((count2 - count1) <= 3);

//     Command::cargo_bin("forge")
//         .unwrap()
//         .args(["--repo-dir", dir.path().to_str().unwrap(), "checkout", &c1])
//         .assert()
//         .success();
//     let c1_hash = blake3::hash(&fs::read(&path).unwrap());

//     Command::cargo_bin("forge")
//         .unwrap()
//         .args(["--repo-dir", dir.path().to_str().unwrap(), "checkout", &c2])
//         .assert()
//         .success();
//     let c2_hash = blake3::hash(&fs::read(&path).unwrap());

//     assert_ne!(c1_hash, c2_hash);
// }

// #[test]
// fn test_status_shows_changes() {
//     let dir = tempdir().unwrap();
//     Repository::init(dir.path()).unwrap();
//     fs::write(dir.path().join("f.txt"), b"hello").unwrap();

//     Command::cargo_bin("forge")
//         .unwrap()
//         .args(["--repo-dir", dir.path().to_str().unwrap(), "add", "f.txt"])
//         .assert()
//         .success();
//     Command::cargo_bin("forge")
//         .unwrap()
//         .args(["--repo-dir", dir.path().to_str().unwrap(), "commit", "-m", "base"])
//         .assert()
//         .success();

//     fs::write(dir.path().join("f.txt"), b"hello world").unwrap();
//     fs::write(dir.path().join("new.txt"), b"new").unwrap();

//     Command::cargo_bin("forge")
//         .unwrap()
//         .args(["--repo-dir", dir.path().to_str().unwrap(), "status"])
//         .assert()
//         .success()
//         .stdout(predicates::str::contains("M modified f.txt"))
//         .stdout(predicates::str::contains("? untracked new.txt"));
// }

// #[test]
// fn test_log_shows_history() {
//     let dir = tempdir().unwrap();
//     Repository::init(dir.path()).unwrap();
//     fs::write(dir.path().join("f.txt"), b"v1").unwrap();

//     Command::cargo_bin("forge")
//         .unwrap()
//         .args(["--repo-dir", dir.path().to_str().unwrap(), "add", "f.txt"])
//         .assert()
//         .success();
//     Command::cargo_bin("forge")
//         .unwrap()
//         .args(["--repo-dir", dir.path().to_str().unwrap(), "commit", "-m", "first"])
//         .assert()
//         .success();

//     fs::write(dir.path().join("f.txt"), b"v2").unwrap();
//     Command::cargo_bin("forge")
//         .unwrap()
//         .args(["--repo-dir", dir.path().to_str().unwrap(), "add", "f.txt"])
//         .assert()
//         .success();
//     Command::cargo_bin("forge")
//         .unwrap()
//         .args(["--repo-dir", dir.path().to_str().unwrap(), "commit", "-m", "second"])
//         .assert()
//         .success();

//     Command::cargo_bin("forge")
//         .unwrap()
//         .args(["--repo-dir", dir.path().to_str().unwrap(), "log", "-n", "10"])
//         .assert()
//         .success()
//         .stdout(predicates::str::contains("second"))
//         .stdout(predicates::str::contains("first"));
// }

// #[test]
// fn test_structure_aware_uasset_chunking() {
//     let mut data = Vec::new();
//     data.extend_from_slice(&[0xC1, 0x83, 0x2A, 0x9E]);
//     data.extend_from_slice(&[0u8; 20]);
//     data.extend_from_slice(&(1024u32).to_le_bytes());
//     data.extend_from_slice(&vec![1u8; 1024]);
//     data.extend_from_slice(&vec![2u8; 1024 * 1024]);

//     let config = ChunkConfig::default();
//     let chunks1 = chunk_uasset(&data, &config);
//     assert!(!chunks1.is_empty());
//     assert_eq!(chunks1[0].offset, 0);
//     assert_eq!(chunks1[0].length, 1024);

//     let mut modified = data.clone();
//     modified[100] ^= 1;
//     let chunks2 = chunk_uasset(&modified, &config);

//     let bulk1: Vec<_> = chunks1.iter().skip(1).map(|c| c.hash).collect();
//     let bulk2: Vec<_> = chunks2.iter().skip(1).map(|c| c.hash).collect();
//     assert_eq!(bulk1, bulk2);
// }

// #[test]
// fn test_compression_roundtrip() {
//     let mut data = Vec::with_capacity(1024 * 1024);
//     for i in 0..(1024 * 1024) {
//         data.push((i % 17) as u8);
//     }

//     let compressed = compression::compress(&data, 8).unwrap();
//     assert!(compressed.len() < data.len());
//     let restored = compression::decompress(&compressed).unwrap();
//     assert_eq!(restored, data);
// }

// #[test]
// fn test_forgeignore() {
//     let dir = tempdir().unwrap();
//     fs::write(dir.path().join(".forgeignore"), "*.tmp\nbuild/\n").unwrap();

//     let ignore = ForgeIgnore::load(dir.path());
//     assert!(ignore.is_ignored(&dir.path().join("foo.tmp")));
//     assert!(ignore.is_ignored(&dir.path().join("build/output.bin")));
//     assert!(!ignore.is_ignored(&dir.path().join("foo.png")));
//     assert!(!ignore.is_ignored(&dir.path().join("src/main.rs")));
// }
