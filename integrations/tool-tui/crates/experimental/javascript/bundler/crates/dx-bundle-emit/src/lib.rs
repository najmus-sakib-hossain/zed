//! Zero-copy output generation with source map support
//!
//! Emit final bundle with minimal allocations and proper source maps

use dx_bundle_core::error::BundleResult;
use dx_bundle_core::{BundleConfig, ModuleFormat, TransformedModule};
use sourcemap::SourceMapBuilder;

mod source_map;
pub use source_map::{generate_source_map, SourceMapGenerator, SourceMapMerger};

/// Bundle output with optional source map
pub struct BundleOutput {
    /// The bundled code
    pub code: Vec<u8>,
    /// The source map (if enabled)
    pub source_map: Option<Vec<u8>>,
}

/// Bundle emitter for zero-copy concatenation
pub struct BundleEmitter<'a> {
    config: &'a BundleConfig,
}

impl<'a> BundleEmitter<'a> {
    /// Create new emitter
    pub fn new(config: &'a BundleConfig) -> Self {
        Self { config }
    }

    /// Emit complete bundle with source map
    pub fn emit_with_source_map(
        &self,
        modules: &[TransformedModule],
    ) -> BundleResult<BundleOutput> {
        // Pre-calculate total size
        let total_size = self.calculate_size(modules);
        let mut output = Vec::with_capacity(total_size);

        // Track line/column for source map generation
        let mut current_line = 0u32;
        let mut current_column = 0u32;

        // Source map builder (if enabled)
        let mut sm_builder = if self.config.source_maps {
            Some(SourceMapBuilder::new(None))
        } else {
            None
        };

        // Emit format wrapper prefix
        let prefix = self.config.format.prefix();
        output.extend_from_slice(prefix);
        self.update_position(prefix, &mut current_line, &mut current_column);

        // Emit runtime header (if IIFE/CJS)
        if matches!(self.config.format, ModuleFormat::CJS | ModuleFormat::IIFE) {
            self.emit_runtime(&mut output)?;
            self.update_position(RUNTIME_HEADER, &mut current_line, &mut current_column);
        }

        // Emit each module with source map tracking
        for module in modules {
            self.emit_module_with_source_map(
                &mut output,
                module,
                &mut sm_builder,
                &mut current_line,
                &mut current_column,
            )?;
        }

        // Emit format wrapper suffix
        output.extend_from_slice(self.config.format.suffix());

        // Build final source map
        let source_map = if let Some(builder) = sm_builder {
            let sm = builder.into_sourcemap();
            let mut sm_bytes = Vec::new();
            sm.to_writer(&mut sm_bytes).map_err(|e| {
                dx_bundle_core::error::BundleError::transform_error(format!(
                    "Source map error: {}",
                    e
                ))
            })?;
            Some(sm_bytes)
        } else {
            None
        };

        Ok(BundleOutput {
            code: output,
            source_map,
        })
    }

    /// Emit complete bundle (legacy method without source map return)
    pub fn emit(&self, modules: &[TransformedModule]) -> BundleResult<Vec<u8>> {
        Ok(self.emit_with_source_map(modules)?.code)
    }

    /// Calculate total output size (for pre-allocation)
    fn calculate_size(&self, modules: &[TransformedModule]) -> usize {
        let mut size = 0;

        // Format wrappers
        size += self.config.format.prefix().len();
        size += self.config.format.suffix().len();

        // Runtime (if needed)
        if matches!(self.config.format, ModuleFormat::CJS | ModuleFormat::IIFE) {
            size += RUNTIME_HEADER.len();
        }

        // Modules
        for module in modules {
            size += 50; // Wrapper overhead
            size += module.content.len();
        }

        size
    }

    /// Emit runtime header
    fn emit_runtime(&self, output: &mut Vec<u8>) -> BundleResult<()> {
        output.extend_from_slice(RUNTIME_HEADER);
        Ok(())
    }

