faster than="Validates: Requirements"

# Design Document: CLI Production Ready

## Overview

This design document outlines the technical approach for making the DX CLI production-ready. The changes span bug fixes, code cleanup, new features, and improved testing. The implementation prioritizes backward compatibility while adding professional polish expected of production software.

## Architecture

The changes are organized into three categories: -Bug Fixes - Signal handler, version compatibility enforcement -New Features - Shell completions, update checker, doctor command -Quality Improvements - Dead code cleanup, error context, JSON standardization, testing @flow:TD[]

## Components and Interfaces

### 1. Signal Handler Fix (signal.rs)

The current implementation has a bug where the second signal check always evaluates to true because the flag was just set. Current (Buggy):
```rust
SHUTDOWN_REQUESTED.store(true,Ordering::SeqCst);if SHUTDOWN_REQUESTED.load(Ordering::SeqCst){std::process::exit(130);}
```
Fixed Implementation:
```rust
use std::sync::atomic::{AtomicU32,Ordering};static SIGNAL_COUNT:AtomicU32 =AtomicU32::new(0);pub fn register_handlers()->anyhow::Result<()>{ctrlc::set_handler(move ||{let count =SIGNAL_COUNT.fetch_add(1,Ordering::SeqCst);if count ==0 {tracing::info!("Shutdown signal received, initiating graceful shutdown...");}else {tracing::warn!("Second shutdown signal received, forcing exit");std::process::exit(130);}})?;Ok(())}pub fn is_shutdown_requested()->bool {SIGNAL_COUNT.load(Ordering::SeqCst)>0 }
```

### 2. Daemon Client Reconnection (daemon_client.rs)

Add retry logic with exponential backoff:
```rust
pub struct RetryConfig {pub max_attempts:u32,pub initial_delay_ms:u64,pub max_delay_ms:u64,}impl Default for RetryConfig {fn default()->Self {Self {max_attempts:3,initial_delay_ms:100,max_delay_ms:2000,}}}impl DaemonClient {pub fn connect_with_retry(config:RetryConfig)->Result<Self>{let mut last_error =None;let mut delay =config.initial_delay_ms;for attempt in 1..=config.max_attempts {tracing::debug!("Connection attempt {}/{}",attempt,config.max_attempts);match Self::connect(){Ok(client)=>return Ok(client),Err(e)=>{last_error =Some(e);if attempt <config.max_attempts {std::thread::sleep(Duration::from_millis(delay));delay =(delay *2).min(config.max_delay_ms);}}}}Err(last_error.unwrap().context("Failed to connect to daemon after multiple attempts. \ Is the daemon running? Try 'dx forge start'"))}}
```

### 3. Version Compatibility Enforcement

Modify `perform_handshake` to fail hard on incompatibility:
```rust
pub fn perform_handshake(&mut self)->Result<HandshakeResponse>{if !response.compatible {return Err(anyhow!("Protocol version mismatch: CLI={}, Daemon={}. \ Please run 'dx forge stop' and restart the daemon.",DAEMON_PROTOCOL_VERSION,response.daemon_protocol_version ));}Ok(response)}
```

### 4. Shell Completion Generation

Add to `main.rs`:
```rust
use clap::CommandFactory;use clap_complete::{generate,Shell};#[derive(Parser)]struct Cli {#[arg(long,value_name ="SHELL")]generate_completion:Option<Shell>,}fn main(){let cli =Cli::parse();if let Some(shell)=cli.generate_completion {let mut cmd =Cli::command();generate(shell,&mut cmd,"dx",&mut std::io::stdout());return;}}
```

### 5. Update Checker

