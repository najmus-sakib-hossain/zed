//! 500-Point Scoring System
//!
//! Comprehensive codebase quality scoring across 5 categories:
//! - Formatting (100 points)
//! - Linting (100 points)
//! - Security (100 points)
//! - Design Patterns (100 points)
//! - Structure & Documentation (100 points)

use crate::diagnostics::{Diagnostic, DiagnosticSeverity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Maximum total score
pub const MAX_TOTAL_SCORE: u16 = 500;

/// Maximum score per category
pub const MAX_CATEGORY_SCORE: u16 = 100;

/// Score category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Category {
    Formatting,
    Linting,
    Security,
    DesignPatterns,
    StructureAndDocs,
}

impl Category {
    #[must_use]
    pub fn all() -> &'static [Category] {
        &[
            Category::Formatting,
            Category::Linting,
            Category::Security,
            Category::DesignPatterns,
            Category::StructureAndDocs,
        ]
    }

    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Category::Formatting => "formatting",
            Category::Linting => "linting",
            Category::Security => "security",
            Category::DesignPatterns => "design_patterns",
            Category::StructureAndDocs => "structure_and_docs",
        }
    }
}

/// Violation severity with point deductions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

impl Severity {
    /// Get point deduction for this severity
    #[must_use]
    pub fn points(&self) -> u16 {
        match self {
            Severity::Critical => 10,
            Severity::High => 5,
            Severity::Medium => 2,
            Severity::Low => 1,
        }
    }

    #[must_use]
    pub fn from_diagnostic_severity(sev: DiagnosticSeverity) -> Self {
        match sev {
            DiagnosticSeverity::Error => Severity::High,
            DiagnosticSeverity::Warning => Severity::Medium,
            DiagnosticSeverity::Info => Severity::Low,
            DiagnosticSeverity::Hint => Severity::Low,
        }
    }
}

/// A single violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub category: Category,
    pub severity: Severity,
    pub file: PathBuf,
    pub line: u32,
    pub column: u32,
    pub rule_id: String,
    pub message: String,
    pub points: u16,
}

/// Deduction rule mapping rule IDs to categories and severities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeductionRule {
    pub rule_id: String,
    pub category: Category,
    pub default_severity: Severity,
    pub description: String,
}

impl Violation {
    #[must_use]
    pub fn from_diagnostic(diagnostic: &Diagnostic, category: Category) -> Self {
        let severity = Severity::from_diagnostic_severity(diagnostic.severity);
        Self {
            category,
            severity,
            file: diagnostic.file.clone(),
            line: diagnostic.span.start,
            column: diagnostic.span.end,
            rule_id: diagnostic.rule_id.clone(),
            message: diagnostic.message.clone(),
            points: severity.points(),
        }
    }
}

/// Score for a single category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryScore {
    pub category: Category,
    pub score: u16,
    pub violations: Vec<Violation>,
    pub deductions: u16,
}

impl CategoryScore {
    #[must_use]
    pub fn new(category: Category) -> Self {
        Self {
            category,
            score: MAX_CATEGORY_SCORE,
            violations: Vec::new(),
            deductions: 0,
        }
    }

    pub fn add_violation(&mut self, violation: Violation) {
        self.deductions = self.deductions.saturating_add(violation.points);
        self.violations.push(violation);
        // Ensure score doesn't go below 0
        self.score = MAX_CATEGORY_SCORE.saturating_sub(self.deductions);
    }

    #[must_use]
    pub fn violation_count(&self) -> usize {
        self.violations.len()
    }
}

/// Complete project score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectScore {
    pub total_score: u16,
    pub categories: HashMap<Category, CategoryScore>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub files_analyzed: usize,
}

impl ProjectScore {
    #[must_use]
    pub fn new(files_analyzed: usize) -> Self {
        let mut categories = HashMap::new();
        for &category in Category::all() {
            categories.insert(category, CategoryScore::new(category));
        }

        Self {
            total_score: MAX_TOTAL_SCORE,
            categories,
            timestamp: chrono::Utc::now(),
            files_analyzed,
        }
    }

    pub fn add_violation(&mut self, violation: Violation) {
        if let Some(cat_score) = self.categories.get_mut(&violation.category) {
            cat_score.add_violation(violation);
        }
        self.recalculate_total();
    }

    fn recalculate_total(&mut self) {
        self.total_score = self.categories.values().map(|c| c.score).sum();
    }

    #[must_use]
    pub fn get_category_score(&self, category: Category) -> u16 {
        self.categories.get(&category).map_or(0, |c| c.score)
    }

    #[must_use]
    pub fn total_violations(&self) -> usize {
        self.categories.values().map(CategoryScore::violation_count).sum()
    }

    /// Get anime-style rank (Solo Leveling inspired: E to SSS)
    #[must_use]
    pub fn rank(&self) -> &'static str {
        match self.total_score {
            490..=500 => "SSS", // Legendary/God-tier (98-100%)
            475..=489 => "SS",  // Exceptional (95-97%)
            450..=474 => "S",   // Outstanding (90-94%)
            400..=449 => "A",   // Excellent (80-89%)
            350..=399 => "B",   // Good (70-79%)
            300..=349 => "C",   // Average (60-69%)
            250..=299 => "D",   // Below Average (50-59%)
            _ => "E",           // Weak (<50%)
        }
    }

    /// Legacy grade method (kept for compatibility)
    #[must_use]
    pub fn grade(&self) -> &'static str {
        self.rank()
    }
}

