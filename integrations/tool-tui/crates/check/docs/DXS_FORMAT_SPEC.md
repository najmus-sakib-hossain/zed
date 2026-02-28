faster than="boolean,✅,Whether"

#.sr File Format Specification DX Serializer (SR) Rule Definition Format Version: 1.0 Status: Draft Date: December 27, 2025

## Overview

`.sr` files are human-readable rule definition files that use dx-serializer's LLM format. They serve as the source of truth for lint and format rules, which are then compiled into binary `.dxm` files for runtime execution.

## File Naming Convention

```
<language>-rules.sr ```
Examples: -`js-rules.sr` - JavaScript/TypeScript rules -`py-rules.sr` - Python rules -`rust-rules.sr` - Rust rules -`go-rules.sr` - Go rules


## File Structure


t:0(#Language,Rules,for)[]


## Field Specifications



### @meta Section


+------------+--------+----------+-------------+
| Field      | Type   | Required | Description |
+============+========+==========+=============+
| `language` | string | ✅        | Language    |
+------------+--------+----------+-------------+


### @rule Section


+--------+--------+----------+-------------+
| Field  | Type   | Required | Description |
+========+========+==========+=============+
| `name` | string | ✅        | Rule        |
+--------+--------+----------+-------------+


## Example: js-rules.sr


```sr
@meta language: js source: dx-check,biome,oxc version: 1.0.0 total_rules: 8 @rule 1 name: no-console prefixed_name: js/no-console category: suspicious severity: warn fixable: true recommended: true is_formatter: false description: |
Disallow the use of console statements.
Console statements are often used for debugging and should be removed before production deployment.
docs_url: https:
options_schema: | { "type": "object", "properties": { "allow": { "type": "array", "items": {"type": "string"}, "description": "List of console methods to allow"
}
}
}
related_rules:
- js/no-debugger
- js/no-alert
examples:
- type: incorrect
code: | console.log('debug');
console.error('error');
- type: correct
code: | logger.info('production log');
@rule 2 name: no-debugger prefixed_name: js/no-debugger category: suspicious severity: error fixable: true recommended: true is_formatter: false description: |
Disallow the use of debugger statements.
Debugger statements should never reach production code.
docs_url: https:
examples:
- type: incorrect
code: | function debug() { debugger;
}
- type: correct
code: | function debug() { }
```


## Example: rust-rules.sr


```sr
@meta language: rs source: rustfmt,clippy version: 1.0.0 total_rules: 16 @rule 1 name: fmt prefixed_name: rs/fmt category: format severity: warn fixable: true recommended: true is_formatter: true description: | Format Rust code using rustfmt.
Ensures consistent code style across Rust projects.
docs_url: https:
@rule 2 name: clippy::unwrap_used prefixed_name: rs/clippy::unwrap_used category: correctness severity: warn fixable: false recommended: true is_formatter: false description: | Disallow the use of .unwrap().
Use proper error handling with Result<T, E> and ? operator instead.
docs_url: https:
related_rules:
- rs/clippy::expect_used
- rs/clippy::panic
examples:
- type: incorrect
code: | let value = option.unwrap();
- type: correct
code: | let value = option?;
```


## Parsing Rules


- Comments: Lines starting with `#` are comments (ignored)
- Sections: Start with `@` followed by section name
- Indentation: Two spaces for nested properties
- Multi-line Values: Use `|` after colon, indent content
- Arrays: Each item on new line with `-` prefix
- Booleans: `true`, `false` (lowercase)
- Enums: Use exact enum variant names (case-sensitive)


## Validation Rules


- All required fields must be present
- Rule IDs must be unique within a file
- Prefixed names must match pattern: `<language>/<name>`
- Categories must be valid enum variants
- Severity must be: warn, error, or off
- JSON schema in `options_schema` must be valid JSON
- Examples must specify `type: correct` or `type: incorrect`


## Compilation Process


```
.sr files Parser Validator DxRuleDatabase Compiler .dxm binary ```
- Parse: Read.sr files and parse into DxRule structs
- Validate: Check all rules meet specification
- Merge: Combine all language rules into single database
- Compile: Serialize to binary.dxm format
- Verify: Validate binary format integrity

## File Watching

The dx-serializer will watch for changes to: -Root `dx` config file -All `*.sr` files in project root or `rules/` directory On change: -Re-parse affected.sr files -Re-validate rules -Re-compile to.dxm -Notify dx-check to reload rules (hot-reload in dev mode)

## Benefits

- Human-Readable: Easy to edit and review
- Version Control Friendly: Clear diffs, merge-friendly
- Contributor Accessible: No binary editing required
- Documented: Examples and descriptions inline
- Type-Safe: Validated before compilation
- Fast Runtime: Compiled to 0.70ns access binary format

## Migration Path

Existing extractor.rs code will be converted to generate.sr files:
```rust
extract_all_rules()generate_dxs_files()
```
Each language gets its own.sr file, making it easy for contributors to: -Add new rules -Update rule descriptions -Modify rule configurations -Add examples Status: Specification complete, ready for implementation.
