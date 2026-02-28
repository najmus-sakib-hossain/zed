//! Scale-Adaptive Intelligence
//!
//! Automatically detects project scale and adjusts workflow recommendations.

mod analyzers;
mod detector;

#[cfg(test)]
mod property_tests;

pub use analyzers::*;
pub use detector::ScaleDetector;

use serde::{Deserialize, Serialize};

/// Project scale classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectScale {
    /// Quick fix - minimal planning, rapid execution
    BugFix,
    /// Feature - standard workflow with tech spec
    Feature,
    /// Product - full BMAD workflow with PRD and architecture
    Product,
    /// Enterprise - governance-enhanced workflow with compliance
    Enterprise,
}

impl std::fmt::Display for ProjectScale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectScale::BugFix => write!(f, "Bug Fix"),
            ProjectScale::Feature => write!(f, "Feature"),
            ProjectScale::Product => write!(f, "Product"),
            ProjectScale::Enterprise => write!(f, "Enterprise"),
        }
    }
}

impl ProjectScale {
    /// Get recommended workflows for this scale
    pub fn recommended_workflows(&self) -> Vec<&'static str> {
        match self {
            ProjectScale::BugFix => vec!["quick-bug-fix", "quick-refactor"],
            ProjectScale::Feature => vec!["quick-feature", "tech-spec", "dev-story", "code-review"],
            ProjectScale::Product => vec![
                "product-brief",
                "prd",
                "architecture",
                "epics-and-stories",
                "sprint-planning",
                "dev-story",
                "code-review",
                "retrospective",
            ],
            ProjectScale::Enterprise => vec![
                "product-brief",
                "competitive-analysis",
                "prd",
                "architecture",
                "api-design",
                "data-model",
                "security-review",
                "epics-and-stories",
                "implementation-readiness",
                "sprint-planning",
                "dev-story",
                "code-review",
                "test-design",
                "test-automation",
                "deployment",
                "retrospective",
            ],
        }
    }

    /// Get the minimum recommended agents for this scale
    pub fn recommended_agents(&self) -> Vec<&'static str> {
        match self {
            ProjectScale::BugFix => vec!["developer", "reviewer"],
            ProjectScale::Feature => vec!["developer", "architect", "reviewer", "test-architect"],
            ProjectScale::Product => vec![
                "pm",
                "architect",
                "developer",
                "ux-designer",
                "test-architect",
                "reviewer",
                "scrum-master",
            ],
            ProjectScale::Enterprise => vec![
                "pm",
                "architect",
                "developer",
                "ux-designer",
                "test-architect",
                "analyst",
                "tech-writer",
                "scrum-master",
                "security",
                "performance",
                "devops",
                "data-engineer",
                "reviewer",
            ],
        }
    }
}

/// Result of scale detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaleRecommendation {
    /// Detected project scale
    pub detected_scale: ProjectScale,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Reasoning for the detection
    pub reasoning: Vec<String>,
    /// Suggested workflows for this scale
    pub suggested_workflows: Vec<String>,
    /// Suggested agents for this scale
    pub suggested_agents: Vec<String>,
}

impl ScaleRecommendation {
    /// Create a new scale recommendation
    pub fn new(scale: ProjectScale, confidence: f32) -> Self {
        Self {
            detected_scale: scale,
            confidence,
            reasoning: Vec::new(),
            suggested_workflows: scale
                .recommended_workflows()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            suggested_agents: scale.recommended_agents().iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Add a reasoning point
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reasoning.push(reason.into());
        self
    }

    /// Add multiple reasoning points
    pub fn with_reasons(mut self, reasons: Vec<String>) -> Self {
        self.reasoning.extend(reasons);
        self
    }
}

/// Context for scale detection
#[derive(Debug, Clone, Default)]
pub struct ProjectContext {
    /// Root path of the project
    pub root_path: Option<std::path::PathBuf>,
    /// Total lines of code
    pub lines_of_code: Option<usize>,
    /// Number of files
    pub file_count: Option<usize>,
    /// Number of dependencies
    pub dependency_count: Option<usize>,
    /// Number of contributors
    pub contributor_count: Option<usize>,
    /// Commit count
    pub commit_count: Option<usize>,
    /// Whether the project has CI/CD
    pub has_ci_cd: bool,
    /// Whether the project has tests
    pub has_tests: bool,
    /// Whether the project has documentation
    pub has_docs: bool,
    /// Project type (rust, typescript, python, etc.)
    pub project_type: Option<String>,
    /// Manual scale override
    pub scale_override: Option<ProjectScale>,
}

impl ProjectContext {
    /// Create a new project context
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the root path
    pub fn with_root_path(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.root_path = Some(path.into());
        self
    }

    /// Set lines of code
    pub fn with_lines_of_code(mut self, loc: usize) -> Self {
        self.lines_of_code = Some(loc);
        self
    }

    /// Set file count
    pub fn with_file_count(mut self, count: usize) -> Self {
        self.file_count = Some(count);
        self
    }

    /// Set dependency count
    pub fn with_dependency_count(mut self, count: usize) -> Self {
        self.dependency_count = Some(count);
        self
    }

    /// Set contributor count
    pub fn with_contributor_count(mut self, count: usize) -> Self {
        self.contributor_count = Some(count);
        self
    }

    /// Set commit count
    pub fn with_commit_count(mut self, count: usize) -> Self {
        self.commit_count = Some(count);
        self
    }

    /// Set CI/CD status
    pub fn with_ci_cd(mut self, has_ci_cd: bool) -> Self {
        self.has_ci_cd = has_ci_cd;
        self
    }

    /// Set tests status
    pub fn with_tests(mut self, has_tests: bool) -> Self {
        self.has_tests = has_tests;
        self
    }

    /// Set docs status
    pub fn with_docs(mut self, has_docs: bool) -> Self {
        self.has_docs = has_docs;
        self
    }

    /// Set project type
    pub fn with_project_type(mut self, project_type: impl Into<String>) -> Self {
        self.project_type = Some(project_type.into());
        self
    }

    /// Set manual scale override
    pub fn with_scale_override(mut self, scale: ProjectScale) -> Self {
        self.scale_override = Some(scale);
        self
    }
}