    /// Emit single module with source map tracking
    fn emit_module_with_source_map(
        &self,
        output: &mut Vec<u8>,
        module: &TransformedModule,
        sm_builder: &mut Option<SourceMapBuilder>,
        current_line: &mut u32,
        current_column: &mut u32,
    ) -> BundleResult<()> {
        // Get source file name (use module ID as fallback)
        let source_name = format!("module_{}.js", module.id);

        match self.config.format {
            ModuleFormat::ESM => {
                // ESM: Just emit content directly with source mapping
                if let Some(builder) = sm_builder.as_mut() {
                    // Add source file
                    let source_id = builder.add_source(&source_name);

                    // Add source content if available
                    let content_str = String::from_utf8_lossy(&module.content);
                    builder.set_source_contents(source_id, Some(&content_str));

                    // Map each line
                    for (line_idx, _line) in content_str.lines().enumerate() {
                        builder.add_raw(
                            *current_line,
                            *current_column,
                            line_idx as u32,
                            0,
                            Some(source_id),
                            None,
                            false,
                        );
                        *current_line += 1;
                        *current_column = 0;
                    }
                }

                output.extend_from_slice(&module.content);
                output.push(b'\n');
                *current_line += 1;
                *current_column = 0;
            }
            ModuleFormat::CJS | ModuleFormat::IIFE => {
                // CJS/IIFE: Wrap in __dx_define
                let wrapper_start = b"__dx_define(";
                output.extend_from_slice(wrapper_start);
                *current_column += wrapper_start.len() as u32;

                let id_str = module.id.to_string();
                output.extend_from_slice(id_str.as_bytes());
                *current_column += id_str.len() as u32;

                let wrapper_mid = b",function(exports,require,module){\n";
                output.extend_from_slice(wrapper_mid);
                *current_line += 1;
                *current_column = 0;

                // Add source mapping for module content
                if let Some(builder) = sm_builder.as_mut() {
                    let source_id = builder.add_source(&source_name);
                    let content_str = String::from_utf8_lossy(&module.content);
                    builder.set_source_contents(source_id, Some(&content_str));

                    for (line_idx, _line) in content_str.lines().enumerate() {
                        builder.add_raw(
                            *current_line,
                            0,
                            line_idx as u32,
                            0,
                            Some(source_id),
                            None,
                            false,
                        );
                        *current_line += 1;
                    }
                }

                output.extend_from_slice(&module.content);
                self.update_position(&module.content, current_line, current_column);

                output.extend_from_slice(b"\n});\n");
                *current_line += 2;
                *current_column = 0;
            }
            ModuleFormat::UMD => {
                // UMD: Similar to IIFE
                output.extend_from_slice(b"__dx_define(");
                output.extend_from_slice(module.id.to_string().as_bytes());
                output.extend_from_slice(b",function(exports,require,module){\n");
                *current_line += 1;
                *current_column = 0;

                if let Some(builder) = sm_builder.as_mut() {
                    let source_id = builder.add_source(&source_name);
                    let content_str = String::from_utf8_lossy(&module.content);
                    builder.set_source_contents(source_id, Some(&content_str));

                    for (line_idx, _line) in content_str.lines().enumerate() {
                        builder.add_raw(
                            *current_line,
                            0,
                            line_idx as u32,
                            0,
                            Some(source_id),
                            None,
                            false,
                        );
                        *current_line += 1;
                    }
                }

                output.extend_from_slice(&module.content);
                self.update_position(&module.content, current_line, current_column);

                output.extend_from_slice(b"\n});\n");
                *current_line += 2;
                *current_column = 0;
            }
        }

        Ok(())
    }

    /// Emit single module (legacy method)
    #[allow(dead_code)]
    fn emit_module(&self, output: &mut Vec<u8>, module: &TransformedModule) -> BundleResult<()> {
        match self.config.format {
            ModuleFormat::ESM => {
                output.extend_from_slice(&module.content);
                output.push(b'\n');
            }
            ModuleFormat::CJS | ModuleFormat::IIFE => {
                output.extend_from_slice(b"__dx_define(");
                output.extend_from_slice(module.id.to_string().as_bytes());
                output.extend_from_slice(b",function(exports,require,module){\n");
                output.extend_from_slice(&module.content);
                output.extend_from_slice(b"\n});\n");
            }
            ModuleFormat::UMD => {
                output.extend_from_slice(b"__dx_define(");
                output.extend_from_slice(module.id.to_string().as_bytes());
                output.extend_from_slice(b",function(exports,require,module){\n");
                output.extend_from_slice(&module.content);
                output.extend_from_slice(b"\n});\n");
            }
        }

        Ok(())
    }

