//! Fuzz test for the JavaScript/TypeScript parser
//!
//! **Validates: Requirement 10.7 - Fuzz tests for parser edge cases**

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use std::path::Path;

/// Arbitrary input for parser fuzzing
#[derive(Debug, Arbitrary)]
struct ParserInput {
    /// The source code to parse
    source: String,
    /// File extension to use
    extension: FileExtension,
}

#[derive(Debug, Arbitrary)]
enum FileExtension {
    Js,
    Jsx,
    Ts,
    Tsx,
    Mjs,
    Cjs,
}

impl FileExtension {
    fn as_str(&self) -> &'static str {
        match self {
            FileExtension::Js => "js",
            FileExtension::Jsx => "jsx",
            FileExtension::Ts => "ts",
            FileExtension::Tsx => "tsx",
            FileExtension::Mjs => "mjs",
            FileExtension::Cjs => "cjs",
        }
    }
}

fuzz_target!(|input: ParserInput| {
    use dx_check::config::CheckerConfig;
    use dx_check::engine::Checker;

    let checker = Checker::new(CheckerConfig::default());
    let path = format!("test.{}", input.extension.as_str());

    // The parser should never panic on any input
    let _ = checker.check_source(Path::new(&path), &input.source);
});
