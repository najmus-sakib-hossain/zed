//! Coverage Reporting - Track executed lines during test runs
//!
//! Generates LCOV format coverage reports.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

/// Coverage data for a single file
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileCoverage {
    /// File path
    pub path: PathBuf,
    /// Lines that were executed (line number -> hit count)
    pub lines_hit: HashMap<u32, u32>,
    /// All executable lines in the file
    pub executable_lines: HashSet<u32>,
    /// Functions that were executed (name -> hit count)
    pub functions_hit: HashMap<String, u32>,
    /// All functions in the file
    pub functions: HashSet<String>,
    /// Branches that were executed (branch_id -> (taken, not_taken))
    pub branches: HashMap<u32, (u32, u32)>,
}

impl FileCoverage {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            ..Default::default()
        }
    }

    /// Record a line hit
    pub fn hit_line(&mut self, line: u32) {
        *self.lines_hit.entry(line).or_insert(0) += 1;
    }

    /// Record a function hit
    pub fn hit_function(&mut self, name: &str) {
        *self.functions_hit.entry(name.to_string()).or_insert(0) += 1;
    }

    /// Record a branch hit
    pub fn hit_branch(&mut self, branch_id: u32, taken: bool) {
        let entry = self.branches.entry(branch_id).or_insert((0, 0));
        if taken {
            entry.0 += 1;
        } else {
            entry.1 += 1;
        }
    }

    /// Calculate line coverage percentage
    pub fn line_coverage_percent(&self) -> f64 {
        if self.executable_lines.is_empty() {
            return 100.0;
        }
        let covered = self.lines_hit.len();
        (covered as f64 / self.executable_lines.len() as f64) * 100.0
    }

    /// Calculate function coverage percentage
    pub fn function_coverage_percent(&self) -> f64 {
        if self.functions.is_empty() {
            return 100.0;
        }
        let covered = self.functions_hit.len();
        (covered as f64 / self.functions.len() as f64) * 100.0
    }

    /// Calculate branch coverage percentage
    pub fn branch_coverage_percent(&self) -> f64 {
        if self.branches.is_empty() {
            return 100.0;
        }
        let covered = self.branches.values().filter(|(t, n)| *t > 0 && *n > 0).count();
        (covered as f64 / self.branches.len() as f64) * 100.0
    }
}

/// Coverage collector for all files
#[derive(Debug, Default)]
pub struct CoverageCollector {
    /// Coverage data per file
    pub files: HashMap<PathBuf, FileCoverage>,
}