New module `src/update.rs`:
```rust
use anyhow::Result;use serde::{Deserialize,Serialize};use std::path::PathBuf;use std::time::{Duration,SystemTime};const UPDATE_CHECK_URL:&str ="api.github.com/repos/dx-org/dx/releases/latest";const CACHE_DURATION:Duration =Duration::from_secs(24 *60 *60);#[derive(Debug,Serialize,Deserialize)]struct UpdateCache {checked_at:SystemTime,latest_version:String,}pub struct UpdateChecker {cache_path:PathBuf,}impl UpdateChecker {pub fn new()->Self {let cache_dir =dirs::cache_dir().unwrap_or_else(std::env::temp_dir);Self {cache_path:cache_dir.join("dx").join("update-cache.json"),}}pub async fn check(&self)->Result<UpdateResult>{if let Some(cached)=self.read_cache()?{if cached.checked_at.elapsed()?<CACHE_DURATION {return Ok(self.compare_versions(&cached.latest_version));}}let latest =self.fetch_latest_version().await?;self.write_cache(&latest)?;Ok(self.compare_versions(&latest))}fn compare_versions(&self,latest:&str)->UpdateResult {let current =env!("CARGO_PKG_VERSION");if latest !=current {UpdateResult::UpdateAvailable {current:current.to_string(),latest:latest.to_string(),}}else {UpdateResult::UpToDate }}}pub enum UpdateResult {UpToDate,UpdateAvailable {current:String,latest:String },}
```

### 6. Doctor Command

New command in `src/commands/doctor.rs`:
```rust
use anyhow::Result;use clap::Args;#[derive(Args)]pub struct DoctorCommand;impl DoctorCommand {pub async fn execute(&self)->Result<()>{use console::style;println!();println!("{}",style("DX CLI Diagnostics").cyan().bold());println!();println!("{}:",style("Version").bold());println!(" CLI: {}",env!("CARGO_PKG_VERSION"));println!(" OS: {} {}",std::env::consts::OS,std::env::consts::ARCH);println!();println!("{}:",style("Daemon").bold());if DaemonClient::is_daemon_running(){println!(" Status: {}",style("Running").green());if let Ok(mut client)=DaemonClient::connect(){if let Ok(status)=client.get_status(){println!(" Uptime: {}s",status.uptime_seconds);}}}else {println!(" Status: {}",style("Stopped").yellow());}println!();println!("{}:",style("Configuration").bold());match DxConfig::load(){Ok(config)=>{println!(" Project: {}",config.project.name);println!(" Version: {}",config.project.version);}Err(_)=>{println!(" {}",style("No configuration file found").dim());}}println!();println!("{}:",style("Checks").bold());self.run_checks();Ok(())}fn run_checks(&self){use console::style;let checks =vec![("Daemon socket",self.check_daemon_socket()),("Config file",self.check_config()),("Cache directory",self.check_cache_dir()),];for (name,result)in checks {match result {Ok(())=>println!(" {} {}",style("✓").green(),name),Err(msg)=>println!(" {} {} - {}",style("✗").red(),name,msg),}}}}
```

### 7. JSON Output Standardization

The `output.rs` module already has `SuccessResponse` and `ErrorResponse` structs. Ensure all commands use them consistently:
```rust
match format {OutputFormat::Json =>{let response =SuccessResponse::with_results(results,results.len());print_json(&response)?;}OutputFormat::Table =>{}OutputFormat::Simple =>{}}if matches!(format,OutputFormat::Json){let error =ErrorResponse::new(e.to_string(),error_codes::FS_ERROR).with_hint("Check file permissions");print_json(&error)?;std::process::exit(1);}
```

## Data Models

### Update Cache Model

```rust


#[derive(Debug,Serialize,Deserialize)]pub struct UpdateCache {pub checked_at:SystemTime,pub latest_version:String,pub release_url:Option<String>,}


```

### Diagnostic Result Model

```rust


#[derive(Debug,Serialize)]pub struct DiagnosticResult {pub cli_version:String,pub os:String,pub arch:String,pub daemon_running:bool,pub daemon_uptime:Option<u64>,pub config_found:bool,pub config_path:Option<PathBuf>,pub checks:Vec<CheckResult>,}#[derive(Debug,Serialize)]pub struct CheckResult {pub name:String,pub passed:bool,pub message:Option<String>,}


```

