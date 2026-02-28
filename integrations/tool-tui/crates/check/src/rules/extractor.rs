//! Rule Extractor
//!
//! Extracts lint and format rules from all 12 language submodules:
//! - biome (JS/TS/JSON/CSS)
//! - oxc (JS/TS)
//! - ruff (Python)
//! - mago (PHP)
//! - gofmt.rs, gold (Go)
//! - rustfmt, rust-clippy (Rust)
//! - taplo (TOML)
//! - rumdl (Markdown)
//! - cpp-linter-rs (C/C++)
//! - ktlint (Kotlin)

use super::builtin;
use super::schema::{DxCategory, DxRule, DxRuleDatabase, DxSeverity, Language, RuleSource};

/// Extract all rules from all sources into a unified database
#[must_use]
pub fn extract_all_rules() -> DxRuleDatabase {
    let mut db = DxRuleDatabase::new();
    let mut next_id = 1u16;

    // Extract from all sources
    next_id = extract_builtin_rules(&mut db, next_id);
    next_id = extract_biome_rules(&mut db, next_id);
    next_id = extract_oxc_rules(&mut db, next_id);
    next_id = extract_ruff_rules(&mut db, next_id);
    next_id = extract_mago_rules(&mut db, next_id);
    next_id = extract_go_rules(&mut db, next_id);
    next_id = extract_rust_rules(&mut db, next_id);
    next_id = extract_toml_rules(&mut db, next_id);
    next_id = extract_markdown_rules(&mut db, next_id);
    next_id = extract_cpp_rules(&mut db, next_id);
    next_id = extract_kotlin_rules(&mut db, next_id);
    next_id = extract_json_rules(&mut db, next_id);
    next_id = extract_css_rules(&mut db, next_id);
    next_id = extract_html_rules(&mut db, next_id);
    let _ = extract_yaml_rules(&mut db, next_id);

    println!("✅ Extracted {} total rules", db.rule_count);
    db
}

/// Extract built-in dx-check rules
fn extract_builtin_rules(db: &mut DxRuleDatabase, mut rule_id: u16) -> u16 {
    println!("📦 Extracting dx-check built-in rules...");

    let rules = builtin::all_rules();
    for rule in &rules {
        let meta = rule.meta();

        let category = match meta.category {
            super::Category::Correctness => DxCategory::Correctness,
            super::Category::Suspicious => DxCategory::Suspicious,
            super::Category::Style => DxCategory::Style,
            super::Category::Performance => DxCategory::Performance,
            super::Category::Security => DxCategory::Security,
            super::Category::Complexity => DxCategory::Complexity,
            super::Category::A11y => DxCategory::Accessibility,
            super::Category::Imports => DxCategory::Imports,
        };

        let severity = match meta.default_severity {
            super::Severity::Off => DxSeverity::Off,
            super::Severity::Warn => DxSeverity::Warn,
            super::Severity::Error => DxSeverity::Error,
        };

        let mut dx_rule = DxRule::new(
            rule_id,
            Language::JavaScript,
            meta.name,
            meta.description,
            category,
            RuleSource::DxCheck,
        )
        .severity(severity)
        .fixable(meta.fixable)
        .recommended(meta.recommended);

        if let Some(url) = meta.docs_url {
            dx_rule = dx_rule.docs(url);
        }

        db.add_rule(dx_rule);
        rule_id += 1;
    }

    println!("  ✓ {} dx-check rules", rules.len());
    rule_id
}

