//! Score subcommand - 500-point code quality scoring system
//!
//! Categories (100 points each, NO double-counting):
//! 1. Formatting (100) - Third-party formatters
//! 2. Linting (100) - Third-party linters
//! 3. Security (100) - Built-in security rules
//! 4. Design Patterns (100) - Plugin system
//! 5. Structure/Docs (100) - Plugin system

use anyhow::Result;
use clap::{Args, ValueEnum};
use std::collections::HashSet;
use std::path::PathBuf;

use super::OutputFormat;
use crate::ui::theme::Theme;

/// Calculate 500-point code quality score
#[derive(Args, Clone)]
pub struct ScoreCommand {
    /// Paths to analyze
    #[arg(index = 1)]
    pub paths: Vec<PathBuf>,

    /// Minimum score threshold (fail if below)
    #[arg(long, short)]
    pub threshold: Option<u32>,

    /// Categories to include (default: all)
    #[arg(long, short)]
    pub categories: Vec<Category>,

    /// Disable deduplication (allow double-counting)
    #[arg(long)]
    pub no_dedup: bool,
}

/// Scoring category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum)]
pub enum Category {
    /// Formatting rules (third-party)
    Formatting,
    /// Linting rules (third-party)
    Linting,
    /// Security rules (built-in)
    Security,
    /// Design patterns rules (plugin)
    Patterns,
    /// Structure and documentation rules (plugin)
    Structure,
}

impl Category {
    pub fn max_score(&self) -> u32 {
        100
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Formatting => "Formatting",
            Self::Linting => "Linting",
            Self::Security => "Security",
            Self::Patterns => "Design Patterns",
            Self::Structure => "Structure/Docs",
        }
    }

    pub fn all() -> Vec<Category> {
        vec![
            Self::Formatting,
            Self::Linting,
            Self::Security,
            Self::Patterns,
            Self::Structure,
        ]
    }
}

/// A deduction from the score
#[derive(Debug, Clone)]
pub struct Deduction {
    pub rule_id: String,
    pub category: Category,
    pub points: u32,
    pub message: String,
    pub path: PathBuf,
    pub line: Option<u32>,
}

/// Score for a single category
#[derive(Debug, Clone, Default)]
pub struct CategoryScore {
    pub category: Option<Category>,
    pub max_score: u32,
    pub score: u32,
    pub deductions: Vec<Deduction>,
}

impl CategoryScore {
    pub fn new(category: Category) -> Self {
        Self {
            category: Some(category),
            max_score: category.max_score(),
            score: category.max_score(),
            deductions: vec![],
        }
    }

    pub fn deduct(&mut self, deduction: Deduction) {
        let points = deduction.points.min(self.score);
        self.score = self.score.saturating_sub(points);
        self.deductions.push(deduction);
    }
}

/// Complete score breakdown
#[derive(Debug, Clone)]
pub struct Score {
    pub total: u32,
    pub max_total: u32,
    pub categories: Vec<CategoryScore>,
    pub files_analyzed: u32,
    pub dedup_savings: u32,
}

impl Score {
    pub fn new() -> Self {
        let categories = Category::all().into_iter().map(CategoryScore::new).collect();
        Self {
            total: 500,
            max_total: 500,
            categories,
            files_analyzed: 0,
            dedup_savings: 0,
        }
    }

    pub fn recalculate(&mut self) {
        self.total = self.categories.iter().map(|c| c.score).sum();
    }

    /// Convert to LLM format (token-efficient)
    pub fn to_llm_format(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("score={}/{}\n", self.total, self.max_total));
        output.push_str(&format!("files={}\n", self.files_analyzed));

        for cat in &self.categories {
            if let Some(category) = &cat.category {
                output.push_str(&format!(
                    "{}={}/{}\n",
                    category.name().to_lowercase().replace(' ', "_").replace('/', "_"),
                    cat.score,
                    cat.max_score
                ));
            }
        }

        if !self.categories.iter().all(|c| c.deductions.is_empty()) {
            output.push_str("deductions:\n");
            for cat in &self.categories {
                for d in &cat.deductions {
                    output.push_str(&format!(
                        "  -{} {}:{} {}\n",
                        d.points,
                        d.path.display(),
                        d.line.unwrap_or(0),
                        d.rule_id
                    ));
                }
            }
        }

        output
    }
}

impl Default for Score {
    fn default() -> Self {
        Self::new()
    }
}

/// Score calculator with deduplication
pub struct ScoreCalculator {
    seen_rules: HashSet<(String, PathBuf)>,
    enable_dedup: bool,
}

impl ScoreCalculator {
    pub fn new(enable_dedup: bool) -> Self {
        Self {
            seen_rules: HashSet::new(),
            enable_dedup,
        }
    }

    /// Add a deduction, respecting deduplication
    pub fn add_deduction(&mut self, score: &mut Score, deduction: Deduction) -> bool {
        if self.enable_dedup {
            let key = (deduction.rule_id.clone(), deduction.path.clone());
            if self.seen_rules.contains(&key) {
                score.dedup_savings += deduction.points;
                return false;
            }
            self.seen_rules.insert(key);
        }

        // Find category and apply deduction
        for cat in &mut score.categories {
            if cat.category == Some(deduction.category) {
                cat.deduct(deduction);
                score.recalculate();
                return true;
            }
        }

        false
    }
}

