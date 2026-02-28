//! Snapshot tests for dx-check formatter output
//!
//! Uses insta for snapshot testing to verify formatter output stability.
//! **Validates: Requirement 10.6 - Snapshot tests for formatter output**

use dx_check::config::CheckerConfig;
use dx_check::engine::Checker;
use dx_check::languages::{FileProcessor, GoHandler, MarkdownHandler, PythonHandler, TomlHandler};
use insta::assert_snapshot;
use std::path::Path;

// ============================================================================
// JavaScript/TypeScript Formatter Snapshots
// ============================================================================

#[test]
fn snapshot_js_simple_formatting() {
    let input = r#"const   x=1;const y  =  2;
function foo(  a,b,c  ){return a+b+c}"#;

    let checker = Checker::new(CheckerConfig::default());
    let diagnostics = checker.check_source(Path::new("test.js"), input);

    assert_snapshot!("js_simple_diagnostics", format!("{:?}", diagnostics));
}

#[test]
fn snapshot_js_with_issues() {
    let input = r#"
var x = 1;
console.log(x);
debugger;
if (x == 1) {
    eval('alert("hello")');
}
"#;

    let checker = Checker::new(CheckerConfig::default());
    let diagnostics = checker.check_source(Path::new("test.js"), input);

    assert_snapshot!("js_with_issues_diagnostics", format!("{:?}", diagnostics));
}

#[test]
fn snapshot_ts_interface() {
    let input = r#"
interface User{
id:number;
name:string;
email:string;
}

async function fetchUser(id:number):Promise<User>{
const response=await fetch(`/api/users/${id}`);
return response.json();
}
"#;

    let checker = Checker::new(CheckerConfig::default());
    let diagnostics = checker.check_source(Path::new("test.ts"), input);

    assert_snapshot!("ts_interface_diagnostics", format!("{:?}", diagnostics));
}

#[test]
fn snapshot_jsx_component() {
    let input = r#"
import React from 'react';

function Button({onClick,children,disabled}){
return(
<button
className="btn btn-primary"
onClick={onClick}
disabled={disabled}
>
{children}
</button>
);
}
"#;

    let checker = Checker::new(CheckerConfig::default());
    let diagnostics = checker.check_source(Path::new("test.jsx"), input);

    assert_snapshot!("jsx_component_diagnostics", format!("{:?}", diagnostics));
}

// ============================================================================
// TOML Formatter Snapshots
// ============================================================================

#[test]
fn snapshot_toml_formatting() {
    let input = r#"[package]
name="test"
version="1.0.0"
[dependencies]
serde={version="1.0",features=["derive"]}
"#;

    let handler = TomlHandler::new();
    let result = handler.format(Path::new("test.toml"), input, false);

    assert_snapshot!("toml_formatting", format!("{:?}", result));
}

#[test]
fn snapshot_toml_cargo_config() {
    let input = r#"[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"
authors = ["Test Author <test@example.com>"]

[dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }

[dev-dependencies]
criterion = "0.5"

[[bin]]
name = "my-binary"
path = "src/main.rs"
"#;

    let handler = TomlHandler::new();
    let result = handler.format(Path::new("Cargo.toml"), input, false);

    assert_snapshot!("toml_cargo_config", format!("{:?}", result));
}

// ============================================================================
// Markdown Formatter Snapshots
// ============================================================================

#[test]
fn snapshot_markdown_formatting() {
    let input = r#"#Heading without space

##Another heading

-  List item with extra spaces
-List item without space

1.Numbered item
2.  Another numbered item

Some paragraph with   multiple   spaces.
"#;

    let handler = MarkdownHandler::new();
    let result = handler.format(Path::new("test.md"), input, false);

    assert_snapshot!("markdown_formatting", format!("{:?}", result));
}

#[test]
fn snapshot_markdown_code_blocks() {
    let input = r#"# Code Examples

Here is some code:

```javascript
const x = 1;
console.log(x);
```

And some more:

```rust
fn main() {
    println!("Hello");
}
```
"#;

    let handler = MarkdownHandler::new();
    let result = handler.format(Path::new("test.md"), input, false);

    assert_snapshot!("markdown_code_blocks", format!("{:?}", result));
}

// ============================================================================
// Python Formatter Snapshots
// ============================================================================

#[test]
fn snapshot_python_formatting() {
    let input = r#"def foo(a,b,c):
    return a+b+c

class MyClass:
    def __init__(self,x,y):
        self.x=x
        self.y=y
"#;

    let handler = PythonHandler::new();
    let result = handler.format(Path::new("test.py"), input, false);

    assert_snapshot!("python_formatting", format!("{:?}", result));
}

#[test]
fn snapshot_python_imports() {
    let input = r#"import os
import sys
from typing import List,Dict,Optional
from collections import defaultdict,Counter

def process(items:List[str])->Dict[str,int]:
    result=defaultdict(int)
    for item in items:
        result[item]+=1
    return dict(result)
"#;

    let handler = PythonHandler::new();
    let result = handler.format(Path::new("test.py"), input, false);

    assert_snapshot!("python_imports", format!("{:?}", result));
}

