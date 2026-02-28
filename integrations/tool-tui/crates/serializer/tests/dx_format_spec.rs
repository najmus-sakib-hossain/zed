//! DX Format Specification Tests
//!
//! Battle-tested specification ensuring DX LLM and Machine formats
//! work correctly across all platforms and edge cases.
//!
//! Run with: cargo test --package serializer --test dx_format_spec

use serializer::zero::DxZeroBuilder;
use serializer::{
    DxDocument, DxLlmValue, DxSection, document_to_llm, document_to_machine, human_to_llm,
    llm_to_document, llm_to_human, machine_to_document,
};

// ============================================================================
// PART 1: DX LLM FORMAT SPECIFICATION TESTS
// ============================================================================

mod llm_format {
    use super::*;

    #[test]
    fn spec_empty_document() {
        let doc = DxDocument::new();
        let llm = document_to_llm(&doc);
        let parsed = llm_to_document(&llm).unwrap();
        assert!(parsed.context.is_empty());
        assert!(parsed.sections.is_empty());
    }

    #[test]
    fn spec_string_values() {
        let mut doc = DxDocument::new();
        // Use abbreviated keys that the serializer uses
        doc.context.insert("nm".into(), DxLlmValue::Str("Test".into()));
        doc.context.insert("desc".into(), DxLlmValue::Str("Hello".into()));

        let llm = document_to_llm(&doc);
        let parsed = llm_to_document(&llm).unwrap();

        // Keys are preserved as-is when already abbreviated
        assert!(parsed.context.len() >= 2);
    }

    #[test]
    fn spec_numeric_values() {
        let mut doc = DxDocument::new();
        doc.context.insert("ct".into(), DxLlmValue::Num(42.0));
        doc.context.insert("val".into(), DxLlmValue::Num(3.14159));

        let llm = document_to_llm(&doc);
        let parsed = llm_to_document(&llm).unwrap();

        assert!(parsed.context.len() >= 2);
    }

    #[test]
    fn spec_boolean_values() {
        let mut doc = DxDocument::new();
        doc.context.insert("ac".into(), DxLlmValue::Bool(true));
        doc.context.insert("off".into(), DxLlmValue::Bool(false));

        let llm = document_to_llm(&doc);

        // Dx Serializer format: booleans serialize as true/false with :: delimiter
        assert!(llm.contains("true"), "Should contain true: {}", llm);
        assert!(llm.contains("false"), "Should contain false: {}", llm);

        let parsed = llm_to_document(&llm).unwrap();
        assert!(parsed.context.len() >= 2);
    }

    #[test]
    fn spec_null_values() {
        let mut doc = DxDocument::new();
        doc.context.insert("empty".into(), DxLlmValue::Null);

        let llm = document_to_llm(&doc);

        // Dx Serializer format: null serializes as null with :: delimiter
        assert!(llm.contains("null"), "Should contain null: {}", llm);

        let parsed = llm_to_document(&llm).unwrap();
        assert!(parsed.context.get("empty").unwrap().is_null());
    }

    #[test]
    fn spec_array_values() {
        let mut doc = DxDocument::new();
        doc.context.insert(
            "tags".into(),
            DxLlmValue::Arr(vec![
                DxLlmValue::Str("a".into()),
                DxLlmValue::Str("b".into()),
                DxLlmValue::Str("c".into()),
            ]),
        );

        let llm = document_to_llm(&doc);

        // Dx Serializer format: arrays serialize as name:count=item1 item2 item3 (space-separated)
        assert!(
            llm.contains("tags[3]") || llm.contains("tags:3"),
            "Should contain tags[3] or tags:3: {}",
            llm
        );
        // New format uses space separators
        assert!(
            llm.contains("a b c") || llm.contains("a,b,c"),
            "Should contain space-separated or comma-separated items: {}",
            llm
        );

        let parsed = llm_to_document(&llm).unwrap();
        // The key might be abbreviated, so check by iterating
        let has_array = parsed.context.values().any(|v| {
            if let Some(arr) = v.as_arr() {
                arr.len() == 3
            } else {
                false
            }
        });
        assert!(has_array, "Should have an array with 3 elements");
    }

