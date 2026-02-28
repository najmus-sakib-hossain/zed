//! Test result reporter

use anyhow::Result;
use std::io::Write;

use super::{OutputFormat, TestStatus, TestSummary};

/// Report test results
pub fn report(summary: &TestSummary, format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Pretty => report_pretty(summary),
        OutputFormat::Json => report_json(summary),
        OutputFormat::Junit => report_junit(summary),
        OutputFormat::Tap => report_tap(summary),
    }
}

fn report_pretty(summary: &TestSummary) -> Result<()> {
    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Test Results");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Summary
    let status_icon = if summary.failed == 0 { "✓" } else { "✗" };
    let status_color = if summary.failed == 0 {
        "\x1b[32m"
    } else {
        "\x1b[31m"
    };
    let reset = "\x1b[0m";

    println!(
        "  {}{}{} {} tests, {} passed, {} failed, {} skipped",
        status_color,
        status_icon,
        reset,
        summary.total,
        summary.passed,
        summary.failed,
        summary.skipped
    );

    println!("  ⏱  Duration: {:.2}s", summary.duration.as_secs_f64());
    println!("  📊 Success rate: {:.1}%", summary.success_rate());
    println!();

    // Failed tests details
    if summary.failed > 0 {
        println!("  Failed tests:");
        for result in &summary.results {
            if result.status == TestStatus::Failed {
                println!("    ✗ {}", result.name);
                if let Some(ref msg) = result.message {
                    for line in msg.lines() {
                        println!("      {}", line);
                    }
                }
            }
        }
        println!();
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}

fn report_json(summary: &TestSummary) -> Result<()> {
    // Simple JSON output without serde
    println!("{{");
    println!("  \"total\": {},", summary.total);
    println!("  \"passed\": {},", summary.passed);
    println!("  \"failed\": {},", summary.failed);
    println!("  \"skipped\": {},", summary.skipped);
    println!("  \"duration_ms\": {},", summary.duration.as_millis());
    println!("  \"success_rate\": {:.2},", summary.success_rate());
    println!("  \"results\": [");

    for (i, result) in summary.results.iter().enumerate() {
        let status = match result.status {
            TestStatus::Passed => "passed",
            TestStatus::Failed => "failed",
            TestStatus::Skipped => "skipped",
            TestStatus::TimedOut => "timeout",
        };

        let comma = if i < summary.results.len() - 1 {
            ","
        } else {
            ""
        };

        println!("    {{");
        println!("      \"name\": \"{}\",", escape_json(&result.name));
        println!("      \"status\": \"{}\",", status);
        println!("      \"duration_ms\": {}", result.duration.as_millis());
        println!("    }}{}", comma);
    }

    println!("  ]");
    println!("}}");

    Ok(())
}

fn report_junit(summary: &TestSummary) -> Result<()> {
    println!(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    println!(
        r#"<testsuites tests="{}" failures="{}" time="{:.3}">"#,
        summary.total,
        summary.failed,
        summary.duration.as_secs_f64()
    );

    // Group by suite (use first part of name before ::)
    println!(
        r#"  <testsuite name="dx-tests" tests="{}" failures="{}" time="{:.3}">"#,
        summary.total,
        summary.failed,
        summary.duration.as_secs_f64()
    );

    for result in &summary.results {
        let (suite, name) = if let Some(pos) = result.name.rfind("::") {
            (&result.name[..pos], &result.name[pos + 2..])
        } else {
            ("default", result.name.as_str())
        };

        println!(
            r#"    <testcase classname="{}" name="{}" time="{:.3}">"#,
            escape_xml(suite),
            escape_xml(name),
            result.duration.as_secs_f64()
        );

        match result.status {
            TestStatus::Failed => {
                println!(
                    r#"      <failure message="{}">"#,
                    escape_xml(result.message.as_deref().unwrap_or("Test failed"))
                );
                if let Some(ref trace) = result.stack_trace {
                    println!("{}", escape_xml(trace));
                }
                println!("      </failure>");
            }
            TestStatus::Skipped => {
                println!("      <skipped/>");
            }
            TestStatus::TimedOut => {
                println!(r#"      <failure message="Test timed out"/>"#);
            }
            _ => {}
        }

        println!("    </testcase>");
    }

    println!("  </testsuite>");
    println!("</testsuites>");

    Ok(())
}

fn report_tap(summary: &TestSummary) -> Result<()> {
    // TAP (Test Anything Protocol) format
    println!("TAP version 14");
    println!("1..{}", summary.total);

    for (i, result) in summary.results.iter().enumerate() {
        let num = i + 1;

        match result.status {
            TestStatus::Passed => {
                println!("ok {} - {}", num, result.name);
            }
            TestStatus::Failed => {
                println!("not ok {} - {}", num, result.name);
                if let Some(ref msg) = result.message {
                    println!("  ---");
                    println!("  message: {}", msg);
                    println!("  ...");
                }
            }
            TestStatus::Skipped => {
                println!("ok {} - {} # SKIP", num, result.name);
            }
            TestStatus::TimedOut => {
                println!("not ok {} - {} # TIMEOUT", num, result.name);
            }
        }
    }

    Ok(())
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Write report to file
pub fn write_to_file(
    summary: &TestSummary,
    format: &OutputFormat,
    path: &std::path::Path,
) -> Result<()> {
    let mut file = std::fs::File::create(path)?;

    // Capture output
    let output = match format {
        OutputFormat::Pretty => format_pretty(summary),
        OutputFormat::Json => format_json(summary),
        OutputFormat::Junit => format_junit(summary),
        OutputFormat::Tap => format_tap(summary),
    };

    file.write_all(output.as_bytes())?;

    Ok(())
}

fn format_pretty(summary: &TestSummary) -> String {
    let mut output = String::new();

    output.push_str("\n");
    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    output.push_str("  Test Results\n");
    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    output.push_str(&format!(
        "  {} tests, {} passed, {} failed, {} skipped\n",
        summary.total, summary.passed, summary.failed, summary.skipped
    ));

    output
}

fn format_json(summary: &TestSummary) -> String {
    format!(
        r#"{{"total":{},"passed":{},"failed":{},"skipped":{},"duration_ms":{}}}"#,
        summary.total,
        summary.passed,
        summary.failed,
        summary.skipped,
        summary.duration.as_millis()
    )
}

fn format_junit(summary: &TestSummary) -> String {
    let mut output = String::new();
    output.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    output.push_str(&format!(
        r#"<testsuites tests="{}" failures="{}" time="{:.3}"/>"#,
        summary.total,
        summary.failed,
        summary.duration.as_secs_f64()
    ));
    output
}

fn format_tap(summary: &TestSummary) -> String {
    format!("TAP version 14\n1..{}\n", summary.total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_escape_json() {
        assert_eq!(escape_json("hello\"world"), "hello\\\"world");
        assert_eq!(escape_json("line1\nline2"), "line1\\nline2");
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("<test>"), "&lt;test&gt;");
        assert_eq!(escape_xml("a & b"), "a &amp; b");
    }
}
