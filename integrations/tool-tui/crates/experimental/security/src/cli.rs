//! CLI Integration for dx-security
//!
//! Provides the SecurityCommand struct and scan orchestration for CLI usage.
//! _Requirements: 11.1, 11.2, 11.3_

use crate::diff::DifferentialScanner;
use crate::error::{Result, SecurityError};
use crate::graph::BinaryDependencyGraph;
use crate::index::BinaryVulnerabilityIndex;
use crate::mapper::MemoryMapper;
use crate::scanner::{SimdMode, SimdSecretScanner};
use crate::score::{ScanFindings, calculate_score};
use crate::stream::{Finding, FindingEmitter, Severity, StreamOutput};
use std::path::{Path, PathBuf};

/// Exit codes for CLI
pub mod exit_codes {
    /// Success - score above threshold
    pub const SUCCESS: i32 = 0;
    /// Security violations - score below threshold
    pub const SECURITY_VIOLATION: i32 = 1;
    /// Runtime error
    pub const RUNTIME_ERROR: i32 = 2;
}

/// Output format for scan results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Binary .sr format
    Binary,
    /// JSON format
    Json,
    /// Markdown report
    Markdown,
    /// Terminal output (default)
    #[default]
    Terminal,
}

impl OutputFormat {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "binary" | "sr" => Some(Self::Binary),
            "json" => Some(Self::Json),
            "markdown" | "md" => Some(Self::Markdown),
            "terminal" | "term" => Some(Self::Terminal),
            _ => None,
        }
    }
}

/// CLI command configuration for security scanning
/// _Requirements: 11.1, 11.2_
#[derive(Debug, Clone)]
pub struct SecurityCommand {
    /// Path to scan (file or directory)
    pub path: PathBuf,
    /// Minimum score threshold (fail if below)
    pub fail_under: Option<u8>,
    /// Sign the report with Ed25519
    pub sign: bool,
    /// Output file path
    pub output: Option<PathBuf>,
    /// Output format
    pub format: OutputFormat,
    /// Path to custom rules configuration
    pub config: Option<PathBuf>,
    /// Enable verbose output
    pub verbose: bool,
    /// Use incremental scanning (skip unchanged files)
    pub incremental: bool,
}

impl Default for SecurityCommand {
    fn default() -> Self {
        Self {
            path: PathBuf::from("."),
            fail_under: None,
            sign: false,
            output: None,
            format: OutputFormat::Terminal,
            config: None,
            verbose: false,
            incremental: true,
        }
    }
}

impl SecurityCommand {
    /// Create a new security command for the given path
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            ..Default::default()
        }
    }

    /// Set the fail-under threshold
    pub fn with_fail_under(mut self, threshold: u8) -> Self {
        self.fail_under = Some(threshold);
        self
    }

    /// Enable report signing
    pub fn with_signing(mut self) -> Self {
        self.sign = true;
        self
    }

    /// Set output path
    pub fn with_output(mut self, path: PathBuf) -> Self {
        self.output = Some(path);
        self
    }

    /// Set output format
    pub fn with_format(mut self, format: OutputFormat) -> Self {
        self.format = format;
        self
    }

    /// Set custom rules config
    pub fn with_config(mut self, config: PathBuf) -> Self {
        self.config = Some(config);
        self
    }

    /// Enable verbose output
    pub fn with_verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    /// Disable incremental scanning
    pub fn without_incremental(mut self) -> Self {
        self.incremental = false;
        self
    }
}

/// Scan result containing findings and score
#[derive(Debug, Clone)]
pub struct ScanResult {
    /// Calculated security score (0-100)
    pub score: u8,
    /// Scan findings used for score calculation
    pub findings: ScanFindings,
    /// List of detected secrets
    pub secrets: Vec<SecretFinding>,
    /// List of vulnerable dependencies
    pub vulnerabilities: Vec<VulnerabilityFinding>,
    /// Number of files scanned
    pub files_scanned: usize,
    /// Number of files skipped (unchanged)
    pub files_skipped: usize,
    /// Scan duration in milliseconds
    pub duration_ms: u64,
}

/// Secret finding details
#[derive(Debug, Clone)]
pub struct SecretFinding {
    /// File path
    pub file_path: PathBuf,
    /// Line number
    pub line_number: u32,
    /// Column number
    pub column: u16,
    /// Secret type description
    pub secret_type: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
}

/// Vulnerability finding details
#[derive(Debug, Clone)]
pub struct VulnerabilityFinding {
    /// Package name
    pub package: String,
    /// Package version
    pub version: String,
    /// CVE ID
    pub cve_id: String,
    /// Severity level
    pub severity: Severity,
    /// Description
    pub description: String,
}

