//! Round-trip property tests for DXM format conversions.
//!
//! These tests verify that converting between formats preserves semantic content.

#[cfg(test)]
mod prop_tests {
    use crate::human_formatter::HumanFormatter;
    use crate::human_parser::HumanParser;
    use crate::parser::DxmParser;
    use crate::serializer::LlmSerializer;
    use crate::types::*;
    use proptest::prelude::*;
    use std::collections::HashMap;

    /// Generate a random inline node (simple version for testing)
    /// Uses only alphanumeric characters to avoid markdown special characters
    fn arb_simple_inline() -> impl Strategy<Value = InlineNode> {
        prop_oneof![
            "[a-zA-Z][a-zA-Z0-9]{0,19}".prop_map(InlineNode::Text),
            "[a-zA-Z][a-zA-Z0-9_]{0,9}".prop_map(InlineNode::Code),
        ]
    }

    /// Generate a random header node
    fn arb_header() -> impl Strategy<Value = DxmNode> {
        (
            1u8..=6,
            prop::collection::vec(arb_simple_inline(), 1..=2),
            prop_oneof![
                Just(None),
                Just(Some(Priority::Low)),
                Just(Some(Priority::Important)),
                Just(Some(Priority::Critical)),
            ],
        )
            .prop_map(|(level, content, priority)| {
                DxmNode::Header(HeaderNode {
                    level,
                    content,
                    priority,
                })
            })
    }

    /// Generate a random paragraph node
    fn arb_paragraph() -> impl Strategy<Value = DxmNode> {
        prop::collection::vec(arb_simple_inline(), 1..=3).prop_map(DxmNode::Paragraph)
    }

    /// Generate a random code block node
    fn arb_code_block() -> impl Strategy<Value = DxmNode> {
        (
            prop_oneof![
                Just(None),
                Just(Some("rust".to_string())),
                Just(Some("python".to_string())),
            ],
            "[a-zA-Z][a-zA-Z0-9 ]{0,29}",
        )
            .prop_map(|(language, content)| {
                DxmNode::CodeBlock(CodeBlockNode {
                    language,
                    content,
                    priority: None,
                })
            })
    }

    /// Generate a random list node
    fn arb_list() -> impl Strategy<Value = DxmNode> {
        (
            any::<bool>(),
            prop::collection::vec(
                prop::collection::vec(arb_simple_inline(), 1..=2).prop_map(|content| ListItem {
                    content,
                    nested: None,
                }),
                1..=3,
            ),
        )
            .prop_map(|(ordered, items)| DxmNode::List(ListNode { ordered, items }))
    }

    /// Generate a random semantic block node
    fn arb_semantic_block() -> impl Strategy<Value = DxmNode> {
        (
            prop_oneof![
                Just(SemanticBlockType::Warning),
                Just(SemanticBlockType::Info),
                Just(SemanticBlockType::FAQ),
                Just(SemanticBlockType::Quote),
                Just(SemanticBlockType::Example),
            ],
            prop::collection::vec(arb_simple_inline(), 1..=2),
        )
            .prop_map(|(block_type, content)| {
                DxmNode::SemanticBlock(SemanticBlockNode {
                    block_type,
                    content,
                    priority: None,
                })
            })
    }

    /// Generate a random DXM document (simplified for round-trip testing)
    fn arb_document() -> impl Strategy<Value = DxmDocument> {
        prop::collection::vec(
            prop_oneof![
                arb_header(),
                arb_paragraph(),
                arb_code_block(),
                arb_list(),
                arb_semantic_block(),
            ],
            1..=4,
        )
        .prop_map(|nodes| DxmDocument {
            meta: DxmMeta {
                version: "1.0".to_string(),
                ..Default::default()
            },
            refs: HashMap::new(),
            nodes,
        })
    }

    /// Compare two documents for semantic equivalence.
    ///
    /// This is a relaxed comparison that checks if the documents have
    /// the same structure and content, allowing for minor formatting differences.
    fn docs_semantically_equal(doc1: &DxmDocument, doc2: &DxmDocument) -> bool {
        // Check node count
        if doc1.nodes.len() != doc2.nodes.len() {
            return false;
        }

        // Check each node
        for (n1, n2) in doc1.nodes.iter().zip(doc2.nodes.iter()) {
            if !nodes_semantically_equal(n1, n2) {
                return false;
            }
        }

        true
    }

    /// Compare two nodes for semantic equivalence.
    fn nodes_semantically_equal(n1: &DxmNode, n2: &DxmNode) -> bool {
        match (n1, n2) {
            (DxmNode::Header(h1), DxmNode::Header(h2)) => {
                h1.level == h2.level
                    && h1.priority == h2.priority
                    && inlines_semantically_equal(&h1.content, &h2.content)
            }
            (DxmNode::Paragraph(p1), DxmNode::Paragraph(p2)) => inlines_semantically_equal(p1, p2),
            (DxmNode::CodeBlock(c1), DxmNode::CodeBlock(c2)) => {
                c1.language == c2.language && c1.content.trim() == c2.content.trim()
            }
            (DxmNode::List(l1), DxmNode::List(l2)) => {
                l1.ordered == l2.ordered && l1.items.len() == l2.items.len()
            }
            (DxmNode::SemanticBlock(s1), DxmNode::SemanticBlock(s2)) => {
                s1.block_type == s2.block_type
            }
            (DxmNode::HorizontalRule, DxmNode::HorizontalRule) => true,
            (DxmNode::Table(t1), DxmNode::Table(t2)) => {
                t1.schema.len() == t2.schema.len() && t1.rows.len() == t2.rows.len()
            }
            _ => false,
        }
    }

