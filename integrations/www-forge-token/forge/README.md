In our codebase please create rust forge cli using latest rust crates and follow this plan!!!

```markdown


# Single Mega-Prompt for Claude Opus 4.6 in GitHub Copilot

Copy everything below the line and paste it as ONE prompt. It tells the model to generate every file in the entire project at once.

---

```
Create the entire Forge CLI project — a high-performance version control system for massive media assets (games, VFX, anime, manga). Generate ALL files listed below in full, with complete working implementations. No placeholders, no TODOs, no stubs. Every function must be fully implemented and the project must compile with `cargo build --release`.

Date: 23 Feb 2026. Use Rust 2021 edition.

## PROJECT STRUCTURE — Generate every single file listed:

forge/
├── Cargo.toml
├── build.rs
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── init.rs
│   │   ├── add.rs
│   │   ├── commit.rs
│   │   ├── status.rs
│   │   ├── log.rs
│   │   ├── diff.rs
│   │   ├── checkout.rs
│   │   ├── push.rs
│   │   ├── pull.rs
│   │   └── train_dict.rs
│   ├── core/
│   │   ├── mod.rs
│   │   ├── repository.rs
│   │   ├── manifest.rs
│   │   ├── chunk.rs
│   │   └── hash.rs
│   ├── chunking/
│   │   ├── mod.rs
│   │   ├── cdc.rs
│   │   └── structure_aware/
│   │       ├── mod.rs
│   │       ├── uasset.rs
│   │       ├── mp4.rs
│   │       ├── exr.rs
│   │       └── csp.rs
│   ├── store/
│   │   ├── mod.rs
│   │   ├── cas.rs
│   │   ├── compression.rs
│   │   └── pack.rs
│   ├── db/
│   │   ├── mod.rs
│   │   └── metadata.rs
│   ├── transport/
│   │   ├── mod.rs
│   │   ├── protocol.rs
│   │   └── quic.rs
│   └── util/
│       ├── mod.rs
│       ├── ignore.rs
│       ├── progress.rs
│       └── human.rs
├── tests/
│   └── integration.rs
└── benches/
    └── ingest.rs

## Cargo.toml

```toml
[package]
name = "forge"
version = "0.1.0"
edition = "2021"
description = "Blazing-fast version control for massive media assets"
license = "MIT OR Apache-2.0"
readme = "README.md"

[dependencies]
clap = { version = "4.5", features = ["derive", "env", "color"] }
anyhow = "1.0"
thiserror = "2.0"
blake3 = "1.5"
fastcdc = "3.1"
rkyv = { version = "0.8", features = ["validation"] }
bytecheck = "0.8"
redb = "2.1"
memmap2 = "0.9"
zstd = { version = "0.13", features = ["zstdmt"] }
tokio = { version = "1", features = ["full"] }
s2n-quic = "1.2"
num_cpus = "1.16"
hex = "0.4"
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
indicatif = "0.17"
walkdir = "2"
bytes = "1"
pathdiff = "0.2"
glob = "0.3"
mimalloc = { version = "0.1", default-features = false }
crossbeam-channel = "0.5"

[dev-dependencies]
tempfile = "3"
criterion = { version = "0.5", features = ["html_reports"] }
rand = "0.8"
assert_cmd = "2"
predicates = "3"

[[bench]]
name = "ingest"
harness = false

[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true
```

## DETAILED SPECIFICATIONS FOR EVERY FILE:

### src/main.rs
- Set mimalloc as global allocator
- Parse CLI with clap derive API
- Subcommands: init, add, commit, status, log, diff, checkout, push, pull, train-dict
- init: takes optional path (default ".")
- add: takes Vec<String> paths, --force flag
- commit: takes -m message (required)
- status: no args
- log: takes -n/--count (default 20)
- diff: takes optional path, optional --commit1 and --commit2 hex strings
- checkout: takes commit_id (hex string)
- push: takes optional remote name (default "origin")
- pull: takes optional remote name (default "origin")
- train-dict: takes --file-type (string), --samples (directory path), --output (file path)
- Global flags: --verbose, --repo-dir (override repo discovery)
- Initialize tracing-subscriber with env filter, debug if verbose else info
- Match on subcommand and call the appropriate cli module function
- Every cli function returns anyhow::Result<()>