/// Extract Biome rules (JS/TS/JSON/CSS)
fn extract_biome_rules(db: &mut DxRuleDatabase, mut rule_id: u16) -> u16 {
    println!("📦 Extracting Biome rules...");

    // Common JavaScript/TypeScript rules from Biome
    let biome_js_rules = vec![
        (
            "noAccumulatingSpread",
            "Disallow spreading objects that may have large number of properties",
            DxCategory::Performance,
        ),
        (
            "noAssignInExpressions",
            "Disallow assignments in expressions",
            DxCategory::Suspicious,
        ),
        (
            "noAsyncPromiseExecutor",
            "Disallow async Promise executors",
            DxCategory::Suspicious,
        ),
        (
            "noCatchAssign",
            "Disallow reassigning exceptions in catch clauses",
            DxCategory::Suspicious,
        ),
        ("noClassAssign", "Disallow reassigning class members", DxCategory::Suspicious),
        (
            "noCommentText",
            "Prevent comments from being inserted as text nodes",
            DxCategory::Suspicious,
        ),
        ("noCompareNegZero", "Disallow comparing against -0", DxCategory::Suspicious),
        ("noConstAssign", "Disallow reassigning const variables", DxCategory::Correctness),
        ("noConstantCondition", "Disallow constant conditions", DxCategory::Correctness),
        (
            "noControlCharactersInRegex",
            "Disallow control characters in regex",
            DxCategory::Suspicious,
        ),
        ("noDebugger", "Disallow debugger statements", DxCategory::Suspicious),
        ("noDoubleEquals", "Require === and !==", DxCategory::Suspicious),
        ("noDuplicateCase", "Disallow duplicate case labels", DxCategory::Suspicious),
        (
            "noDuplicateClassMembers",
            "Disallow duplicate class members",
            DxCategory::Suspicious,
        ),
        (
            "noDuplicateObjectKeys",
            "Disallow duplicate keys in object literals",
            DxCategory::Suspicious,
        ),
        ("noDuplicateParameters", "Disallow duplicate parameters", DxCategory::Suspicious),
        (
            "noEmptyCharacterClassInRegex",
            "Disallow empty character classes in regex",
            DxCategory::Suspicious,
        ),
        (
            "noEmptyPattern",
            "Disallow empty destructuring patterns",
            DxCategory::Suspicious,
        ),
        (
            "noExtraNonNullAssertion",
            "Disallow extra non-null assertions",
            DxCategory::Suspicious,
        ),
        (
            "noFallthroughSwitchClause",
            "Disallow fallthrough switch cases",
            DxCategory::Suspicious,
        ),
        (
            "noGlobalObjectCalls",
            "Disallow calling global objects as functions",
            DxCategory::Suspicious,
        ),
        (
            "noInnerDeclarations",
            "Disallow variable or function declarations in nested blocks",
            DxCategory::Suspicious,
        ),
        (
            "noInvalidConstructorSuper",
            "Disallow invalid super() calls",
            DxCategory::Correctness,
        ),
        (
            "noNonoctalDecimalEscape",
            "Disallow \\8 and \\9 escape sequences",
            DxCategory::Suspicious,
        ),
        (
            "noPrecisionLoss",
            "Disallow number literals that lose precision",
            DxCategory::Correctness,
        ),
        (
            "noPrototypeBuiltins",
            "Disallow direct use of Object.prototype builtins",
            DxCategory::Suspicious,
        ),
        ("noRedeclare", "Disallow variable redeclaration", DxCategory::Suspicious),
        (
            "noSelfAssign",
            "Disallow assignments where both sides are exactly the same",
            DxCategory::Correctness,
        ),
        (
            "noShadowRestrictedNames",
            "Disallow shadowing of restricted names",
            DxCategory::Suspicious,
        ),
        ("noSparseArray", "Disallow sparse arrays", DxCategory::Suspicious),
        (
            "noUnsafeFinally",
            "Disallow control flow statements in finally blocks",
            DxCategory::Correctness,
        ),
        (
            "noUnsafeOptionalChaining",
            "Disallow unsafe optional chaining",
            DxCategory::Correctness,
        ),
        ("noUnusedLabels", "Disallow unused labels", DxCategory::Correctness),
        ("noUnusedVariables", "Disallow unused variables", DxCategory::Correctness),
        (
            "useValidForDirection",
            "Enforce for loop update clause moving the counter in the right direction",
            DxCategory::Correctness,
        ),
        (
            "useYield",
            "Require generator functions to contain yield",
            DxCategory::Correctness,
        ),
    ];

    for (name, description, category) in biome_js_rules {
        db.add_rule(
            DxRule::new(
                rule_id,
                Language::JavaScript,
                name,
                description,
                category,
                RuleSource::Biome,
            )
            .recommended(true),
        );
        rule_id += 1;
    }

    // TypeScript-specific rules
    let typescript_rules = vec![
        ("noExplicitAny", "Disallow explicit any", DxCategory::Types),
        (
            "noExtraNonNullAssertion",
            "Disallow extra non-null assertions",
            DxCategory::Types,
        ),
        ("useEnumInitializers", "Require enum initializers", DxCategory::Style),
        ("useExportType", "Prefer type-only exports when possible", DxCategory::Style),
        ("useImportType", "Prefer type-only imports when possible", DxCategory::Style),
    ];

    for (name, description, category) in typescript_rules {
        db.add_rule(
            DxRule::new(
                rule_id,
                Language::TypeScript,
                name,
                description,
                category,
                RuleSource::Biome,
            )
            .recommended(true),
        );
        rule_id += 1;
    }

    // JSON rules
    let biome_json_rules = vec![
        ("noDuplicateKeys", "Disallow duplicate keys", DxCategory::Correctness),
        ("noTrailingCommas", "Disallow trailing commas", DxCategory::Style),
        ("noComments", "Disallow comments in JSON", DxCategory::Correctness),
    ];

    for (name, description, category) in biome_json_rules {
        db.add_rule(
            DxRule::new(rule_id, Language::Json, name, description, category, RuleSource::Biome)
                .recommended(true),
        );
        rule_id += 1;
    }

    // CSS rules
    let biome_css_rules = vec![
        ("noDuplicateSelectors", "Disallow duplicate selectors", DxCategory::Correctness),
        ("noEmptyBlock", "Disallow empty blocks", DxCategory::Style),
        (
            "noInvalidPositionAtImportRule",
            "Disallow invalid position for @import",
            DxCategory::Correctness,
        ),
    ];

    for (name, description, category) in biome_css_rules {
        db.add_rule(
            DxRule::new(rule_id, Language::Css, name, description, category, RuleSource::Biome)
                .recommended(true),
        );
        rule_id += 1;
    }

    let total = 36 + 5 + 3 + 3;
    println!("  ✓ {total} biome rules (JS/TS/JSON/CSS)");
    rule_id
}

