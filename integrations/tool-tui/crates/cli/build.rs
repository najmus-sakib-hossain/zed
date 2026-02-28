use serializer::llm::convert::human_to_machine;
use std::fs;
use std::path::Path;

fn main() {
    // Read theme.sr (human format)
    let theme_sr = fs::read_to_string("theme.sr").expect("Failed to read theme.sr");

    // Convert to machine format (binary with LZ4 compression)
    let machine = human_to_machine(&theme_sr).expect("Failed to convert to machine format");

    // Create output directory in workspace root .dx folder
    let out_dir = "../../.dx/serializer/crates/cli";
    fs::create_dir_all(out_dir).expect("Failed to create output directory");

    // Write machine format
    let machine_path = Path::new(out_dir).join("theme.machine");
    fs::write(&machine_path, &machine.data).expect("Failed to write theme.machine");

    println!("cargo:rerun-if-changed=theme.sr");
    eprintln!(
        "Theme compiled: {} bytes (human) -> {} bytes (machine compressed)",
        theme_sr.len(),
        machine.data.len()
    );
}