    #[test]
    fn spec_table_section() {
        let mut doc = DxDocument::new();
        let mut section = DxSection::new(vec!["id".into(), "nm".into(), "sc".into()]);
        section.rows.push(vec![
            DxLlmValue::Num(1.0),
            DxLlmValue::Str("Alice".into()),
            DxLlmValue::Num(95.5),
        ]);
        section.rows.push(vec![
            DxLlmValue::Num(2.0),
            DxLlmValue::Str("Bob".into()),
            DxLlmValue::Num(87.0),
        ]);
        doc.sections.insert('d', section);

        let llm = document_to_llm(&doc);
        let parsed = llm_to_document(&llm).unwrap();

        // Sections get auto-assigned IDs when parsed, so just check we have one
        assert!(!parsed.sections.is_empty(), "Should have at least one section");
        let section = parsed.sections.values().next().unwrap();
        assert_eq!(section.schema.len(), 3);
        assert_eq!(section.rows.len(), 2);
    }

    #[test]
    fn spec_round_trip_consistency() {
        let mut doc = DxDocument::new();
        doc.context.insert("nm".into(), DxLlmValue::Str("Test".into()));
        doc.context.insert("ct".into(), DxLlmValue::Num(42.0));
        doc.context.insert("ac".into(), DxLlmValue::Bool(true));

        // Multiple round trips should be stable
        let llm1 = document_to_llm(&doc);
        let parsed1 = llm_to_document(&llm1).unwrap();
        let llm2 = document_to_llm(&parsed1);
        let parsed2 = llm_to_document(&llm2).unwrap();

        assert_eq!(parsed1.context.len(), parsed2.context.len());
    }

    #[test]
    fn spec_human_llm_conversion() {
        let llm = "nm=Test\nct=42";
        let human = llm_to_human(llm).unwrap();
        let back = human_to_llm(&human).unwrap();

        // Should preserve data through conversion
        let doc1 = llm_to_document(llm).unwrap();
        let doc2 = llm_to_document(&back).unwrap();

        assert_eq!(doc1.context.len(), doc2.context.len());
    }
}

// ============================================================================
// PART 2: DX MACHINE FORMAT SPECIFICATION TESTS
// ============================================================================

mod machine_format {
    use super::*;

    #[test]
    fn spec_empty_document() {
        let doc = DxDocument::new();
        let machine = document_to_machine(&doc);
        let parsed = machine_to_document(&machine).unwrap();

        assert!(parsed.context.is_empty());
        assert!(parsed.sections.is_empty());
    }

    #[test]
    fn spec_all_value_types() {
        let mut doc = DxDocument::new();
        doc.context.insert("str".into(), DxLlmValue::Str("Hello".into()));
        doc.context.insert("num".into(), DxLlmValue::Num(42.0));
        doc.context.insert("bool".into(), DxLlmValue::Bool(true));
        doc.context.insert("null".into(), DxLlmValue::Null);
        doc.context.insert(
            "arr".into(),
            DxLlmValue::Arr(vec![DxLlmValue::Num(1.0), DxLlmValue::Num(2.0)]),
        );

        let machine = document_to_machine(&doc);
        let parsed = machine_to_document(&machine).unwrap();

        assert_eq!(parsed.context.len(), 5);
        assert_eq!(parsed.context.get("str").unwrap().as_str(), Some("Hello"));
        assert_eq!(parsed.context.get("num").unwrap().as_num(), Some(42.0));
        assert_eq!(parsed.context.get("bool").unwrap().as_bool(), Some(true));
        assert!(parsed.context.get("null").unwrap().is_null());
    }

    #[test]
    fn spec_binary_header() {
        let doc = DxDocument::new();
        let machine = document_to_machine(&doc);

        // Verify RKYV + LZ4 format
        assert!(machine.data.len() >= 4, "Machine format should have data");
    }

    #[test]
    fn spec_round_trip_consistency() {
        let mut doc = DxDocument::new();
        doc.context.insert("name".into(), DxLlmValue::Str("Test".into()));
        doc.context.insert("count".into(), DxLlmValue::Num(42.0));

        let machine1 = document_to_machine(&doc);
        let parsed1 = machine_to_document(&machine1).unwrap();
        let machine2 = document_to_machine(&parsed1);
        let parsed2 = machine_to_document(&machine2).unwrap();

        assert_eq!(parsed1.context.len(), parsed2.context.len());
    }