/// Extract OXC rules (JS/TS)
fn extract_oxc_rules(db: &mut DxRuleDatabase, mut rule_id: u16) -> u16 {
    println!("📦 Extracting OXC rules...");

    let oxc_rules = vec![
        (
            "for-direction",
            "Enforce for loop update clause moving the counter in the right direction",
            DxCategory::Correctness,
        ),
        ("getter-return", "Enforce return statements in getters", DxCategory::Correctness),
        (
            "no-async-promise-executor",
            "Disallow async Promise executors",
            DxCategory::Suspicious,
        ),
        ("no-compare-neg-zero", "Disallow comparing against -0", DxCategory::Suspicious),
        (
            "no-cond-assign",
            "Disallow assignment operators in conditional expressions",
            DxCategory::Suspicious,
        ),
        (
            "no-const-assign",
            "Disallow reassigning const variables",
            DxCategory::Correctness,
        ),
        (
            "no-constant-binary-expression",
            "Disallow expressions where the operation doesn't affect the value",
            DxCategory::Correctness,
        ),
        (
            "no-dupe-else-if",
            "Disallow duplicate conditions in if-else-if chains",
            DxCategory::Correctness,
        ),
        ("no-duplicate-case", "Disallow duplicate case labels", DxCategory::Suspicious),
        (
            "no-empty-character-class",
            "Disallow empty character classes in regex",
            DxCategory::Suspicious,
        ),
        (
            "no-ex-assign",
            "Disallow reassigning exceptions in catch clauses",
            DxCategory::Suspicious,
        ),
        (
            "no-fallthrough",
            "Disallow fallthrough of case statements",
            DxCategory::Suspicious,
        ),
        (
            "no-func-assign",
            "Disallow reassigning function declarations",
            DxCategory::Suspicious,
        ),
        (
            "no-import-assign",
            "Disallow assigning to imported bindings",
            DxCategory::Correctness,
        ),
        (
            "no-inner-declarations",
            "Disallow variable or function declarations in nested blocks",
            DxCategory::Suspicious,
        ),
        (
            "no-irregular-whitespace",
            "Disallow irregular whitespace",
            DxCategory::Suspicious,
        ),
        (
            "no-self-assign",
            "Disallow assignments where both sides are exactly the same",
            DxCategory::Correctness,
        ),
        ("no-sparse-arrays", "Disallow sparse arrays", DxCategory::Suspicious),
        (
            "no-unreachable",
            "Disallow unreachable code after return, throw, continue, and break",
            DxCategory::Correctness,
        ),
        (
            "no-unsafe-negation",
            "Disallow negating the left operand of relational operators",
            DxCategory::Suspicious,
        ),
    ];

    for (name, description, category) in &oxc_rules {
        db.add_rule(
            DxRule::new(
                rule_id,
                Language::JavaScript,
                *name,
                *description,
                *category,
                RuleSource::Oxc,
            )
            .recommended(true),
        );
        rule_id += 1;
    }

    println!("  ✓ {} oxc rules", oxc_rules.len());
    rule_id
}

