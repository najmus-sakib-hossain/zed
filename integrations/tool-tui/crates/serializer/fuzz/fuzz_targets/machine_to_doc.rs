#![no_main]

use libfuzzer_sys::fuzz_target;
use serializer::{machine_to_document, MachineFormat};

fuzz_target!(|data: &[u8]| {
    // Fuzz the machine_to_document() conversion function
    // Create a MachineFormat from raw bytes and attempt to parse
    let machine = MachineFormat { data: data.to_vec() };
    let _ = machine_to_document(&machine);
});