/// Run score command
pub async fn run(cmd: ScoreCommand, format: OutputFormat, theme: &Theme) -> Result<()> {
    use owo_colors::OwoColorize;
    use std::time::Instant;

    let start = Instant::now();

    let paths = if cmd.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        cmd.paths.clone()
    };

    let mut score = Score::new();
    let mut calculator = ScoreCalculator::new(!cmd.no_dedup);

    // Count files
    let mut file_count = 0u32;
    for path in &paths {
        if path.is_file() {
            file_count += 1;
            analyze_file(path, &mut score, &mut calculator).await?;
        } else if path.is_dir() {
            for entry in walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                // Skip hidden files and common non-code directories
                let path_str = entry.path().to_string_lossy();
                if path_str.contains("/.")
                    || path_str.contains("\\.")
                    || path_str.contains("node_modules")
                    || path_str.contains("target/")
                    || path_str.contains(".git")
                {
                    continue;
                }

                file_count += 1;
                analyze_file(entry.path(), &mut score, &mut calculator).await?;
            }
        }
    }

    score.files_analyzed = file_count;

    let elapsed = start.elapsed();

    // Output based on format
    match format {
        OutputFormat::Llm => {
            println!("{}", score.to_llm_format());
            return Ok(());
        }
        OutputFormat::Json => {
            let json = serde_json::json!({
                "score": score.total,
                "maxScore": score.max_total,
                "filesAnalyzed": score.files_analyzed,
                "categories": score.categories.iter().map(|c| {
                    serde_json::json!({
                        "name": c.category.map(|cat| cat.name()).unwrap_or("unknown"),
                        "score": c.score,
                        "maxScore": c.max_score,
                        "deductions": c.deductions.len()
                    })
                }).collect::<Vec<_>>()
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
            return Ok(());
        }
        _ => {}
    }

    // Human-readable output
    theme.print_section("dx check score: Code Quality Score");
    eprintln!();

    // Score bar
    let percentage = (score.total as f64 / score.max_total as f64) * 100.0;
    let bar_width = 40;
    let filled = ((percentage / 100.0) * bar_width as f64) as usize;
    let empty = bar_width - filled;

    let bar_color = if percentage >= 80.0 {
        "green"
    } else if percentage >= 60.0 {
        "yellow"
    } else {
        "red"
    };

    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));

    let colored_bar = match bar_color {
        "green" => bar.green().to_string(),
        "yellow" => bar.yellow().to_string(),
        _ => bar.red().to_string(),
    };

    eprintln!(
        "  {} {}/{} ({:.1}%)",
        colored_bar,
        score.total.to_string().bold(),
        score.max_total,
        percentage
    );
    eprintln!();

    // Category breakdown
    eprintln!("  {} Category Breakdown:", "■".cyan().bold());
    eprintln!();

    for cat in &score.categories {
        let cat_name = cat.category.map(|c| c.name()).unwrap_or("Unknown");
        let cat_pct = (cat.score as f64 / cat.max_score as f64) * 100.0;

        let status = if cat.score == cat.max_score {
            "✓".green().to_string()
        } else if cat.score >= cat.max_score * 8 / 10 {
            "●".yellow().to_string()
        } else {
            "✗".red().to_string()
        };

        eprintln!(
            "    {} {:15} {:>3}/{} ({:>5.1}%) {}",
            status,
            cat_name,
            cat.score,
            cat.max_score,
            cat_pct,
            if cat.deductions.is_empty() {
                "".to_string()
            } else {
                format!("[{} issues]", cat.deductions.len()).bright_black().to_string()
            }
        );
    }

    eprintln!();

    // Deduplication savings
    if score.dedup_savings > 0 {
        eprintln!(
            "  {} Deduplication saved {} points from double-counting",
            "ℹ".cyan(),
            score.dedup_savings.to_string().cyan().bold()
        );
        eprintln!();
    }

    // Summary
    eprintln!(
        "  {} {} files analyzed in {:.2}s",
        "✓".green().bold(),
        score.files_analyzed,
        elapsed.as_secs_f64()
    );
    eprintln!();

    // Threshold check
    if let Some(threshold) = cmd.threshold {
        if score.total < threshold {
            eprintln!(
                "  {} Score {} is below threshold {}",
                "✗".red().bold(),
                score.total,
                threshold
            );
            anyhow::bail!("Score {} is below threshold {}", score.total, threshold);
        } else {
            eprintln!(
                "  {} Score {} meets threshold {}",
                "✓".green().bold(),
                score.total,
                threshold
            );
        }
        eprintln!();
    }

    Ok(())
}

async fn analyze_file(
    _path: &std::path::Path,
    _score: &mut Score,
    _calculator: &mut ScoreCalculator,
) -> Result<()> {
    // In production, this would:
    // 1. Run formatting check (Category::Formatting)
    // 2. Run linting (Category::Linting)
    // 3. Run security analysis (Category::Security)
    // 4. Run design pattern checks (Category::Patterns)
    // 5. Check structure/docs (Category::Structure)
    //
    // Each violation creates a Deduction with proper category tagging
    // The calculator handles deduplication

    Ok(())
}
