//! Scale Analyzers
//!
//! Individual analyzers that contribute to scale detection.

use super::{ProjectContext, ProjectScale};

/// Trait for scale analyzers
pub trait ScaleAnalyzer: Send + Sync {
    /// Analyze the project context and return a scale suggestion with confidence
    fn analyze(&self, context: &ProjectContext) -> Option<(ProjectScale, f32)>;

    /// Get the name of this analyzer
    fn name(&self) -> &'static str;
}

/// Analyzes project scale based on file size metrics
#[derive(Debug, Default)]
pub struct FileSizeAnalyzer;

impl FileSizeAnalyzer {
    /// Create a new file size analyzer
    pub fn new() -> Self {
        Self
    }
}

impl ScaleAnalyzer for FileSizeAnalyzer {
    fn analyze(&self, context: &ProjectContext) -> Option<(ProjectScale, f32)> {
        let loc = context.lines_of_code?;
        let file_count = context.file_count.unwrap_or(0);

        // Scale thresholds based on lines of code
        let (scale, confidence) = match loc {
            0..=500 => (ProjectScale::BugFix, 0.9),
            501..=5_000 => (ProjectScale::Feature, 0.8),
            5_001..=50_000 => (ProjectScale::Product, 0.7),
            _ => (ProjectScale::Enterprise, 0.8),
        };

        // Adjust confidence based on file count correlation
        let adjusted_confidence = if file_count > 0 {
            let expected_files = match scale {
                ProjectScale::BugFix => 1..=10,
                ProjectScale::Feature => 5..=50,
                ProjectScale::Product => 20..=500,
                ProjectScale::Enterprise => 100..=10000,
            };

            if expected_files.contains(&file_count) {
                confidence
            } else {
                confidence * 0.8
            }
        } else {
            confidence * 0.9
        };

        Some((scale, adjusted_confidence))
    }

    fn name(&self) -> &'static str {
        "FileSizeAnalyzer"
    }
}

/// Analyzes project scale based on dependency count
#[derive(Debug, Default)]
pub struct DependencyAnalyzer;

impl DependencyAnalyzer {
    /// Create a new dependency analyzer
    pub fn new() -> Self {
        Self
    }
}

impl ScaleAnalyzer for DependencyAnalyzer {
    fn analyze(&self, context: &ProjectContext) -> Option<(ProjectScale, f32)> {
        let deps = context.dependency_count?;

        let (scale, confidence) = match deps {
            0..=5 => (ProjectScale::BugFix, 0.6),
            6..=20 => (ProjectScale::Feature, 0.7),
            21..=100 => (ProjectScale::Product, 0.7),
            _ => (ProjectScale::Enterprise, 0.8),
        };

        Some((scale, confidence))
    }

    fn name(&self) -> &'static str {
        "DependencyAnalyzer"
    }
}

/// Analyzes project scale based on team size (contributors)
#[derive(Debug, Default)]
pub struct TeamSizeAnalyzer;

impl TeamSizeAnalyzer {
    /// Create a new team size analyzer
    pub fn new() -> Self {
        Self
    }
}

impl ScaleAnalyzer for TeamSizeAnalyzer {
    fn analyze(&self, context: &ProjectContext) -> Option<(ProjectScale, f32)> {
        let contributors = context.contributor_count?;

        let (scale, confidence) = match contributors {
            0..=1 => (ProjectScale::BugFix, 0.5),
            2..=3 => (ProjectScale::Feature, 0.7),
            4..=10 => (ProjectScale::Product, 0.8),
            _ => (ProjectScale::Enterprise, 0.9),
        };

        Some((scale, confidence))
    }

    fn name(&self) -> &'static str {
        "TeamSizeAnalyzer"
    }
}

/// Analyzes project scale based on git history
#[derive(Debug, Default)]
pub struct HistoryAnalyzer;

impl HistoryAnalyzer {
    /// Create a new history analyzer
    pub fn new() -> Self {
        Self
    }
}

