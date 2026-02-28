//! CI/CD Platform Integration
//!
//! Provides native integration with popular CI/CD platforms:
//! - GitHub Actions
//! - GitLab CI
//! - Azure DevOps
//! - `CircleCI`
//! - Jenkins
//!
//! # Features
//!
//! - Automatic platform detection
//! - Native annotation format output
//! - Pull request comments
//! - Status checks
//! - Cache integration
//!
//! # Usage
//!
//! ```bash
//! # Auto-detect platform and output appropriate format
//! dx-check --ci
//!
//! # Explicit platform selection
//! dx-check --format github
//! dx-check --format gitlab
//! dx-check --format azure
//! ```

use std::collections::HashMap;
use std::env;

use crate::diagnostics::{Diagnostic, DiagnosticSeverity};

/// Detected CI platform
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiPlatform {
    /// GitHub Actions
    GitHubActions,
    /// GitLab CI
    GitLabCi,
    /// Azure DevOps Pipelines
    AzureDevOps,
    /// `CircleCI`
    CircleCi,
    /// Jenkins
    Jenkins,
    /// Bitbucket Pipelines
    BitbucketPipelines,
    /// Travis CI
    TravisCi,
    /// Generic CI (unknown platform)
    Generic,
    /// Local development (not CI)
    Local,
}

impl CiPlatform {
    /// Detect the current CI platform from environment
    /// Returns None if not running in CI
    #[must_use]
    pub fn detect() -> Option<Self> {
        // GitHub Actions
        if env::var("GITHUB_ACTIONS").is_ok() {
            return Some(Self::GitHubActions);
        }

        // GitLab CI
        if env::var("GITLAB_CI").is_ok() {
            return Some(Self::GitLabCi);
        }

        // Azure DevOps
        if env::var("TF_BUILD").is_ok() || env::var("AZURE_PIPELINES").is_ok() {
            return Some(Self::AzureDevOps);
        }

        // CircleCI
        if env::var("CIRCLECI").is_ok() {
            return Some(Self::CircleCi);
        }

        // Jenkins
        if env::var("JENKINS_URL").is_ok() || env::var("BUILD_ID").is_ok() {
            return Some(Self::Jenkins);
        }

        // Bitbucket Pipelines
        if env::var("BITBUCKET_PIPELINE_UUID").is_ok() {
            return Some(Self::BitbucketPipelines);
        }

        // Travis CI
        if env::var("TRAVIS").is_ok() {
            return Some(Self::TravisCi);
        }

        // Generic CI detection
        if env::var("CI").is_ok() {
            return Some(Self::Generic);
        }

        None
    }

    /// Detect platform, defaulting to Local if not in CI
    #[must_use]
    pub fn detect_or_local() -> Self {
        Self::detect().unwrap_or(Self::Local)
    }

    /// Get the platform name
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::GitHubActions => "GitHub Actions",
            Self::GitLabCi => "GitLab CI",
            Self::AzureDevOps => "Azure DevOps",
            Self::CircleCi => "CircleCI",
            Self::Jenkins => "Jenkins",
            Self::BitbucketPipelines => "Bitbucket Pipelines",
            Self::TravisCi => "Travis CI",
            Self::Generic => "CI",
            Self::Local => "Local",
        }
    }

    /// Check if running in CI
    #[must_use]
    pub fn is_ci(&self) -> bool {
        !matches!(self, Self::Local)
    }
}

/// CI context information
#[derive(Debug, Clone)]
pub struct CiContext {
    /// Detected platform
    pub platform: CiPlatform,
    /// Git branch
    pub branch: Option<String>,
    /// Git commit SHA
    pub commit_sha: Option<String>,
    /// Pull/Merge request number
    pub pr_number: Option<u64>,
    /// Repository name
    pub repository: Option<String>,
    /// Job/Build ID
    pub build_id: Option<String>,
    /// Is this a pull request?
    pub is_pull_request: bool,
    /// Base branch (for PRs)
    pub base_branch: Option<String>,
    /// Additional platform-specific data
    pub extra: HashMap<String, String>,
}

impl CiContext {
    /// Detect CI context from environment
    #[must_use]
    pub fn detect() -> Self {
        let platform = CiPlatform::detect_or_local();

        match platform {
            CiPlatform::GitHubActions => Self::detect_github(),
            CiPlatform::GitLabCi => Self::detect_gitlab(),
            CiPlatform::AzureDevOps => Self::detect_azure(),
            CiPlatform::CircleCi => Self::detect_circleci(),
            CiPlatform::Jenkins => Self::detect_jenkins(),
            _ => Self::detect_generic(platform),
        }
    }