/// Security scanner orchestrator
///
/// Coordinates mapper, scanners, index, and graph to perform a complete scan.
/// _Requirements: 10.1_
pub struct SecurityScanner {
    mapper: MemoryMapper,
    secret_scanner: SimdSecretScanner,
    diff_scanner: DifferentialScanner,
    vuln_index: Option<BinaryVulnerabilityIndex>,
    dep_graph: BinaryDependencyGraph,
    emitter: Option<FindingEmitter>,
}

impl Default for SecurityScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityScanner {
    /// Create a new security scanner
    pub fn new() -> Self {
        Self {
            mapper: MemoryMapper::new(),
            secret_scanner: SimdSecretScanner::new(),
            diff_scanner: DifferentialScanner::new(),
            vuln_index: None,
            dep_graph: BinaryDependencyGraph::new(),
            emitter: None,
        }
    }

    /// Set the finding emitter for real-time output
    pub fn with_emitter(mut self, emitter: FindingEmitter) -> Self {
        self.emitter = Some(emitter);
        self
    }

    /// Load vulnerability index from path
    pub fn load_vuln_index(&mut self, path: &Path) -> Result<()> {
        self.vuln_index = Some(BinaryVulnerabilityIndex::load(path)?);
        Ok(())
    }

    /// Execute a security scan
    /// _Requirements: 10.1_
    pub fn scan(&mut self, cmd: &SecurityCommand) -> Result<ScanResult> {
        let start = std::time::Instant::now();
        let mut findings = ScanFindings::default();
        let mut secrets = Vec::new();
        let mut vulnerabilities = Vec::new();
        let mut files_scanned = 0usize;
        let mut files_skipped = 0usize;

        // Scan for secrets in files
        if cmd.path.is_file() {
            self.scan_file(
                &cmd.path,
                cmd.incremental,
                &mut findings,
                &mut secrets,
                &mut files_scanned,
                &mut files_skipped,
            )?;
        } else if cmd.path.is_dir() {
            self.scan_directory(
                &cmd.path,
                cmd.incremental,
                &mut findings,
                &mut secrets,
                &mut files_scanned,
                &mut files_skipped,
            )?;
        }

        // Scan for vulnerable dependencies
        self.scan_dependencies(&cmd.path, &mut findings, &mut vulnerabilities)?;

        // Calculate score
        let score = calculate_score(&findings);

        // Emit score finding
        if let Some(emitter) = &mut self.emitter {
            let _ = emitter.emit(Finding::score(score));
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(ScanResult {
            score,
            findings,
            secrets,
            vulnerabilities,
            files_scanned,
            files_skipped,
            duration_ms,
        })
    }

    /// Scan a single file for secrets
    fn scan_file(
        &mut self,
        path: &Path,
        incremental: bool,
        findings: &mut ScanFindings,
        secrets: &mut Vec<SecretFinding>,
        files_scanned: &mut usize,
        files_skipped: &mut usize,
    ) -> Result<()> {
        // Map file into memory
        let mapped = match self.mapper.map_file(path) {
            Ok(m) => m,
            Err(_) => return Ok(()), // Skip files we can't read
        };

        // Check if file has changed (incremental scanning)
        if incremental && !self.diff_scanner.has_changed(path, &mapped.data) {
            *files_skipped += 1;
            return Ok(());
        }

        *files_scanned += 1;

        // Scan for secrets
        let secret_findings = self.secret_scanner.scan(&mapped.data);
        let secret_count = secret_findings.len();

        for sf in secret_findings {
            findings.secrets_leaked += 1;

            let secret_finding = SecretFinding {
                file_path: path.to_path_buf(),
                line_number: sf.line_number,
                column: 0,
                secret_type: format!("{:?}", sf.pattern_type),
                confidence: sf.confidence,
            };

            // Emit finding in real-time
            if let Some(emitter) = &mut self.emitter {
                let _ = emitter.emit(Finding::secret(
                    path.to_path_buf(),
                    sf.line_number,
                    0,
                    format!("{:?} detected", sf.pattern_type),
                ));
            }

            secrets.push(secret_finding);
        }

        // Update cache
        let hash = DifferentialScanner::hash(&mapped.data);
        self.diff_scanner.update_cache(path, hash, secret_count);

        Ok(())
    }

    /// Scan a directory recursively for secrets
    fn scan_directory(
        &mut self,
        path: &Path,
        incremental: bool,
        findings: &mut ScanFindings,
        secrets: &mut Vec<SecretFinding>,
        files_scanned: &mut usize,
        files_skipped: &mut usize,
    ) -> Result<()> {
        let walker = walkdir::WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file());

        for entry in walker {
            let file_path = entry.path();

            // Skip common non-source files
            if should_skip_file(file_path) {
                continue;
            }

            self.scan_file(
                file_path,
                incremental,
                findings,
                secrets,
                files_scanned,
                files_skipped,
            )?;
        }

        Ok(())
    }

