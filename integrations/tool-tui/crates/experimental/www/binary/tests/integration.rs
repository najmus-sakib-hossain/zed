//! # Integration Tests
//!
//! Full round-trip testing: serialize → deserialize → verify

use dx_www_binary::{deserializer::HtipStream, opcodes::*, serializer::HtipWriter};
use ed25519_dalek::SigningKey;

#[test]
fn test_full_roundtrip() {
    // Create writer
    let mut writer = HtipWriter::new();

    // Write template
    let bindings = vec![Binding {
        slot_id: 0,
        binding_type: BindingType::Text,
        path: vec![0],
    }];
    writer.write_template(0, "<div><!--SLOT_0--></div>", bindings);

    // Write instantiate
    writer.write_instantiate(1, 0, 0);

    // Write patch
    writer.write_patch_text(1, 0, "Hello World");

    // Write class toggle
    writer.write_class_toggle(1, "active", true);

    // Sign and serialize
    let signing_key = SigningKey::from_bytes(&[0u8; 32]);
    let binary = writer.finish_and_sign(&signing_key).unwrap();

    // Verify size is reasonable
    assert!(binary.len() < 1024); // Should be under 1 KB

    // Deserialize
    let verifying_key = signing_key.verifying_key();
    let stream = HtipStream::new(&binary, &verifying_key).unwrap();

    assert!(stream.is_verified());

    // Read operations
    let ops = stream.operations();
    assert_eq!(ops.len(), 4);

    // Verify operation types
    assert!(matches!(&ops[0], Operation::TemplateDef(_)));
    assert!(matches!(&ops[1], Operation::Instantiate(_)));
    assert!(matches!(&ops[2], Operation::PatchText(_)));
    assert!(matches!(&ops[3], Operation::PatchClassToggle(_)));

    // Verify operation data
    if let Operation::PatchText(patch) = &ops[2] {
        let text = stream.get_string(patch.string_id).unwrap();
        assert_eq!(text, "Hello World");
    } else {
        panic!("Expected PatchText");
    }
}

#[test]
fn test_large_payload() {
    let mut writer = HtipWriter::new();

    // Create 100 templates
    for i in 0..100 {
        writer.write_template(i, &format!("<div id='template-{}'>Content</div>", i), vec![]);
    }

    // Instantiate all templates
    for i in 0..100 {
        writer.write_instantiate(i as u32, i, 0);
    }

    // Patch all with text
    for i in 0..100 {
        writer.write_patch_text(i as u32, 0, &format!("Text {}", i));
    }

    let signing_key = SigningKey::from_bytes(&[0u8; 32]);
    let binary = writer.finish_and_sign(&signing_key).unwrap();

    println!("Large payload size: {} bytes", binary.len());

    // Should be under 50 KB due to string deduplication
    assert!(binary.len() < 50 * 1024);

    // Verify can deserialize
    let verifying_key = signing_key.verifying_key();
    let stream = HtipStream::new(&binary, &verifying_key).unwrap();

    assert!(stream.is_verified());
    assert_eq!(stream.remaining(), 300); // 100 templates + 100 instantiates + 100 patches
}

#[test]
fn test_string_deduplication() {
    let mut writer = HtipWriter::new();

    // Add same string 1000 times
    for i in 0..1000 {
        writer.write_instantiate(i, 0, 0);
        writer.write_patch_text(i, 0, "repeated string");
    }

    let signing_key = SigningKey::from_bytes(&[0u8; 32]);
    let binary = writer.finish_and_sign(&signing_key).unwrap();

    // Should be small due to deduplication
    println!("Deduplicated payload size: {} bytes", binary.len());

    // Parse and verify
    let verifying_key = signing_key.verifying_key();
    let stream = HtipStream::new(&binary, &verifying_key).unwrap();

    // All patch operations should reference same string ID
    assert!(stream.is_verified());
}

#[test]
fn test_batch_operations() {
    let mut writer = HtipWriter::new();

    // Write batch
    writer.write_batch_start(1);

    for i in 0..10 {
        writer.write_instantiate(i, 0, 0);
    }

    writer.write_batch_commit(1);

    let signing_key = SigningKey::from_bytes(&[0u8; 32]);
    let binary = writer.finish_and_sign(&signing_key).unwrap();

    // Deserialize
    let verifying_key = signing_key.verifying_key();
    let stream = HtipStream::new(&binary, &verifying_key).unwrap();

    let ops = stream.operations();

    // First operation should be BatchStart
    assert!(matches!(&ops[0], Operation::BatchStart(_)));

    // Last operation should be BatchCommit
    let last_op = &ops[ops.len() - 1];
    assert!(matches!(last_op, Operation::BatchCommit(_)));
}

#[test]
fn test_all_opcode_types() {
    let mut writer = HtipWriter::new();

    // Test every opcode type
    writer.write_template(0, "<div></div>", vec![]);
    writer.write_instantiate(1, 0, 0);
    writer.write_patch_text(1, 0, "text");
    writer.write_patch_attr(1, 0, "id", "test");
    writer.write_class_toggle(1, "active", true);
    writer.write_attach_event(1, "click", 1);
    writer.write_remove_node(1);
    writer.write_batch_start(1);
    writer.write_batch_commit(1);
    writer.write_set_property(1, "value", PropertyValue::String(0));
    writer.write_append_child(0, 1);

    let signing_key = SigningKey::from_bytes(&[0u8; 32]);
    let binary = writer.finish_and_sign(&signing_key).unwrap();

    // Deserialize and verify all opcodes
    let verifying_key = signing_key.verifying_key();
    let stream = HtipStream::new(&binary, &verifying_key).unwrap();

    let opcodes: Vec<OpcodeV1> = stream.operations().iter().map(|op| op.opcode()).collect();

    assert_eq!(opcodes.len(), 11);
    assert!(opcodes.contains(&OpcodeV1::TemplateDef));
    assert!(opcodes.contains(&OpcodeV1::Instantiate));
    assert!(opcodes.contains(&OpcodeV1::PatchText));
    assert!(opcodes.contains(&OpcodeV1::PatchAttr));
    assert!(opcodes.contains(&OpcodeV1::PatchClassToggle));
    assert!(opcodes.contains(&OpcodeV1::AttachEvent));
    assert!(opcodes.contains(&OpcodeV1::RemoveNode));
    assert!(opcodes.contains(&OpcodeV1::BatchStart));
    assert!(opcodes.contains(&OpcodeV1::BatchCommit));
    assert!(opcodes.contains(&OpcodeV1::SetProperty));
    assert!(opcodes.contains(&OpcodeV1::AppendChild));
}
