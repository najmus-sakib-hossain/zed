//! Fuzz test for configuration parsing
//!
//! **Validates: Requirement 10.7 - Fuzz tests for parser edge cases**

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    use dx_check::config::CheckerConfig;

    // Try to parse as TOML config
    if let Ok(s) = std::str::from_utf8(data) {
        // The config parser should never panic on any input
        let _ = toml::from_str::<CheckerConfig>(s);
    }
});
