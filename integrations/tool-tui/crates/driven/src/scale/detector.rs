//! Scale Detector
//!
//! Combines multiple analyzers to detect project scale.

use super::{
    ProjectContext, ProjectScale, ScaleRecommendation,
    analyzers::{
        ComplexityAnalyzer, DependencyAnalyzer, FileSizeAnalyzer, HistoryAnalyzer, ScaleAnalyzer,
        TeamSizeAnalyzer,
    },
};
use std::collections::HashMap;

/// Detects project scale by combining multiple analyzers
pub struct ScaleDetector {
    analyzers: Vec<Box<dyn ScaleAnalyzer>>,
}

impl Default for ScaleDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ScaleDetector {
    /// Create a new scale detector with default analyzers
    pub fn new() -> Self {
        Self {
            analyzers: vec![
                Box::new(FileSizeAnalyzer::new()),
                Box::new(DependencyAnalyzer::new()),
                Box::new(TeamSizeAnalyzer::new()),
                Box::new(HistoryAnalyzer::new()),
                Box::new(ComplexityAnalyzer::new()),
            ],
        }
    }

    /// Create a scale detector with custom analyzers
    pub fn with_analyzers(analyzers: Vec<Box<dyn ScaleAnalyzer>>) -> Self {
        Self { analyzers }
    }

    /// Add an analyzer to the detector
    pub fn add_analyzer(&mut self, analyzer: Box<dyn ScaleAnalyzer>) {
        self.analyzers.push(analyzer);
    }

    /// Detect the project scale from context
    pub fn detect(&self, context: &ProjectContext) -> ProjectScale {
        // Check for manual override first
        if let Some(override_scale) = context.scale_override {
            return override_scale;
        }

        // Collect votes from all analyzers
        let mut votes: HashMap<ProjectScale, f32> = HashMap::new();

        for analyzer in &self.analyzers {
            if let Some((scale, confidence)) = analyzer.analyze(context) {
                *votes.entry(scale).or_insert(0.0) += confidence;
            }
        }

        // Return the scale with highest weighted votes
        // In case of ties, prefer larger scale (more comprehensive workflow)
        votes
            .into_iter()
            .max_by(|a, b| {
                match a.1.partial_cmp(&b.1) {
                    Some(std::cmp::Ordering::Equal) => {
                        // Tie-breaker: prefer larger scale for determinism
                        Self::scale_order(&a.0).cmp(&Self::scale_order(&b.0))
                    }
                    Some(ord) => ord,
                    None => std::cmp::Ordering::Equal,
                }
            })
            .map(|(scale, _)| scale)
            .unwrap_or(ProjectScale::Feature) // Default to Feature if no data
    }

    /// Get a numeric order for scales (for deterministic tie-breaking)
    fn scale_order(scale: &ProjectScale) -> u8 {
        match scale {
            ProjectScale::BugFix => 0,
            ProjectScale::Feature => 1,
            ProjectScale::Product => 2,
            ProjectScale::Enterprise => 3,
        }
    }

    /// Get a detailed recommendation with reasoning
    pub fn recommend(&self, context: &ProjectContext) -> ScaleRecommendation {
        // Check for manual override first
        if let Some(override_scale) = context.scale_override {
            return ScaleRecommendation::new(override_scale, 1.0)
                .with_reason("Scale manually overridden by user".to_string());
        }

        // Collect votes and reasoning from all analyzers
        let mut votes: HashMap<ProjectScale, f32> = HashMap::new();
        let mut reasoning: Vec<String> = Vec::new();
        let mut total_confidence = 0.0;
        let mut analyzer_count = 0;

        for analyzer in &self.analyzers {
            if let Some((scale, confidence)) = analyzer.analyze(context) {
                *votes.entry(scale).or_insert(0.0) += confidence;
                total_confidence += confidence;
                analyzer_count += 1;

                reasoning.push(format!(
                    "{} suggests {} (confidence: {:.0}%)",
                    analyzer.name(),
                    scale,
                    confidence * 100.0
                ));
            }
        }

        // Determine winning scale
        let (detected_scale, scale_confidence) = votes
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(scale, conf)| (*scale, *conf))
            .unwrap_or((ProjectScale::Feature, 0.5));

        // Calculate overall confidence
        let overall_confidence = if analyzer_count > 0 {
            (scale_confidence / total_confidence).min(1.0)
        } else {
            0.5
        };