/// Extract Ruff rules (Python)
fn extract_ruff_rules(db: &mut DxRuleDatabase, mut rule_id: u16) -> u16 {
    println!("📦 Extracting Ruff rules (Python)...");

    // Pyflakes (F) rules
    let ruff_rules = vec![
        ("F401", "Unused import", DxCategory::Imports),
        ("F402", "Import shadowed by loop variable", DxCategory::Imports),
        (
            "F403",
            "Star imports used; unable to detect undefined names",
            DxCategory::Imports,
        ),
        ("F404", "Late future import", DxCategory::Imports),
        (
            "F405",
            "Name may be undefined, or defined from star imports",
            DxCategory::Correctness,
        ),
        (
            "F406",
            "from __future__ imports must occur at the beginning of the file",
            DxCategory::Imports,
        ),
        ("F407", "Future feature not defined", DxCategory::Imports),
        ("F501", "Invalid % format string", DxCategory::Correctness),
        ("F502", "% format expected mapping but got sequence", DxCategory::Correctness),
        ("F503", "% format expected sequence but got mapping", DxCategory::Correctness),
        ("F504", "% format unused named arguments", DxCategory::Correctness),
        ("F505", "% format missing named arguments", DxCategory::Correctness),
        ("F506", "% format mixed positional and named arguments", DxCategory::Correctness),
        (
            "F507",
            "% format mismatch of placeholder and argument count",
            DxCategory::Correctness,
        ),
        ("F508", "% format with * specifier requires a sequence", DxCategory::Correctness),
        ("F509", "% format with unsupported format character", DxCategory::Correctness),
        ("F521", ".format(...) invalid format string", DxCategory::Correctness),
        ("F522", ".format(...) unused named arguments", DxCategory::Correctness),
        ("F523", ".format(...) unused positional arguments", DxCategory::Correctness),
        ("F524", ".format(...) missing argument", DxCategory::Correctness),
        (
            "F525",
            ".format(...) mixing automatic and manual numbering",
            DxCategory::Correctness,
        ),
        ("F541", "f-string without any placeholders", DxCategory::Style),
        ("F601", "Multi-value repeated key in dictionary", DxCategory::Correctness),
        ("F602", "Multi-value repeated key in dictionary", DxCategory::Correctness),
        (
            "F621",
            "Too many expressions in star-unpacking assignment",
            DxCategory::Correctness,
        ),
        ("F622", "Two starred expressions in assignment", DxCategory::Correctness),
        ("F631", "Assert test is a tuple", DxCategory::Suspicious),
        ("F632", "Use == to compare constant literals", DxCategory::Suspicious),
        ("F633", "Use of >> is invalid with print function", DxCategory::Correctness),
        ("F634", "If test is a tuple", DxCategory::Suspicious),
        ("F701", "Break outside loop", DxCategory::Correctness),
        ("F702", "Continue outside loop", DxCategory::Correctness),
        ("F704", "Yield outside function", DxCategory::Correctness),
        ("F706", "Return outside function", DxCategory::Correctness),
        ("F707", "Except block not the last exception handler", DxCategory::Correctness),
        ("F811", "Redefinition of unused name", DxCategory::Correctness),
        ("F821", "Undefined name", DxCategory::Correctness),
        ("F822", "Undefined name in __all__", DxCategory::Correctness),
        ("F823", "Local variable referenced before assignment", DxCategory::Correctness),
        ("F841", "Local variable is assigned to but never used", DxCategory::Correctness),
        ("F842", "Local variable is annotated but never used", DxCategory::Correctness),
        ("F901", "Raise with no exception", DxCategory::Correctness),
    ];

    for (name, description, category) in &ruff_rules {
        db.add_rule(
            DxRule::new(
                rule_id,
                Language::Python,
                *name,
                *description,
                *category,
                RuleSource::Ruff,
            )
            .recommended(true),
        );
        rule_id += 1;
    }

    println!("  ✓ {} ruff rules", ruff_rules.len());
    rule_id
}

/// Extract Mago rules (PHP)
fn extract_mago_rules(db: &mut DxRuleDatabase, mut rule_id: u16) -> u16 {
    println!("📦 Extracting Mago rules (PHP)...");

    let mago_rules = vec![
        ("no-unused-variable", "Disallow unused variables", DxCategory::Correctness),
        ("no-undefined-variable", "Disallow undefined variables", DxCategory::Correctness),
        ("no-unused-import", "Disallow unused imports", DxCategory::Imports),
        (
            "no-duplicate-property",
            "Disallow duplicate class properties",
            DxCategory::Correctness,
        ),
        ("no-empty-block", "Disallow empty blocks", DxCategory::Style),
        ("prefer-null-coalesce", "Prefer null coalesce operator", DxCategory::Style),
        (
            "no-deprecated-function",
            "Disallow deprecated functions",
            DxCategory::Deprecated,
        ),
    ];

    for (name, description, category) in &mago_rules {
        db.add_rule(
            DxRule::new(rule_id, Language::Php, *name, *description, *category, RuleSource::Mago)
                .recommended(true),
        );
        rule_id += 1;
    }

    println!("  ✓ {} mago rules", mago_rules.len());
    rule_id
}

