#![no_main]

use libfuzzer_sys::fuzz_target;
use serializer::{human_to_document, document_to_human};

fuzz_target!(|data: &[u8]| {
    // Fuzz human format parsing and round-trip
    if let Ok(s) = std::str::from_utf8(data) {
        if let Ok(doc) = human_to_document(s) {
            // If parsing succeeds, serialization should not panic
            let _ = document_to_human(&doc);
        }
    }
});
