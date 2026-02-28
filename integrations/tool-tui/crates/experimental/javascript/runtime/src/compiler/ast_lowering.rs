//! Complete AST to MIR lowering
//!
//! This module walks the OXC AST and generates Typed MIR that can be
//! compiled to native code by Cranelift.

use crate::compiler::mir::*;
use crate::compiler::parser::ParsedAST;
use crate::compiler::statements::StatementLowerer;
use crate::error::{DxError, DxResult};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use std::collections::HashMap;

/// AST to MIR lowering context
pub struct AstLowering<'a> {
    /// Source code
    source: &'a str,
    /// Filename (for determining source type)
    filename: &'a str,
    /// All lowered functions
    functions: Vec<TypedFunction>,
    /// Global variables
    globals: Vec<TypedGlobal>,
    /// Type layouts
    type_layouts: HashMap<TypeId, TypeLayout>,
    /// Next IDs
    next_function_id: u32,
    /// Reserved for future type ID generation
    #[allow(dead_code)]
    next_type_id: u32,
    /// String constants - reserved for string interning optimization
    #[allow(dead_code)]
    string_constants: Vec<String>,
}

impl<'a> AstLowering<'a> {
    pub fn new(source: &'a str, filename: &'a str) -> Self {
        Self {
            source,
            filename,
            functions: Vec::new(),
            globals: Vec::new(),
            type_layouts: HashMap::new(),
            next_function_id: 0,
            next_type_id: 0,
            string_constants: Vec::new(),
        }
    }

    /// Lower the entire program
    pub fn lower(&mut self, _ast: &ParsedAST) -> DxResult<TypedMIR> {
        // Create main function
        let main_id = FunctionId(self.next_function_id);
        self.next_function_id += 1;
        let builder = FunctionBuilder::new(main_id, "__dx_main__".to_string());

        // Parse and lower the AST using OXC
        let builder = self.lower_source(builder)?;

        // Finalize main function
        self.functions.push(builder.build());

        Ok(TypedMIR {
            functions: std::mem::take(&mut self.functions),
            globals: std::mem::take(&mut self.globals),
            entry_point: Some(main_id),
            type_layouts: std::mem::take(&mut self.type_layouts),
            source_file: self.filename.to_string(),
        })
    }

    fn lower_source(&mut self, builder: FunctionBuilder) -> DxResult<FunctionBuilder> {
        // Parse source with OXC - use filename to determine source type (JS vs TS)
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(self.filename).unwrap_or_default();
        let parser_result = Parser::new(&allocator, self.source, source_type).parse();

        if !parser_result.errors.is_empty() {
            return Err(DxError::ParseError(
                parser_result
                    .errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
            ));
        }

        let program = parser_result.program;

        // Create statement lowerer
        let mut lowerer = StatementLowerer::new(builder);

        // Lower each statement, tracking the last expression result
        let mut last_result = None;
        for stmt in &program.body {
            last_result = lowerer.lower_statement(stmt)?;
        }

        // If the last statement was an expression, return its value
        // This enables REPL-style evaluation where the last expression is the result
        let mut builder = lowerer.finish();
        if let Some(result) = last_result {
            // Check if we already have a return terminator with a value
            let current_block = builder.current_block;
            let has_return_with_value = builder
                .blocks
                .iter()
                .find(|b| b.id == current_block)
                .map(|b| matches!(b.terminator, Terminator::Return(Some(_))))
                .unwrap_or(false);

            if !has_return_with_value {
                builder.set_terminator(Terminator::Return(Some(result)));
            }
        } else {
            // No expression result - return undefined (keep the default Return(None))
        }

        // Return the finished builder
        Ok(builder)
    }
}

/// Lower parsed AST to MIR
pub fn lower_ast_to_mir(source: &str, ast: &ParsedAST) -> DxResult<TypedMIR> {
    let mut lowering = AstLowering::new(source, &ast.filename);
    lowering.lower(ast)
}
