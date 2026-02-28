//! Coverage Reporting
//!
//! Captures and reports code coverage in DX Serializer format.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Line coverage status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineCoverage {
    /// Line was covered (executed)
    Covered,
    /// Line was not covered
    Uncovered,
    /// Line is not executable (comment, blank, etc.)
    NonExecutable,
}

impl LineCoverage {
    #[must_use]
    pub fn as_char(&self) -> char {
        match self {
            LineCoverage::Covered => '✓',
            LineCoverage::Uncovered => '✗',
            LineCoverage::NonExecutable => ' ',
        }
    }
}

/// Coverage data for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCoverage {
    pub file: PathBuf,
    pub lines: Vec<LineCoverage>,
    pub covered_lines: u32,
    pub total_lines: u32,
    pub uncovered_ranges: Vec<(u32, u32)>,
}

impl FileCoverage {
    /// Create new file coverage
    #[must_use]
    pub fn new(file: PathBuf, total_lines: u32) -> Self {
        Self {
            file,
            lines: vec![LineCoverage::NonExecutable; total_lines as usize],
            covered_lines: 0,
            total_lines,
            uncovered_ranges: Vec::new(),
        }
    }

    /// Mark a line as covered
    pub fn mark_covered(&mut self, line: u32) {
        if let Some(l) = self.lines.get_mut(line as usize - 1)
            && *l != LineCoverage::Covered
        {
            *l = LineCoverage::Covered;
            self.covered_lines += 1;
        }
    }

    /// Mark a line as uncovered (executable but not hit)
    pub fn mark_uncovered(&mut self, line: u32) {
        if let Some(l) = self.lines.get_mut(line as usize - 1) {
            *l = LineCoverage::Uncovered;
        }
    }

    /// Calculate coverage percentage
    #[must_use]
    pub fn coverage_percent(&self) -> f64 {
        if self.total_lines == 0 {
            return 100.0;
        }
        (f64::from(self.covered_lines) / f64::from(self.total_lines)) * 100.0
    }

    /// Calculate uncovered ranges for compact reporting
    pub fn calculate_uncovered_ranges(&mut self) {
        self.uncovered_ranges.clear();
        let mut start: Option<u32> = None;

        for (i, &line) in self.lines.iter().enumerate() {
            let line_num = (i + 1) as u32;

            match line {
                LineCoverage::Uncovered => {
                    if start.is_none() {
                        start = Some(line_num);
                    }
                }
                _ => {
                    if let Some(s) = start.take() {
                        self.uncovered_ranges.push((s, line_num - 1));
                    }
                }
            }
        }

        if let Some(s) = start {
            self.uncovered_ranges.push((s, self.lines.len() as u32));
        }
    }
}

/// Complete coverage report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    pub files: Vec<FileCoverage>,
    pub total_lines: u32,
    pub covered_lines: u32,
    pub threshold: f64,
}