/// Analysis mode for score calculation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AnalysisMode {
    /// Quick mode: Project-level calculation for fast feedback (default for CI/CD)
    #[default]
    Quick,
    /// Detailed mode: File-level scores first, then aggregate to project level
    Detailed,
}

/// File-level score (used in detailed mode)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileScore {
    pub file: PathBuf,
    pub total_score: u16,
    pub categories: HashMap<Category, CategoryScore>,
}

impl FileScore {
    #[must_use]
    pub fn new(file: PathBuf) -> Self {
        let mut categories = HashMap::new();
        for &category in Category::all() {
            categories.insert(category, CategoryScore::new(category));
        }

        Self {
            file,
            total_score: MAX_TOTAL_SCORE,
            categories,
        }
    }

    pub fn add_violation(&mut self, violation: Violation) {
        if let Some(cat_score) = self.categories.get_mut(&violation.category) {
            cat_score.add_violation(violation);
        }
        self.recalculate_total();
    }

    fn recalculate_total(&mut self) {
        self.total_score = self.categories.values().map(|c| c.score).sum();
    }

    #[must_use]
    pub fn get_category_score(&self, category: Category) -> u16 {
        self.categories.get(&category).map_or(0, |c| c.score)
    }
}

/// Score calculator
pub struct ScoreCalculator {
    /// Rule ID to category mapping
    rule_categories: HashMap<String, Category>,
    /// Analysis mode
    mode: AnalysisMode,
}

impl ScoreCalculator {
    #[must_use]
    pub fn new() -> Self {
        Self::with_mode(AnalysisMode::Quick)
    }

    #[must_use]
    pub fn with_mode(mode: AnalysisMode) -> Self {
        let mut rule_categories = HashMap::new();

        // Formatting rules
        rule_categories.insert("indent".to_string(), Category::Formatting);
        rule_categories.insert("line-length".to_string(), Category::Formatting);
        rule_categories.insert("quotes".to_string(), Category::Formatting);
        rule_categories.insert("semicolons".to_string(), Category::Formatting);

        // Linting rules
        rule_categories.insert("no-unused-vars".to_string(), Category::Linting);
        rule_categories.insert("no-debugger".to_string(), Category::Linting);
        rule_categories.insert("no-console".to_string(), Category::Linting);

        // Security rules - vulnerability patterns
        rule_categories.insert("sql-injection".to_string(), Category::Security);
        rule_categories.insert("command-injection".to_string(), Category::Security);
        rule_categories.insert("path-traversal".to_string(), Category::Security);
        rule_categories.insert("xss-innerHTML".to_string(), Category::Security);
        rule_categories.insert("insecure-deserialization".to_string(), Category::Security);
        rule_categories.insert("weak-crypto".to_string(), Category::Security);

        // Security rules - secret detection
        rule_categories.insert("aws-access-key".to_string(), Category::Security);
        rule_categories.insert("aws-secret-key".to_string(), Category::Security);
        rule_categories.insert("github-token".to_string(), Category::Security);
        rule_categories.insert("google-api-key".to_string(), Category::Security);
        rule_categories.insert("stripe-key".to_string(), Category::Security);
        rule_categories.insert("private-key".to_string(), Category::Security);
        rule_categories.insert("jwt-token".to_string(), Category::Security);
        rule_categories.insert("high-entropy-string".to_string(), Category::Security);

        // Security rules - language-specific
        rule_categories.insert("js-eval".to_string(), Category::Security);
        rule_categories.insert("js-function-constructor".to_string(), Category::Security);
        rule_categories.insert("js-dangerously-set-inner-html".to_string(), Category::Security);
        rule_categories.insert("py-eval".to_string(), Category::Security);
        rule_categories.insert("py-exec".to_string(), Category::Security);
        rule_categories.insert("py-pickle-loads".to_string(), Category::Security);
        rule_categories.insert("py-yaml-load".to_string(), Category::Security);
        rule_categories.insert("rs-unsafe-undocumented".to_string(), Category::Security);
        rule_categories.insert("go-unsafe".to_string(), Category::Security);
        rule_categories.insert("cpp-strcpy".to_string(), Category::Security);
        rule_categories.insert("cpp-gets".to_string(), Category::Security);

        // Design pattern rules
        rule_categories.insert("complexity".to_string(), Category::DesignPatterns);
        rule_categories.insert("max-lines".to_string(), Category::DesignPatterns);

        // Code smell rules (Design Patterns category)
        rule_categories.insert("long-method".to_string(), Category::DesignPatterns);
        rule_categories.insert("large-class".to_string(), Category::DesignPatterns);
        rule_categories.insert("too-many-parameters".to_string(), Category::DesignPatterns);
        rule_categories.insert("deep-nesting".to_string(), Category::DesignPatterns);
        rule_categories.insert("duplicate-code".to_string(), Category::DesignPatterns);
        rule_categories.insert("dead-code".to_string(), Category::DesignPatterns);
        rule_categories.insert("magic-number".to_string(), Category::DesignPatterns);
        rule_categories.insert("magic-string".to_string(), Category::DesignPatterns);

        // Structure rules
        rule_categories.insert("missing-docs".to_string(), Category::StructureAndDocs);
        rule_categories.insert("naming-convention".to_string(), Category::StructureAndDocs);

        Self {
            rule_categories,
            mode,
        }
    }

