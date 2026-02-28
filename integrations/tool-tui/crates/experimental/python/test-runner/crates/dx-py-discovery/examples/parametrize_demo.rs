//! Demo of parametrize parsing functionality
//!
//! Run with: cargo run --example parametrize_demo -p dx-py-discovery

use dx_py_discovery::{ParametrizeExpander, TestScanner};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Parametrize Parsing Demo ===\n");

    // Scan the demo test file
    let test_file = Path::new("test_parametrize_demo.py");
    
    if !test_file.exists() {
        eprintln!("Error: test_parametrize_demo.py not found at {:?}", test_file.canonicalize().unwrap_or_else(|_| test_file.to_path_buf()));
        eprintln!("Current directory: {:?}", std::env::current_dir()?);
        eprintln!("Please ensure test_parametrize_demo.py exists in the test-runner directory");
        return Ok(());
    }
    
    let mut scanner = TestScanner::new()?;
    let tests = scanner.scan_file(test_file)?;

    println!("Found {} test functions\n", tests.len());

    // Expand parametrized tests
    let expander = ParametrizeExpander::new();

    for test in &tests {
        println!("Test: {}", test.name);
        println!("  Line: {}", test.line_number);
        println!("  Markers: {:?}", test.markers);

        let expanded = expander.expand(test);
        println!("  Expanded to {} test cases:", expanded.len());

        for (idx, exp_test) in expanded.iter().enumerate() {
            println!("    [{}] ID: {}", idx, exp_test.full_id());
            println!("        Parameters: {:?}", exp_test.param_values);
            if exp_test.expected_failure {
                println!("        Expected to fail: yes");
            }
            if let Some(reason) = &exp_test.skip_reason {
                println!("        Skip reason: {}", reason);
            }
        }
        println!();
    }

    // Summary
    let total_expanded: usize = tests.iter().map(|t| expander.expand(t).len()).sum();
    println!("=== Summary ===");
    println!("Original tests: {}", tests.len());
    println!("Expanded tests: {}", total_expanded);

    Ok(())
}