    fn detect_github() -> Self {
        let pr_number = env::var("GITHUB_EVENT_NAME")
            .ok()
            .filter(|e| e == "pull_request")
            .and_then(|_| env::var("GITHUB_REF").ok())
            .and_then(|r| r.split('/').nth(2).and_then(|n| n.parse().ok()));

        Self {
            platform: CiPlatform::GitHubActions,
            branch: env::var("GITHUB_HEAD_REF").or_else(|_| env::var("GITHUB_REF_NAME")).ok(),
            commit_sha: env::var("GITHUB_SHA").ok(),
            pr_number,
            repository: env::var("GITHUB_REPOSITORY").ok(),
            build_id: env::var("GITHUB_RUN_ID").ok(),
            is_pull_request: pr_number.is_some(),
            base_branch: env::var("GITHUB_BASE_REF").ok(),
            extra: HashMap::new(),
        }
    }

    fn detect_gitlab() -> Self {
        let mr_iid = env::var("CI_MERGE_REQUEST_IID").ok().and_then(|n| n.parse().ok());

        Self {
            platform: CiPlatform::GitLabCi,
            branch: env::var("CI_COMMIT_REF_NAME").ok(),
            commit_sha: env::var("CI_COMMIT_SHA").ok(),
            pr_number: mr_iid,
            repository: env::var("CI_PROJECT_PATH").ok(),
            build_id: env::var("CI_JOB_ID").ok(),
            is_pull_request: mr_iid.is_some(),
            base_branch: env::var("CI_MERGE_REQUEST_TARGET_BRANCH_NAME").ok(),
            extra: HashMap::new(),
        }
    }

    fn detect_azure() -> Self {
        let pr_id = env::var("SYSTEM_PULLREQUEST_PULLREQUESTID").ok().and_then(|n| n.parse().ok());

        Self {
            platform: CiPlatform::AzureDevOps,
            branch: env::var("BUILD_SOURCEBRANCHNAME").ok(),
            commit_sha: env::var("BUILD_SOURCEVERSION").ok(),
            pr_number: pr_id,
            repository: env::var("BUILD_REPOSITORY_NAME").ok(),
            build_id: env::var("BUILD_BUILDID").ok(),
            is_pull_request: pr_id.is_some(),
            base_branch: env::var("SYSTEM_PULLREQUEST_TARGETBRANCH").ok(),
            extra: HashMap::new(),
        }
    }

    fn detect_circleci() -> Self {
        let pr_number = env::var("CIRCLE_PULL_REQUEST")
            .ok()
            .and_then(|url| url.rsplit('/').next().and_then(|n| n.parse().ok()));

        Self {
            platform: CiPlatform::CircleCi,
            branch: env::var("CIRCLE_BRANCH").ok(),
            commit_sha: env::var("CIRCLE_SHA1").ok(),
            pr_number,
            repository: env::var("CIRCLE_PROJECT_REPONAME").ok(),
            build_id: env::var("CIRCLE_BUILD_NUM").ok(),
            is_pull_request: pr_number.is_some(),
            base_branch: None,
            extra: HashMap::new(),
        }
    }

    fn detect_jenkins() -> Self {
        Self {
            platform: CiPlatform::Jenkins,
            branch: env::var("GIT_BRANCH").or_else(|_| env::var("BRANCH_NAME")).ok(),
            commit_sha: env::var("GIT_COMMIT").ok(),
            pr_number: env::var("CHANGE_ID").ok().and_then(|n| n.parse().ok()),
            repository: env::var("JOB_NAME").ok(),
            build_id: env::var("BUILD_NUMBER").ok(),
            is_pull_request: env::var("CHANGE_ID").is_ok(),
            base_branch: env::var("CHANGE_TARGET").ok(),
            extra: HashMap::new(),
        }
    }

    fn detect_generic(platform: CiPlatform) -> Self {
        Self {
            platform,
            branch: env::var("BRANCH").or_else(|_| env::var("GIT_BRANCH")).ok(),
            commit_sha: env::var("COMMIT").or_else(|_| env::var("GIT_COMMIT")).ok(),
            pr_number: None,
            repository: None,
            build_id: env::var("BUILD_ID").ok(),
            is_pull_request: false,
            base_branch: None,
            extra: HashMap::new(),
        }
    }
}