// ============================================================================
// Go Formatter Snapshots
// ============================================================================

#[test]
fn snapshot_go_formatting() {
    let input = r#"package main

import "fmt"

func main(){
fmt.Println("Hello")
}

func add(a,b int)int{
return a+b
}
"#;

    let handler = GoHandler::new();
    let result = handler.format(Path::new("test.go"), input, false);

    assert_snapshot!("go_formatting", format!("{:?}", result));
}

#[test]
fn snapshot_go_struct() {
    let input = r#"package main

type User struct{
ID int
Name string
Email string
}

func NewUser(id int,name,email string)*User{
return &User{ID:id,Name:name,Email:email}
}
"#;

    let handler = GoHandler::new();
    let result = handler.format(Path::new("test.go"), input, false);

    assert_snapshot!("go_struct", format!("{:?}", result));
}

// ============================================================================
// Diagnostic Output Snapshots
// ============================================================================

#[test]
fn snapshot_diagnostic_pretty_output() {
    use dx_check::diagnostics::{Diagnostic, DiagnosticSeverity, Span};
    use std::path::PathBuf;

    let diagnostics = vec![
        Diagnostic::error(
            PathBuf::from("test.js"),
            Span::new(10, 20),
            "no-console",
            "Unexpected console statement",
        ),
        Diagnostic::warn(
            PathBuf::from("test.js"),
            Span::new(30, 40),
            "no-debugger",
            "Unexpected debugger statement",
        ),
    ];

    assert_snapshot!("diagnostic_output", format!("{:#?}", diagnostics));
}

#[test]
fn snapshot_diagnostic_with_fix() {
    use dx_check::diagnostics::{Diagnostic, Fix, Span};
    use std::path::PathBuf;

    let diagnostic = Diagnostic::warn(
        PathBuf::from("test.js"),
        Span::new(0, 9),
        "no-debugger",
        "Unexpected debugger statement",
    )
    .with_fix(Fix::delete("Remove debugger", Span::new(0, 10)));

    assert_snapshot!("diagnostic_with_fix", format!("{:#?}", diagnostic));
}

// ============================================================================
// Check Result Snapshots
// ============================================================================

#[test]
fn snapshot_check_result_clean() {
    let input = r#"
const x = 1;
const y = 2;
const sum = x + y;
"#;

    let checker = Checker::new(CheckerConfig::default());
    let result = checker.check_source(Path::new("test.js"), input);

    assert_snapshot!("check_result_clean", format!("{:?}", result));
}

#[test]
fn snapshot_check_result_with_errors() {
    let input = r#"
var x = 1;
console.log(x);
debugger;
eval("alert('hello')");
"#;

    let checker = Checker::new(CheckerConfig::default());
    let result = checker.check_source(Path::new("test.js"), input);

    assert_snapshot!("check_result_with_errors", format!("{:?}", result));
}

// ============================================================================
// Multi-Language Processor Snapshots
// ============================================================================

#[test]
fn snapshot_file_processor_routing() {
    let processor = FileProcessor::new();

    let extensions = vec![
        "js", "jsx", "ts", "tsx", "py", "go", "rs", "toml", "md", "php", "kt", "c", "cpp", "h",
    ];

    let mut routing_info = Vec::new();
    for ext in extensions {
        let path = format!("test.{}", ext);
        let handler = processor.get_handler(Path::new(&path));
        let handler_name = handler.map(|h| h.name()).unwrap_or("none");
        routing_info.push(format!("{}: {}", ext, handler_name));
    }

    assert_snapshot!("file_processor_routing", routing_info.join("\n"));
}

// ============================================================================
// Rule Registry Snapshots
// ============================================================================

#[test]
fn snapshot_builtin_rules() {
    use dx_check::rules::RuleRegistry;

    let registry = RuleRegistry::with_builtins();
    let mut rules: Vec<String> = registry
        .rule_names()
        .iter()
        .map(|name| {
            let rule = registry.get(name).unwrap();
            let meta = rule.meta();
            format!(
                "{}: {} (category: {}, fixable: {}, recommended: {})",
                meta.name,
                meta.description,
                meta.category.as_str(),
                meta.fixable,
                meta.recommended
            )
        })
        .collect();

    rules.sort();
    assert_snapshot!("builtin_rules", rules.join("\n"));
}

// ============================================================================
// Configuration Snapshots
// ============================================================================

#[test]
fn snapshot_default_config() {
    let config = CheckerConfig::default();
    let toml_str = toml::to_string_pretty(&config).unwrap();

    assert_snapshot!("default_config", toml_str);
}

#[test]
fn snapshot_custom_config() {
    let mut config = CheckerConfig::default();
    config.parallel.threads = 4;
    config.cache.enabled = true;
    config.cache.max_size = 512 * 1024 * 1024;
    config.format.indent_width = 4;
    config.format.line_width = 100;

    let toml_str = toml::to_string_pretty(&config).unwrap();

    assert_snapshot!("custom_config", toml_str);
}