### src/lib.rs
- pub mod cli, core, chunking, store, db, transport, util
- Re-export key types: Repository, Manifest, Commit, FileEntry, ChunkRef, ChunkStore, MetadataDb

### src/core/repository.rs
- Repository struct with root: PathBuf, forge_dir: PathBuf
- Repository::discover(start: &Path) -> Result<Self> — walk up directories looking for .forge/
- Repository::init(path: &Path) -> Result<Self> — create .forge/ directory structure:
  .forge/objects/chunks/, .forge/objects/packs/, .forge/refs/heads/, .forge/refs/remotes/,
  .forge/manifests/, .forge/dictionaries/
  Write HEAD file with "ref: refs/heads/main\n"
  Write default config TOML (chunk sizes, compression level, remote placeholder)
  Create metadata.redb with all tables initialized
- Helper methods: objects_dir(), chunk_path(hash), metadata_db_path(), config_path(), head_path()
- read_head() -> Result<Option<[u8;32]>> — parse HEAD file, resolve ref to commit hash
- update_head(commit_id: &[u8;32]) -> Result<()> — update the ref that HEAD points to
- Config struct (deserialize from TOML): chunk_min, chunk_avg, chunk_max, compression_level, dict_size, remote_url Option<String>
- read_config() -> Result<Config>

### src/core/manifest.rs
- All types derive rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug, Clone
- #[archive(check_bytes)] on all
- ChunkRef { hash: [u8;32], offset: u64, length: u32, compressed_length: u32 }
- FileEntry { path: String, size: u64, file_hash: [u8;32], chunks: Vec<ChunkRef>, mode: u32, mtime_ns: i64, file_type: FileType }
- FileType enum: Unknown, UAsset, Exr, Mp4, Csp, Png, Psd, Blend, Graphite
- FileType::detect(path: &Path, header: &[u8]) -> Self — check extension first, then magic bytes for EXR (0x762F3101), PNG (89504E47), MP4 (ftyp at offset 4), PSD (38425053), Blender (BLENDER)
- Commit { id: [u8;32], parents: Vec<[u8;32]>, files: Vec<FileEntry>, message: String, author: String, timestamp_ns: i64 }
- Implement serialize_commit(commit: &Commit) -> Result<Vec<u8>> using rkyv::to_bytes
- Implement deserialize_commit(bytes: &[u8]) -> Result<Commit> using rkyv::check_archived_root then deserialize

### src/core/chunk.rs
- ChunkData struct { hash: blake3::Hash, offset: usize, length: usize, data: Vec<u8> }
- Implement Display for ChunkData showing hash hex and size

### src/core/hash.rs
- hash_bytes(data: &[u8]) -> blake3::Hash wrapper
- hash_file(path: &Path) -> Result<blake3::Hash> — mmap the file with memmap2, hash the mapped bytes. For files < 1MB, just fs::read instead.
- hash_to_hex(hash: &blake3::Hash) -> String
- hex_to_hash(hex: &str) -> Result<blake3::Hash>

### src/chunking/mod.rs
- pub mod cdc, structure_aware
- pub fn chunk_file(data: &[u8], file_type: FileType, config: &ChunkConfig) -> Vec<ChunkResult>
  Match on file_type: UAsset→uasset::chunk, Mp4→mp4::chunk, Exr→exr::chunk, Csp→csp::chunk, _→cdc::chunk_data