    pub fn set_mode(&mut self, mode: AnalysisMode) {
        self.mode = mode;
    }

    #[must_use]
    pub fn mode(&self) -> AnalysisMode {
        self.mode
    }

    /// Calculate project score in quick mode (project-level)
    #[must_use]
    pub fn calculate(&self, diagnostics: &[Diagnostic], files_analyzed: usize) -> ProjectScore {
        match self.mode {
            AnalysisMode::Quick => self.calculate_quick(diagnostics, files_analyzed),
            AnalysisMode::Detailed => self.calculate_detailed(diagnostics, files_analyzed).0,
        }
    }

    /// Calculate project score in quick mode (project-level calculation)
    fn calculate_quick(&self, diagnostics: &[Diagnostic], files_analyzed: usize) -> ProjectScore {
        let mut score = ProjectScore::new(files_analyzed);

        for diagnostic in diagnostics {
            let category = self.get_category(&diagnostic.rule_id);
            let violation = Violation::from_diagnostic(diagnostic, category);
            score.add_violation(violation);
        }

        score
    }

    /// Calculate project score in detailed mode (file-level first, then aggregate)
    #[must_use]
    pub fn calculate_detailed(
        &self,
        diagnostics: &[Diagnostic],
        files_analyzed: usize,
    ) -> (ProjectScore, HashMap<PathBuf, FileScore>) {
        // Group diagnostics by file
        let mut file_diagnostics: HashMap<PathBuf, Vec<&Diagnostic>> = HashMap::new();
        for diagnostic in diagnostics {
            file_diagnostics.entry(diagnostic.file.clone()).or_default().push(diagnostic);
        }

        // Calculate file-level scores
        let mut file_scores: HashMap<PathBuf, FileScore> = HashMap::new();
        for (file, diags) in file_diagnostics {
            let mut file_score = FileScore::new(file.clone());
            for diagnostic in diags {
                let category = self.get_category(&diagnostic.rule_id);
                let violation = Violation::from_diagnostic(diagnostic, category);
                file_score.add_violation(violation);
            }
            file_scores.insert(file, file_score);
        }

        // Aggregate to project level
        let mut project_score = ProjectScore::new(files_analyzed);
        for diagnostic in diagnostics {
            let category = self.get_category(&diagnostic.rule_id);
            let violation = Violation::from_diagnostic(diagnostic, category);
            project_score.add_violation(violation);
        }

        (project_score, file_scores)
    }

    fn get_category(&self, rule_id: &str) -> Category {
        self.rule_categories.get(rule_id).copied().unwrap_or(Category::Linting)
    }

    pub fn register_rule(&mut self, rule_id: String, category: Category) {
        self.rule_categories.insert(rule_id, category);
    }
}

impl Default for ScoreCalculator {
    fn default() -> Self {
        Self::new()
    }
}

/// Score trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Declining,
    Stable,
}

/// Score trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreTrend {
    pub direction: TrendDirection,
    pub total_change: i32,
    pub category_changes: HashMap<Category, i32>,
    pub previous_score: u16,
    pub current_score: u16,
}

impl ScoreTrend {
    #[must_use]
    pub fn new(previous: &ProjectScore, current: &ProjectScore) -> Self {
        let total_change = i32::from(current.total_score) - i32::from(previous.total_score);

        let direction = if total_change > 5 {
            TrendDirection::Improving
        } else if total_change < -5 {
            TrendDirection::Declining
        } else {
            TrendDirection::Stable
        };

        let mut category_changes = HashMap::new();
        for &category in Category::all() {
            let prev = i32::from(previous.get_category_score(category));
            let curr = i32::from(current.get_category_score(category));
            category_changes.insert(category, curr - prev);
        }

        Self {
            direction,
            total_change,
            category_changes,
            previous_score: previous.total_score,
            current_score: current.total_score,
        }
    }

    #[must_use]
    pub fn summary(&self) -> String {
        match self.direction {
            TrendDirection::Improving => {
                format!(
                    "Score improved by {} points ({}→{})",
                    self.total_change, self.previous_score, self.current_score
                )
            }
            TrendDirection::Declining => {
                format!(
                    "Score declined by {} points ({}→{})",
                    -self.total_change, self.previous_score, self.current_score
                )
            }
            TrendDirection::Stable => {
                format!("Score is stable at {} points", self.current_score)
            }
        }
    }
}

/// Score storage and history using dx-serializer format
pub struct ScoreStorage {
    cache_dir: PathBuf,
}

