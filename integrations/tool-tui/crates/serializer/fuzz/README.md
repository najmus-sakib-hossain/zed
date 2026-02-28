
# Fuzzing dx-serializer

This directory contains fuzz targets for testing the dx-serializer crate with cargo-fuzz.

## Prerequisites

Install cargo-fuzz (requires nightly Rust):
```bash
rustup install nightly cargo +nightly install cargo-fuzz ```


## Fuzz Targets


+---------+-------------+--------+--------+
| Target  | Description | Entry  | Point  |
+=========+=============+========+========+
| `parse` | Main        | binary | parser |
+---------+-------------+--------+--------+


## Running Fuzz Tests


List all available targets:
```bash
cargo +nightly fuzz list ```
Run a specific target:
```bash
cargo +nightly fuzz run parse cargo +nightly fuzz run llm_to_doc cargo +nightly fuzz run base62 cargo +nightly fuzz run tokenizer cargo +nightly fuzz run human_format cargo +nightly fuzz run converters cargo +nightly fuzz run roundtrip ```


## Running for a Specific Duration


Run fuzzing for 1 hour (3600 seconds):
```bash
cargo +nightly fuzz run parse -- -max_total_time=3600 ```
Run with multiple jobs in parallel:
```bash
cargo +nightly fuzz run parse -- -jobs=4 -workers=4 ```


## Viewing Crashes


If a crash is found, it will be saved in `artifacts/<target_name>/`:
```bash
ls artifacts/parse/ ```
To reproduce a crash:
```bash
cargo +nightly fuzz run parse artifacts/parse/crash-<hash> ```


## Adding Crashes as Regression Tests


When a crash is found and fixed, add the input to the corpus to prevent regression:
```bash
cp artifacts/parse/crash-<hash> corpus/parse/ ```

## Corpus Management

The `corpus/` directory contains test inputs that have been found to exercise different code paths. These are automatically managed by libFuzzer but can be manually curated.

## Coverage

To see code coverage from fuzzing:
```bash
cargo +nightly fuzz coverage parse ```


## Security Requirements


According to the design document (Requirement 10), fuzzing verifies: -No panics or crashes on any generated input -Parser handles malicious input gracefully -Security limits are enforced:-Input size: 100 MB max -Recursion depth: 1,000 levels max -Table rows: 10 million max -UTF-8 validation works correctly with proper error offsets


## Recommended Fuzzing Strategy


For comprehensive coverage, run all targets:
```bash

# Quick smoke test (1 minute each)

for target in parse llm_to_doc machine_to_doc base62 tokenizer human_format converters roundtrip; do cargo +nightly fuzz run $target -- -max_total_time=60 done

# Extended fuzzing (1 hour each for CI)

for target in parse llm_to_doc base62 tokenizer roundtrip; do cargo +nightly fuzz run $target -- -max_total_time=3600 done ```
When fuzzing runs for 1 hour without crashes, the parser meets the security requirements.