### src/chunking/cdc.rs
- ChunkConfig { min_size: u32, avg_size: u32, max_size: u32 } with Default (64KB, 256KB, 1MB)
- ChunkResult { offset: usize, length: usize, hash: blake3::Hash }
- chunk_data(data: &[u8], config: &ChunkConfig) -> Vec<ChunkResult>
  Use fastcdc::v2020::FastCDC, iterate chunks, blake3 hash each, return results

### src/chunking/structure_aware/mod.rs
- pub mod uasset, mp4, exr, csp

### src/chunking/structure_aware/uasset.rs
- const UASSET_MAGIC: u32 = 0x9E2A83C1
- chunk_uasset(data: &[u8], config: &ChunkConfig) -> Vec<ChunkResult>
- Verify magic bytes at offset 0. If no match, fall back to cdc::chunk_data.
- Read header_size from offset 24 as u32 LE (clamp to data.len())
- First chunk = entire header region (0..header_size), hash it
- Remaining data (header_size..end): pass through cdc::chunk_data, adjust offsets by adding header_size
- Return combined vec

### src/chunking/structure_aware/mp4.rs
- chunk_mp4(data: &[u8], config: &ChunkConfig) -> Vec<ChunkResult>
- Walk MP4 boxes: read box_size (u32 BE at offset 0) and box_type (4 bytes at offset 4)
- Handle box_size == 1 (64-bit extended size at offset 8)
- Find "mdat" box — this contains the actual media data
- Everything before mdat = metadata region, chunk as single piece
- mdat contents: use cdc::chunk_data with adjusted offsets
- If parsing fails at any point, fall back to cdc::chunk_data
- find_box(data: &[u8], box_type: &[u8;4]) -> Option<(usize, usize)> helper that returns (offset, size)

### src/chunking/structure_aware/exr.rs
- chunk_exr(data: &[u8], config: &ChunkConfig) -> Vec<ChunkResult>
- EXR magic: 0x762F3101 at offset 0
- EXR has a header followed by scanline blocks or tiles
- Simple approach: Read until first scanline offset table (after header), split header vs pixel data
- Header = first chunk, pixel data = cdc::chunk_data
- Fallback to cdc::chunk_data if parsing fails

### src/chunking/structure_aware/csp.rs  
- chunk_csp(data: &[u8], config: &ChunkConfig) -> Vec<ChunkResult>
- CSP .clip files are SQLite databases internally
- Simple approach: Find the SQLite header (first 16 bytes: "SQLite format 3\000")
- If confirmed, identify page boundaries (page size at offset 16 as u16 BE)
- Chunk at page boundaries for better dedup when individual layers change
- Fallback to cdc::chunk_data

### src/store/mod.rs
- pub mod cas, compression, pack

### src/store/cas.rs
- ChunkStore struct with base_dir: PathBuf
- ChunkStore::new(base_dir: PathBuf) -> Self
- chunk_path(&self, hash: &blake3::Hash) -> PathBuf — 2-char prefix sharding: ab/cdef01234...
- contains(&self, hash: &blake3::Hash) -> bool
- store(&self, hash: &blake3::Hash, compressed_data: &[u8]) -> Result<bool> — write to temp file then atomic rename, return true if new
- read(&self, hash: &blake3::Hash) -> Result<Vec<u8>>
- remove(&self, hash: &blake3::Hash) -> Result<bool>
- list_all(&self) -> Result<Vec<blake3::Hash>> — walk all shard directories, parse hex filenames back to hashes
- total_size(&self) -> Result<u64> — sum of all chunk file sizes
- chunk_count(&self) -> Result<usize>

### src/store/compression.rs
- compress(data: &[u8], level: i32) -> Result<Vec<u8>> using zstd::encode_all
- decompress(data: &[u8]) -> Result<Vec<u8>> using zstd::decode_all
- compress_with_dict(data: &[u8], level: i32, dict: &[u8]) -> Result<Vec<u8>>
- decompress_with_dict(data: &[u8], dict: &[u8]) -> Result<Vec<u8>>
- train_dictionary(samples: &[Vec<u8>], dict_size: usize) -> Result<Vec<u8>> using zstd::dict::from_samples