    /// Scan for vulnerable dependencies
    fn scan_dependencies(
        &mut self,
        path: &Path,
        findings: &mut ScanFindings,
        vulnerabilities: &mut Vec<VulnerabilityFinding>,
    ) -> Result<()> {
        // Look for lockfiles
        let lockfiles = find_lockfiles(path);

        for lockfile in lockfiles {
            if self.dep_graph.from_lockfile(&lockfile).is_err() {
                continue; // Skip invalid lockfiles
            }

            // Check for vulnerabilities if index is loaded
            if self.vuln_index.is_some() {
                self.dep_graph.propagate_vulnerabilities();

                for node in self.dep_graph.vulnerable_nodes() {
                    let severity = if node.flags & 0x10 != 0 {
                        Severity::Critical
                    } else if node.flags & 0x08 != 0 {
                        Severity::High
                    } else {
                        Severity::Medium
                    };

                    match severity {
                        Severity::Critical => findings.critical_cves += 1,
                        Severity::High => findings.high_cves += 1,
                        Severity::Medium => findings.medium_cves += 1,
                        _ => findings.low_cves += 1,
                    }

                    let name = self.dep_graph.get_name(node.id).unwrap_or_default();
                    vulnerabilities.push(VulnerabilityFinding {
                        package: name.to_string(),
                        version: format!(
                            "{}.{}.{}",
                            node.version >> 16,
                            (node.version >> 8) & 0xFF,
                            node.version & 0xFF
                        ),
                        cve_id: String::new(),
                        severity,
                        description: "Vulnerable dependency".to_string(),
                    });

                    // Emit finding
                    if let Some(emitter) = &mut self.emitter {
                        let _ = emitter.emit(Finding::vulnerability(
                            severity,
                            lockfile.clone(),
                            0,
                            format!("Vulnerable dependency: {}", name),
                            None,
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Get the SIMD mode being used
    pub fn simd_mode(&self) -> SimdMode {
        self.secret_scanner.simd_mode()
    }
}

/// Check if a file should be skipped during scanning
fn should_skip_file(path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    // Skip common non-source directories
    if path_str.contains("node_modules")
        || path_str.contains("target")
        || path_str.contains(".git")
        || path_str.contains("__pycache__")
        || path_str.contains(".venv")
    {
        return true;
    }

    // Skip binary files by extension
    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        matches!(
            ext.as_str(),
            "exe"
                | "dll"
                | "so"
                | "dylib"
                | "bin"
                | "o"
                | "a"
                | "png"
                | "jpg"
                | "jpeg"
                | "gif"
                | "ico"
                | "svg"
                | "woff"
                | "woff2"
                | "ttf"
                | "eot"
                | "zip"
                | "tar"
                | "gz"
                | "rar"
                | "7z"
                | "pdf"
                | "doc"
                | "docx"
        )
    } else {
        false
    }
}

/// Find lockfiles in a directory
fn find_lockfiles(path: &Path) -> Vec<PathBuf> {
    let mut lockfiles = Vec::new();

    if path.is_file() {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name == "Cargo.lock" || name == "package-lock.json" {
            lockfiles.push(path.to_path_buf());
        }
    } else if path.is_dir() {
        // Check for lockfiles in the directory
        let cargo_lock = path.join("Cargo.lock");
        if cargo_lock.exists() {
            lockfiles.push(cargo_lock);
        }

        let package_lock = path.join("package-lock.json");
        if package_lock.exists() {
            lockfiles.push(package_lock);
        }
    }

    lockfiles
}

/// Check threshold and return appropriate exit code
/// _Requirements: 11.3_
pub fn check_threshold(score: u8, threshold: Option<u8>) -> i32 {
    match threshold {
        Some(t) if score < t => exit_codes::SECURITY_VIOLATION,
        _ => exit_codes::SUCCESS,
    }
}

/// Execute a security scan and return exit code
/// _Requirements: 11.1, 11.3_
pub fn execute(cmd: SecurityCommand) -> Result<i32> {
    let output = match &cmd.output {
        Some(path) => StreamOutput::File(path.clone()),
        None => StreamOutput::Terminal,
    };

    let emitter = FindingEmitter::new(output);
    let mut scanner = SecurityScanner::new().with_emitter(emitter);

    let result = scanner.scan(&cmd)?;

    // Check threshold
    let exit_code = check_threshold(result.score, cmd.fail_under);

    if exit_code == exit_codes::SECURITY_VIOLATION {
        return Err(SecurityError::ThresholdError {
            score: result.score,
            threshold: cmd.fail_under.unwrap_or(0),
        });
    }

    Ok(exit_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_from_str() {
        assert_eq!(OutputFormat::from_str("binary"), Some(OutputFormat::Binary));
        assert_eq!(OutputFormat::from_str("sr"), Some(OutputFormat::Binary));
        assert_eq!(OutputFormat::from_str("json"), Some(OutputFormat::Json));
        assert_eq!(OutputFormat::from_str("markdown"), Some(OutputFormat::Markdown));
        assert_eq!(OutputFormat::from_str("md"), Some(OutputFormat::Markdown));
        assert_eq!(OutputFormat::from_str("terminal"), Some(OutputFormat::Terminal));
        assert_eq!(OutputFormat::from_str("term"), Some(OutputFormat::Terminal));
        assert_eq!(OutputFormat::from_str("invalid"), None);
    }

    #[test]
    fn test_security_command_builder() {
        let cmd = SecurityCommand::new(PathBuf::from("/test"))
            .with_fail_under(80)
            .with_signing()
            .with_output(PathBuf::from("report.dx"))
            .with_format(OutputFormat::Binary)
            .with_verbose()
            .without_incremental();

        assert_eq!(cmd.path, PathBuf::from("/test"));
        assert_eq!(cmd.fail_under, Some(80));
        assert!(cmd.sign);
        assert_eq!(cmd.output, Some(PathBuf::from("report.dx")));
        assert_eq!(cmd.format, OutputFormat::Binary);
        assert!(cmd.verbose);
        assert!(!cmd.incremental);
    }

    #[test]
    fn test_check_threshold_success() {
        assert_eq!(check_threshold(100, Some(80)), exit_codes::SUCCESS);
        assert_eq!(check_threshold(80, Some(80)), exit_codes::SUCCESS);
        assert_eq!(check_threshold(50, None), exit_codes::SUCCESS);
    }

    #[test]
    fn test_check_threshold_violation() {
        assert_eq!(check_threshold(79, Some(80)), exit_codes::SECURITY_VIOLATION);
        assert_eq!(check_threshold(0, Some(1)), exit_codes::SECURITY_VIOLATION);
    }

    #[test]
    fn test_should_skip_file() {
        assert!(should_skip_file(Path::new("node_modules/package/index.js")));
        assert!(should_skip_file(Path::new("target/debug/binary")));
        assert!(should_skip_file(Path::new(".git/objects/abc")));
        assert!(should_skip_file(Path::new("image.png")));
        assert!(should_skip_file(Path::new("archive.zip")));

        assert!(!should_skip_file(Path::new("src/main.rs")));
        assert!(!should_skip_file(Path::new("lib/utils.js")));
    }

    #[test]
    fn test_security_scanner_new() {
        let scanner = SecurityScanner::new();
        // Just verify it creates without panic
        assert!(matches!(
            scanner.simd_mode(),
            SimdMode::Scalar | SimdMode::Avx2 | SimdMode::Avx512 | SimdMode::Neon
        ));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: dx-security, Property 9: CLI Threshold Exit Code**
        /// **Validates: Requirements 11.1, 11.3**
        ///
        /// For any security score S and threshold T:
        /// - If S < T, the CLI SHALL exit with non-zero status
        /// - If S >= T, the CLI SHALL exit with zero status
        #[test]
        fn prop_cli_threshold_exit_code(score in 0u8..=100, threshold in 0u8..=100) {
            let exit_code = check_threshold(score, Some(threshold));

            if score < threshold {
                prop_assert_eq!(
                    exit_code,
                    exit_codes::SECURITY_VIOLATION,
                    "Score {} < threshold {} should return SECURITY_VIOLATION",
                    score,
                    threshold
                );
            } else {
                prop_assert_eq!(
                    exit_code,
                    exit_codes::SUCCESS,
                    "Score {} >= threshold {} should return SUCCESS",
                    score,
                    threshold
                );
            }
        }

        /// No threshold should always succeed
        #[test]
        fn prop_no_threshold_always_succeeds(score in 0u8..=100) {
            let exit_code = check_threshold(score, None);
            prop_assert_eq!(
                exit_code,
                exit_codes::SUCCESS,
                "No threshold should always return SUCCESS"
            );
        }

        /// Threshold of 0 should always succeed (any score >= 0)
        #[test]
        fn prop_zero_threshold_always_succeeds(score in 0u8..=100) {
            let exit_code = check_threshold(score, Some(0));
            prop_assert_eq!(
                exit_code,
                exit_codes::SUCCESS,
                "Threshold of 0 should always return SUCCESS"
            );
        }

        /// Threshold of 101 should always fail (no score can reach it)
        /// Note: We use 100 as max threshold since u8 max is 255
        #[test]
        fn prop_max_threshold_behavior(score in 0u8..100) {
            // Score less than 100 with threshold 100 should fail
            let exit_code = check_threshold(score, Some(100));
            prop_assert_eq!(
                exit_code,
                exit_codes::SECURITY_VIOLATION,
                "Score {} < 100 should fail with threshold 100",
                score
            );
        }
    }
}