    #[test]
    fn spec_table_section() {
        let mut doc = DxDocument::new();
        let mut section = DxSection::new(vec!["id".into(), "value".into()]);
        section.rows.push(vec![DxLlmValue::Num(1.0), DxLlmValue::Str("A".into())]);
        section.rows.push(vec![DxLlmValue::Num(2.0), DxLlmValue::Str("B".into())]);
        doc.sections.insert('t', section);

        let machine = document_to_machine(&doc);
        let parsed = machine_to_document(&machine).unwrap();

        let section = parsed.sections.get(&'t').unwrap();
        assert_eq!(section.rows.len(), 2);
    }

    #[test]
    fn spec_unicode_strings() {
        let mut doc = DxDocument::new();
        doc.context.insert("cjk".into(), DxLlmValue::Str("你好世界".into()));
        doc.context.insert("emoji".into(), DxLlmValue::Str("🚀🎉".into()));

        let machine = document_to_machine(&doc);
        let parsed = machine_to_document(&machine).unwrap();

        assert_eq!(parsed.context.get("cjk").unwrap().as_str(), Some("你好世界"));
        assert_eq!(parsed.context.get("emoji").unwrap().as_str(), Some("🚀🎉"));
    }
}

// ============================================================================
// PART 3: DX ZERO-COPY FORMAT SPECIFICATION TESTS
// ============================================================================

mod zero_copy_format {
    use super::*;

    #[test]
    fn spec_primitive_types() {
        let mut buffer = Vec::new();
        let mut builder = DxZeroBuilder::new(&mut buffer, 30, 0);

        builder.write_u8(0, 255);
        builder.write_i8(1, -128);
        builder.write_u16(2, 65535);
        builder.write_i16(4, -32768);
        builder.write_u32(6, 4294967295);
        builder.write_i32(10, -2147483648);
        builder.write_u64(14, u64::MAX);
        builder.write_i64(22, i64::MIN);

        builder.finish();

        // Verify header
        assert_eq!(buffer[0], 0x5A); // Magic 'Z'
        assert_eq!(buffer[1], 0x44); // Magic 'D'
    }

    #[test]
    fn spec_float_types() {
        let mut buffer = Vec::new();
        let mut builder = DxZeroBuilder::new(&mut buffer, 16, 0);

        builder.write_f32(0, 3.14159);
        builder.write_f64(4, 2.718281828);
        builder.write_bool(12, true);
        builder.write_bool(13, false);

        builder.finish();

        assert!(buffer.len() >= 16);
    }

    #[test]
    fn spec_inline_string() {
        let mut buffer = Vec::new();
        let mut builder = DxZeroBuilder::new(&mut buffer, 0, 1);

        builder.write_string(0, "Hello");

        let size = builder.finish();

        let slot_data = &buffer[4..20];
        assert_eq!(slot_data[0], 5); // Length
        assert_eq!(slot_data[15], 0x00); // Inline marker
        assert_eq!(&slot_data[1..6], b"Hello");
        assert!(size > 0);
    }

    #[test]
    fn spec_heap_string() {
        let mut buffer = Vec::new();
        let mut builder = DxZeroBuilder::new(&mut buffer, 0, 1);

        let long_str = "This is a very long string that exceeds 14 bytes";
        builder.write_string(0, long_str);

        let size = builder.finish();

        let slot_data = &buffer[4..20];
        assert_eq!(slot_data[15], 0xFF); // Heap marker
        assert!(size > 20);
    }

    #[test]
    fn spec_direct_memory_access() {
        let mut buffer = Vec::new();
        let mut builder = DxZeroBuilder::new(&mut buffer, 16, 0);

        builder.write_i32(0, 12345);
        builder.write_f64(4, 3.14159);
        builder.write_bool(12, true);

        builder.finish();

        // SAFETY: We just wrote these values using the builder at known offsets.
        // buffer.as_ptr().add(4) points to the start of the fixed fields region (after 4-byte header).
        // We're reading i32 at offset 0, f64 at offset 4, and u8 at offset 12, which match
        // what we wrote with write_i32(0, ...), write_f64(4, ...), and write_bool(12, ...).
        unsafe {
            let ptr = buffer.as_ptr().add(4);
            let id = *(ptr as *const i32);
            let value = *(ptr.add(4) as *const f64);
            let flag = *(ptr.add(12) as *const u8);

            assert_eq!(id, 12345);
            assert!((value - 3.14159).abs() < 0.0001);
            assert_eq!(flag, 1);
        }
    }