impl ScaleAnalyzer for HistoryAnalyzer {
    fn analyze(&self, context: &ProjectContext) -> Option<(ProjectScale, f32)> {
        let commits = context.commit_count?;

        let (scale, confidence) = match commits {
            0..=10 => (ProjectScale::BugFix, 0.5),
            11..=100 => (ProjectScale::Feature, 0.6),
            101..=1000 => (ProjectScale::Product, 0.7),
            _ => (ProjectScale::Enterprise, 0.8),
        };

        Some((scale, confidence))
    }

    fn name(&self) -> &'static str {
        "HistoryAnalyzer"
    }
}

/// Analyzes project scale based on infrastructure complexity
#[derive(Debug, Default)]
pub struct ComplexityAnalyzer;

impl ComplexityAnalyzer {
    /// Create a new complexity analyzer
    pub fn new() -> Self {
        Self
    }
}

impl ScaleAnalyzer for ComplexityAnalyzer {
    fn analyze(&self, context: &ProjectContext) -> Option<(ProjectScale, f32)> {
        // Score based on infrastructure indicators
        let mut score = 0u32;
        let mut factors = 0u32;

        if context.has_ci_cd {
            score += 2;
            factors += 1;
        }

        if context.has_tests {
            score += 1;
            factors += 1;
        }

        if context.has_docs {
            score += 1;
            factors += 1;
        }

        if factors == 0 {
            return None;
        }

        let (scale, confidence) = match score {
            0 => (ProjectScale::BugFix, 0.5),
            1 => (ProjectScale::Feature, 0.6),
            2..=3 => (ProjectScale::Product, 0.7),
            _ => (ProjectScale::Enterprise, 0.8),
        };

        Some((scale, confidence))
    }

    fn name(&self) -> &'static str {
        "ComplexityAnalyzer"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_size_analyzer_bug_fix() {
        let analyzer = FileSizeAnalyzer::new();
        let context = ProjectContext::new().with_lines_of_code(100);

        let result = analyzer.analyze(&context);
        assert!(result.is_some());
        let (scale, confidence) = result.unwrap();
        assert_eq!(scale, ProjectScale::BugFix);
        assert!(confidence > 0.5);
    }

    #[test]
    fn test_file_size_analyzer_enterprise() {
        let analyzer = FileSizeAnalyzer::new();
        let context = ProjectContext::new().with_lines_of_code(100_000);

        let result = analyzer.analyze(&context);
        assert!(result.is_some());
        let (scale, _) = result.unwrap();
        assert_eq!(scale, ProjectScale::Enterprise);
    }

    #[test]
    fn test_dependency_analyzer() {
        let analyzer = DependencyAnalyzer::new();
        let context = ProjectContext::new().with_dependency_count(50);

        let result = analyzer.analyze(&context);
        assert!(result.is_some());
        let (scale, _) = result.unwrap();
        assert_eq!(scale, ProjectScale::Product);
    }

    #[test]
    fn test_team_size_analyzer() {
        let analyzer = TeamSizeAnalyzer::new();
        let context = ProjectContext::new().with_contributor_count(15);

        let result = analyzer.analyze(&context);
        assert!(result.is_some());
        let (scale, _) = result.unwrap();
        assert_eq!(scale, ProjectScale::Enterprise);
    }

    #[test]
    fn test_history_analyzer() {
        let analyzer = HistoryAnalyzer::new();
        let context = ProjectContext::new().with_commit_count(500);

        let result = analyzer.analyze(&context);
        assert!(result.is_some());
        let (scale, _) = result.unwrap();
        assert_eq!(scale, ProjectScale::Product);
    }

    #[test]
    fn test_complexity_analyzer() {
        let analyzer = ComplexityAnalyzer::new();
        let context = ProjectContext::new().with_ci_cd(true).with_tests(true).with_docs(true);

        let result = analyzer.analyze(&context);
        assert!(result.is_some());
        let (scale, _) = result.unwrap();
        assert!(scale == ProjectScale::Product || scale == ProjectScale::Enterprise);
    }

    #[test]
    fn test_analyzer_returns_none_without_data() {
        let analyzer = FileSizeAnalyzer::new();
        let context = ProjectContext::new();

        let result = analyzer.analyze(&context);
        assert!(result.is_none());
    }
}