impl CoverageReport {
    /// Create a new empty coverage report
    #[must_use]
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            total_lines: 0,
            covered_lines: 0,
            threshold: 80.0,
        }
    }

    /// Create with coverage threshold
    #[must_use]
    pub fn with_threshold(threshold: f64) -> Self {
        Self {
            files: Vec::new(),
            total_lines: 0,
            covered_lines: 0,
            threshold,
        }
    }

    /// Add file coverage
    pub fn add_file(&mut self, mut file_coverage: FileCoverage) {
        file_coverage.calculate_uncovered_ranges();
        self.total_lines += file_coverage.total_lines;
        self.covered_lines += file_coverage.covered_lines;
        self.files.push(file_coverage);
    }

    /// Calculate overall coverage percentage
    #[must_use]
    pub fn coverage_percent(&self) -> f64 {
        if self.total_lines == 0 {
            return 100.0;
        }
        (f64::from(self.covered_lines) / f64::from(self.total_lines)) * 100.0
    }

    /// Check if coverage meets threshold
    #[must_use]
    pub fn meets_threshold(&self) -> bool {
        self.coverage_percent() >= self.threshold
    }

    /// Get files below threshold
    #[must_use]
    pub fn files_below_threshold(&self) -> Vec<&FileCoverage> {
        self.files.iter().filter(|f| f.coverage_percent() < self.threshold).collect()
    }

    /// Parse LCOV format coverage data
    #[must_use]
    pub fn from_lcov(content: &str) -> Self {
        let mut report = CoverageReport::new();
        let mut current_file: Option<FileCoverage> = None;
        let mut file_path: Option<PathBuf> = None;

        for line in content.lines() {
            let line = line.trim();

            if line.starts_with("SF:") {
                // Source file
                file_path = Some(PathBuf::from(&line[3..]));
            } else if line.starts_with("DA:") {
                // Line data: DA:line_number,hit_count
                if let Some(ref path) = file_path {
                    let parts: Vec<&str> = line[3..].split(',').collect();
                    if parts.len() >= 2 {
                        let line_num: u32 = parts[0].parse().unwrap_or(0);
                        let hits: u32 = parts[1].parse().unwrap_or(0);

                        let file_cov = current_file.get_or_insert_with(|| {
                            FileCoverage::new(path.clone(), 1000) // Will resize
                        });

                        // Ensure lines vector is large enough
                        while file_cov.lines.len() < line_num as usize {
                            file_cov.lines.push(LineCoverage::NonExecutable);
                        }

                        if hits > 0 {
                            file_cov.mark_covered(line_num);
                        } else {
                            file_cov.mark_uncovered(line_num);
                        }
                        file_cov.total_lines = file_cov.lines.len() as u32;
                    }
                }
            } else if line == "end_of_record" {
                if let Some(file_cov) = current_file.take() {
                    report.add_file(file_cov);
                }
                file_path = None;
            }
        }

        // Add any remaining file
        if let Some(file_cov) = current_file {
            report.add_file(file_cov);
        }

        report
    }

    /// Parse Cobertura XML format
    #[must_use]
    pub fn from_cobertura(content: &str) -> Self {
        let mut report = CoverageReport::new();

        // Simple regex-based parsing (consider using proper XML parser)
        let file_re = regex::Regex::new(
            r#"<class.*?filename="([^"]+)".*?line-rate="([^"]+)".*?>(.*?)</class>"#,
        )
        .ok();
        let line_re = regex::Regex::new(r#"<line number="(\d+)" hits="(\d+)".*?/>"#).ok();

        if let Some(ref file_re) = file_re {
            for caps in file_re.captures_iter(content) {
                let filename = &caps[1];
                let _line_rate = &caps[2];
                let class_content = &caps[3];

                let mut file_cov = FileCoverage::new(PathBuf::from(filename), 0);

                if let Some(ref line_re) = line_re {
                    for line_caps in line_re.captures_iter(class_content) {
                        let line_num: u32 = line_caps[1].parse().unwrap_or(0);
                        let hits: u32 = line_caps[2].parse().unwrap_or(0);

                        while file_cov.lines.len() < line_num as usize {
                            file_cov.lines.push(LineCoverage::NonExecutable);
                        }

                        if hits > 0 {
                            file_cov.mark_covered(line_num);
                        } else {
                            file_cov.mark_uncovered(line_num);
                        }
                    }
                }

                file_cov.total_lines = file_cov.lines.len() as u32;
                report.add_file(file_cov);
            }
        }

        report
    }

    /// Convert to DX Serializer format (LLM optimized)
    #[must_use]
    pub fn to_dx_format(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("coverage={:.1}%", self.coverage_percent()));
        lines.push(format!("threshold={:.1}%", self.threshold));
        lines.push(format!(
            "status={}",
            if self.meets_threshold() {
                "pass"
            } else {
                "fail"
            }
        ));
        lines.push(format!("total_lines={}", self.total_lines));
        lines.push(format!("covered_lines={}", self.covered_lines));

        // Only include files below threshold for compact output
        let below_threshold: Vec<_> = self.files_below_threshold();

        if !below_threshold.is_empty() {
            lines.push(format!("below_threshold:{}[", below_threshold.len()));

            for file in below_threshold {
                let file_str = file.file.to_string_lossy().replace(' ', "_");
                let ranges: Vec<String> = file
                    .uncovered_ranges
                    .iter()
                    .map(|(s, e)| {
                        if s == e {
                            format!("{s}")
                        } else {
                            format!("{s}-{e}")
                        }
                    })
                    .collect();

                lines.push(format!(
                    "  {} {:.1}% uncovered:[{}];",
                    file_str,
                    file.coverage_percent(),
                    ranges.join(",")
                ));
            }

            lines.push("]".to_string());
        }

        lines.join("\n")
    }

    /// Convert to DX Serializer format (human readable)
    #[must_use]
    pub fn to_dx_human_format(&self) -> String {
        let mut output = String::new();

        // Header
        let status = if self.meets_threshold() {
            "✓ PASS"
        } else {
            "✗ FAIL"
        };
        output.push_str(&format!(
            "Coverage Report: {} | {:.1}% (threshold: {:.1}%)\n",
            status,
            self.coverage_percent(),
            self.threshold
        ));
        output.push_str(&"─".repeat(60));
        output.push('\n');

        // File breakdown
        for file in &self.files {
            let icon = if file.coverage_percent() >= self.threshold {
                "✓"
            } else {
                "✗"
            };
            output.push_str(&format!(
                "{} {:50} {:>6.1}%\n",
                icon,
                file.file.display(),
                file.coverage_percent()
            ));

            // Show uncovered ranges for files below threshold
            if file.coverage_percent() < self.threshold && !file.uncovered_ranges.is_empty() {
                let ranges: Vec<String> = file
                    .uncovered_ranges
                    .iter()
                    .take(5)
                    .map(|(s, e)| {
                        if s == e {
                            format!("L{s}")
                        } else {
                            format!("L{s}-{e}")
                        }
                    })
                    .collect();

                output.push_str(&format!("    Uncovered: {}", ranges.join(", ")));
                if file.uncovered_ranges.len() > 5 {
                    output.push_str(&format!(" (+{} more)", file.uncovered_ranges.len() - 5));
                }
                output.push('\n');
            }
        }

        output.push_str(&"─".repeat(60));
        output.push('\n');
        output.push_str(&format!(
            "Total: {}/{} lines covered\n",
            self.covered_lines, self.total_lines
        ));

        output
    }
}

