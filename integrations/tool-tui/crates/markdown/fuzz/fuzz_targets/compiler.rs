#![no_main]

use libfuzzer_sys::fuzz_target;
use markdown::{DxMarkdown, CompilerConfig};

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string, skip if invalid UTF-8
    if let Ok(input) = std::str::from_utf8(data) {
        // Skip if too large (fuzzer will generate huge inputs)
        if input.len() > 1_000_000 {
            return;
        }
        
        // Try to compile with default config
        let compiler = match DxMarkdown::new(CompilerConfig::default()) {
            Ok(c) => c,
            Err(_) => return,
        };
        
        // Compilation should never panic, only return errors
        let _ = compiler.compile(input);
    }
});