### src/store/pack.rs
- PackFile struct — for future use, combines many small chunks into one large file with an index
- PackIndex { entries: Vec<PackIndexEntry> } where PackIndexEntry { hash: [u8;32], offset: u64, length: u32 }
- For now, implement create_pack(store: &ChunkStore, chunk_hashes: &[[u8;32]], output: &Path) -> Result<PackIndex>
  that concatenates chunks into one file with a trailer index
- read_from_pack(pack_path: &Path, index_entry: &PackIndexEntry) -> Result<Vec<u8>>

### src/db/mod.rs
- pub mod metadata

### src/db/metadata.rs
- Table definitions using redb::TableDefinition:
  FILES_TABLE: &str -> &[u8] (path -> rkyv FileEntry bytes)
  CHUNKS_TABLE: &[u8] -> u32 (32-byte hash -> refcount) — use fixed 32-byte key via a wrapper
  COMMITS_TABLE: &str -> &[u8] (hex commit id -> rkyv Commit bytes)
  STAGING_TABLE: &str -> &[u8] (path -> rkyv FileEntry bytes)
  
  NOTE: redb doesn't support &[u8; 32] directly as key. Use &str (hex encoded hash) for CHUNKS_TABLE key instead, or use &[u8] and do length checks.
  
- MetadataDb struct with db: redb::Database
- MetadataDb::create(path: &Path) -> Result<Self> — create database and all tables
- MetadataDb::open(path: &Path) -> Result<Self> — open existing database
- is_chunk_known(&self, hash: &[u8;32]) -> Result<bool>
- insert_chunk(&self, hash: &[u8;32]) -> Result<()> — increment refcount
- decrement_chunk(&self, hash: &[u8;32]) -> Result<bool> — decrement refcount, return true if reached 0
- stage_file(&self, path: &str, entry_bytes: &[u8]) -> Result<()>
- unstage_file(&self, path: &str) -> Result<()>
- get_staged_files(&self) -> Result<Vec<(String, Vec<u8>)>>
- clear_staging(&self) -> Result<()>
- store_commit(&self, id_hex: &str, commit_bytes: &[u8]) -> Result<()>
- get_commit(&self, id_hex: &str) -> Result<Option<Vec<u8>>>
- store_file_entry(&self, path: &str, entry_bytes: &[u8]) -> Result<()> — for HEAD tree tracking
- get_file_entry(&self, path: &str) -> Result<Option<Vec<u8>>>
- get_all_tracked_files(&self) -> Result<Vec<(String, Vec<u8>)>>

### src/cli/init.rs
- pub fn run(path: &str) -> Result<()>
- Call Repository::init, print success message with path

### src/cli/add.rs
- pub fn run(paths: &[String], force: bool) -> Result<()>
- Discover repo, open ChunkStore, open MetadataDb, read Config for chunk sizes
- Walk all paths recursively (respect .forgeignore unless --force)
- For each file:
  - Read file contents (use memmap2 for files > 4MB, fs::read for smaller)
  - Detect FileType from path + header bytes
  - Chunk using chunking::chunk_file with structure-aware dispatch
  - For each chunk: blake3 hash, check if known in db, if not: compress with zstd and store in CAS, insert into db
  - Build FileEntry with all ChunkRefs
  - Serialize FileEntry with rkyv, stage in db
- Show progress bar with indicatif (ProgressBar for total bytes)
- Print summary: N files staged, N new chunks (X MB), N deduped chunks (Y MB saved)