/// Extract Go rules (gofmt.rs + gold)
fn extract_go_rules(db: &mut DxRuleDatabase, mut rule_id: u16) -> u16 {
    println!("📦 Extracting Go rules...");

    // gofmt.rs formatter rules
    let gofmt_rules = vec![
        ("fmt", "Format Go code using gofmt", DxCategory::Format),
        ("simplify", "Simplify code using gofmt -s", DxCategory::Style),
    ];

    for (name, description, category) in &gofmt_rules {
        db.add_rule(
            DxRule::new(rule_id, Language::Go, *name, *description, *category, RuleSource::GofmtRs)
                .formatter()
                .fixable(true)
                .recommended(true),
        );
        rule_id += 1;
    }

    // gold linter rules
    let gold_rules = vec![
        ("errcheck", "Check for unchecked errors", DxCategory::Correctness),
        ("ineffassign", "Detect ineffectual assignments", DxCategory::Correctness),
        ("unused", "Check for unused code", DxCategory::Correctness),
        ("deadcode", "Find unused code", DxCategory::Correctness),
        (
            "varcheck",
            "Find unused global variables and constants",
            DxCategory::Correctness,
        ),
    ];

    for (name, description, category) in &gold_rules {
        db.add_rule(
            DxRule::new(rule_id, Language::Go, *name, *description, *category, RuleSource::Gold)
                .recommended(true),
        );
        rule_id += 1;
    }

    println!("  ✓ {} go rules (gofmt + gold)", gofmt_rules.len() + gold_rules.len());
    rule_id
}

/// Extract Rust rules (rustfmt + clippy)
fn extract_rust_rules(db: &mut DxRuleDatabase, mut rule_id: u16) -> u16 {
    println!("📦 Extracting Rust rules...");

    // rustfmt formatter rule
    db.add_rule(
        DxRule::new(
            rule_id,
            Language::Rust,
            "fmt",
            "Format Rust code using rustfmt",
            DxCategory::Format,
            RuleSource::Rustfmt,
        )
        .formatter()
        .fixable(true)
        .recommended(true),
    );
    rule_id += 1;

    // clippy linter rules (most common ones)
    let clippy_rules = vec![
        ("clippy::unwrap_used", "Disallow unwrap()", DxCategory::Correctness),
        ("clippy::expect_used", "Disallow expect()", DxCategory::Correctness),
        ("clippy::panic", "Disallow panic!()", DxCategory::Correctness),
        ("clippy::todo", "Disallow todo!()", DxCategory::Correctness),
        ("clippy::unimplemented", "Disallow unimplemented!()", DxCategory::Correctness),
        (
            "clippy::missing_errors_doc",
            "Missing errors documentation",
            DxCategory::Documentation,
        ),
        (
            "clippy::missing_panics_doc",
            "Missing panics documentation",
            DxCategory::Documentation,
        ),
        (
            "clippy::cognitive_complexity",
            "Function is too complex",
            DxCategory::Complexity,
        ),
        ("clippy::too_many_arguments", "Too many arguments", DxCategory::Complexity),
        ("clippy::type_complexity", "Type is too complex", DxCategory::Complexity),
        ("clippy::needless_return", "Needless return statement", DxCategory::Style),
        ("clippy::redundant_closure", "Redundant closure", DxCategory::Style),
        ("clippy::clone_on_copy", "Cloning a copy type", DxCategory::Performance),
        ("clippy::unnecessary_unwrap", "Unnecessary unwrap", DxCategory::Correctness),
        (
            "clippy::match_wildcard_for_single_variants",
            "Match wildcard for single variants",
            DxCategory::Style,
        ),
    ];

    for (name, description, category) in &clippy_rules {
        db.add_rule(
            DxRule::new(
                rule_id,
                Language::Rust,
                *name,
                *description,
                *category,
                RuleSource::Clippy,
            )
            .recommended(true),
        );
        rule_id += 1;
    }

    println!("  ✓ {} rust rules (rustfmt + clippy)", 1 + clippy_rules.len());
    rule_id
}

/// Extract TOML rules (taplo)
fn extract_toml_rules(db: &mut DxRuleDatabase, mut rule_id: u16) -> u16 {
    println!("📦 Extracting TOML rules (taplo)...");

    let taplo_rules = vec![
        ("fmt", "Format TOML files", DxCategory::Format),
        ("no-duplicate-keys", "Disallow duplicate keys", DxCategory::Correctness),
        ("no-invalid-datetime", "Disallow invalid datetime", DxCategory::Correctness),
        ("no-trailing-comma", "Disallow trailing commas", DxCategory::Style),
    ];

    for (name, description, category) in &taplo_rules {
        let mut rule =
            DxRule::new(rule_id, Language::Toml, *name, *description, *category, RuleSource::Taplo)
                .recommended(true);

        if *name == "fmt" {
            rule = rule.formatter().fixable(true);
        }

        db.add_rule(rule);
        rule_id += 1;
    }

    println!("  ✓ {} taplo rules", taplo_rules.len());
    rule_id
}

