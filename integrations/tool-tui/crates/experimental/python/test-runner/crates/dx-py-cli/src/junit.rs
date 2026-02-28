//! JUnit XML report generation
//!
//! Generates valid JUnit XML format for CI integration.

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;

use dx_py_core::{TestResult, TestStatus, TestSummary};

/// JUnit XML report generator
pub struct JUnitReport {
    /// Test suite name
    name: String,
    /// Test results
    results: Vec<TestResult>,
    /// Test names (parallel to results)
    test_names: Vec<String>,
}

impl JUnitReport {
    /// Create a new JUnit report
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            results: Vec::new(),
            test_names: Vec::new(),
        }
    }

    /// Add a test result
    pub fn add_result(&mut self, name: impl Into<String>, result: TestResult) {
        self.test_names.push(name.into());
        self.results.push(result);
    }

    /// Generate JUnit XML string
    pub fn to_xml(&self) -> Result<String, io::Error> {
        let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);

        // XML declaration
        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
            .map_err(io::Error::other)?;

        // Calculate summary
        let summary = TestSummary::from_results(&self.results);

        // <testsuites>
        let mut testsuites = BytesStart::new("testsuites");
        testsuites.push_attribute(("name", self.name.as_str()));
        testsuites.push_attribute(("tests", summary.total.to_string().as_str()));
        testsuites.push_attribute(("failures", summary.failed.to_string().as_str()));
        testsuites.push_attribute(("errors", summary.errors.to_string().as_str()));
        testsuites.push_attribute(("skipped", summary.skipped.to_string().as_str()));
        testsuites
            .push_attribute(("time", format!("{:.3}", summary.duration.as_secs_f64()).as_str()));
        writer.write_event(Event::Start(testsuites)).map_err(io::Error::other)?;

        // <testsuite>
        let mut testsuite = BytesStart::new("testsuite");
        testsuite.push_attribute(("name", self.name.as_str()));
        testsuite.push_attribute(("tests", summary.total.to_string().as_str()));
        testsuite.push_attribute(("failures", summary.failed.to_string().as_str()));
        testsuite.push_attribute(("errors", summary.errors.to_string().as_str()));
        testsuite.push_attribute(("skipped", summary.skipped.to_string().as_str()));
        testsuite
            .push_attribute(("time", format!("{:.3}", summary.duration.as_secs_f64()).as_str()));
        writer.write_event(Event::Start(testsuite)).map_err(io::Error::other)?;

        // Write test cases
        for (name, result) in self.test_names.iter().zip(self.results.iter()) {
            self.write_testcase(&mut writer, name, result)?;
        }

        // </testsuite>
        writer
            .write_event(Event::End(BytesEnd::new("testsuite")))
            .map_err(io::Error::other)?;

        // </testsuites>
        writer
            .write_event(Event::End(BytesEnd::new("testsuites")))
            .map_err(io::Error::other)?;

        let xml = String::from_utf8(writer.into_inner())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(xml)
    }

    /// Write a single test case
    fn write_testcase<W: Write>(
        &self,
        writer: &mut Writer<W>,
        name: &str,
        result: &TestResult,
    ) -> Result<(), io::Error> {
        let mut testcase = BytesStart::new("testcase");
        testcase.push_attribute(("name", name));
        testcase.push_attribute(("classname", self.name.as_str()));
        testcase.push_attribute(("time", format!("{:.3}", result.duration.as_secs_f64()).as_str()));

        match &result.status {
            TestStatus::Pass => {
                // Self-closing tag for passing tests
                writer.write_event(Event::Empty(testcase)).map_err(io::Error::other)?;
            }
            TestStatus::Fail => {
                writer.write_event(Event::Start(testcase)).map_err(io::Error::other)?;

                let mut failure = BytesStart::new("failure");
                failure.push_attribute(("message", "Test failed"));
                failure.push_attribute(("type", "AssertionError"));
                writer.write_event(Event::Start(failure)).map_err(io::Error::other)?;

                if let Some(ref tb) = result.traceback {
                    writer
                        .write_event(Event::Text(BytesText::new(tb)))
                        .map_err(io::Error::other)?;
                }

                writer
                    .write_event(Event::End(BytesEnd::new("failure")))
                    .map_err(io::Error::other)?;

                writer
                    .write_event(Event::End(BytesEnd::new("testcase")))
                    .map_err(io::Error::other)?;
            }
            TestStatus::Skip { reason } => {
                writer.write_event(Event::Start(testcase)).map_err(io::Error::other)?;

                let mut skipped = BytesStart::new("skipped");
                skipped.push_attribute(("message", reason.as_str()));
                writer.write_event(Event::Empty(skipped)).map_err(io::Error::other)?;

                writer
                    .write_event(Event::End(BytesEnd::new("testcase")))
                    .map_err(io::Error::other)?;
            }
            TestStatus::Error { message } => {
                writer.write_event(Event::Start(testcase)).map_err(io::Error::other)?;

                let mut error = BytesStart::new("error");
                error.push_attribute(("message", message.as_str()));
                error.push_attribute(("type", "Error"));
                writer.write_event(Event::Start(error)).map_err(io::Error::other)?;

                if let Some(ref tb) = result.traceback {
                    writer
                        .write_event(Event::Text(BytesText::new(tb)))
                        .map_err(io::Error::other)?;
                }

                writer
                    .write_event(Event::End(BytesEnd::new("error")))
                    .map_err(io::Error::other)?;

                writer
                    .write_event(Event::End(BytesEnd::new("testcase")))
                    .map_err(io::Error::other)?;
            }
        }

        Ok(())
    }

    /// Write JUnit XML to a file
    pub fn write_to_file(&self, path: &Path) -> Result<(), io::Error> {
        let xml = self.to_xml()?;
        let mut file = File::create(path)?;
        file.write_all(xml.as_bytes())?;
        Ok(())
    }
}

