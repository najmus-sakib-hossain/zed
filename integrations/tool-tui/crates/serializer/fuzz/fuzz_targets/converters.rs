#![no_main]

use libfuzzer_sys::fuzz_target;
use serializer::{toon_to_dx, dx_to_toon};

fuzz_target!(|data: &[u8]| {
    // Fuzz format converters
    if let Ok(s) = std::str::from_utf8(data) {
        // Test TOON to DX conversion
        if let Ok(dx) = toon_to_dx(s) {
            // If conversion succeeds, reverse conversion should not panic
            let _ = dx_to_toon(&dx);
        }
    }
});