/// Extract Markdown rules (rumdl)
fn extract_markdown_rules(db: &mut DxRuleDatabase, mut rule_id: u16) -> u16 {
    println!("📦 Extracting Markdown rules (rumdl)...");

    // Common markdown linting rules
    let rumdl_rules = vec![
        (
            "MD001",
            "Header levels should only increment by one level at a time",
            DxCategory::Style,
        ),
        ("MD003", "Header style", DxCategory::Style),
        ("MD004", "Unordered list style", DxCategory::Style),
        ("MD005", "Inconsistent indentation for list items", DxCategory::Style),
        ("MD007", "Unordered list indentation", DxCategory::Style),
        ("MD009", "Trailing spaces", DxCategory::Style),
        ("MD010", "Hard tabs", DxCategory::Style),
        ("MD011", "Reversed link syntax", DxCategory::Correctness),
        ("MD012", "Multiple consecutive blank lines", DxCategory::Style),
        ("MD013", "Line length", DxCategory::Style),
        ("MD014", "Dollar signs used before commands", DxCategory::Style),
        ("MD018", "No space after hash on atx style header", DxCategory::Style),
        ("MD019", "Multiple spaces after hash on atx style header", DxCategory::Style),
        ("MD022", "Headers should be surrounded by blank lines", DxCategory::Style),
        ("MD023", "Headers must start at the beginning of the line", DxCategory::Style),
        ("MD024", "Multiple headers with the same content", DxCategory::Suspicious),
        ("MD025", "Multiple top level headers in the same document", DxCategory::Style),
        ("MD026", "Trailing punctuation in header", DxCategory::Style),
        ("MD027", "Multiple spaces after blockquote symbol", DxCategory::Style),
        ("MD028", "Blank line inside blockquote", DxCategory::Style),
        ("MD029", "Ordered list item prefix", DxCategory::Style),
        ("MD030", "Spaces after list markers", DxCategory::Style),
        (
            "MD031",
            "Fenced code blocks should be surrounded by blank lines",
            DxCategory::Style,
        ),
        ("MD032", "Lists should be surrounded by blank lines", DxCategory::Style),
        ("MD033", "Inline HTML", DxCategory::Style),
        ("MD034", "Bare URL used", DxCategory::Style),
        ("MD037", "Spaces inside emphasis markers", DxCategory::Style),
        ("MD038", "Spaces inside code span elements", DxCategory::Style),
        ("MD039", "Spaces inside link text", DxCategory::Style),
        (
            "MD040",
            "Fenced code blocks should have a language specified",
            DxCategory::Style,
        ),
        ("MD041", "First line in file should be a top level header", DxCategory::Style),
        ("MD042", "No empty links", DxCategory::Correctness),
        ("MD043", "Required header structure", DxCategory::Style),
        (
            "MD044",
            "Proper names should have the correct capitalization",
            DxCategory::Style,
        ),
        ("MD045", "Images should have alternate text", DxCategory::Accessibility),
        ("MD046", "Code block style", DxCategory::Style),
        ("MD047", "Files should end with a single newline character", DxCategory::Style),
        ("MD048", "Code fence style", DxCategory::Style),
    ];

    for (name, description, category) in &rumdl_rules {
        let mut rule = DxRule::new(
            rule_id,
            Language::Markdown,
            *name,
            *description,
            *category,
            RuleSource::Rumdl,
        );

        // Some rules are fixable
        if matches!(*name, "MD009" | "MD010" | "MD012" | "MD022" | "MD031" | "MD032" | "MD047") {
            rule = rule.fixable(true);
        }

        db.add_rule(rule);
        rule_id += 1;
    }

    println!("  ✓ {} rumdl rules", rumdl_rules.len());
    rule_id
}

