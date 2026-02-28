//! Main publisher that orchestrates the publish workflow.
//!
//! Combines validation, packaging, signing, and submission
//! into a single unified workflow.

use std::path::PathBuf;

use super::config::PublishConfig;
use super::package::{AuthorInfo, Package, PackageFormat, PackageManifest};
use super::pipeline::{PipelineResult, PublishPipeline};
use super::signing::Ed25519Signer;
use super::submission::{GitHubSubmission, PluginInfo, PullRequest};

/// Result of a publish operation.
#[derive(Debug)]
pub enum PublishResult {
    /// Plugin published successfully
    Success(PublishSuccess),
    /// Validation failed
    ValidationFailed(PipelineResult),
    /// Packaging failed
    PackagingFailed(String),
    /// Signing failed
    SigningFailed(String),
    /// Submission failed
    SubmissionFailed(String),
    /// Dry run completed
    DryRun(DryRunResult),
}

/// Successful publish result.
#[derive(Debug)]
pub struct PublishSuccess {
    /// Pull request created
    pub pull_request: PullRequest,
    /// Package file path
    pub package_path: PathBuf,
    /// Package checksum
    pub checksum: String,
    /// Signature
    pub signature: String,
}

/// Dry run result.
#[derive(Debug)]
pub struct DryRunResult {
    /// Validation result
    pub validation: PipelineResult,
    /// Would create package at
    pub package_path: PathBuf,
    /// Package size
    pub package_size: u64,
}

/// Main publisher for DX plugins.
#[derive(Debug)]
pub struct Publisher {
    config: PublishConfig,
}

impl Publisher {
    /// Create a new publisher with the given configuration.
    pub fn new(config: PublishConfig) -> Result<Self, PublishError> {
        // Validate source path exists
        if !config.source_path.exists() {
            return Err(PublishError::SourceNotFound(config.source_path.clone()));
        }

        Ok(Self { config })
    }

    /// Run the publish workflow.
    pub async fn publish(&self) -> PublishResult {
        // Step 1: Run validation pipeline
        if !self.config.skip_validation {
            let mut pipeline = PublishPipeline::new(&self.config.source_path);
            let validation_result = pipeline.run();

            if !validation_result.passed {
                return PublishResult::ValidationFailed(validation_result);
            }

            // If dry run, return here
            if self.config.dry_run {
                return PublishResult::DryRun(DryRunResult {
                    validation: validation_result,
                    package_path: self
                        .config
                        .output_path
                        .join(format!("{}-{}.dxp", self.config.name, self.config.version)),
                    package_size: estimate_package_size(&self.config.source_path),
                });
            }
        }

        // Step 2: Create package
        let manifest = PackageManifest {
            name: self.config.name.clone(),
            version: self.config.version.clone(),
            description: self.config.description.clone(),
            author: AuthorInfo {
                name: String::new(),
                email: self.config.author_email.clone(),
                github: self.config.github_username.clone(),
            },
            ..PackageManifest::new(&self.config.name, &self.config.version)
        };

        let package = match Package::from_directory(
            manifest,
            &self.config.source_path,
            PackageFormat::DxPackage,
        ) {
            Ok(p) => p,
            Err(e) => return PublishResult::PackagingFailed(e.to_string()),
        };

        // Step 3: Write package
        let package_path = match package.write_to(&self.config.output_path) {
            Ok(p) => p,
            Err(e) => return PublishResult::PackagingFailed(e.to_string()),
        };

        // Step 4: Sign package
        let signer = if let Some(ref key_path) = self.config.signing_key_path {
            match Ed25519Signer::from_file(key_path) {
                Ok(s) => s,
                Err(e) => return PublishResult::SigningFailed(e.to_string()),
            }
        } else {
            Ed25519Signer::generate()
        };

        let package_bytes = match std::fs::read(&package_path) {
            Ok(b) => b,
            Err(e) => return PublishResult::SigningFailed(e.to_string()),
        };

        let signature = signer.sign(&package_bytes);

        // Step 5: Submit to repository (if not dry run)
        if self.config.dry_run {
            let validation = PublishPipeline::new(&self.config.source_path).run();
            return PublishResult::DryRun(DryRunResult {
                validation,
                package_path,
                package_size: package.size,
            });
        }

        let submission = GitHubSubmission::new(&self.config.target_repo);

        let plugin_info = PluginInfo {
            name: self.config.name.clone(),
            version: self.config.version.clone(),
            description: self.config.description.clone(),
            author_name: String::new(),
            author_email: self.config.author_email.clone(),
            github_username: self.config.github_username.clone(),
        };

        match submission.submit(&package_path, &plugin_info).await {
            Ok(pr) => PublishResult::Success(PublishSuccess {
                pull_request: pr,
                package_path,
                checksum: format!("{:?}", package.checksum),
                signature: signature.signature_hex(),
            }),
            Err(e) => PublishResult::SubmissionFailed(e.to_string()),
        }
    }

    /// Get validation report without publishing.
    pub fn validate(&self) -> PipelineResult {
        let mut pipeline = PublishPipeline::new(&self.config.source_path);
        pipeline.run()
    }

    /// Create package without submitting.
    pub fn package_only(&self) -> Result<PathBuf, PublishError> {
        let manifest = PackageManifest {
            name: self.config.name.clone(),
            version: self.config.version.clone(),
            description: self.config.description.clone(),
            ..PackageManifest::new(&self.config.name, &self.config.version)
        };

        let package =
            Package::from_directory(manifest, &self.config.source_path, PackageFormat::DxPackage)
                .map_err(|e| PublishError::PackagingFailed(e.to_string()))?;

        package
            .write_to(&self.config.output_path)
            .map_err(|e| PublishError::PackagingFailed(e.to_string()))
    }
}

/// Errors that can occur during publishing.
#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error("Source directory not found: {0}")]
    SourceNotFound(PathBuf),

    #[error("Packaging failed: {0}")]
    PackagingFailed(String),

    #[error("Signing failed: {0}")]
    SigningFailed(String),

    #[error("Submission failed: {0}")]
    SubmissionFailed(String),
}

/// Estimate package size before creating it.
fn estimate_package_size(source_path: &std::path::Path) -> u64 {
    let mut size = 0u64;

    if let Ok(entries) = std::fs::read_dir(source_path) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    size += metadata.len();
                } else if metadata.is_dir() {
                    size += estimate_package_size(&entry.path());
                }
            }
        }
    }

    size
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::publish::submission::SubmissionStatus;

    #[test]
    fn test_publish_result_variants() {
        // Just testing that all variants exist and can be created
        let _success = PublishResult::Success(PublishSuccess {
            pull_request: PullRequest {
                number: 1,
                url: String::new(),
                title: String::new(),
                body: String::new(),
                branch: String::new(),
                status: SubmissionStatus::Pending,
                auto_merge_enabled: true,
            },
            package_path: PathBuf::new(),
            checksum: String::new(),
            signature: String::new(),
        });

        let _dry_run = PublishResult::DryRun(DryRunResult {
            validation: PublishPipeline::new(std::path::Path::new(".")).run(),
            package_path: PathBuf::new(),
            package_size: 0,
        });
    }
}