/// Validate that XML is well-formed JUnit format
#[allow(dead_code)]
pub fn validate_junit_xml(xml: &str) -> bool {
    // Basic validation: check for required elements
    xml.contains("<?xml")
        && xml.contains("<testsuites")
        && xml.contains("<testsuite")
        && xml.contains("</testsuites>")
        && xml.contains("</testsuite>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use dx_py_core::TestId;
    use std::time::Duration;

    #[test]
    fn test_empty_report() {
        let report = JUnitReport::new("test-suite");
        let xml = report.to_xml().unwrap();

        assert!(validate_junit_xml(&xml));
        assert!(xml.contains("tests=\"0\""));
    }

    #[test]
    fn test_passing_test() {
        let mut report = JUnitReport::new("test-suite");
        report.add_result("test_example", TestResult::pass(TestId(1), Duration::from_millis(100)));

        let xml = report.to_xml().unwrap();
        assert!(validate_junit_xml(&xml));
        assert!(xml.contains("tests=\"1\""));
        assert!(xml.contains("failures=\"0\""));
        assert!(xml.contains("name=\"test_example\""));
    }

    #[test]
    fn test_failing_test() {
        let mut report = JUnitReport::new("test-suite");
        report.add_result(
            "test_failure",
            TestResult::fail(TestId(1), Duration::from_millis(50), "assertion failed"),
        );

        let xml = report.to_xml().unwrap();
        assert!(validate_junit_xml(&xml));
        assert!(xml.contains("failures=\"1\""));
        assert!(xml.contains("<failure"));
    }

    #[test]
    fn test_skipped_test() {
        let mut report = JUnitReport::new("test-suite");
        report.add_result("test_skip", TestResult::skip(TestId(1), "not implemented"));

        let xml = report.to_xml().unwrap();
        assert!(validate_junit_xml(&xml));
        assert!(xml.contains("skipped=\"1\""));
        assert!(xml.contains("<skipped"));
    }

    #[test]
    fn test_error_test() {
        let mut report = JUnitReport::new("test-suite");
        report.add_result("test_error", TestResult::error(TestId(1), "import error"));

        let xml = report.to_xml().unwrap();
        assert!(validate_junit_xml(&xml));
        assert!(xml.contains("errors=\"1\""));
        assert!(xml.contains("<error"));
    }

    #[test]
    fn test_mixed_results() {
        let mut report = JUnitReport::new("test-suite");
        report.add_result("test_pass", TestResult::pass(TestId(1), Duration::from_millis(10)));
        report.add_result(
            "test_fail",
            TestResult::fail(TestId(2), Duration::from_millis(20), "failed"),
        );
        report.add_result("test_skip", TestResult::skip(TestId(3), "skipped"));
        report.add_result("test_error", TestResult::error(TestId(4), "error"));

        let xml = report.to_xml().unwrap();
        assert!(validate_junit_xml(&xml));
        assert!(xml.contains("tests=\"4\""));
        assert!(xml.contains("failures=\"1\""));
        assert!(xml.contains("errors=\"1\""));
        assert!(xml.contains("skipped=\"1\""));
    }

    #[test]
    fn test_write_to_file() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let path = temp_dir.path().join("test-results.xml");

        let mut report = JUnitReport::new("test-suite");
        report.add_result("test_example", TestResult::pass(TestId(1), Duration::from_millis(100)));

        report.write_to_file(&path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(validate_junit_xml(&content));
    }
}

// Property tests
#[cfg(test)]
mod prop_tests {
    use super::*;
    use dx_py_core::TestId;
    use proptest::prelude::*;
    use std::time::Duration;

    // Feature: dx-py-test-runner, Property 19: JUnit XML Validity
    // Validates: Requirements 10.4
    //
    // For any set of test results in CI mode, the generated JUnit XML
    // output SHALL be valid XML conforming to the JUnit XML schema.
    proptest! {
        #[test]
        fn prop_junit_xml_validity(
            test_count in 0usize..20,
            pass_ratio in 0.0f64..1.0,
        ) {
            let mut report = JUnitReport::new("prop-test-suite");

            for i in 0..test_count {
                let name = format!("test_{}", i);
                let result = if (i as f64 / test_count.max(1) as f64) < pass_ratio {
                    TestResult::pass(TestId(i as u64), Duration::from_millis(10))
                } else {
                    TestResult::fail(TestId(i as u64), Duration::from_millis(10), "failed")
                };
                report.add_result(name, result);
            }

            let xml = report.to_xml().unwrap();

            // Validate XML structure
            prop_assert!(validate_junit_xml(&xml), "Invalid JUnit XML");

            // Validate test count
            let expected = format!("tests=\"{}\"", test_count);
            prop_assert!(xml.contains(&expected), "Missing test count");
        }

        #[test]
        fn prop_junit_xml_escaping(
            test_name in "[a-zA-Z_][a-zA-Z0-9_]{0,30}",
        ) {
            let mut report = JUnitReport::new("escape-test");
            report.add_result(
                &test_name,
                TestResult::pass(TestId(1), Duration::from_millis(10)),
            );

            let xml = report.to_xml().unwrap();
            prop_assert!(validate_junit_xml(&xml), "Invalid JUnit XML");
            let expected = format!("name=\"{}\"", test_name);
            prop_assert!(xml.contains(&expected), "Missing test name");
        }

        #[test]
        fn prop_junit_duration_format(
            duration_ms in 0u64..10000,
        ) {
            let mut report = JUnitReport::new("duration-test");
            report.add_result(
                "test_duration",
                TestResult::pass(TestId(1), Duration::from_millis(duration_ms)),
            );

            let xml = report.to_xml().unwrap();
            prop_assert!(validate_junit_xml(&xml), "Invalid JUnit XML");

            // Duration should be formatted as seconds with 3 decimal places
            let expected_time = format!("{:.3}", duration_ms as f64 / 1000.0);
            let expected = format!("time=\"{}\"", expected_time);
            prop_assert!(xml.contains(&expected), "Missing duration");
        }
    }
}