## Correctness Properties

A property is a characteristic or behavior that should hold true across all valid executions of a system—, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.

### Property 1: Retry with Exponential Backoff

For any sequence of connection failures up to the retry limit, the delay between attempts SHALL increase exponentially (doubling each time) starting from the initial delay, capped at the maximum delay. faster than 3.1

### Property 2: Incompatible Handshake Returns Error

For any handshake response where `compatible` is `false`, the `perform_handshake` function SHALL return an `Err` result, and no subsequent daemon operations SHALL be attempted. faster than 4.1, 4.3

### Property 3: Error Messages Include Relevant Context

For any error originating from a file operation, daemon connection, or configuration parsing, the error message SHALL contain the relevant identifier (file path, socket path/port, or field name respectively). faster than 8.1, 8.2, 8.3

### Property 4: Update Check Caching

For any sequence of update check calls within a 24-hour window, only the first call SHALL make a network request; subsequent calls SHALL return the cached result. faster than 9.5

### Property 5: JSON Output is Valid JSON

For any command executed with `--format json`, the stdout output SHALL be parseable as valid JSON, regardless of whether the command succeeds or fails. faster than 10.1, 10.4

### Property 6: JSON Output Has Required Structure

For any JSON output from the CLI, the parsed JSON object SHALL contain a `success` boolean field and a `version` string field. For error responses, it SHALL additionally contain `error` and `code` string fields. faster than 10.2, 10.3, 10.5

## Error Handling

### Error Context Strategy

All fallible operations must use `anyhow::Context` to add relevant information:
```rust
std::fs::read_to_string(&path).with_context(||format!("Failed to read file: {}",path.display()))?;UnixStream::connect(&socket_path).with_context(||format!("Failed to connect to daemon at {}",socket_path.display()))?;toml::from_str(&content).with_context(||format!("Failed to parse config file: {}",path.display()))?;
```

### Error Codes

Standardized error codes for JSON output: +------------------------------------------------+--------------------------------------------+
| Code                                           | Description                                |
+================================================+============================================+
| `INVALID_ARG`                                  | Invalid command arguments `NOT_FOUND`      |
+------------------------------------------------+--------------------------------------------+
| Resource not found `DAEMON_ERROR`              | Daemon communication error `NETWORK_ERROR` |
+------------------------------------------------+--------------------------------------------+
| Network operation failed `FS_ERROR`            | File system error `PARSE_ERROR`            |
+------------------------------------------------+--------------------------------------------+
| Parse/deserialization error `VERSION_MISMATCH` | Protocol version incompatibility           |
+------------------------------------------------+--------------------------------------------+



## Testing Strategy

### Unit Tests

Unit tests verify specific examples and edge cases: -Signal handler counter increments correctly -Retry delay calculation is correct -Version comparison logic works -JSON serialization produces expected output -Error context is properly attached

### Property-Based Tests

Property-based tests verify universal properties using `proptest`: -Exponential Backoff Property - Generate random retry configs and verify delay sequence -Handshake Compatibility Property - Generate handshake responses and verify error behavior -Error Context Property - Generate various error scenarios and verify context presence -Update Cache Property - Simulate time progression and verify caching behavior -JSON Validity Property - Generate various command outputs and verify JSON validity -JSON Structure Property - Parse JSON outputs and verify required fields exist

### Integration Tests

Integration tests verify end-to-end behavior: -Shell completion generation produces valid scripts -Doctor command runs without errors -Update check handles network failures gracefully -Config subcommand loads configuration correctly -JSON output format is consistent across commands

### Test Configuration

- Property tests: minimum 100 iterations per property
- Use `tempfile` crate for temporary directories
- Use platform-agnostic path construction
- Mock network calls for update checker tests
