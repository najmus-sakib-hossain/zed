// Unit tests for coverage integration

use super::coverage::*;
use std::path::PathBuf;

#[test]
fn test_coverage_report_format_from_str() {
    assert_eq!(CoverageReportFormat::from_str("term"), Some(CoverageReportFormat::Term));
    assert_eq!(CoverageReportFormat::from_str("term-missing"), Some(CoverageReportFormat::TermMissing));
    assert_eq!(CoverageReportFormat::from_str("html"), Some(CoverageReportFormat::Html));
    assert_eq!(CoverageReportFormat::from_str("xml"), Some(CoverageReportFormat::Xml));
    assert_eq!(CoverageReportFormat::from_str("json"), Some(CoverageReportFormat::Json));
    assert_eq!(CoverageReportFormat::from_str("lcov"), Some(CoverageReportFormat::Lcov));
    assert_eq!(CoverageReportFormat::from_str("invalid"), None);
}

#[test]
fn test_coverage_args_parse() {
    let args = vec![
        "--cov".to_string(),
        "src".to_string(),
        "--cov-report".to_string(),
        "html".to_string(),
        "--cov-fail-under".to_string(),
        "80".to_string(),
        "--cov-branch".to_string(),
    ];
    
    let parsed = CoverageArgs::parse(&args);
    
    assert!(parsed.is_enabled());
    assert_eq!(parsed.cov, vec!["src".to_string()]);
    assert_eq!(parsed.cov_report, vec!["html".to_string()]);
    assert_eq!(parsed.cov_fail_under, Some(80.0));
    assert!(parsed.cov_branch);
}

#[test]
fn test_coverage_args_parse_equals_syntax() {
    let args = vec![
        "--cov=src".to_string(),
        "--cov-report=xml".to_string(),
        "--cov-fail-under=90".to_string(),
    ];
    
    let parsed = CoverageArgs::parse(&args);
    
    assert!(parsed.is_enabled());
    assert_eq!(parsed.cov, vec!["src".to_string()]);
    assert_eq!(parsed.cov_report, vec!["xml".to_string()]);
    assert_eq!(parsed.cov_fail_under, Some(90.0));
}

#[test]
fn test_coverage_config_default() {
    let config = CoverageConfig::default();
    
    assert!(config.source.is_empty());
    assert!(!config.omit.is_empty()); // Has default omit patterns
    assert_eq!(config.report_formats, vec![CoverageReportFormat::Term]);
    assert!(config.fail_under.is_none());
    assert!(!config.branch);
}

#[test]
fn test_coverage_config_from_args() {
    let args = CoverageArgs {
        cov: vec!["src".to_string(), "lib".to_string()],
        cov_report: vec!["html".to_string(), "xml".to_string()],
        cov_fail_under: Some(85.0),
        cov_branch: true,
        cov_append: false,
        cov_config: None,
    };
    
    let config = CoverageConfig::from_args(&args);
    
    assert_eq!(config.source.len(), 2);
    assert_eq!(config.report_formats.len(), 2);
    assert_eq!(config.fail_under, Some(85.0));
    assert!(config.branch);
}

#[test]
fn test_file_coverage_calculate_line_rate() {
    let file_cov = FileCoverage {
        path: PathBuf::from("test.py"),
        executed_lines: vec![1, 2, 3, 4, 5, 6, 7, 8],
        missing_lines: vec![9, 10],
        branches_taken: Vec::new(),
        branches_missing: Vec::new(),
        line_rate: 0.0,
        branch_rate: None,
    };
    
    let rate = file_cov.calculate_line_rate();
    assert!((rate - 80.0).abs() < 0.01);
}

#[test]
fn test_file_coverage_empty() {
    let file_cov = FileCoverage {
        path: PathBuf::from("empty.py"),
        executed_lines: Vec::new(),
        missing_lines: Vec::new(),
        branches_taken: Vec::new(),
        branches_missing: Vec::new(),
        line_rate: 0.0,
        branch_rate: None,
    };
    
    let rate = file_cov.calculate_line_rate();
    assert!((rate - 100.0).abs() < 0.01);
}

#[test]
fn test_coverage_data_calculate_totals() {
    let mut data = CoverageData::default();
    
    data.files.insert(
        PathBuf::from("file1.py"),
        FileCoverage {
            path: PathBuf::from("file1.py"),
            executed_lines: vec![1, 2, 3],
            missing_lines: vec![4, 5],
            branches_taken: Vec::new(),
            branches_missing: Vec::new(),
            line_rate: 60.0,
            branch_rate: None,
        },
    );
    
    data.files.insert(
        PathBuf::from("file2.py"),
        FileCoverage {
            path: PathBuf::from("file2.py"),
            executed_lines: vec![1, 2, 3, 4, 5],
            missing_lines: Vec::new(),
            branches_taken: Vec::new(),
            branches_missing: Vec::new(),
            line_rate: 100.0,
            branch_rate: None,
        },
    );
    
    data.calculate_totals();
    
    assert_eq!(data.lines_covered, 8);
    assert_eq!(data.lines_total, 10);
    assert!((data.total_line_rate - 80.0).abs() < 0.01);
}

#[test]
fn test_coverage_data_meets_threshold() {
    let data = CoverageData {
        total_line_rate: 85.0,
        ..Default::default()
    };
    
    assert!(data.meets_threshold(80.0));
    assert!(data.meets_threshold(85.0));
    assert!(!data.meets_threshold(90.0));
}

#[test]
fn test_coverage_collector_start() {
    let config = CoverageConfig {
        source: vec![PathBuf::from("src")],
        branch: true,
        ..Default::default()
    };
    
    let mut collector = CoverageCollector::new(config);
    let code = collector.start();
    
    assert!(code.contains("import coverage"));
    assert!(code.contains("_cov = coverage.Coverage"));
    assert!(code.contains("source=['src']"));
    assert!(code.contains("branch=True"));
    assert!(code.contains("_cov.start()"));
    assert!(collector.is_active());
}