/// Extract C/C++ rules (cpp-linter-rs)
fn extract_cpp_rules(db: &mut DxRuleDatabase, mut rule_id: u16) -> u16 {
    println!("📦 Extracting C/C++ rules (cpp-linter-rs)...");

    // Common clang-tidy checks
    let cpp_rules = vec![
        ("bugprone-assert-side-effect", "Assert has side effects", DxCategory::Suspicious),
        ("bugprone-dangling-handle", "Dangling handle detected", DxCategory::Correctness),
        ("bugprone-infinite-loop", "Infinite loop detected", DxCategory::Correctness),
        ("bugprone-unused-return-value", "Unused return value", DxCategory::Correctness),
        (
            "cert-err58-cpp",
            "Do not throw in constructor of non-local variable",
            DxCategory::Correctness,
        ),
        ("cppcoreguidelines-avoid-goto", "Avoid goto", DxCategory::Style),
        ("cppcoreguidelines-no-malloc", "Avoid malloc/free", DxCategory::Style),
        ("modernize-use-nullptr", "Use nullptr", DxCategory::Style),
        ("modernize-use-override", "Use override keyword", DxCategory::Style),
        (
            "performance-unnecessary-copy-initialization",
            "Unnecessary copy initialization",
            DxCategory::Performance,
        ),
        (
            "readability-const-return-type",
            "Const return type is unnecessary",
            DxCategory::Style,
        ),
        (
            "readability-identifier-naming",
            "Identifier naming convention",
            DxCategory::Style,
        ),
        (
            "readability-implicit-bool-conversion",
            "Implicit bool conversion",
            DxCategory::Style,
        ),
    ];

    for (name, description, category) in &cpp_rules {
        let mut rule = DxRule::new(
            rule_id,
            Language::Cpp,
            *name,
            *description,
            *category,
            RuleSource::CppLinter,
        );

        // Some modernize rules are fixable
        if name.starts_with("modernize-") {
            rule = rule.fixable(true);
        }

        db.add_rule(rule);
        rule_id += 1;
    }

    // Formatter rule (clang-format)
    db.add_rule(
        DxRule::new(
            rule_id,
            Language::Cpp,
            "clang-format",
            "Format C/C++ code using clang-format",
            DxCategory::Format,
            RuleSource::CppLinter,
        )
        .formatter()
        .fixable(true)
        .recommended(true),
    );
    rule_id += 1;

    println!("  ✓ {} cpp-linter rules", cpp_rules.len() + 1);
    rule_id
}

/// Extract Kotlin rules (ktlint)
fn extract_kotlin_rules(db: &mut DxRuleDatabase, mut rule_id: u16) -> u16 {
    println!("📦 Extracting Kotlin rules (ktlint)...");

    let ktlint_rules = vec![
        ("no-wildcard-imports", "Disallow wildcard imports", DxCategory::Imports),
        ("no-unused-imports", "Disallow unused imports", DxCategory::Imports),
        (
            "no-consecutive-blank-lines",
            "Disallow consecutive blank lines",
            DxCategory::Style,
        ),
        ("no-trailing-spaces", "Disallow trailing spaces", DxCategory::Style),
        ("indent", "Enforce consistent indentation", DxCategory::Style),
        ("final-newline", "Require newline at end of file", DxCategory::Style),
        ("no-unit-return", "Disallow explicit Unit return", DxCategory::Style),
        ("chain-wrapping", "Enforce chain wrapping", DxCategory::Style),
        ("comment-spacing", "Enforce spacing in comments", DxCategory::Style),
        ("filename", "Enforce filename conventions", DxCategory::Style),
        ("import-ordering", "Enforce import ordering", DxCategory::Style),
        ("max-line-length", "Enforce maximum line length", DxCategory::Style),
        ("modifier-order", "Enforce modifier ordering", DxCategory::Style),
        ("no-empty-class-body", "Disallow empty class bodies", DxCategory::Style),
        ("no-semicolons", "Disallow unnecessary semicolons", DxCategory::Style),
    ];

    for (name, description, category) in &ktlint_rules {
        db.add_rule(
            DxRule::new(
                rule_id,
                Language::Kotlin,
                *name,
                *description,
                *category,
                RuleSource::Ktlint,
            )
            .fixable(true)
            .recommended(true),
        );
        rule_id += 1;
    }

    println!("  ✓ {} ktlint rules", ktlint_rules.len());
    rule_id
}
/// Extract JSON rules from Biome
fn extract_json_rules(db: &mut DxRuleDatabase, mut rule_id: u16) -> u16 {
    println!("📦 Extracting JSON rules (biome)...");

    let json_rules = vec![
        DxRule::new(
            rule_id,
            Language::Json,
            "noComments",
            "Disallow comments in JSON files",
            DxCategory::Correctness,
            RuleSource::Biome,
        )
        .severity(DxSeverity::Error)
        .docs("https://biomejs.dev/linter/rules/no-comments")
        .recommended(true),
        DxRule::new(
            rule_id + 1,
            Language::Json,
            "noDuplicateKeys",
            "Disallow duplicate keys in JSON objects",
            DxCategory::Correctness,
            RuleSource::Biome,
        )
        .severity(DxSeverity::Error)
        .docs("https://biomejs.dev/linter/rules/no-duplicate-keys")
        .recommended(true),
        DxRule::new(
            rule_id + 2,
            Language::Json,
            "noTrailingCommas",
            "Disallow trailing commas in JSON",
            DxCategory::Correctness,
            RuleSource::Biome,
        )
        .severity(DxSeverity::Error)
        .docs("https://biomejs.dev/linter/rules/no-trailing-commas")
        .recommended(true),
    ];

    rule_id += json_rules.len() as u16;
    for rule in &json_rules {
        db.add_rule(rule.clone());
    }

    println!("  ✅ {} JSON rules", json_rules.len());
    rule_id
}