        // Add context-based reasoning
        if let Some(loc) = context.lines_of_code {
            reasoning.push(format!("Project has {} lines of code", loc));
        }
        if let Some(files) = context.file_count {
            reasoning.push(format!("Project has {} files", files));
        }
        if let Some(deps) = context.dependency_count {
            reasoning.push(format!("Project has {} dependencies", deps));
        }
        if let Some(contributors) = context.contributor_count {
            reasoning.push(format!("Project has {} contributors", contributors));
        }

        ScaleRecommendation::new(detected_scale, overall_confidence).with_reasons(reasoning)
    }

    /// Select appropriate workflows based on detected scale
    pub fn select_workflows(&self, context: &ProjectContext) -> Vec<&'static str> {
        let scale = self.detect(context);
        scale.recommended_workflows()
    }

    /// Select appropriate agents based on detected scale
    pub fn select_agents(&self, context: &ProjectContext) -> Vec<&'static str> {
        let scale = self.detect(context);
        scale.recommended_agents()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_bug_fix_scale() {
        let detector = ScaleDetector::new();
        let context = ProjectContext::new()
            .with_lines_of_code(200)
            .with_file_count(5)
            .with_dependency_count(3);

        let scale = detector.detect(&context);
        assert_eq!(scale, ProjectScale::BugFix);
    }

    #[test]
    fn test_detect_feature_scale() {
        let detector = ScaleDetector::new();
        let context = ProjectContext::new()
            .with_lines_of_code(3000)
            .with_file_count(30)
            .with_dependency_count(15)
            .with_contributor_count(2);

        let scale = detector.detect(&context);
        assert_eq!(scale, ProjectScale::Feature);
    }

    #[test]
    fn test_detect_product_scale() {
        let detector = ScaleDetector::new();
        let context = ProjectContext::new()
            .with_lines_of_code(25000)
            .with_file_count(200)
            .with_dependency_count(50)
            .with_contributor_count(5)
            .with_ci_cd(true)
            .with_tests(true);

        let scale = detector.detect(&context);
        assert_eq!(scale, ProjectScale::Product);
    }

    #[test]
    fn test_detect_enterprise_scale() {
        let detector = ScaleDetector::new();
        let context = ProjectContext::new()
            .with_lines_of_code(200000)
            .with_file_count(2000)
            .with_dependency_count(200)
            .with_contributor_count(50)
            .with_commit_count(5000)
            .with_ci_cd(true)
            .with_tests(true)
            .with_docs(true);

        let scale = detector.detect(&context);
        assert_eq!(scale, ProjectScale::Enterprise);
    }

    #[test]
    fn test_manual_override() {
        let detector = ScaleDetector::new();
        let context = ProjectContext::new()
            .with_lines_of_code(200000) // Would normally be Enterprise
            .with_scale_override(ProjectScale::BugFix);

        let scale = detector.detect(&context);
        assert_eq!(scale, ProjectScale::BugFix);
    }

    #[test]
    fn test_recommend_with_reasoning() {
        let detector = ScaleDetector::new();
        let context = ProjectContext::new()
            .with_lines_of_code(10000)
            .with_file_count(100)
            .with_dependency_count(30);

        let recommendation = detector.recommend(&context);
        assert!(!recommendation.reasoning.is_empty());
        assert!(recommendation.confidence > 0.0);
        assert!(!recommendation.suggested_workflows.is_empty());
    }

    #[test]
    fn test_select_workflows() {
        let detector = ScaleDetector::new();
        let context = ProjectContext::new().with_lines_of_code(100).with_file_count(3);

        let workflows = detector.select_workflows(&context);
        assert!(workflows.contains(&"quick-bug-fix"));
    }

    #[test]
    fn test_select_agents() {
        let detector = ScaleDetector::new();
        let context = ProjectContext::new().with_lines_of_code(100000).with_contributor_count(20);

        let agents = detector.select_agents(&context);
        assert!(agents.contains(&"security"));
        assert!(agents.contains(&"devops"));
    }

    #[test]
    fn test_default_scale_without_data() {
        let detector = ScaleDetector::new();
        let context = ProjectContext::new();

        let scale = detector.detect(&context);
        assert_eq!(scale, ProjectScale::Feature); // Default
    }
}
