//! Unit tests for built-in rules from .sr files
//! **Validates: Requirement 6.3**

use std::path::PathBuf;

fn get_rules_dir() -> PathBuf {
    PathBuf::from("rules/sr")
}

fn count_sr_files(dir: &PathBuf) -> usize {
    if !dir.exists() {
        return 0;
    }
    std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("sr"))
        .count()
}

fn read_rule_file(path: &PathBuf) -> String {
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn test_total_rule_count() {
    let categories = [
        "formatting",
        "linting",
        "security",
        "design",
        "structure",
        "examples",
    ];
    let total: usize = categories.iter().map(|c| count_sr_files(&get_rules_dir().join(c))).sum();
    assert!(total >= 50, "Expected at least 50 rules, found {}", total);
}

#[test]
fn test_formatting_rules_count() {
    let count = count_sr_files(&get_rules_dir().join("formatting"));
    assert!(count >= 10, "Expected at least 10 formatting rules, found {}", count);
}

#[test]
fn test_security_rules_count() {
    let count = count_sr_files(&get_rules_dir().join("security"));
    assert!(count >= 10, "Expected at least 10 security rules, found {}", count);
}

#[test]
fn test_rule_no_eval_exists() {
    let path = get_rules_dir().join("security/js-no-eval.sr");
    assert!(path.exists());
    let content = read_rule_file(&path);
    assert!(content.contains("language: js"));
    assert!(content.contains("category: security"));
    assert!(content.contains("severity: error"));
}

#[test]
fn test_formatting_rules_are_fixable() {
    let dir = get_rules_dir().join("formatting");
    let mut fixable = 0;
    let mut total = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|s| s.to_str()) == Some("sr") {
            total += 1;
            if read_rule_file(&path).contains("fixable: true") {
                fixable += 1;
            }
        }
    }
    assert!(fixable as f64 / total as f64 >= 0.9);
}

#[test]
fn test_design_rules_not_fixable() {
    let dir = get_rules_dir().join("design");
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|s| s.to_str()) == Some("sr") {
            assert!(read_rule_file(&path).contains("fixable: false"));
        }
    }
}