/// Extract CSS rules from Biome
fn extract_css_rules(db: &mut DxRuleDatabase, mut rule_id: u16) -> u16 {
    println!("📦 Extracting CSS rules (biome)...");

    let css_rules = vec![
        DxRule::new(
            rule_id,
            Language::Css,
            "noDuplicateSelectors",
            "Disallow duplicate selectors",
            DxCategory::Suspicious,
            RuleSource::Biome,
        )
        .recommended(true),
        DxRule::new(
            rule_id + 1,
            Language::Css,
            "noInvalidPositionAtImportRule",
            "Disallow invalid position for @import rules",
            DxCategory::Correctness,
            RuleSource::Biome,
        )
        .severity(DxSeverity::Error)
        .recommended(true),
        DxRule::new(
            rule_id + 2,
            Language::Css,
            "noUnknownUnit",
            "Disallow unknown CSS units",
            DxCategory::Correctness,
            RuleSource::Biome,
        )
        .severity(DxSeverity::Error)
        .recommended(true),
        DxRule::new(
            rule_id + 3,
            Language::Css,
            "noShorthandPropertyOverrides",
            "Disallow shorthand properties that override related longhand properties",
            DxCategory::Suspicious,
            RuleSource::Biome,
        )
        .recommended(true),
    ];

    rule_id += css_rules.len() as u16;
    for rule in &css_rules {
        db.add_rule(rule.clone());
    }

    println!("  ✅ {} CSS rules", css_rules.len());
    rule_id
}

/// Extract HTML rules from Biome
fn extract_html_rules(db: &mut DxRuleDatabase, mut rule_id: u16) -> u16 {
    println!("📦 Extracting HTML rules (biome)...");

    let html_rules = vec![
        DxRule::new(
            rule_id,
            Language::Html,
            "noBlankTarget",
            "Disallow target='_blank' without rel='noopener noreferrer'",
            DxCategory::Security,
            RuleSource::Biome,
        )
        .docs("https://biomejs.dev/linter/rules/no-blank-target")
        .recommended(true),
        DxRule::new(
            rule_id + 1,
            Language::Html,
            "useValidAnchor",
            "Enforce valid anchor elements",
            DxCategory::Accessibility,
            RuleSource::Biome,
        )
        .recommended(true),
        DxRule::new(
            rule_id + 2,
            Language::Html,
            "useButtonType",
            "Enforce explicit type attribute for button elements",
            DxCategory::Suspicious,
            RuleSource::Biome,
        )
        .recommended(true),
    ];

    rule_id += html_rules.len() as u16;
    for rule in &html_rules {
        db.add_rule(rule.clone());
    }

    println!("  ✅ {} HTML rules", html_rules.len());
    rule_id
}

/// Extract YAML rules
fn extract_yaml_rules(db: &mut DxRuleDatabase, mut rule_id: u16) -> u16 {
    println!("📦 Extracting YAML rules...");

    let yaml_rules = vec![
        DxRule::new(
            rule_id,
            Language::Yaml,
            "noDuplicateKeys",
            "Disallow duplicate keys in YAML mappings",
            DxCategory::Correctness,
            RuleSource::DxCheck,
        )
        .severity(DxSeverity::Error)
        .recommended(true),
        DxRule::new(
            rule_id + 1,
            Language::Yaml,
            "noTabs",
            "Disallow tabs in YAML files",
            DxCategory::Style,
            RuleSource::DxCheck,
        )
        .recommended(true),
        DxRule::new(
            rule_id + 2,
            Language::Yaml,
            "noTrailingSpaces",
            "Disallow trailing spaces",
            DxCategory::Style,
            RuleSource::DxCheck,
        )
        .fixable(true)
        .recommended(true),
    ];

    rule_id += yaml_rules.len() as u16;
    for rule in &yaml_rules {
        db.add_rule(rule.clone());
    }

    println!("  ✅ {} YAML rules", yaml_rules.len());
    rule_id
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_all_rules() {
        let db = extract_all_rules();
        assert!(db.rule_count > 100, "Should have over 100 rules");
        assert!(db.validate().is_ok(), "Database should be valid");
    }

    #[test]
    fn test_rules_have_unique_ids() {
        let db = extract_all_rules();
        let mut ids = std::collections::HashSet::new();
        for rule in &db.rules {
            assert!(ids.insert(rule.rule_id), "Duplicate rule ID: {}", rule.rule_id);
        }
    }

    #[test]
    fn test_language_coverage() {
        let db = extract_all_rules();

        // Ensure we have rules for all major languages
        assert!(!db.get_by_language(Language::JavaScript).is_empty());
        assert!(!db.get_by_language(Language::Python).is_empty());
        assert!(!db.get_by_language(Language::Go).is_empty());
        assert!(!db.get_by_language(Language::Rust).is_empty());
        assert!(!db.get_by_language(Language::Markdown).is_empty());
    }
}
