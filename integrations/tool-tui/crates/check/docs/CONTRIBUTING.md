
# Contributing to dx-check

Thank you for your interest in contributing to dx-check!

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Git

### Setup

```bash
git clone github.com/nicholasoxford/dx.git cd dx cargo build -p dx-check --release cargo test -p dx-check --release cargo clippy -p dx-check ```


## Project Structure


@tree:crates/check[]


## Adding a New Rule



### 1. Create the Rule File


Create a new file in `src/rules/builtin/`:
```rust
use crate::diagnostics::{Diagnostic,DiagnosticSeverity,Span};use crate::rules::{Rule,RuleCategory,RuleId};use oxc_ast::ast::*;use oxc_ast::visit::Visit;pub struct MyRule;impl Rule for MyRule {fn id(&self)->RuleId {RuleId::new("my-rule")}fn name(&self)->&'static str { "my-rule"
}
fn category(&self) -> RuleCategory { RuleCategory::Suspicious }
fn description(&self) -> &'static str {"Description of what this rule checks"}fn default_severity(&self)->DiagnosticSeverity {DiagnosticSeverity::Warning }fn is_fixable(&self)->bool {false }fn check(&self,source:&str,ast:&Program)->Vec<Diagnostic>{let mut visitor =MyRuleVisitor {source,diagnostics:Vec::new(),};visitor.visit_program(ast);visitor.diagnostics }}struct MyRuleVisitor<'a> { source: &'a str,diagnostics:Vec<Diagnostic>,}impl<'a> Visit<'a>for MyRuleVisitor<'a> { fn visit_call_expression(&mut self, expr: &CallExpression<'a>){}}
```


### 2. Register the Rule


Add to `src/rules/builtin/mod.rs`:
```rust
mod my_rule;pub use my_rule::MyRule;
```
Add to `src/rules/registry.rs`:
```rust
pub fn default_rules()->Vec<Box<dyn Rule>>{vec![Box::new(builtin::MyRule),]}
```


### 3. Add Tests


Add tests in the rule file:
```rust

#[cfg(test)]mod tests {use super::*;#[test]fn test_my_rule_detects_violation(){let rule =MyRule;let source ="// code that violates the rule";let ast =parse(source);let diagnostics =rule.check(source,&ast);assert_eq!(diagnostics.len(),1);}#[test]fn test_my_rule_allows_valid_code(){let rule =MyRule;let source ="// valid code";let ast =parse(source);let diagnostics =rule.check(source,&ast);assert!(diagnostics.is_empty());}}

```


### 4. Add Documentation


Update `docs/RULES.md` with: -Rule description -Rationale -Examples (good and bad) -Configuration options


### 5. Add Fix (Optional)


If the rule is fixable, implement the fix:
```rust
fn fix(&self,source:&str,diagnostic:&Diagnostic)->Option<Fix>{Some(Fix::new("Remove the problematic code",vec![TextEdit::delete(diagnostic.span)],))}
```


## Testing



### Unit Tests


```bash
cargo test -p dx-check --release ```

### Integration Tests

```bash
cargo test -p dx-check --release --test '*' ```


### Property Tests


Property tests use the `proptest` crate:
```bash
cargo test -p dx-check --release -- --ignored ```

### Benchmarks

```bash
cargo bench -p dx-check ```


## Code Style



### Formatting


```bash
cargo fmt -p dx-check ```

### Linting

```bash
cargo clippy -p dx-check -- -D warnings ```


### Documentation


All public items should have doc comments:
```rust
pub fn example(){}
```


## Pull Request Process


- Fork the repository
- Create a feature branch: `git checkout
- b feature/my-feature`
- Make your changes
- Run tests: `cargo test
- p dx-check
- release`
- Run clippy: `cargo clippy
- p dx-check`
- Run fmt: `cargo fmt
- p dx-check`
- Commit with a descriptive message
- Push to your fork
- Open a Pull Request


### PR Checklist


- Tests pass
- Clippy passes with no warnings
- Code is formatted
- Documentation is updated
- CHANGELOG is updated (for user-facing changes)


## Commit Messages


Use conventional commits:
```
feat: add new rule for detecting X fix: correct false positive in no-console docs: update configuration guide test: add property tests for fix application refactor: simplify diagnostic builder perf: optimize AST traversal ```

## Reporting Issues

When reporting issues, please include: -dx-check version (`dx-check --version`) -Operating system -Minimal reproduction case -Expected vs actual behavior

## Feature Requests

Feature requests are welcome! Please: -Check existing issues first -Describe the use case -Provide examples if possible

## Questions

For questions, open a Discussion on GitHub.

## License

By contributing, you agree that your contributions will be licensed under the MIT OR Apache-2.0 license.
