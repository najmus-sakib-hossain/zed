#![no_main]

use libfuzzer_sys::fuzz_target;
use serializer::{encode_base62, decode_base62};

fuzz_target!(|data: &[u8]| {
    // Fuzz base62 encoding/decoding
    
    // Test decoding arbitrary strings
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = decode_base62(s);
    }
    
    // Test encoding arbitrary u64 values
    if data.len() >= 8 {
        let value = u64::from_le_bytes([
            data[0], data[1], data[2], data[3],
            data[4], data[5], data[6], data[7],
        ]);
        let encoded = encode_base62(value);
        // Round-trip should work
        if let Ok(decoded) = decode_base62(&encoded) {
            assert_eq!(decoded, value);
        }
    }
});