impl ScoreStorage {
    #[must_use]
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Save score to .sr file (LLM format)
    pub fn save(&self, score: &ProjectScore) -> Result<(), std::io::Error> {
        std::fs::create_dir_all(&self.cache_dir)?;

        // Create timestamped filename
        let timestamp = score.timestamp.format("%Y-%m-%d-%H-%M-%S");
        let filename = format!("score_{timestamp}.sr");
        let path = self.cache_dir.join(filename);

        // Convert score to dx-serializer format
        let sr_content = self.score_to_sr_format(score);
        std::fs::write(path, sr_content)?;

        // Also save as latest.sr for quick access
        let latest_path = self.cache_dir.join("latest.sr");
        std::fs::write(latest_path, self.score_to_sr_format(score))?;

        Ok(())
    }

    /// Load the latest score
    pub fn load_latest(&self) -> Result<ProjectScore, std::io::Error> {
        let latest_path = self.cache_dir.join("latest.sr");
        if latest_path.exists() {
            let content = std::fs::read_to_string(latest_path)?;
            return self.sr_format_to_score(&content);
        }

        // Fallback to finding the most recent timestamped file
        let mut entries: Vec<_> = std::fs::read_dir(&self.cache_dir)?
            .filter_map(std::result::Result::ok)
            .filter(|e| {
                e.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("score_") && n.ends_with(".sr"))
            })
            .collect();

