use serializer::llm::convert::{llm_to_machine, machine_to_document};
use serializer::llm::types::DxLlmValue;
use std::fs;

#[test]
fn test_theme_machine_format() {
    // Read the theme.sr file
    let theme_sr = fs::read_to_string("theme.sr").expect("Failed to read theme.sr");

    // Convert to machine format
    let machine_bytes = llm_to_machine(&theme_sr).expect("Failed to convert to machine format");

    println!(
        "Theme size: {} bytes (LLM) -> {} bytes (machine)",
        theme_sr.len(),
        machine_bytes.len()
    );

    // Verify we can read it back
    let doc = machine_to_document(&machine_bytes).expect("Failed to parse machine format");

    // Verify metadata
    assert_eq!(doc.context.get("name"), Some(&DxLlmValue::Str("dx".to_string())));
    assert_eq!(doc.context.get("version"), Some(&DxLlmValue::Str("1.0.0".to_string())));
    assert_eq!(doc.context.get("description"), Some(&DxLlmValue::Str("Dx_theme".to_string())));

    // Verify dark theme section exists
    assert!(doc.section_names.iter().any(|(_, name)| name == "dark"));

    // Verify light theme section exists
    assert!(doc.section_names.iter().any(|(_, name)| name == "light"));

    // Verify dark_modes section exists
    assert!(doc.section_names.iter().any(|(_, name)| name == "dark_modes"));

    // Verify light_modes section exists
    assert!(doc.section_names.iter().any(|(_, name)| name == "light_modes"));

    println!("✓ Theme machine format verified successfully");
}

#[test]
fn test_theme_color_parsing() {
    let theme_sr = fs::read_to_string("theme.sr").expect("Failed to read theme.sr");

    let machine_bytes = llm_to_machine(&theme_sr).expect("Failed to convert to machine format");

    let doc = machine_to_document(&machine_bytes).expect("Failed to parse machine format");

    // Find dark theme section
    let dark_section = doc
        .section_names
        .iter()
        .find(|(_, name)| name.as_str() == "dark")
        .and_then(|(id, _)| doc.sections.get(id))
        .expect("Dark section not found");

    // Verify it has the expected number of fields (19)
    if let Some(row) = dark_section.rows.first() {
        if let Some(DxLlmValue::Obj(obj)) = row.first() {
            assert_eq!(obj.len(), 19, "Dark theme should have 19 color fields");

            // Verify some key colors exist
            assert!(obj.contains_key("background"));
            assert!(obj.contains_key("foreground"));
            assert!(obj.contains_key("primary"));
            assert!(obj.contains_key("accent"));
            assert!(obj.contains_key("border"));

            println!("✓ Dark theme has all {} color fields", obj.len());
        } else {
            panic!("Dark section row should contain an object");
        }
    } else {
        panic!("Dark section should have rows");
    }

    // Find dark_modes section
    let modes_section = doc
        .section_names
        .iter()
        .find(|(_, name)| name.as_str() == "dark_modes")
        .and_then(|(id, _)| doc.sections.get(id))
        .expect("Dark_modes section not found");

    // Verify it has 3 mode colors
    if let Some(row) = modes_section.rows.first() {
        if let Some(DxLlmValue::Obj(obj)) = row.first() {
            assert_eq!(obj.len(), 3, "Dark_modes should have 3 mode colors");
            assert!(obj.contains_key("agent"));
            assert!(obj.contains_key("plan"));
            assert!(obj.contains_key("ask"));

            println!("✓ Mode colors verified");
        } else {
            panic!("Modes section row should contain an object");
        }
    } else {
        panic!("Modes section should have rows");
    }
}
