#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string, skip if invalid UTF-8
    if let Ok(input) = std::str::from_utf8(data) {
        // Skip if too large
        if input.len() > 100_000 {
            return;
        }
        
        // Try to parse markdown tables - should never panic
        // This tests the table parsing logic specifically
        use pulldown_cmark::{Parser, Event, Tag, TagEnd, Options};
        
        let options = Options::ENABLE_TABLES;
        let parser = Parser::new_ext(input, options);
        
        let mut in_table = false;
        let mut in_table_head = false;
        
        for event in parser {
            match event {
                Event::Start(Tag::Table(_)) => in_table = true,
                Event::End(TagEnd::Table) => in_table = false,
                Event::Start(Tag::TableHead) => in_table_head = true,
                Event::End(TagEnd::TableHead) => in_table_head = false,
                Event::Text(_) if in_table => {
                    // Processing table text - should never panic
                }
                _ => {}
            }
        }
    }
});