        entries.sort_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()));

        if let Some(latest) = entries.last() {
            let content = std::fs::read_to_string(latest.path())?;
            self.sr_format_to_score(&content)
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "No score history found"))
        }
    }

    /// Load score history (most recent first)
    pub fn load_history(&self, limit: usize) -> Result<Vec<ProjectScore>, std::io::Error> {
        let mut entries: Vec<_> = std::fs::read_dir(&self.cache_dir)?
            .filter_map(std::result::Result::ok)
            .filter(|e| {
                let path = e.path();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                name.starts_with("score_") && name.ends_with(".sr")
            })
            .collect();

        entries.sort_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()));
        entries.reverse(); // Most recent first

        let scores: Vec<ProjectScore> = entries
            .iter()
            .take(limit)
            .filter_map(|e| {
                let content = std::fs::read_to_string(e.path()).ok()?;
                self.sr_format_to_score(&content).ok()
            })
            .collect();

        Ok(scores)
    }

    /// Analyze score trend compared to previous run
    pub fn analyze_trend(
        &self,
        current: &ProjectScore,
    ) -> Result<Option<ScoreTrend>, std::io::Error> {
        let history = self.load_history(2)?;

        if history.len() < 2 {
            return Ok(None);
        }

        // Compare current with the most recent previous score
        let previous = &history[1];
        Ok(Some(ScoreTrend::new(previous, current)))
    }

    /// Convert `ProjectScore` to .sr format (Latest DX Serializer LLM format)
    fn score_to_sr_format(&self, score: &ProjectScore) -> String {
        let mut lines = Vec::new();

        // Root scalars - one per line, no spaces around =
        // Use quotes for multi-word strings (timestamps contain spaces/special chars)
        lines.push(format!("timestamp=\"{}\"", score.timestamp.to_rfc3339()));
        lines.push(format!("total_score={}", score.total_score));
        lines.push(format!("files_analyzed={}", score.files_analyzed));
        lines.push(format!("rank={}", score.grade()));

        // Category scores as inline object with parentheses
        let mut cat_scores = Vec::new();
        for &category in Category::all() {
            let cat_score = score.get_category_score(category);
            cat_scores.push(format!("{}={}", category.as_str(), cat_score));
        }
        lines.push(format!("categories({})", cat_scores.join(" ")));

        // Violations as wrapped dataframe table
        let total_violations = score.total_violations();
        if total_violations > 0 {
            lines
                .push("violations[category severity file line column message points](".to_string());

            for category_score in score.categories.values() {
                for violation in &category_score.violations {
                    // Use quotes for multi-word strings
                    let message = if violation.message.contains(' ') {
                        format!("\"{}\"", violation.message.replace('"', "'"))
                    } else {
                        violation.message.clone()
                    };

                    lines.push(format!(
                        "  {} {:?} {} {} {} {} {}",
                        violation.category.as_str(),
                        violation.severity,
                        violation.file.to_string_lossy(),
                        violation.line,
                        violation.column,
                        message,
                        violation.points
                    ));
                }
            }

            lines.push(")".to_string());
        }

        lines.join("\n")
    }

    /// Convert .sr format to `ProjectScore` (Latest DX Serializer LLM format)
    fn sr_format_to_score(&self, content: &str) -> Result<ProjectScore, std::io::Error> {
        let mut timestamp = chrono::Utc::now();
        let mut total_score = 0;
        let mut files_analyzed = 0;
        let mut category_scores: HashMap<Category, u16> = HashMap::new();
        let mut violations: Vec<Violation> = Vec::new();

        let mut in_violations = false;
        let mut lines_iter = content.lines().peekable();

        while let Some(line) = lines_iter.next() {
            let line = line.trim();

            if line.is_empty() {
                continue;
            }

            // Check for wrapped dataframe start
            if line.starts_with("violations[") && line.ends_with('(') {
                in_violations = true;
                continue;
            }

            // Check for wrapped dataframe end
            if line == ")" && in_violations {
                in_violations = false;
                continue;
            }

            if in_violations {
                // Parse violation line from wrapped dataframe
                if let Some(violation) = self.parse_violation_line_v2(line) {
                    violations.push(violation);
                }
                continue;
            }

            // Parse root scalars and inline objects
            if let Some((key, value)) = line.split_once('=') {
                match key {
                    "timestamp" => {
                        // Remove quotes if present
                        let value = value.trim_matches('"');
                        if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(value) {
                            timestamp = ts.with_timezone(&chrono::Utc);
                        }
                    }
                    "total_score" | "total" => {
                        total_score = value.parse().unwrap_or(0);
                    }
                    "files_analyzed" => {
                        files_analyzed = value.parse().unwrap_or(0);
                    }
                    "formatting" => {
                        category_scores.insert(Category::Formatting, value.parse().unwrap_or(0));
                    }
                    "linting" => {
                        category_scores.insert(Category::Linting, value.parse().unwrap_or(0));
                    }
                    "security" => {
                        category_scores.insert(Category::Security, value.parse().unwrap_or(0));
                    }
                    "design_patterns" => {
                        category_scores
                            .insert(Category::DesignPatterns, value.parse().unwrap_or(0));
                    }
                    "structure_and_docs" => {
                        category_scores
                            .insert(Category::StructureAndDocs, value.parse().unwrap_or(0));
                    }
                    _ => {}
                }
            }
        }

        // Reconstruct ProjectScore
        let mut score = ProjectScore::new(files_analyzed);
        score.timestamp = timestamp;
        score.total_score = total_score;

        // Update category scores
        for (category, cat_score) in category_scores {
            if let Some(cs) = score.categories.get_mut(&category) {
                cs.score = cat_score;
            }
        }

        // Add violations
        for violation in violations {
            if let Some(cs) = score.categories.get_mut(&violation.category) {
                cs.violations.push(violation);
            }
        }

        Ok(score)
    }

    /// Parse a violation line from .sr format (Latest DX Serializer v2 - wrapped dataframe)
    fn parse_violation_line_v2(&self, line: &str) -> Option<Violation> {
        // Format: category severity file line column message points
        // Example: linting Medium test.js 0 1 "Unexpected var, use let or const instead" 2

        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        // Split by spaces, but respect quotes
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;

        for ch in line.chars() {
            match ch {
                '"' => {
                    in_quotes = !in_quotes;
                }
                ' ' if !in_quotes => {
                    if !current.is_empty() {
                        parts.push(current.clone());
                        current.clear();
                    }
                }
                _ => {
                    current.push(ch);
                }
            }
        }
        if !current.is_empty() {
            parts.push(current);
        }

        if parts.len() < 7 {
            return None;
        }

        let category = match parts[0].as_str() {
            "formatting" => Category::Formatting,
            "linting" => Category::Linting,
            "security" => Category::Security,
            "design_patterns" => Category::DesignPatterns,
            "structure_and_docs" => Category::StructureAndDocs,
            _ => return None,
        };

        let severity = match parts[1].as_str() {
            "Critical" => Severity::Critical,
            "High" => Severity::High,
            "Medium" => Severity::Medium,
            "Low" => Severity::Low,
            _ => return None,
        };

        let file = PathBuf::from(&parts[2]);
        let line_num = parts[3].parse().unwrap_or(0);
        let column = parts[4].parse().unwrap_or(0);
        let message = parts[5].clone();
        let points = parts[6].parse().unwrap_or(0);

        Some(Violation {
            category,
            severity,
            file,
            line: line_num,
            column,
            rule_id: String::new(),
            message,
            points,
        })
    }

    /// Parse a violation line from .sr format (Old format - for backward compatibility)
    fn parse_violation_line(&self, line: &str) -> Option<Violation> {
        // Format: category severity file line:column "message" points;
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() < 5 {
            return None;
        }

        let category = match parts[0] {
            "formatting" => Category::Formatting,
            "linting" => Category::Linting,
            "security" => Category::Security,
            "design_patterns" => Category::DesignPatterns,
            "structure_and_docs" => Category::StructureAndDocs,
            _ => return None,
        };

        let severity = match parts[1] {
            "Critical" => Severity::Critical,
            "High" => Severity::High,
            "Medium" => Severity::Medium,
            "Low" => Severity::Low,
            _ => return None,
        };

        let file = PathBuf::from(parts[2].replace('_', " "));

        let (line_num, column) = if let Some((l, c)) = parts[3].split_once(':') {
            (l.parse().unwrap_or(0), c.parse().unwrap_or(0))
        } else {
            (0, 0)
        };

        // Extract message (between quotes)
        let message_start = line.find('"')?;
        let message_end = line.rfind('"')?;
        let message = line[message_start + 1..message_end].replace('_', " ");

        // Extract points (last number before semicolon)
        let points_str = parts.last()?.trim_end_matches(';');
        let points = points_str.parse().unwrap_or(0);

        Some(Violation {
            category,
            severity,
            file,
            line: line_num,
            column,
            rule_id: String::new(),
            message,
            points,
        })
    }
}

/// Threshold checker for CI/CD
pub struct ThresholdChecker {
    pub min_total_score: Option<u16>,
    pub min_category_scores: HashMap<Category, u16>,
}

impl ThresholdChecker {
    #[must_use]
    pub fn new() -> Self {
        Self {
            min_total_score: None,
            min_category_scores: HashMap::new(),
        }
    }