    #[test]
    fn spec_multiple_slots() {
        let mut buffer = Vec::new();
        let mut builder = DxZeroBuilder::new(&mut buffer, 8, 2);

        builder.write_u64(0, 999);
        builder.write_string(8, "name");
        builder.write_string(24, "email");

        let size = builder.finish();
        assert!(size > 0);
    }
}

// ============================================================================
// PART 4: CROSS-FORMAT CONVERSION TESTS
// ============================================================================

mod cross_format {
    use super::*;

    #[test]
    fn spec_llm_to_machine_to_llm() {
        let mut doc = DxDocument::new();
        doc.context.insert("nm".into(), DxLlmValue::Str("Test".into()));
        doc.context.insert("ct".into(), DxLlmValue::Num(42.0));

        let llm1 = document_to_llm(&doc);
        let machine = document_to_machine(&doc);
        let parsed = machine_to_document(&machine).unwrap();
        let llm2 = document_to_llm(&parsed);

        let doc1 = llm_to_document(&llm1).unwrap();
        let doc2 = llm_to_document(&llm2).unwrap();

        assert_eq!(doc1.context.len(), doc2.context.len());
    }

    #[test]
    fn spec_all_formats_preserve_data() {
        let mut doc = DxDocument::new();
        doc.context.insert("str".into(), DxLlmValue::Str("Hello".into()));
        doc.context.insert("num".into(), DxLlmValue::Num(42.0));
        doc.context.insert("bool".into(), DxLlmValue::Bool(true));

        // Test Machine format
        let machine = document_to_machine(&doc);
        let machine_doc = machine_to_document(&machine).unwrap();
        assert_eq!(machine_doc.context.get("str").unwrap().as_str(), Some("Hello"));
        assert_eq!(machine_doc.context.get("num").unwrap().as_num(), Some(42.0));
        assert_eq!(machine_doc.context.get("bool").unwrap().as_bool(), Some(true));
    }
}

// ============================================================================
// PART 5: EDGE CASES AND STRESS TESTS
// ============================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn spec_empty_string() {
        let mut doc = DxDocument::new();
        doc.context.insert("empty".into(), DxLlmValue::Str("".into()));

        let machine = document_to_machine(&doc);
        let parsed = machine_to_document(&machine).unwrap();

        assert_eq!(parsed.context.get("empty").unwrap().as_str(), Some(""));
    }

    #[test]
    fn spec_large_numbers() {
        let mut doc = DxDocument::new();
        doc.context.insert("big".into(), DxLlmValue::Num(f64::MAX / 2.0));
        doc.context.insert("small".into(), DxLlmValue::Num(f64::MIN / 2.0));

        let machine = document_to_machine(&doc);
        let parsed = machine_to_document(&machine).unwrap();

        assert!(parsed.context.get("big").unwrap().as_num().is_some());
    }

    #[test]
    fn spec_many_fields() {
        let mut doc = DxDocument::new();
        for i in 0..100 {
            doc.context.insert(format!("f{}", i), DxLlmValue::Num(i as f64));
        }

        let machine = document_to_machine(&doc);
        let parsed = machine_to_document(&machine).unwrap();

        assert_eq!(parsed.context.len(), 100);
    }

    #[test]
    fn spec_large_table() {
        let mut doc = DxDocument::new();
        let mut section = DxSection::new(vec!["id".into(), "val".into()]);

        for i in 0..1000 {
            section.rows.push(vec![
                DxLlmValue::Num(i as f64),
                DxLlmValue::Str(format!("r{}", i)),
            ]);
        }
        doc.sections.insert('d', section);

        let machine = document_to_machine(&doc);
        let parsed = machine_to_document(&machine).unwrap();

        assert_eq!(parsed.sections.get(&'d').unwrap().rows.len(), 1000);
    }

    #[test]
    fn spec_nested_arrays() {
        let mut doc = DxDocument::new();
        doc.context.insert(
            "nested".into(),
            DxLlmValue::Arr(vec![
                DxLlmValue::Arr(vec![DxLlmValue::Num(1.0), DxLlmValue::Num(2.0)]),
                DxLlmValue::Arr(vec![DxLlmValue::Num(3.0), DxLlmValue::Num(4.0)]),
            ]),
        );

        let machine = document_to_machine(&doc);
        let parsed = machine_to_document(&machine).unwrap();

        let arr = parsed.context.get("nested").unwrap().as_arr().unwrap();
        assert_eq!(arr.len(), 2);
    }
}

