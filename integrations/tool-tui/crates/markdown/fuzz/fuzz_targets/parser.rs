#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string, skip if invalid UTF-8
    if let Ok(input) = std::str::from_utf8(data) {
        // Skip if too large
        if input.len() > 1_000_000 {
            return;
        }
        
        // Try to parse DXM format - should never panic
        // Note: This uses internal API for fuzzing purposes
        // let _ = dx_markdown::parser::DxmParser::parse(input);
        
        // For now, just test that markdown parsing doesn't panic
        use pulldown_cmark::{Parser, Options};
        let options = Options::all();
        let parser = Parser::new_ext(input, options);
        
        // Consume the parser - should never panic
        for _ in parser {}
    }
});