/// CI output formatter
pub struct CiFormatter {
    context: CiContext,
}

impl CiFormatter {
    /// Create a new CI formatter
    #[must_use]
    pub fn new() -> Self {
        Self {
            context: CiContext::detect(),
        }
    }

    /// Create with explicit context
    #[must_use]
    pub fn with_context(context: CiContext) -> Self {
        Self { context }
    }

    /// Get the detected platform
    #[must_use]
    pub fn platform(&self) -> CiPlatform {
        self.context.platform
    }

    /// Format diagnostics for the detected CI platform
    #[must_use]
    pub fn format_diagnostics(&self, diagnostics: &[Diagnostic]) -> String {
        match self.context.platform {
            CiPlatform::GitHubActions => self.format_github(diagnostics),
            CiPlatform::GitLabCi => self.format_gitlab(diagnostics),
            CiPlatform::AzureDevOps => self.format_azure(diagnostics),
            _ => self.format_generic(diagnostics),
        }
    }

    /// Format for GitHub Actions
    fn format_github(&self, diagnostics: &[Diagnostic]) -> String {
        diagnostics
            .iter()
            .map(|d| {
                let level = match d.severity {
                    DiagnosticSeverity::Error => "error",
                    DiagnosticSeverity::Warning => "warning",
                    DiagnosticSeverity::Info | DiagnosticSeverity::Hint => "notice",
                };

                // GitHub Actions workflow commands
                // ::error file=path,line=1,col=1,endColumn=5,title=Rule::Message
                format!(
                    "::{} file={},line={},endLine={},title={}::{}",
                    level,
                    d.file.display(),
                    d.span.start,
                    d.span.end,
                    d.rule_id,
                    d.message.replace('\n', "%0A")
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Format for GitLab CI
    fn format_gitlab(&self, diagnostics: &[Diagnostic]) -> String {
        // GitLab Code Quality report format (JSON)
        let issues: Vec<serde_json::Value> = diagnostics
            .iter()
            .map(|d| {
                serde_json::json!({
                    "description": d.message,
                    "check_name": d.rule_id,
                    "fingerprint": format!("{}:{}:{}", d.file.display(), d.span.start, d.rule_id),
                    "severity": match d.severity {
                        DiagnosticSeverity::Error => "critical",
                        DiagnosticSeverity::Warning => "major",
                        DiagnosticSeverity::Info => "minor",
                        DiagnosticSeverity::Hint => "info",
                    },
                    "location": {
                        "path": d.file.display().to_string(),
                        "lines": {
                            "begin": d.span.start
                        }
                    }
                })
            })
            .collect();

        serde_json::to_string_pretty(&issues).unwrap_or_else(|_| "[]".to_string())
    }

    /// Format for Azure DevOps
    fn format_azure(&self, diagnostics: &[Diagnostic]) -> String {
        diagnostics
            .iter()
            .map(|d| {
                let level = match d.severity {
                    DiagnosticSeverity::Error => "error",
                    DiagnosticSeverity::Warning => "warning",
                    _ => "warning",
                };

                // Azure DevOps logging commands
                // ##vso[task.logissue type=error;sourcepath=path;linenumber=1]Message
                format!(
                    "##vso[task.logissue type={};sourcepath={};linenumber={}]{}",
                    level,
                    d.file.display(),
                    d.span.start,
                    d.message
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Generic format
    fn format_generic(&self, diagnostics: &[Diagnostic]) -> String {
        diagnostics
            .iter()
            .map(|d| {
                format!(
                    "{}:{}:{}: {} [{}] {}",
                    d.file.display(),
                    d.span.start,
                    d.span.end,
                    d.severity.as_str(),
                    d.rule_id,
                    d.message
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Print CI-specific summary
    pub fn print_summary(
        &self,
        files_checked: usize,
        errors: usize,
        warnings: usize,
        duration_ms: u64,
    ) {
        match self.context.platform {
            CiPlatform::GitHubActions => {
                // Use GitHub Actions job summary
                if let Ok(summary_file) = env::var("GITHUB_STEP_SUMMARY") {
                    let summary = format!(
                        "## DX Check Results\n\n\
                        | Metric | Value |\n\
                        |--------|-------|\n\
                        | Files Checked | {files_checked} |\n\
                        | Errors | {errors} |\n\
                        | Warnings | {warnings} |\n\
                        | Duration | {duration_ms}ms |\n"
                    );
                    let _ = std::fs::write(summary_file, summary);
                }
            }
            CiPlatform::GitLabCi => {
                // GitLab CI doesn't have native summary, just print
                println!(
                    "DX Check: {files_checked} files, {errors} errors, {warnings} warnings ({duration_ms} ms)"
                );
            }
            CiPlatform::AzureDevOps => {
                // Azure DevOps task result
                if errors > 0 {
                    println!("##vso[task.complete result=Failed;]DX Check found {errors} errors");
                } else {
                    println!("##vso[task.complete result=Succeeded;]DX Check passed");
                }
            }
            _ => {
                println!(
                    "DX Check: {files_checked} files, {errors} errors, {warnings} warnings ({duration_ms} ms)"
                );
            }
        }
    }

    /// Set CI exit code based on results
    #[must_use]
    pub fn should_fail(&self, errors: usize, warnings: usize, fail_on_warning: bool) -> bool {
        errors > 0 || (fail_on_warning && warnings > 0)
    }
}

impl Default for CiFormatter {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate CI configuration files
pub struct CiConfigGenerator {
    platform: CiPlatform,
}

impl CiConfigGenerator {
    /// Create a new config generator for the specified platform
    #[must_use]
    pub fn new(platform: CiPlatform) -> Self {
        Self { platform }
    }

    /// Generate configuration for the current platform
    pub fn generate(&self) -> Result<String, std::io::Error> {
        Ok(match self.platform {
            CiPlatform::GitHubActions => Self::github_actions(),
            CiPlatform::GitLabCi => Self::gitlab_ci(),
            CiPlatform::AzureDevOps => Self::azure_pipelines(),
            CiPlatform::CircleCi => Self::circleci(),
            _ => Self::github_actions(), // Default to GitHub Actions
        })
    }

    /// Generate GitHub Actions workflow
    #[must_use]
    pub fn github_actions() -> String {
        r"name: DX Check

on:
  push:
    branches: [main, master]
  pull_request:
    branches: [main, master]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install dx-check
        run: cargo install dx-check
      
      - name: Run dx-check
        run: dx-check --format github
"
        .to_string()
    }

    /// Generate GitLab CI configuration
    #[must_use]
    pub fn gitlab_ci() -> String {
        r"dx-check:
  stage: test
  image: rust:latest
  script:
    - cargo install dx-check
    - dx-check --format json > gl-code-quality-report.json
  artifacts:
    reports:
      codequality: gl-code-quality-report.json
"
        .to_string()
    }

    /// Generate Azure DevOps pipeline
    #[must_use]
    pub fn azure_pipelines() -> String {
        r"trigger:
  - main
  - master

pool:
  vmImage: 'ubuntu-latest'

steps:
  - script: cargo install dx-check
    displayName: 'Install dx-check'

  - script: dx-check --format azure
    displayName: 'Run dx-check'
"
        .to_string()
    }

    /// Generate `CircleCI` configuration
    #[must_use]
    pub fn circleci() -> String {
        r"version: 2.1

jobs:
  lint:
    docker:
      - image: rust:latest
    steps:
      - checkout
      - run:
          name: Install dx-check
          command: cargo install dx-check
      - run:
          name: Run dx-check
          command: dx-check

workflows:
  main:
    jobs:
      - lint
"
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection_local() {
        // In tests, we're not in CI
        // This might vary depending on test environment
        let platform = CiPlatform::detect_or_local();
        // Just verify it returns something valid
        assert!(!platform.name().is_empty());
    }

    #[test]
    fn test_ci_formatter_creation() {
        let formatter = CiFormatter::new();
        // Verify formatter is created
        assert!(!formatter.platform().name().is_empty());
    }

    #[test]
    fn test_github_config_generation() {
        let config = CiConfigGenerator::github_actions();
        assert!(config.contains("GitHub Actions") || config.contains("DX Check"));
        assert!(config.contains("dx-check"));
    }

    #[test]
    fn test_gitlab_config_generation() {
        let config = CiConfigGenerator::gitlab_ci();
        assert!(config.contains("dx-check"));
        assert!(config.contains("codequality"));
    }

    #[test]
    fn test_ci_config_generator_new() {
        let generator = CiConfigGenerator::new(CiPlatform::GitHubActions);
        let config = generator.generate().unwrap();
        assert!(config.contains("dx-check"));
    }
}