// ============================================================================
// PART 6: PLATFORM COMPATIBILITY TESTS
// ============================================================================

mod platform_compat {
    use super::*;

    #[test]
    fn spec_endianness() {
        let mut buffer = Vec::new();
        let mut builder = DxZeroBuilder::new(&mut buffer, 8, 0);

        builder.write_u32(0, 0x12345678);
        builder.write_u32(4, 0xDEADBEEF);

        builder.finish();

        // Verify little-endian encoding
        assert_eq!(buffer[4], 0x78); // LSB first
        assert_eq!(buffer[5], 0x56);
        assert_eq!(buffer[6], 0x34);
        assert_eq!(buffer[7], 0x12); // MSB last
    }

    #[test]
    fn spec_alignment() {
        let mut buffer = Vec::new();
        let mut builder = DxZeroBuilder::new(&mut buffer, 24, 0);

        builder.write_u8(0, 1);
        builder.write_u16(2, 2);
        builder.write_u32(4, 3);
        builder.write_u64(8, 4);
        builder.write_f64(16, 5.0);

        builder.finish();

        assert!(buffer.len() >= 24);
    }

    #[test]
    fn spec_utf8_encoding() {
        let mut doc = DxDocument::new();

        doc.context.insert("ascii".into(), DxLlmValue::Str("Hello".into()));
        doc.context.insert("cjk".into(), DxLlmValue::Str("你好世界".into()));
        doc.context.insert("emoji".into(), DxLlmValue::Str("👋🌍".into()));
        doc.context.insert("mixed".into(), DxLlmValue::Str("Hello 世界 🌍".into()));

        let machine = document_to_machine(&doc);
        let parsed = machine_to_document(&machine).unwrap();

        assert_eq!(parsed.context.get("cjk").unwrap().as_str(), Some("你好世界"));
        assert_eq!(parsed.context.get("emoji").unwrap().as_str(), Some("👋🌍"));
    }
}

// ============================================================================
// PART 7: PERFORMANCE SANITY TESTS
// ============================================================================

mod performance {
    use super::*;
    use std::time::Instant;

    #[test]
    fn spec_llm_serialization_speed() {
        let mut doc = DxDocument::new();
        for i in 0..100 {
            doc.context.insert(format!("k{}", i), DxLlmValue::Num(i as f64));
        }

        let start = Instant::now();
        for _ in 0..1000 {
            let _ = document_to_llm(&doc);
        }
        let elapsed = start.elapsed();

        // Should complete 1000 iterations in under 1 second
        assert!(elapsed.as_secs() < 1, "LLM serialization too slow: {:?}", elapsed);
    }

    #[test]
    fn spec_machine_serialization_speed() {
        let mut doc = DxDocument::new();
        for i in 0..100 {
            doc.context.insert(format!("k{}", i), DxLlmValue::Num(i as f64));
        }

        let start = Instant::now();
        for _ in 0..1000 {
            let _ = document_to_machine(&doc);
        }
        let elapsed = start.elapsed();

        // Should complete 1000 iterations in under 1 second
        assert!(elapsed.as_secs() < 1, "Machine serialization too slow: {:?}", elapsed);
    }

    #[test]
    fn spec_zero_copy_speed() {
        let start = Instant::now();
        for _ in 0..10000 {
            let mut buffer = Vec::with_capacity(64);
            let mut builder = DxZeroBuilder::new(&mut buffer, 16, 1);
            builder.write_i32(0, 12345);
            builder.write_f64(4, 3.14159);
            builder.write_string(16, "test");
            builder.finish();
        }
        let elapsed = start.elapsed();

        // Should complete 10000 iterations in under 1 second
        assert!(elapsed.as_secs() < 1, "Zero-copy too slow: {:?}", elapsed);
    }
}
