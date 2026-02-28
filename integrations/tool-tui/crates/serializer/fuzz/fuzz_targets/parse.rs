#![no_main]

use libfuzzer_sys::fuzz_target;
use serializer::parse;

fuzz_target!(|data: &[u8]| {
    // Fuzz the main parse() entry point
    // The parser should never panic on any input
    let _ = parse(data);
});