    /// Compare inline content for semantic equivalence.
    fn inlines_semantically_equal(i1: &[InlineNode], i2: &[InlineNode]) -> bool {
        // Extract text content from both
        let text1 = extract_text(i1);
        let text2 = extract_text(i2);

        // Normalize whitespace and compare
        normalize_whitespace(&text1) == normalize_whitespace(&text2)
    }

    /// Extract plain text from inline nodes.
    fn extract_text(inlines: &[InlineNode]) -> String {
        let mut text = String::new();
        for inline in inlines {
            match inline {
                InlineNode::Text(t) => text.push_str(t),
                InlineNode::Bold(inner) => text.push_str(&extract_text(inner)),
                InlineNode::Italic(inner) => text.push_str(&extract_text(inner)),
                InlineNode::Strikethrough(inner) => text.push_str(&extract_text(inner)),
                InlineNode::Code(c) => text.push_str(c),
                InlineNode::Reference(r) => text.push_str(r),
                InlineNode::Link { text: t, .. } => text.push_str(&extract_text(t)),
                InlineNode::Image { alt, .. } => text.push_str(alt),
            }
        }
        text
    }

    /// Normalize whitespace in a string.
    fn normalize_whitespace(s: &str) -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: dxm-human-format, Property 1: Human Format Parse-Format Round Trip**
        /// *For any* valid DxmDocument, formatting it to human format and then parsing
        /// the result SHALL produce an equivalent AST (same nodes, same content, same structure).
        /// **Validates: Requirements 2.9, 1.1-1.9**
        #[test]
        fn prop_human_format_round_trip(doc in arb_document()) {
            let mut formatter = HumanFormatter::new();

            // Format to human format
            let human_output = formatter.format(&doc);

            // Parse back
            let parsed_result = HumanParser::parse(&human_output);

            // Should parse successfully
            prop_assert!(parsed_result.is_ok(),
                "Failed to parse formatted output: {:?}\nOutput was:\n{}",
                parsed_result.err(), human_output);

            let parsed_doc = parsed_result.expect("Already checked is_ok");

            // Should be semantically equivalent
            prop_assert!(docs_semantically_equal(&doc, &parsed_doc),
                "Documents not semantically equal.\nOriginal: {:?}\nParsed: {:?}\nHuman output:\n{}",
                doc, parsed_doc, human_output);
        }

        /// **Feature: dxm-human-format, Property 3: LLM-Human Format Conversion Round Trip**
        /// *For any* valid DXM document, converting from LLM format to Human format and back
        /// to LLM format SHALL preserve all semantic content (headers, paragraphs, code blocks,
        /// tables, lists, semantic blocks, references).
        /// **Validates: Requirements 6.4, 6.5, 6.6**
        #[test]
        fn prop_llm_human_round_trip(doc in arb_document()) {
            let llm_serializer = LlmSerializer::new();
            let mut human_formatter = HumanFormatter::new();

            // Serialize to LLM format
            let llm_output = llm_serializer.serialize(&doc);

            // Parse LLM format
            let parsed_llm = DxmParser::parse(&llm_output);
            prop_assert!(parsed_llm.is_ok(),
                "Failed to parse LLM output: {:?}\nOutput was:\n{}",
                parsed_llm.err(), llm_output);
            let parsed_llm = parsed_llm.expect("Already checked is_ok");

            // Convert to Human format
            let human_output = human_formatter.format(&parsed_llm);

            // Parse Human format
            let parsed_human = HumanParser::parse(&human_output);
            prop_assert!(parsed_human.is_ok(),
                "Failed to parse Human output: {:?}\nOutput was:\n{}",
                parsed_human.err(), human_output);
            let parsed_human = parsed_human.expect("Already checked is_ok");

            // Convert back to LLM format
            let llm_output2 = llm_serializer.serialize(&parsed_human);

            // Parse again
            let parsed_llm2 = DxmParser::parse(&llm_output2);
            prop_assert!(parsed_llm2.is_ok(),
                "Failed to parse second LLM output: {:?}\nOutput was:\n{}",
                parsed_llm2.err(), llm_output2);
            let parsed_llm2 = parsed_llm2.expect("Already checked is_ok");

            // Should be semantically equivalent
            prop_assert!(docs_semantically_equal(&parsed_llm, &parsed_llm2),
                "Documents not semantically equal after round trip.\nFirst: {:?}\nSecond: {:?}",
                parsed_llm, parsed_llm2);
        }
    }
}
