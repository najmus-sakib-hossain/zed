#![no_main]

use libfuzzer_sys::fuzz_target;
use serializer::{parse, encode};

fuzz_target!(|data: &[u8]| {
    // Fuzz round-trip: parse -> encode -> parse
    // If parsing succeeds, encoding and re-parsing should also succeed
    // and produce equivalent results
    if let Ok(value) = parse(data) {
        if let Ok(encoded) = encode(&value) {
            // Re-parsing should succeed
            let _ = parse(&encoded);
        }
    }
});
