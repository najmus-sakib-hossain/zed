//! Test discovery implementation

use anyhow::Result;
use std::path::Path;

use super::{TestCase, TestSuite};

/// Discover tests in a path
pub fn discover(path: &Path) -> Result<Vec<TestSuite>> {
    let mut suites = Vec::new();

    if path.is_file() {
        if let Some(suite) = discover_file(path)? {
            suites.push(suite);
        }
    } else if path.is_dir() {
        discover_directory(path, &mut suites)?;
    }

    Ok(suites)
}

fn discover_directory(path: &Path, suites: &mut Vec<TestSuite>) -> Result<()> {
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Skip hidden directories and common non-test directories
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name.starts_with('.') || name == "node_modules" || name == "target" {
                continue;
            }
            discover_directory(&path, suites)?;
        } else if path.is_file() {
            if let Some(suite) = discover_file(&path)? {
                suites.push(suite);
            }
        }
    }

    Ok(())
}

fn discover_file(path: &Path) -> Result<Option<TestSuite>> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // Check if this is a test file
    let is_test_file = match ext {
        "rs" => {
            name.ends_with("_test.rs")
                || name == "tests.rs"
                || path.to_string_lossy().contains("/tests/")
        }
        "ts" | "js" => {
            name.ends_with(".test.ts")
                || name.ends_with(".test.js")
                || name.ends_with(".spec.ts")
                || name.ends_with(".spec.js")
        }
        "py" => name.starts_with("test_") || name.ends_with("_test.py"),
        _ => false,
    };

    if !is_test_file {
        return Ok(None);
    }

    let content = std::fs::read_to_string(path)?;
    let tests = match ext {
        "rs" => discover_rust_tests(&content),
        "ts" | "js" => discover_js_tests(&content),
        "py" => discover_python_tests(&content),
        _ => vec![],
    };

    if tests.is_empty() {
        return Ok(None);
    }

    Ok(Some(TestSuite {
        name: path.file_stem().unwrap_or_default().to_string_lossy().to_string(),
        path: path.to_path_buf(),
        tests,
    }))
}

fn discover_rust_tests(content: &str) -> Vec<TestCase> {
    let mut tests = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();

        // Look for #[test] or #[tokio::test]
        if line == "#[test]"
            || line.contains("#[tokio::test]")
            || line.contains("#[async_std::test]")
        {
            // Next non-attribute line should be the function
            let remaining: Vec<&str> = content.lines().skip(line_num + 1).collect();

            for next_line in remaining {
                let next_line = next_line.trim();
                if next_line.starts_with('#') {
                    continue; // Skip attributes
                }

                if let Some(name) = extract_rust_fn_name(next_line) {
                    tests.push(TestCase {
                        name: name.clone(),
                        full_name: name,
                        line: (line_num + 1) as u32,
                        tags: vec![],
                    });
                }
                break;
            }
        }
    }

    tests
}

fn extract_rust_fn_name(line: &str) -> Option<String> {
    // Match: fn test_name(...) or async fn test_name(...)
    let line = line.trim();

    let fn_start = if line.starts_with("async fn ") {
        9
    } else if line.starts_with("fn ") {
        3
    } else if line.starts_with("pub fn ") {
        7
    } else if line.starts_with("pub async fn ") {
        13
    } else {
        return None;
    };

    let rest = &line[fn_start..];
    let name_end = rest.find('(')?;
    let name = rest[..name_end].trim();

    if !name.is_empty() {
        Some(name.to_string())
    } else {
        None
    }
}

fn discover_js_tests(content: &str) -> Vec<TestCase> {
    let mut tests = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();

        // Look for test(), it(), describe()
        if let Some(name) = extract_js_test_name(line) {
            tests.push(TestCase {
                name: name.clone(),
                full_name: name,
                line: (line_num + 1) as u32,
                tags: vec![],
            });
        }
    }

    tests
}

fn extract_js_test_name(line: &str) -> Option<String> {
    let patterns = [
        ("test(", ')'),
        ("it(", ')'),
        ("describe(", ')'),
        ("test.only(", ')'),
        ("it.only(", ')'),
        ("test.skip(", ')'),
    ];

    for (pattern, _) in &patterns {
        if let Some(start) = line.find(pattern) {
            let rest = &line[start + pattern.len()..];

            // Extract string literal
            if let Some(name) = extract_string_literal(rest) {
                return Some(name);
            }
        }
    }

    None
}

fn extract_string_literal(s: &str) -> Option<String> {
    let s = s.trim();

    let (_quote, end_quote) = if s.starts_with('\'') {
        ('\'', '\'')
    } else if s.starts_with('"') {
        ('"', '"')
    } else if s.starts_with('`') {
        ('`', '`')
    } else {
        return None;
    };

    let rest = &s[1..];
    let end = rest.find(end_quote)?;

    Some(rest[..end].to_string())
}

fn discover_python_tests(content: &str) -> Vec<TestCase> {
    let mut tests = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();

        // Look for def test_* or async def test_*
        if let Some(name) = extract_python_test_name(line) {
            tests.push(TestCase {
                name: name.clone(),
                full_name: name,
                line: (line_num + 1) as u32,
                tags: vec![],
            });
        }
    }

    tests
}

fn extract_python_test_name(line: &str) -> Option<String> {
    let prefixes = ["def test_", "async def test_"];

    for prefix in &prefixes {
        if line.starts_with(prefix) {
            let rest = &line[prefix.len()..];
            let name_end = rest.find('(')?;
            let name = &rest[..name_end];
            return Some(format!("test_{}", name));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_rust_tests() {
        let content = r#"
#[test]
fn test_something() {
    assert!(true);
}

#[tokio::test]
async fn test_async_something() {
    assert!(true);
}
"#;

        let tests = discover_rust_tests(content);
        assert_eq!(tests.len(), 2);
        assert_eq!(tests[0].name, "test_something");
        assert_eq!(tests[1].name, "test_async_something");
    }

    #[test]
    fn test_discover_js_tests() {
        let content = r#"
describe('math', () => {
    it('should add numbers', () => {
        expect(1 + 1).toBe(2);
    });
    
    test('should subtract', () => {
        expect(2 - 1).toBe(1);
    });
});
"#;

        let tests = discover_js_tests(content);
        assert_eq!(tests.len(), 3);
    }

    #[test]
    fn test_discover_python_tests() {
        let content = r#"
def test_addition():
    assert 1 + 1 == 2

async def test_async_operation():
    result = await fetch()
    assert result
"#;

        let tests = discover_python_tests(content);
        assert_eq!(tests.len(), 2);
    }
}