### src/cli/commit.rs
- pub fn run(message: &str) -> Result<()>
- Discover repo, open MetadataDb
- Get all staged files from db
- If empty, bail with "Nothing staged"
- Deserialize all staged FileEntry from rkyv
- Read current HEAD to get parent commit hash (None if first commit)
- Build Commit struct with id=[0u8;32] placeholder
- Serialize with rkyv, blake3 hash the bytes to get real commit id
- Rebuild Commit with correct id, re-serialize
- Write manifest file to .forge/manifests/{hex_id}
- Store commit in redb COMMITS_TABLE
- Update all staged files into FILES_TABLE (current tree state)
- Update HEAD ref to point to new commit
- Clear staging table
- Print: "Committed {short_hex} — {N} files, message: {message}"

### src/cli/status.rs
- pub fn run() -> Result<()>
- Discover repo, open MetadataDb
- Print current branch (parse HEAD)
- Print current commit (short hex) if any
- List staged files with "+" prefix
- Walk working directory, compare against tracked files in FILES_TABLE:
  - Files in working dir but not tracked: "? untracked"  
  - Files tracked but modified (compare mtime_ns and size first, then hash if needed): "M modified"
  - Files tracked but deleted from working dir: "D deleted"

### src/cli/log.rs
- pub fn run(count: usize) -> Result<()>
- Discover repo, read HEAD
- Walk parent chain up to `count` commits
- For each commit: read manifest from .forge/manifests/{hex}, deserialize with rkyv
- Print formatted output: commit hash (yellow), author, date (formatted with chrono), file count, message
- Handle empty repo gracefully

### src/cli/diff.rs
- pub fn run(path: Option<&str>, commit1: Option<&str>, commit2: Option<&str>) -> Result<()>
- If two commits given: compare their manifests, show added/removed/modified files
- If no commits: compare HEAD commit vs current staging + working tree
- For each changed file: show old size vs new size, number of chunks changed, percentage of chunks reused
- If path filter given, only show that file

### src/cli/checkout.rs
- pub fn run(commit_id_hex: &str) -> Result<()>
- Discover repo, open ChunkStore
- Read manifest from .forge/manifests/{commit_id_hex}
- Deserialize Commit with rkyv
- For each FileEntry in commit:
  - Create parent directories
  - Open output file
  - For each ChunkRef: read compressed chunk from CAS, decompress, write to output
  - Set file permissions from mode field
- Update HEAD to point to this commit (detached HEAD if not a branch tip)
- Print summary

### src/cli/push.rs
- pub fn run(remote: &str) -> Result<()>
- For v0.1: print "Push not yet implemented — network transport coming in v0.2"
- Stub the function signature so it compiles

### src/cli/pull.rs  
- pub fn run(remote: &str) -> Result<()>
- Same as push — stub with message

### src/cli/train_dict.rs
- pub fn run(file_type: &str, samples_dir: &str, output: &str) -> Result<()>
- Walk samples_dir, collect up to 1000 files matching the file type
- Read first 128KB of each file as a sample
- Call compression::train_dictionary with the samples
- Write dictionary to output path
- Print: "Trained dictionary from N samples -> X bytes"

### src/transport/mod.rs
- pub mod protocol, quic
- Placeholder types for v0.2

### src/transport/protocol.rs
- Define message types as enums (for future use):
  ClientMessage: Hello, PushManifest, ChunkData, PullRequest, Ack
  ServerMessage: Ok, NeedChunks, Manifest, ChunkData, AckCommit
- Implement simple serialize/deserialize using serde_json for now (swap to binary later)

### src/transport/quic.rs
- Placeholder async fn start_server and connect_client that return unimplemented!() errors with helpful messages

### src/util/mod.rs
- pub mod ignore, progress, human