#[test]
fn test_coverage_collector_stop() {
    let config = CoverageConfig::default();
    let mut collector = CoverageCollector::new(config);
    collector.start();
    
    let code = collector.stop();
    
    assert!(code.contains("_cov.stop()"));
    assert!(code.contains("_cov.save()"));
    assert!(code.contains("json.dumps"));
    assert!(!collector.is_active());
}

#[test]
fn test_coverage_collector_parse_json() {
    let config = CoverageConfig::default();
    let mut collector = CoverageCollector::new(config);
    
    let json = r#"{
        "files": {
            "test.py": {
                "executed_lines": [1, 2, 3, 4, 5],
                "missing_lines": [6, 7]
            }
        },
        "totals": {
            "lines_covered": 5,
            "lines_total": 7
        }
    }"#;
    
    collector.parse_coverage_json(json).unwrap();
    
    let data = collector.get_data();
    assert_eq!(data.files.len(), 1);
    assert_eq!(data.lines_covered, 5);
    assert_eq!(data.lines_total, 7);
}

#[test]
fn test_coverage_collector_generate_term_report() {
    let config = CoverageConfig::default();
    let mut collector = CoverageCollector::new(config);
    
    let json = r#"{
        "files": {
            "test.py": {
                "executed_lines": [1, 2, 3, 4, 5, 6, 7, 8],
                "missing_lines": [9, 10]
            }
        }
    }"#;
    
    collector.parse_coverage_json(json).unwrap();
    let report = collector.generate_term_report();
    
    assert!(report.contains("test.py"));
    assert!(report.contains("TOTAL"));
    assert!(report.contains("80%"));
}

#[test]
fn test_coverage_collector_generate_json_report() {
    let config = CoverageConfig::default();
    let mut collector = CoverageCollector::new(config);
    
    let json = r#"{
        "files": {
            "test.py": {
                "executed_lines": [1, 2, 3],
                "missing_lines": [4, 5]
            }
        }
    }"#;
    
    collector.parse_coverage_json(json).unwrap();
    let report = collector.generate_json_report();
    
    assert!(report.contains("test.py"));
    assert!(report.contains("executed_lines"));
    assert!(report.contains("missing_lines"));
}

#[test]
fn test_coverage_collector_generate_xml_report() {
    let config = CoverageConfig::default();
    let mut collector = CoverageCollector::new(config);
    
    let json = r#"{
        "files": {
            "test.py": {
                "executed_lines": [1, 2, 3],
                "missing_lines": [4, 5]
            }
        }
    }"#;
    
    collector.parse_coverage_json(json).unwrap();
    let report = collector.generate_xml_report();
    
    assert!(report.contains("<?xml version"));
    assert!(report.contains("<coverage"));
    assert!(report.contains("test.py"));
    assert!(report.contains("<line number=\"1\" hits=\"1\""));
    assert!(report.contains("<line number=\"4\" hits=\"0\""));
}

#[test]
fn test_coverage_collector_generate_lcov_report() {
    let config = CoverageConfig::default();
    let mut collector = CoverageCollector::new(config);
    
    let json = r#"{
        "files": {
            "test.py": {
                "executed_lines": [1, 2, 3],
                "missing_lines": [4, 5]
            }
        }
    }"#;
    
    collector.parse_coverage_json(json).unwrap();
    let report = collector.generate_lcov_report();
    
    assert!(report.contains("SF:test.py"));
    assert!(report.contains("DA:1,1"));
    assert!(report.contains("DA:4,0"));
    assert!(report.contains("LF:5"));
    assert!(report.contains("LH:3"));
    assert!(report.contains("end_of_record"));
}

#[test]
fn test_coverage_collector_check_threshold_pass() {
    let config = CoverageConfig {
        fail_under: Some(70.0),
        ..Default::default()
    };
    let mut collector = CoverageCollector::new(config);
    
    let json = r#"{
        "files": {
            "test.py": {
                "executed_lines": [1, 2, 3, 4, 5, 6, 7, 8],
                "missing_lines": [9, 10]
            }
        }
    }"#;
    
    collector.parse_coverage_json(json).unwrap();
    assert!(collector.check_threshold().is_ok());
}

#[test]
fn test_coverage_collector_check_threshold_fail() {
    let config = CoverageConfig {
        fail_under: Some(90.0),
        ..Default::default()
    };
    let mut collector = CoverageCollector::new(config);
    
    let json = r#"{
        "files": {
            "test.py": {
                "executed_lines": [1, 2, 3, 4, 5, 6, 7, 8],
                "missing_lines": [9, 10]
            }
        }
    }"#;
    
    collector.parse_coverage_json(json).unwrap();
    let result = collector.check_threshold();
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(err_msg.contains("80.0%"));
}

#[test]
fn test_coverage_config_to_coverage_rc() {
    let config = CoverageConfig {
        source: vec![PathBuf::from("src"), PathBuf::from("lib")],
        omit: vec!["*test*".to_string()],
        branch: true,
        show_missing: true,
        skip_covered: true,
        fail_under: Some(80.0),
        ..Default::default()
    };
    
    let rc = config.to_coverage_rc();
    
    assert!(rc.contains("[run]"));
    assert!(rc.contains("source = src,lib"));
    assert!(rc.contains("omit = *test*"));
    assert!(rc.contains("branch = True"));
    assert!(rc.contains("[report]"));
    assert!(rc.contains("show_missing = True"));
    assert!(rc.contains("skip_covered = True"));
    assert!(rc.contains("fail_under = 80"));
}
