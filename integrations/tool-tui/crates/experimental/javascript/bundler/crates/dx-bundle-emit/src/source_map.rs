//! Source map generation and merging utilities
//!
//! Provides tools for generating source maps during bundling and merging
//! multiple source maps from different modules into a single output.

use dx_bundle_core::error::BundleResult;
use sourcemap::{SourceMap, SourceMapBuilder};

/// Source map generator for tracking transformations
pub struct SourceMapGenerator {
    builder: SourceMapBuilder,
    current_line: u32,
    current_column: u32,
}

impl SourceMapGenerator {
    /// Create a new source map generator
    pub fn new(file_name: Option<&str>) -> Self {
        Self {
            builder: SourceMapBuilder::new(file_name),
            current_line: 0,
            current_column: 0,
        }
    }

    /// Add a source file and return its ID
    pub fn add_source(&mut self, source_name: &str) -> u32 {
        self.builder.add_source(source_name)
    }

    /// Set source content for a source file
    pub fn set_source_content(&mut self, source_id: u32, content: &str) {
        self.builder.set_source_contents(source_id, Some(content));
    }

    /// Add a mapping from generated position to original position
    pub fn add_mapping(
        &mut self,
        gen_line: u32,
        gen_column: u32,
        orig_line: u32,
        orig_column: u32,
        source_id: u32,
        name: Option<&str>,
    ) {
        let name_id = name.map(|n| self.builder.add_name(n));
        self.builder.add_raw(
            gen_line,
            gen_column,
            orig_line,
            orig_column,
            Some(source_id),
            name_id,
            false,
        );
    }

    /// Add a simple line-to-line mapping (for unmodified code)
    pub fn add_line_mapping(&mut self, source_id: u32, orig_line: u32) {
        self.builder
            .add_raw(self.current_line, 0, orig_line, 0, Some(source_id), None, false);
        self.current_line += 1;
    }

    /// Advance the current position by the given content
    pub fn advance(&mut self, content: &str) {
        for ch in content.chars() {
            if ch == '\n' {
                self.current_line += 1;
                self.current_column = 0;
            } else {
                self.current_column += 1;
            }
        }
    }

    /// Get current generated line
    pub fn current_line(&self) -> u32 {
        self.current_line
    }

    /// Get current generated column
    pub fn current_column(&self) -> u32 {
        self.current_column
    }

    /// Build the final source map
    pub fn build(self) -> SourceMap {
        self.builder.into_sourcemap()
    }

    /// Build and serialize to JSON bytes
    pub fn build_to_bytes(self) -> BundleResult<Vec<u8>> {
        let sm = self.build();
        let mut bytes = Vec::new();
        sm.to_writer(&mut bytes).map_err(|e| {
            dx_bundle_core::error::BundleError::transform_error(format!("Source map error: {}", e))
        })?;
        Ok(bytes)
    }
}

/// Source map merger for combining multiple source maps
pub struct SourceMapMerger {
    builder: SourceMapBuilder,
    _source_offset: u32,
    line_offset: u32,
}

impl SourceMapMerger {
    /// Create a new source map merger
    pub fn new(output_file: Option<&str>) -> Self {
        Self {
            builder: SourceMapBuilder::new(output_file),
            _source_offset: 0,
            line_offset: 0,
        }
    }

    /// Add a source map to the merged output
    ///
    /// This remaps all positions in the input source map to account for
    /// the current line offset in the merged output.
    pub fn add_source_map(&mut self, sm: &SourceMap, line_offset: u32) -> BundleResult<()> {
        // Add all sources from the input map
        let mut source_id_map: Vec<u32> = Vec::new();

        for (idx, source) in sm.sources().enumerate() {
            let new_id = self.builder.add_source(source);
            source_id_map.push(new_id);

            // Copy source content if available
            if let Some(content) = sm.get_source_contents(idx as u32) {
                self.builder.set_source_contents(new_id, Some(content));
            }
        }

        // Add all names from the input map
        let mut name_id_map: Vec<u32> = Vec::new();
        for name in sm.names() {
            let new_id = self.builder.add_name(name);
            name_id_map.push(new_id);
        }

        // Remap all tokens
        for token in sm.tokens() {
            let src_id = token.get_src_id();
            let new_source_id = source_id_map.get(src_id as usize).copied();

            let name_id = token.get_name_id();
            let new_name_id = name_id_map.get(name_id as usize).copied();

            self.builder.add_raw(
                token.get_dst_line() + line_offset,
                token.get_dst_col(),
                token.get_src_line(),
                token.get_src_col(),
                new_source_id,
                new_name_id,
                false,
            );
        }

        Ok(())
    }

