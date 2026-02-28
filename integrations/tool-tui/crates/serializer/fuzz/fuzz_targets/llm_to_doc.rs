#![no_main]

use libfuzzer_sys::fuzz_target;
use serializer::llm_to_document;

fuzz_target!(|data: &[u8]| {
    // Fuzz the llm_to_document() conversion function
    // Convert bytes to string and attempt to parse as LLM format
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = llm_to_document(s);
    }
});