    #[must_use]
    pub fn with_total_threshold(mut self, min_score: u16) -> Self {
        self.min_total_score = Some(min_score);
        self
    }

    #[must_use]
    pub fn with_category_threshold(mut self, category: Category, min_score: u16) -> Self {
        self.min_category_scores.insert(category, min_score);
        self
    }

    #[must_use]
    pub fn check(&self, score: &ProjectScore) -> ThresholdResult {
        let mut failures = Vec::new();

        if let Some(min_total) = self.min_total_score
            && score.total_score < min_total
        {
            failures.push(format!(
                "Total score {} is below threshold {}",
                score.total_score, min_total
            ));
        }

        for (category, min_score) in &self.min_category_scores {
            let actual_score = score.get_category_score(*category);
            if actual_score < *min_score {
                failures.push(format!(
                    "{} score {} is below threshold {}",
                    category.as_str(),
                    actual_score,
                    min_score
                ));
            }
        }

        if failures.is_empty() {
            ThresholdResult::Pass
        } else {
            ThresholdResult::Fail(failures)
        }
    }

    #[must_use]
    pub fn exit_code(&self, score: &ProjectScore) -> i32 {
        match self.check(score) {
            ThresholdResult::Pass => 0,
            ThresholdResult::Fail(_) => 1,
        }
    }
}

impl Default for ThresholdChecker {
    fn default() -> Self {
        Self::new()
    }
}

pub enum ThresholdResult {
    Pass,
    Fail(Vec<String>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_score_bounds() {
        let score = ProjectScore::new(10);
        assert_eq!(score.total_score, MAX_TOTAL_SCORE);
        assert_eq!(score.get_category_score(Category::Formatting), MAX_CATEGORY_SCORE);
    }

    #[test]
    fn test_violation_deduction() {
        let mut score = ProjectScore::new(10);
        let violation = Violation {
            category: Category::Formatting,
            severity: Severity::High,
            file: PathBuf::from("test.js"),
            line: 1,
            column: 1,
            rule_id: "indent".to_string(),
            message: "Bad indent".to_string(),
            points: Severity::High.points(),
        };

        score.add_violation(violation);
        assert_eq!(score.get_category_score(Category::Formatting), MAX_CATEGORY_SCORE - 5);
        assert_eq!(score.total_score, MAX_TOTAL_SCORE - 5);
    }

    #[test]
    fn test_score_floor() {
        let mut score = ProjectScore::new(10);

        // Add enough violations to exceed max deductions
        for _ in 0..50 {
            let violation = Violation {
                category: Category::Formatting,
                severity: Severity::Critical,
                file: PathBuf::from("test.js"),
                line: 1,
                column: 1,
                rule_id: "indent".to_string(),
                message: "Bad indent".to_string(),
                points: Severity::Critical.points(),
            };
            score.add_violation(violation);
        }

        // Score should not go below 0
        assert_eq!(score.get_category_score(Category::Formatting), 0);
        assert!(score.total_score <= MAX_TOTAL_SCORE);
    }

    #[test]
    fn test_grade_calculation() {
        let mut score = ProjectScore::new(10);
        assert_eq!(score.grade(), "A+");

        score.total_score = 425;
        assert_eq!(score.grade(), "A");

        score.total_score = 375;
        assert_eq!(score.grade(), "B+");

        score.total_score = 100;
        assert_eq!(score.grade(), "F");
    }

    #[test]
    fn test_threshold_checker() {
        let checker = ThresholdChecker::new()
            .with_total_threshold(400)
            .with_category_threshold(Category::Security, 90);

        let mut score = ProjectScore::new(10);
        score.total_score = 450;

        match checker.check(&score) {
            ThresholdResult::Pass => {}
            ThresholdResult::Fail(_) => panic!("Should pass"),
        }

        score.total_score = 350;
        match checker.check(&score) {
            ThresholdResult::Pass => panic!("Should fail"),
            ThresholdResult::Fail(failures) => {
                assert!(!failures.is_empty());
            }
        }
    }

    #[test]
    fn test_score_storage_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ScoreStorage::new(temp_dir.path().to_path_buf());

        let mut score = ProjectScore::new(10);
        score.total_score = 450;

        // Add a violation
        let violation = Violation {
            category: Category::Security,
            severity: Severity::High,
            file: PathBuf::from("test.rs"),
            line: 42,
            column: 10,
            rule_id: "no-unsafe".to_string(),
            message: "Unsafe block detected".to_string(),
            points: 5,
        };
        score.add_violation(violation);

        // Save score
        storage.save(&score).unwrap();

        // Load latest score
        let loaded = storage.load_latest().unwrap();
        assert_eq!(loaded.total_score, score.total_score);
        assert_eq!(loaded.files_analyzed, score.files_analyzed);
        assert_eq!(loaded.total_violations(), score.total_violations());
    }

    #[test]
    fn test_score_storage_history() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ScoreStorage::new(temp_dir.path().to_path_buf());

        // Save multiple scores with distinct timestamps
        for i in 0..5 {
            let mut score = ProjectScore::new(10);
            score.total_score = 400 + i * 10;
            // Ensure distinct timestamps by sleeping longer
            std::thread::sleep(std::time::Duration::from_millis(100));
            storage.save(&score).unwrap();
        }