    /// Update line/column position based on content
    fn update_position(&self, content: &[u8], line: &mut u32, column: &mut u32) {
        for &byte in content {
            if byte == b'\n' {
                *line += 1;
                *column = 0;
            } else {
                *column += 1;
            }
        }
    }

    /// Emit entry point bootstrap
    pub fn emit_entry(&self, output: &mut Vec<u8>, entry_id: u64) -> BundleResult<()> {
        output.extend_from_slice(b"__dx_require(");
        output.extend_from_slice(entry_id.to_string().as_bytes());
        output.extend_from_slice(b");\n");
        Ok(())
    }
}

/// Minimal runtime for CJS/IIFE bundles
const RUNTIME_HEADER: &[u8] = b"(function(){
'use strict';
var __dx_modules={};
var __dx_cache={};
function __dx_define(id,factory){__dx_modules[id]=factory;}
function __dx_require(id){
if(__dx_cache[id])return __dx_cache[id].exports;
var module={exports:{}};
__dx_cache[id]=module;
__dx_modules[id](module.exports,__dx_require,module);
return module.exports;
}
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_esm() {
        let config = BundleConfig {
            format: ModuleFormat::ESM,
            ..Default::default()
        };

        let emitter = BundleEmitter::new(&config);
        let modules = vec![TransformedModule {
            id: 0,
            content: b"console.log('test');".to_vec(),
            source_map: None,
            imports: vec![],
        }];

        let result = emitter.emit(&modules).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_emit_cjs() {
        let config = BundleConfig {
            format: ModuleFormat::CJS,
            ..Default::default()
        };

        let emitter = BundleEmitter::new(&config);
        let modules = vec![TransformedModule {
            id: 0,
            content: b"console.log('test');".to_vec(),
            source_map: None,
            imports: vec![],
        }];

        let result = emitter.emit(&modules).unwrap();
        let result_str = String::from_utf8_lossy(&result);
        assert!(result_str.contains("__dx_define"));
    }

    #[test]
    fn test_emit_with_source_map() {
        let config = BundleConfig {
            format: ModuleFormat::ESM,
            source_maps: true,
            ..Default::default()
        };

        let emitter = BundleEmitter::new(&config);
        let modules = vec![
            TransformedModule {
                id: 1,
                content: b"const x = 1;\nconsole.log(x);".to_vec(),
                source_map: None,
                imports: vec![],
            },
            TransformedModule {
                id: 2,
                content: b"export function hello() {\n  return 'world';\n}".to_vec(),
                source_map: None,
                imports: vec![],
            },
        ];

        let result = emitter.emit_with_source_map(&modules).unwrap();
        assert!(!result.code.is_empty());
        assert!(result.source_map.is_some());

        // Verify source map is valid JSON
        let sm_str = String::from_utf8(result.source_map.unwrap()).unwrap();
        let sm_json: serde_json::Value = serde_json::from_str(&sm_str).unwrap();
        assert_eq!(sm_json["version"], 3);
        assert!(sm_json["sources"].as_array().unwrap().len() >= 2);
    }

    #[test]
    fn test_emit_cjs_with_source_map() {
        let config = BundleConfig {
            format: ModuleFormat::CJS,
            source_maps: true,
            ..Default::default()
        };

        let emitter = BundleEmitter::new(&config);
        let modules = vec![TransformedModule {
            id: 100,
            content: b"module.exports = { foo: 'bar' };".to_vec(),
            source_map: None,
            imports: vec![],
        }];

        let result = emitter.emit_with_source_map(&modules).unwrap();
        assert!(!result.code.is_empty());
        assert!(result.source_map.is_some());

        // Verify output contains module wrapper
        let code_str = String::from_utf8(result.code).unwrap();
        assert!(code_str.contains("__dx_define(100"));
    }
}