### src/util/ignore.rs
- ForgeIgnore struct wrapping a list of glob patterns
- ForgeIgnore::load(repo_root: &Path) -> Self — read .forgeignore from repo root, parse each line as glob pattern, skip comments (#) and empty lines
- is_ignored(&self, path: &Path) -> bool — check if path matches any pattern
- Default patterns: .forge/, .git/, .DS_Store, Thumbs.db, *.tmp

### src/util/progress.rs
- create_progress_bar(total: u64) -> ProgressBar with nice styling
- create_spinner(message: &str) -> ProgressBar

### src/util/human.rs
- human_bytes(bytes: u64) -> String — format as B/KB/MB/GB/TB with 1 decimal
- human_duration(duration: std::time::Duration) -> String — format as ms/s/min
- short_hex(hash: &[u8;32]) -> String — first 12 hex chars

### src/cli/mod.rs
- pub mod init, add, commit, status, log, diff, checkout, push, pull, train_dict

### build.rs
- Empty build.rs or one that prints cargo:rerun-if-changed=src/

### tests/integration.rs
- Use tempfile::tempdir for all tests
- Use std::process::Command to run the forge binary OR call lib functions directly

Test 1: test_init_creates_forge_dir
  - Create temp dir, call Repository::init, verify .forge/ and all subdirs exist, verify HEAD contains "ref: refs/heads/main", verify metadata.redb exists and is valid

Test 2: test_add_and_commit_single_file
  - Init repo in temp dir
  - Create a 1MB file with random bytes
  - Call add::run with the file path
  - Verify staging table has 1 entry
  - Call commit::run with a message
  - Verify staging is empty, manifest file exists in .forge/manifests/, HEAD ref updated

Test 3: test_deduplication
  - Init repo, create two identical 1MB files with different names
  - Add and commit both
  - Verify chunk store has only the chunks for one file (dedup worked)
  - Check that ChunkStore::chunk_count equals the number of unique chunks, not 2x

Test 4: test_checkout_restores_files
  - Init repo, create 3 files of varying sizes (100KB, 1MB, 5MB)
  - Add and commit
  - Delete all 3 files from working directory
  - Call checkout with the commit hash
  - Verify all 3 files are restored with identical content (blake3 hash matches original)

Test 5: test_incremental_commit
  - Init, create file A (1MB), add, commit (commit1)
  - Modify 1 byte in file A, add, commit (commit2)  
  - Verify commit2 reuses most chunks from commit1 (new_chunks should be small, like 1-2)
  - Checkout commit1, verify original content
  - Checkout commit2, verify modified content

Test 6: test_status_shows_changes
  - Init, create file, add, commit
  - Modify the file
  - Call status, verify it reports the file as modified
  - Create a new file
  - Call status, verify it reports untracked file

Test 7: test_log_shows_history
  - Init, create file, add, commit "first"
  - Modify file, add, commit "second"
  - Call log with count=10, verify both commits appear in reverse chronological order

Test 8: test_structure_aware_uasset_chunking
  - Create a fake uasset file: 4 bytes magic (0xC1832A9E in LE = [0xC1, 0x83, 0x2A, 0x9E]), then 24 bytes padding, then header_size as u32 LE = 1024, then 1024 bytes of header data, then 1MB of "bulk" data
  - Chunk with chunk_uasset
  - Verify first chunk covers exactly the header region
  - Modify only the header bytes, re-chunk
  - Verify bulk data chunks are identical (same hashes)

Test 9: test_compression_roundtrip
  - Generate 1MB of compressible data (repeated patterns)
  - Compress, verify output is smaller
  - Decompress, verify matches original exactly

Test 10: test_forgeignore
  - Create .forgeignore with "*.tmp" and "build/"
  - Verify is_ignored returns true for "foo.tmp", "build/output.bin"
  - Verify is_ignored returns false for "foo.png", "src/main.rs"

### benches/ingest.rs
- Use criterion
- Benchmark: chunk_1mb — chunk a 1MB buffer with default CDC config, measure throughput
- Benchmark: chunk_10mb — same with 10MB
- Benchmark: hash_1mb — blake3::hash on 1MB buffer
- Benchmark: compress_1mb — zstd compress 1MB of realistic data (not random — use repeated pattern with some variation)
- Benchmark: full_pipeline_1mb — chunk + hash + compress + store to temp CAS, measure end-to-end throughput
- Print throughput in MB/s using criterion's throughput feature

## CRITICAL IMPLEMENTATION NOTES:

1. redb key types: redb requires keys to implement redb::Key. &str and &[u8] work. &[u8;32] does NOT work directly. For chunk hash keys, hex-encode them to &str.

2. rkyv 0.8 API: Use rkyv::to_bytes::<_, 1024>(value) for serialization. Use rkyv::from_bytes::<T>(bytes) or rkyv::access::<rkyv::Archived<T>, rkyv::rancor::Error>(bytes) for deserialization. The exact API depends on rkyv 0.8 — if rkyv::to_bytes doesn't exist in 0.8, use rkyv::api::high::to_bytes_with_alloc or the AlignedVec approach. Handle whichever API is current.

3. If rkyv 0.8 API is too uncertain, use serde + bincode as the serialization layer instead. Define all manifest types with #[derive(serde::Serialize, serde::Deserialize)] and use bincode::serialize/deserialize. Add bincode = "1" to dependencies. This is simpler and guaranteed to work. You can swap to rkyv later for zero-copy performance.

4. For memmap2: use unsafe { Mmap::map(&file) } and handle the Result. Only use mmap for read-only operations on files > 4MB.

5. All error handling must use anyhow::Result with .context() on fallible operations. No .unwrap() in library code (only in tests).

6. The .forgeignore parser should handle: empty lines, lines starting with # (comments), glob patterns using the glob crate, negation patterns starting with ! (include previously ignored).

7. For the status command's file modification detection: first compare mtime_ns and file size (fast path). Only hash the file if size matches but mtime differs.

8. Write the actual main.rs so that `forge init`, `forge add .`, `forge commit -m "test"`, `forge status`, `forge log`, `forge checkout <hash>` all work end-to-end.

Generate every file now with complete implementations. Do not split into multiple responses.
```

---

## How to Use This

1. Open your project in VS Code / Cursor with GitHub Copilot (Claude Opus 4.6)
2. Create the `forge/` directory: `mkdir forge && cd forge && cargo init`
3. Open the Copilot chat panel
4. Paste the entire prompt above as **one message**
5. Let it generate everything
6. Copy each file to the correct location
7. Run `cargo build --release` — fix any compilation errors by asking "Fix all compilation errors in this project"
8. Run `cargo test`

## Fallback If It Hits Length Limits

If the model truncates, split into exactly **two** prompts:

**Prompt A:** Everything from `Cargo.toml` through `src/store/pack.rs` (the data layer)

**Prompt B:** "Continue generating the Forge CLI project. You already generated Cargo.toml and src/ up through store/pack.rs. Now generate these remaining files with full implementations, no stubs:" then list `src/db/` through `benches/ingest.rs`

But with Opus 4.6's context window, the single prompt should work.

## After Generation — Verification Checklist

```bash
cargo build --release 2>&1 | head -50    # Must compile clean
cargo test                                # All 10 tests pass
cargo bench                               # Verify throughput numbers

# Manual smoke test:
mkdir /tmp/forge-test && cd /tmp/forge-test
forge init
dd if=/dev/urandom of=bigfile.bin bs=1M count=100
forge add bigfile.bin
forge status
forge commit -m "100MB test"
forge log

# Modify 1 byte and verify dedup:
printf '\x42' | dd of=bigfile.bin bs=1 seek=50000000 count=1 conv=notrunc
forge add bigfile.bin
forge commit -m "1 byte change"
forge log
# Check .forge/objects/chunks/ — should have very few new chunks

# Checkout test:
COMMIT=$(ls .forge/manifests/ | head -1)
rm bigfile.bin
forge checkout $COMMIT
md5sum bigfile.bin  # Should match original
```