    /// Add raw content without an existing source map
    ///
    /// Creates simple line-to-line mappings for the content.
    pub fn add_raw_content(
        &mut self,
        source_name: &str,
        content: &str,
        line_offset: u32,
    ) -> BundleResult<()> {
        let source_id = self.builder.add_source(source_name);
        self.builder.set_source_contents(source_id, Some(content));

        for (line_idx, _line) in content.lines().enumerate() {
            self.builder.add_raw(
                line_offset + line_idx as u32,
                0,
                line_idx as u32,
                0,
                Some(source_id),
                None,
                false,
            );
        }

        Ok(())
    }

    /// Set the current line offset for subsequent additions
    pub fn set_line_offset(&mut self, offset: u32) {
        self.line_offset = offset;
    }

    /// Build the final merged source map
    pub fn build(self) -> SourceMap {
        self.builder.into_sourcemap()
    }

    /// Build and serialize to JSON bytes
    pub fn build_to_bytes(self) -> BundleResult<Vec<u8>> {
        let sm = self.build();
        let mut bytes = Vec::new();
        sm.to_writer(&mut bytes).map_err(|e| {
            dx_bundle_core::error::BundleError::transform_error(format!("Source map error: {}", e))
        })?;
        Ok(bytes)
    }
}

/// Generate a simple source map for transformed code
///
/// This creates a basic source map that maps each line in the output
/// to the corresponding line in the input. Useful for simple transformations
/// that don't significantly change line structure.
pub fn generate_source_map(
    source_name: &str,
    original_content: &str,
    transformed_content: &str,
) -> BundleResult<Vec<u8>> {
    let mut generator = SourceMapGenerator::new(None);
    let source_id = generator.add_source(source_name);
    generator.set_source_content(source_id, original_content);

    // Simple line-to-line mapping
    // For more accurate mapping, we'd need to track transformations
    let orig_lines: Vec<&str> = original_content.lines().collect();
    let trans_lines: Vec<&str> = transformed_content.lines().collect();

    for (trans_idx, _trans_line) in trans_lines.iter().enumerate() {
        // Map to original line (clamped to original line count)
        let orig_idx = trans_idx.min(orig_lines.len().saturating_sub(1));
        generator.add_mapping(trans_idx as u32, 0, orig_idx as u32, 0, source_id, None);
    }

    generator.build_to_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_map_generator() {
        let mut gen = SourceMapGenerator::new(Some("bundle.js"));
        let source_id = gen.add_source("input.js");
        gen.set_source_content(source_id, "const x = 1;");
        gen.add_mapping(0, 0, 0, 0, source_id, None);

        let sm = gen.build();
        assert_eq!(sm.get_source(0), Some("input.js"));
    }

    #[test]
    fn test_source_map_merger() {
        let mut merger = SourceMapMerger::new(Some("bundle.js"));

        // Add first module
        merger.add_raw_content("module1.js", "const a = 1;\nconst b = 2;", 0).unwrap();

        // Add second module at line 2
        merger.add_raw_content("module2.js", "const c = 3;", 2).unwrap();

        let sm = merger.build();
        assert!(sm.get_source_count() >= 2);
    }

    #[test]
    fn test_generate_source_map() {
        let original = "const x = 1;\nconst y = 2;";
        let transformed = "var x = 1;\nvar y = 2;";

        let sm_bytes = generate_source_map("test.js", original, transformed).unwrap();
        assert!(!sm_bytes.is_empty());

        // Verify it's valid JSON
        let sm_str = String::from_utf8(sm_bytes).unwrap();
        let sm_json: serde_json::Value = serde_json::from_str(&sm_str).unwrap();
        assert_eq!(sm_json["version"], 3);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate valid JavaScript-like source code for testing
    fn js_source_strategy() -> impl Strategy<Value = String> {
        prop::collection::vec(
            prop_oneof![
                Just("const x = 1;".to_string()),
                Just("let y = 2;".to_string()),
                Just("var z = 3;".to_string()),
                Just("function foo() { return 42; }".to_string()),
                Just("const arr = [1, 2, 3];".to_string()),
                Just("const obj = { a: 1, b: 2 };".to_string()),
                Just("// comment".to_string()),
                Just("console.log('hello');".to_string()),
                Just("export default function() {}".to_string()),
                Just("import { x } from 'module';".to_string()),
            ],
            1..20,
        )
        .prop_map(|lines| lines.join("\n"))
    }

    /// Generate valid file names for testing
    fn filename_strategy() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-z][a-z0-9_-]{0,20}\\.(js|ts|jsx|tsx)").unwrap()
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: dx-production-ready, Property 2: Bundler Output Validity (source maps)**
        /// **Validates: Requirements 8.2**
        ///
        /// *For any* valid JavaScript source file, generating a source map SHALL produce
        /// output that is valid JSON with version 3 and contains the original source.
        #[test]
        fn prop_source_map_validity(
            source in js_source_strategy(),
            filename in filename_strategy()
        ) {
            // Generate source map
            let sm_bytes = generate_source_map(&filename, &source, &source)?;

            // Property 1: Source map must be valid UTF-8
            let sm_str = String::from_utf8(sm_bytes.clone())
                .map_err(|_| TestCaseError::fail("Source map is not valid UTF-8"))?;

            // Property 2: Source map must be valid JSON
            let sm_json: serde_json::Value = serde_json::from_str(&sm_str)
                .map_err(|e| TestCaseError::fail(format!("Source map is not valid JSON: {}", e)))?;

            // Property 3: Source map version must be 3
            prop_assert_eq!(
                &sm_json["version"],
                &serde_json::json!(3),
                "Source map version must be 3"
            );

            // Property 4: Source map must have sources array
            prop_assert!(
                sm_json["sources"].is_array(),
                "Source map must have sources array"
            );

            // Property 5: Sources array must contain the filename
            let sources = sm_json["sources"].as_array().unwrap();
            prop_assert!(
                sources.iter().any(|s| s.as_str() == Some(&filename)),
                "Source map sources must contain the original filename"
            );

            // Property 6: Source map must have mappings string
            prop_assert!(
                sm_json["mappings"].is_string(),
                "Source map must have mappings string"
            );

            // Property 7: sourcesContent should contain original source
            if let Some(contents) = sm_json["sourcesContent"].as_array() {
                if !contents.is_empty() {
                    prop_assert!(
                        contents.iter().any(|c| c.as_str() == Some(&source)),
                        "sourcesContent should contain the original source"
                    );
                }
            }
        }

        /// **Feature: dx-production-ready, Property 2: Source Map Round-Trip**
        /// **Validates: Requirements 8.2**
        ///
        /// *For any* source map generated by SourceMapGenerator, serializing and
        /// deserializing SHALL preserve all source information.
        #[test]
        fn prop_source_map_round_trip(
            source in js_source_strategy(),
            filename in filename_strategy()
        ) {
            // Create source map using generator
            let mut gen = SourceMapGenerator::new(Some("bundle.js"));
            let source_id = gen.add_source(&filename);
            gen.set_source_content(source_id, &source);

            // Add mappings for each line
            for (line_idx, _) in source.lines().enumerate() {
                gen.add_mapping(
                    line_idx as u32,
                    0,
                    line_idx as u32,
                    0,
                    source_id,
                    None,
                );
            }

            // Serialize
            let sm = gen.build();
            let mut bytes = Vec::new();
            sm.to_writer(&mut bytes)
                .map_err(|e| TestCaseError::fail(format!("Failed to serialize: {}", e)))?;

            // Deserialize
            let parsed = SourceMap::from_reader(&bytes[..])
                .map_err(|e| TestCaseError::fail(format!("Failed to deserialize: {}", e)))?;

            // Verify source is preserved
            prop_assert_eq!(
                parsed.get_source(0),
                Some(filename.as_str()),
                "Source filename must be preserved after round-trip"
            );

            // Verify source content is preserved
            prop_assert_eq!(
                parsed.get_source_contents(0),
                Some(source.as_str()),
                "Source content must be preserved after round-trip"
            );
        }

        /// **Feature: dx-production-ready, Property 2: Source Map Merger Preserves Sources**
        /// **Validates: Requirements 8.2**
        ///
        /// *For any* set of source files merged together, the merged source map
        /// SHALL contain all original sources.
        #[test]
        fn prop_source_map_merger_preserves_sources(
            sources in prop::collection::vec(
                (filename_strategy(), js_source_strategy()),
                1..5
            )
        ) {
            let mut merger = SourceMapMerger::new(Some("bundle.js"));
            let mut line_offset = 0u32;

            // Add all sources
            for (filename, content) in &sources {
                merger.add_raw_content(filename, content, line_offset)
                    .map_err(|e| TestCaseError::fail(format!("Failed to add content: {}", e)))?;
                line_offset += content.lines().count() as u32;
            }

            // Build merged map
            let sm = merger.build();

            // Verify all sources are present
            let source_count = sm.get_source_count();
            prop_assert!(
                source_count >= sources.len() as u32,
                "Merged source map must contain at least {} sources, got {}",
                sources.len(),
                source_count
            );

            // Verify each source filename is present
            for (filename, _) in &sources {
                let found = (0..source_count)
                    .any(|i| sm.get_source(i) == Some(filename.as_str()));
                prop_assert!(
                    found,
                    "Source '{}' must be present in merged source map",
                    filename
                );
            }
        }
    }
}