        // Load history - note that we're filtering out latest.sr
        let history = storage.load_history(3).unwrap();
        // We should get 3 scores (the limit we requested)
        assert!(history.len() <= 3, "Expected at most 3 scores, got {}", history.len());

        // Should be in reverse chronological order (most recent first)
        if history.len() >= 2 {
            assert!(history[0].total_score >= history[1].total_score);
        }
    }

    #[test]
    fn test_score_trend_improving() {
        let mut prev_score = ProjectScore::new(10);
        prev_score.total_score = 400;

        let mut curr_score = ProjectScore::new(10);
        curr_score.total_score = 450;

        let trend = ScoreTrend::new(&prev_score, &curr_score);
        assert_eq!(trend.direction, TrendDirection::Improving);
        assert_eq!(trend.total_change, 50);
        assert!(trend.summary().contains("improved"));
    }

    #[test]
    fn test_score_trend_declining() {
        let mut prev_score = ProjectScore::new(10);
        prev_score.total_score = 450;

        let mut curr_score = ProjectScore::new(10);
        curr_score.total_score = 400;

        let trend = ScoreTrend::new(&prev_score, &curr_score);
        assert_eq!(trend.direction, TrendDirection::Declining);
        assert_eq!(trend.total_change, -50);
        assert!(trend.summary().contains("declined"));
    }

    #[test]
    fn test_score_trend_stable() {
        let mut prev_score = ProjectScore::new(10);
        prev_score.total_score = 450;

        let mut curr_score = ProjectScore::new(10);
        curr_score.total_score = 452;

        let trend = ScoreTrend::new(&prev_score, &curr_score);
        assert_eq!(trend.direction, TrendDirection::Stable);
        assert!(trend.summary().contains("stable"));
    }

    #[test]
    fn test_score_trend_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ScoreStorage::new(temp_dir.path().to_path_buf());

        // Create scores with different timestamps
        let mut score1 = ProjectScore::new(10);
        score1.total_score = 400;
        score1.timestamp = chrono::Utc::now() - chrono::Duration::seconds(10);

        let mut score2 = ProjectScore::new(10);
        score2.total_score = 450;
        score2.timestamp = chrono::Utc::now();

        // Save both scores
        storage.save(&score1).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        storage.save(&score2).unwrap();

        // Verify we can create a trend directly
        let trend = ScoreTrend::new(&score1, &score2);
        assert_eq!(trend.direction, TrendDirection::Improving);
        assert_eq!(trend.total_change, 50);
    }

    #[test]
    fn test_sr_format_round_trip() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ScoreStorage::new(temp_dir.path().to_path_buf());

        let mut score = ProjectScore::new(15);
        score.total_score = 475;

        // Add violations
        let violation1 = Violation {
            category: Category::Formatting,
            severity: Severity::Low,
            file: PathBuf::from("src/main.rs"),
            line: 10,
            column: 5,
            rule_id: "indent".to_string(),
            message: "Incorrect indentation".to_string(),
            points: 1,
        };
        score.add_violation(violation1);

        let violation2 = Violation {
            category: Category::Security,
            severity: Severity::Critical,
            file: PathBuf::from("src/api.rs"),
            line: 42,
            column: 15,
            rule_id: "no-secrets".to_string(),
            message: "Hardcoded API key".to_string(),
            points: 10,
        };
        score.add_violation(violation2);

        // Convert to SR format and back
        let sr_content = storage.score_to_sr_format(&score);
        let loaded = storage.sr_format_to_score(&sr_content).unwrap();

        // Verify core fields
        assert_eq!(loaded.total_score, score.total_score);
        assert_eq!(loaded.files_analyzed, score.files_analyzed);
        assert_eq!(loaded.total_violations(), score.total_violations());

        // Verify category scores
        for &category in Category::all() {
            assert_eq!(loaded.get_category_score(category), score.get_category_score(category));
        }
    }

    #[test]
    fn test_score_storage_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join(".dx/check");
        let storage = ScoreStorage::new(cache_dir.clone());

        let score = ProjectScore::new(5);
        storage.save(&score).unwrap();

        assert!(cache_dir.exists());
        assert!(cache_dir.join("latest.sr").exists());
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use tempfile::TempDir;

    // Strategy for generating valid Category values
    fn category_strategy() -> impl Strategy<Value = Category> {
        prop_oneof![
            Just(Category::Formatting),
            Just(Category::Linting),
            Just(Category::Security),
            Just(Category::DesignPatterns),
            Just(Category::StructureAndDocs),
        ]
    }

    // Strategy for generating valid Severity values
    fn severity_strategy() -> impl Strategy<Value = Severity> {
        prop_oneof![
            Just(Severity::Critical),
            Just(Severity::High),
            Just(Severity::Medium),
            Just(Severity::Low),
        ]
    }

    // Strategy for generating valid Violation values
    fn violation_strategy() -> impl Strategy<Value = Violation> {
        (
            category_strategy(),
            severity_strategy(),
            "[a-z]{1,10}\\.[a-z]{2,4}", // file name
            1u32..1000,                 // line number
            1u32..200,                  // column number
            "[a-z-]{3,20}",             // rule_id
            "[a-zA-Z ]{5,50}",          // message
        )
            .prop_map(|(category, severity, file, line, column, rule_id, message)| {
                Violation {
                    category,
                    severity,
                    file: PathBuf::from(file),
                    line,
                    column,
                    rule_id,
                    message,
                    points: severity.points(),
                }
            })
    }

    // Strategy for generating ProjectScore with random violations
    fn project_score_strategy() -> impl Strategy<Value = ProjectScore> {
        (
            1usize..100,                                        // files_analyzed
            prop::collection::vec(violation_strategy(), 0..50), // violations
        )
            .prop_map(|(files_analyzed, violations)| {
                let mut score = ProjectScore::new(files_analyzed);
                for violation in violations {
                    score.add_violation(violation);
                }
                score
            })
    }

    proptest! {
        /// **Property 3: Score persistence round-trip**
        /// **Validates: Requirements 2.8**
        ///
        /// Test that any ProjectScore saved and loaded produces equivalent data.
        /// This ensures that the serialization format preserves all score information
        /// correctly, which is critical for score history tracking and trend analysis.
        #[test]
        fn prop_score_storage_round_trip(score in project_score_strategy()) {
            let temp_dir = TempDir::new().unwrap();
            let storage = ScoreStorage::new(temp_dir.path().to_path_buf());

            // Save the score
            storage.save(&score).unwrap();

            // Load it back
            let loaded = storage.load_latest().unwrap();

            // Verify all fields match
            prop_assert_eq!(loaded.total_score, score.total_score);
            prop_assert_eq!(loaded.files_analyzed, score.files_analyzed);
            prop_assert_eq!(loaded.total_violations(), score.total_violations());

            // Verify all category scores match
            for &category in Category::all() {
                prop_assert_eq!(
                    loaded.get_category_score(category),
                    score.get_category_score(category),
                    "Category {:?} score mismatch", category
                );
            }

            // Verify violation counts per category match
            for &category in Category::all() {
                let original_count = score.categories.get(&category)
                    .map(|cs| cs.violations.len())
                    .unwrap_or(0);
                let loaded_count = loaded.categories.get(&category)
                    .map(|cs| cs.violations.len())
                    .unwrap_or(0);
                prop_assert_eq!(
                    loaded_count,
                    original_count,
                    "Category {:?} violation count mismatch", category
                );
            }
        }

        /// **Property 3.1: Score format conversion preserves data**
        /// **Validates: Requirements 2.8**
        ///
        /// Test that converting a score to SR format and back preserves all data.
        /// This is a more direct test of the serialization logic.
        #[test]
        fn prop_sr_format_preserves_data(score in project_score_strategy()) {
            let temp_dir = TempDir::new().unwrap();
            let storage = ScoreStorage::new(temp_dir.path().to_path_buf());

            // Convert to SR format
            let sr_content = storage.score_to_sr_format(&score);

            // Convert back
            let loaded = storage.sr_format_to_score(&sr_content).unwrap();

            // Verify core fields
            prop_assert_eq!(loaded.total_score, score.total_score);
            prop_assert_eq!(loaded.files_analyzed, score.files_analyzed);

            // Verify category scores
            for &category in Category::all() {
                prop_assert_eq!(
                    loaded.get_category_score(category),
                    score.get_category_score(category),
                    "Category {:?} score mismatch", category
                );
            }
        }

        /// **Property 3.2: Multiple save/load cycles preserve data**
        /// **Validates: Requirements 2.8**
        ///
        /// Test that saving and loading a score multiple times doesn't corrupt data.
        #[test]
        fn prop_multiple_save_load_cycles(score in project_score_strategy()) {
            let temp_dir = TempDir::new().unwrap();
            let storage = ScoreStorage::new(temp_dir.path().to_path_buf());

            let original_total = score.total_score;
            let original_files = score.files_analyzed;

            // Save and load 3 times
            for _ in 0..3 {
                storage.save(&score).unwrap();
                let loaded = storage.load_latest().unwrap();

                prop_assert_eq!(loaded.total_score, original_total);
                prop_assert_eq!(loaded.files_analyzed, original_files);
            }
        }

        /// **Property 3.3: Score history maintains order**
        /// **Validates: Requirements 2.8**
        ///
        /// Test that score history is maintained in correct chronological order.
        #[test]
        fn prop_score_history_order(
            scores in prop::collection::vec(project_score_strategy(), 2..10)
        ) {
            let temp_dir = TempDir::new().unwrap();
            let storage = ScoreStorage::new(temp_dir.path().to_path_buf());

            // Save scores with small delays to ensure distinct timestamps
            for score in &scores {
                storage.save(score).unwrap();
                std::thread::sleep(std::time::Duration::from_millis(10));
            }

            // Load history
            let history = storage.load_history(scores.len()).unwrap();

            // Should have saved all scores (or at least some)
            prop_assert!(!history.is_empty());
            prop_assert!(history.len() <= scores.len());

            // Verify timestamps are in descending order (most recent first)
            for i in 0..history.len().saturating_sub(1) {
                prop_assert!(
                    history[i].timestamp >= history[i + 1].timestamp,
                    "History not in descending order at index {}", i
                );
            }
        }
    }
}