impl Default for CoverageReport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_coverage() {
        let mut file = FileCoverage::new(PathBuf::from("test.rs"), 10);

        file.mark_covered(1);
        file.mark_covered(2);
        file.mark_covered(3);
        file.mark_uncovered(4);
        file.mark_uncovered(5);

        assert_eq!(file.covered_lines, 3);
        assert_eq!(file.coverage_percent(), 30.0);
    }

    #[test]
    fn test_uncovered_ranges() {
        let mut file = FileCoverage::new(PathBuf::from("test.rs"), 10);

        file.mark_covered(1);
        file.mark_covered(2);
        file.mark_uncovered(3);
        file.mark_uncovered(4);
        file.mark_uncovered(5);
        file.mark_covered(6);
        file.mark_uncovered(8);
        file.mark_uncovered(9);

        file.calculate_uncovered_ranges();

        assert_eq!(file.uncovered_ranges.len(), 2);
        assert_eq!(file.uncovered_ranges[0], (3, 5));
        assert_eq!(file.uncovered_ranges[1], (8, 9));
    }

    #[test]
    fn test_coverage_report() {
        let mut report = CoverageReport::with_threshold(80.0);

        let mut file1 = FileCoverage::new(PathBuf::from("good.rs"), 10);
        for i in 1..=9 {
            file1.mark_covered(i);
        }

        let mut file2 = FileCoverage::new(PathBuf::from("bad.rs"), 10);
        for i in 1..=5 {
            file2.mark_covered(i);
        }

        report.add_file(file1);
        report.add_file(file2);

        assert!(!report.meets_threshold());
        assert_eq!(report.files_below_threshold().len(), 1);
    }

    #[test]
    fn test_lcov_parsing() {
        let lcov = r#"
SF:src/main.rs
DA:1,1
DA:2,1
DA:3,0
DA:4,1
end_of_record
SF:src/lib.rs
DA:1,1
DA:2,0
end_of_record
"#;

        let report = CoverageReport::from_lcov(lcov);
        assert_eq!(report.files.len(), 2);
    }

    #[test]
    fn test_dx_format_output() {
        let mut report = CoverageReport::with_threshold(80.0);
        let mut file = FileCoverage::new(PathBuf::from("test.rs"), 10);
        for i in 1..=8 {
            file.mark_covered(i);
        }
        report.add_file(file);

        let output = report.to_dx_format();
        assert!(output.contains("coverage=80.0%"));
        assert!(output.contains("status=pass"));
    }
}
