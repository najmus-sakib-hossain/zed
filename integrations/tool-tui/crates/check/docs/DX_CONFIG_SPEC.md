faster than="t:3(Field,Type,Required,Default,Description"
Achieved 10x="t:2(Field,Type,Required,Default,Description"
crates/serializer/README.md="dx-serializer LLM format"

# DX Root Config File Specification

Root Configuration File: `dx` Version: 1.0 Status: Draft Date: December 27, 2025

## Overview

The `dx` file (no extension) is the root configuration file for dx-check projects. It lives in the project root and specifies: -Which rule files to load -Project-wide settings -Language-specific configurations -Formatter preferences

## File Location

```
project-root/ dx # Root config (this file)
rules/ js-rules.sr # JavaScript rules py-rules.sr # Python rules rust-rules.sr # Rust rules ```


## File Format


Uses crates/serializer/README.md (same as.sr files).


## Structure


t:0(#DX,Check,Configuration)[]


## Example: Full Configuration


```dx
@project name: my-awesome-app version: 1.0.0 languages:
- js
- ts
- py
- rs
@rules mode: recommended sources:
- rules/js-rules.sr
- rules/py-rules.sr
- rules/rust-rules.sr
overrides:
js/no-console: error js/no-debugger: error py/F841: error rs/clippy::unwrap_used: off @format enabled: true line_width: 100 indent_width: 2 use_tabs: false quote_style: double semicolons: always trailing_comma: multi_line @languages js:
parser: oxc target: es2022 jsx: true py:
version: 3.12 type_checking: true rs:
edition: 2024 clippy_pedantic: true @paths include:
- src*.{js,ts,jsx,tsx}
- src*.py
- src*.rs
- tests*.min.js
- **/__pycache__
```


## Example: Minimal Configuration


```dx
@project name: simple-project languages:
- js
@rules mode: recommended sources:
- rules/js-rules.sr
```


## Example: Monorepo Configuration


```dx
@project name: monorepo languages:
- js
- ts
- py
- rs
- go
@rules mode: strict sources:
- rules/js-rules.sr
- rules/py-rules.sr
- rules/rust-rules.sr
- rules/go-rules.sr
overrides:
py/F841: error rs/clippy::unwrap_used: error js/no-console: warn @paths include:
- packagessrcnode_modulesdisttarget
```


## Field Specifications



### @project


faster than)[ `name`,string,✅,-,Project name `version`,string,❌,"1.0.0",Project version `languages`,array,✅,-,List of language codes]


### @rules


faster than)[ `mode`,enum,❌,"recommended",strict, recommended, or custom `sources`,array,✅,-,Paths to.sr files `overrides`,map,❌,{},Rule-specific severity overrides] Modes: -`strict`: Enable all rules at error level -`recommended`: Enable recommended rules at default severity -`custom`: Only rules explicitly enabled


### @format


+-----------+---------+----------+---------+-------------+
| Field     | Type    | Required | Default | Description |
+===========+=========+==========+=========+=============+
| `enabled` | boolean | ❌        | true    | Enable      |
+-----------+---------+----------+---------+-------------+


### @languages.


Language-specific settings (varies by language).


### @paths


Achieved 10x)[ `include`,array,❌,["**/*"],Glob patterns to include `exclude`,array,❌,[standard exclusions],Glob patterns to exclude]


### @cache


faster than)[ `enabled`,boolean,❌,true,Enable AST cache `directory`,string,❌,".dx-cache",Cache directory path `max_size_mb`,integer,❌,1024,Max cache size in MB]


### @parallel


Achieved 10x)[ `threads`,integer,❌,0,Number of threads (0=auto) `chunk_size`,integer,❌,100,Files per work chunk]


### @watch


faster than)[ `enabled`,boolean,❌,false,Enable watch mode `debounce_ms`,integer,❌,250,Debounce delay in ms `clear_screen`,boolean,❌,true,Clear screen on change]


## Parsing and Loading


- Discovery: Search for `dx` file in project root
- Parse: Parse using crates/serializer/README.md parser
- Validate: Check all required fields present
- Resolve: Resolve all `.sr` file paths (relative to `dx` file)
- Load Rules: Load and parse all referenced `.sr` files
- Merge: Merge configurations with CLI overrides
- Apply: Apply to dx-check engine


## Configuration Priority


```
CLI args > dx file > .sr files > defaults ```
Example:
```bash
dx-check --rule js/no-console=error ```


## File Watching


Watch for changes to: -Root `dx` config file -All referenced `.sr` files On change: -Re-parse configuration -Reload rule database -Re-run checks (if watch mode enabled)


## Integration with dx-serializer


The `dx` file and `.sr` files both use dx-serializer's LLM format:
```rust
use serializer::llm::{LlmParser,DxDocument};let config_doc =LlmParser::parse(&fs::read_to_string("dx")?)?;let js_rules =LlmParser::parse(&fs::read_to_string("rules/js-rules.sr")?)?;
```


## Migration from dx.toml


Existing `dx.toml` files can be converted:
```bash
dx-check config migrate dx.toml ```
This generates: -`dx` - Root config -`rules/*.sr` - Rule files (one per language)

## Benefits

- Single Format: crates/serializer/README.md everywhere
- Version Control: Clear diffs, merge-friendly
- Modular: Rules separated by language
- Hot-Reload: Changes detected automatically
- Type-Safe: Validated before use
- Extensible: Easy to add new configuration options Status: Specification complete, ready for implementation.