impl CoverageCollector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create coverage data for a file
    pub fn get_file(&mut self, path: &Path) -> &mut FileCoverage {
        self.files
            .entry(path.to_path_buf())
            .or_insert_with(|| FileCoverage::new(path.to_path_buf()))
    }

    /// Merge coverage from another collector
    pub fn merge(&mut self, other: CoverageCollector) {
        for (path, coverage) in other.files {
            let entry = self
                .files
                .entry(path)
                .or_insert_with(|| FileCoverage::new(coverage.path.clone()));

            // Merge line hits
            for (line, count) in coverage.lines_hit {
                *entry.lines_hit.entry(line).or_insert(0) += count;
            }

            // Merge executable lines
            entry.executable_lines.extend(coverage.executable_lines);

            // Merge function hits
            for (func, count) in coverage.functions_hit {
                *entry.functions_hit.entry(func).or_insert(0) += count;
            }

            // Merge functions
            entry.functions.extend(coverage.functions);

            // Merge branches
            for (branch_id, (taken, not_taken)) in coverage.branches {
                let e = entry.branches.entry(branch_id).or_insert((0, 0));
                e.0 += taken;
                e.1 += not_taken;
            }
        }
    }

    /// Calculate total line coverage
    pub fn total_line_coverage(&self) -> f64 {
        let total_executable: usize = self.files.values().map(|f| f.executable_lines.len()).sum();
        let total_covered: usize = self.files.values().map(|f| f.lines_hit.len()).sum();

        if total_executable == 0 {
            return 100.0;
        }
        (total_covered as f64 / total_executable as f64) * 100.0
    }

    /// Generate LCOV format report
    pub fn generate_lcov(&self, output_path: &Path) -> std::io::Result<()> {
        let file = File::create(output_path)?;
        let mut writer = BufWriter::new(file);

        for (path, coverage) in &self.files {
            // Source file
            writeln!(writer, "SF:{}", path.display())?;

            // Functions
            for func in &coverage.functions {
                let hits = coverage.functions_hit.get(func).copied().unwrap_or(0);
                writeln!(writer, "FN:0,{}", func)?;
                writeln!(writer, "FNDA:{},{}", hits, func)?;
            }
            writeln!(writer, "FNF:{}", coverage.functions.len())?;
            writeln!(writer, "FNH:{}", coverage.functions_hit.len())?;

            // Branches
            for (branch_id, (taken, not_taken)) in &coverage.branches {
                writeln!(writer, "BRDA:0,{},0,{}", branch_id, taken)?;
                writeln!(writer, "BRDA:0,{},1,{}", branch_id, not_taken)?;
            }
            writeln!(writer, "BRF:{}", coverage.branches.len() * 2)?;
            let branches_hit = coverage.branches.values().filter(|(t, _)| *t > 0).count()
                + coverage.branches.values().filter(|(_, n)| *n > 0).count();
            writeln!(writer, "BRH:{}", branches_hit)?;

            // Lines
            for line in &coverage.executable_lines {
                let hits = coverage.lines_hit.get(line).copied().unwrap_or(0);
                writeln!(writer, "DA:{},{}", line, hits)?;
            }
            writeln!(writer, "LF:{}", coverage.executable_lines.len())?;
            writeln!(writer, "LH:{}", coverage.lines_hit.len())?;

            writeln!(writer, "end_of_record")?;
        }

        writer.flush()?;
        Ok(())
    }

    /// Generate HTML coverage report
    pub fn generate_html(&self, output_dir: &Path) -> std::io::Result<()> {
        fs::create_dir_all(output_dir)?;

        // Generate index.html
        let index_path = output_dir.join("index.html");
        let mut index = File::create(&index_path)?;

        writeln!(index, "<!DOCTYPE html>")?;
        writeln!(index, "<html><head><title>Coverage Report</title>")?;
        writeln!(index, "<style>")?;
        writeln!(index, "body {{ font-family: sans-serif; margin: 20px; }}")?;
        writeln!(index, "table {{ border-collapse: collapse; width: 100%; }}")?;
        writeln!(index, "th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}")?;
        writeln!(index, "th {{ background: #4CAF50; color: white; }}")?;
        writeln!(index, ".high {{ background: #c8e6c9; }}")?;
        writeln!(index, ".medium {{ background: #fff9c4; }}")?;
        writeln!(index, ".low {{ background: #ffcdd2; }}")?;
        writeln!(index, "</style></head><body>")?;
        writeln!(index, "<h1>Coverage Report</h1>")?;
        writeln!(index, "<p>Total Line Coverage: {:.1}%</p>", self.total_line_coverage())?;
        writeln!(index, "<table>")?;
        writeln!(index, "<tr><th>File</th><th>Lines</th><th>Functions</th><th>Branches</th></tr>")?;

        for (path, coverage) in &self.files {
            let line_pct = coverage.line_coverage_percent();
            let func_pct = coverage.function_coverage_percent();
            let branch_pct = coverage.branch_coverage_percent();

            let class = if line_pct >= 80.0 {
                "high"
            } else if line_pct >= 50.0 {
                "medium"
            } else {
                "low"
            };

            writeln!(
                index,
                "<tr class=\"{}\"><td>{}</td><td>{:.1}%</td><td>{:.1}%</td><td>{:.1}%</td></tr>",
                class,
                path.display(),
                line_pct,
                func_pct,
                branch_pct
            )?;
        }

        writeln!(index, "</table></body></html>")?;

        Ok(())
    }
}

/// Coverage reporter that integrates with test execution
pub struct CoverageReporter {
    collector: CoverageCollector,
    output_dir: PathBuf,
}

impl CoverageReporter {
    pub fn new(output_dir: PathBuf) -> Self {
        Self {
            collector: CoverageCollector::new(),
            output_dir,
        }
    }

    /// Record coverage from test execution
    pub fn record(&mut self, file_path: &Path, lines_executed: &[u32]) {
        let coverage = self.collector.get_file(file_path);
        for &line in lines_executed {
            coverage.hit_line(line);
        }
    }

    /// Generate all reports
    pub fn generate_reports(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.output_dir)?;

        // LCOV format
        let lcov_path = self.output_dir.join("lcov.info");
        self.collector.generate_lcov(&lcov_path)?;

        // HTML report
        let html_dir = self.output_dir.join("html");
        self.collector.generate_html(&html_dir)?;

        Ok(())
    }

    /// Get total coverage percentage
    pub fn total_coverage(&self) -> f64 {
        self.collector.total_line_coverage()
    }
}
