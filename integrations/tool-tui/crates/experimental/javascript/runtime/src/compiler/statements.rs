//! Complete JavaScript Statement Lowering
//!
//! Handles all JavaScript statements:
//! - Variable declarations (var, let, const)
//! - If/else statements
//! - Switch statements
//! - For loops (for, for...in, for...of)
//! - While/do-while loops
//! - Try/catch/finally
//! - Throw statements
//! - Break/continue
//! - Return statements
//! - Labeled statements
//! - Block statements
//! - Expression statements

use crate::compiler::expressions::{
    find_captured_variables_in_func, lower_expr, ExprContext, FunctionBuilder,
};
use crate::compiler::mir::{
    BinOpKind, BlockId, Constant, FunctionId, FunctionSignature, LocalId, PrimitiveType,
    Terminator, Type, TypeId, TypedInstruction,
};
use crate::error::{unsupported_feature, DxResult};
use oxc_ast::ast::*;
use std::collections::HashMap;

/// Statement lowering context
pub struct StatementLowerer {
    /// Current function builder
    pub builder: FunctionBuilder,
    /// Variable bindings
    pub variables: HashMap<String, LocalId>,
    /// Break/continue labels
    labels: Vec<String>,
    /// Break targets (label -> block)
    break_blocks: Vec<BlockId>,
    /// Continue targets (label -> block)
    continue_blocks: Vec<BlockId>,
    /// Label to break block mapping
    label_break_blocks: HashMap<String, BlockId>,
    /// Label to continue block mapping
    label_continue_blocks: HashMap<String, BlockId>,
    /// Async function context (if in async function)
    async_context: Option<AsyncContext>,
    /// Super class local (if in a class constructor/method that extends another class)
    /// Requirements: 6.4 - super() calls parent constructor with correct this
    super_class: Option<LocalId>,
    /// Current 'this' local (if in a class constructor/method)
    this_local: Option<LocalId>,
}

/// Context for async function compilation
pub struct AsyncContext {
    /// Function ID of the async function
    pub function_id: FunctionId,
    /// Promise local that will be returned
    pub promise_local: LocalId,
    /// Current await state index
    pub await_state: u32,
}

impl StatementLowerer {
    pub fn new(builder: FunctionBuilder) -> Self {
        Self {
            builder,
            variables: HashMap::new(),
            labels: Vec::new(),
            break_blocks: Vec::new(),
            continue_blocks: Vec::new(),
            label_break_blocks: HashMap::new(),
            label_continue_blocks: HashMap::new(),
            async_context: None,
            super_class: None,
            this_local: None,
        }
    }

    /// Set class context for class constructor/method compilation
    /// Requirements: 6.4, 6.5 - super() and super.method() calls
    pub fn set_class_context(&mut self, super_class: Option<LocalId>, this_local: LocalId) {
        self.super_class = super_class;
        self.this_local = Some(this_local);
    }

    /// Set async context for async function compilation
    pub fn set_async_context(&mut self, function_id: FunctionId, promise_local: LocalId) {
        self.async_context = Some(AsyncContext {
            function_id,
            promise_local,
            await_state: 0,
        });
    }

    /// Check if we're in an async function context
    pub fn is_async(&self) -> bool {
        self.async_context.is_some()
    }

    /// Get the promise local for the current async function
    pub fn get_async_promise(&self) -> Option<LocalId> {
        self.async_context.as_ref().map(|ctx| ctx.promise_local)
    }

    /// Create an ExprContext with the current class context
    fn make_expr_context(&mut self) -> ExprContext<'_> {
        ExprContext {
            builder: &mut self.builder,
            variables: &mut self.variables,
            super_class: self.super_class,
            this_local: self.this_local,
        }
    }

    pub fn lower_statement(&mut self, stmt: &Statement) -> DxResult<Option<LocalId>> {
        match stmt {
            Statement::ExpressionStatement(expr_stmt) => self.lower_expression_statement(expr_stmt),
            Statement::BlockStatement(block) => self.lower_block_statement(block),
            Statement::VariableDeclaration(var_decl) => self.lower_variable_declaration(var_decl),
            Statement::FunctionDeclaration(func_decl) => self.lower_function_declaration(func_decl),
            Statement::ReturnStatement(ret) => self.lower_return_statement(ret),
            Statement::IfStatement(if_stmt) => self.lower_if_statement(if_stmt),
            Statement::SwitchStatement(switch) => self.lower_switch_statement(switch),
            Statement::ForStatement(for_stmt) => self.lower_for_statement(for_stmt),
            Statement::ForInStatement(for_in) => self.lower_for_in_statement(for_in),
            Statement::ForOfStatement(for_of) => self.lower_for_of_statement(for_of),
            Statement::WhileStatement(while_stmt) => self.lower_while_statement(while_stmt),
            Statement::DoWhileStatement(do_while) => self.lower_do_while_statement(do_while),
            Statement::TryStatement(try_stmt) => self.lower_try_statement(try_stmt),
            Statement::ThrowStatement(throw) => self.lower_throw_statement(throw),
            Statement::BreakStatement(brk) => self.lower_break_statement(brk),
            Statement::ContinueStatement(cont) => self.lower_continue_statement(cont),
            Statement::LabeledStatement(labeled) => self.lower_labeled_statement(labeled),
            Statement::EmptyStatement(_) => Ok(None),
            Statement::ClassDeclaration(class_decl) => self.lower_class_declaration(class_decl),

            // Explicitly handle unsupported statements with clear error messages
            // per Requirement 10.1: "WHEN a JavaScript feature is not supported,
            // THE DX_Runtime SHALL throw a SyntaxError or TypeError with a clear
            // message indicating the unsupported feature"
            Statement::WithStatement(_) => Err(unsupported_feature(
                "with statement",
                "The 'with' statement is not supported in DX-JS (deprecated in strict mode)",
                "Use explicit object property access instead",
            )),
            Statement::DebuggerStatement(_) => {
                // Debugger statements are a no-op in production
                // Just return None without error
                Ok(None)
            }
            Statement::ImportDeclaration(_) => {
                // Import declarations should be handled at module level
                // If we encounter one here, it's likely a parsing issue
                Err(unsupported_feature(
                    "import declaration in statement position",
                    "Import declarations must be at the top level of a module",
                    "Move import statements to the top of your file",
                ))
            }
            Statement::ExportDefaultDeclaration(_)
            | Statement::ExportNamedDeclaration(_)
            | Statement::ExportAllDeclaration(_) => {
                // Export declarations should be handled at module level
                Err(unsupported_feature(
                    "export declaration in statement position",
                    "Export declarations must be at the top level of a module",
                    "Move export statements to the top level of your file",
                ))
            }
            Statement::TSTypeAliasDeclaration(_)
            | Statement::TSInterfaceDeclaration(_)
            | Statement::TSEnumDeclaration(_)
            | Statement::TSModuleDeclaration(_)
            | Statement::TSImportEqualsDeclaration(_)
            | Statement::TSExportAssignment(_)
            | Statement::TSNamespaceExportDeclaration(_) => {
                // TypeScript declarations should be stripped during compilation
                // If we encounter them, just ignore them (they have no runtime effect)
                Ok(None)
            }
        }
    }

    fn lower_expression_statement(
        &mut self,
        expr_stmt: &ExpressionStatement,
    ) -> DxResult<Option<LocalId>> {
        let mut ctx = self.make_expr_context();
        let result = lower_expr(&mut ctx, &expr_stmt.expression)?;
        Ok(Some(result))
    }

    fn lower_block_statement(&mut self, block: &BlockStatement) -> DxResult<Option<LocalId>> {
        let mut last = None;
        for stmt in &block.body {
            last = self.lower_statement(stmt)?;
        }
        Ok(last)
    }

    fn lower_variable_declaration(
        &mut self,
        var_decl: &VariableDeclaration,
    ) -> DxResult<Option<LocalId>> {
        let mut last = None;

        for declarator in &var_decl.declarations {
            if let Some(init) = &declarator.init {
                let mut ctx = self.make_expr_context();
                let value = lower_expr(&mut ctx, init)?;

                // Check for null/undefined before destructuring (Requirements: 7.7)
                // Only check for object/array patterns, not simple identifiers
                if matches!(declarator.id.kind, 
                    BindingPatternKind::ObjectPattern(_) | BindingPatternKind::ArrayPattern(_)) {
                    self.emit_destructuring_null_check(value)?;
                }

                // Bind the variable(s) based on the binding pattern
                last = Some(self.lower_binding_pattern(&declarator.id, value)?);
            } else {
                // No initializer - bind to undefined
                let undef = self.builder.add_local("_undef".to_string(), Type::Any);
                self.builder.emit(TypedInstruction::Const {
                    dest: undef,
                    value: Constant::Undefined,
                });
                last = Some(self.lower_binding_pattern(&declarator.id, undef)?);
            }
        }

        Ok(last)
    }

    /// Emit a null/undefined check for destructuring
    /// Requirements: 7.7 - destructuring null/undefined error
    fn emit_destructuring_null_check(&mut self, source: LocalId) -> DxResult<()> {
        // Check if source is nullish
        let is_nullish = self.builder.add_local("_is_nullish".to_string(), Type::Primitive(PrimitiveType::Bool));
        self.builder.emit(TypedInstruction::IsNullish {
            dest: is_nullish,
            src: source,
        });

        // Create blocks for the check
        let throw_block = self.builder.new_block();
        let continue_block = self.builder.new_block();

        // Branch: if nullish, throw; otherwise continue
        self.builder.set_terminator(Terminator::Branch {
            condition: is_nullish,
            then_block: throw_block,
            else_block: continue_block,
        });

        // Throw block - emit TypeError
        self.builder.switch_to_block(throw_block);
        self.builder.emit(TypedInstruction::ThrowDestructuringError {
            source,
        });
        self.builder.set_terminator(Terminator::Unreachable);

        // Continue block
        self.builder.switch_to_block(continue_block);

        Ok(())
    }

    /// Lower a binding pattern, binding variables to values extracted from the source
    /// Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6
    fn lower_binding_pattern(
        &mut self,
        pattern: &BindingPattern,
        source: LocalId,
    ) -> DxResult<LocalId> {
        match &pattern.kind {
            BindingPatternKind::BindingIdentifier(ident) => {
                // Simple identifier binding
                let name = ident.name.to_string();
                self.variables.insert(name, source);
                Ok(source)
            }
            BindingPatternKind::ObjectPattern(obj_pattern) => {
                // Object destructuring: const { a, b } = obj
                // Requirements: 7.2 - extract properties by name
                
                // Collect extracted keys for rest pattern
                let mut extracted_keys: Vec<String> = Vec::new();
                
                for prop in &obj_pattern.properties {
                    let key_name = match &prop.key {
                        PropertyKey::StaticIdentifier(ident) => ident.name.to_string(),
                        PropertyKey::StringLiteral(lit) => lit.value.to_string(),
                        PropertyKey::NumericLiteral(lit) => lit.value.to_string(),
                        _ => continue,
                    };
                    
                    extracted_keys.push(key_name.clone());

                    // Get property from source object
                    let prop_value = self.builder.add_local(format!("_{}", key_name), Type::Any);
                    self.builder.emit(TypedInstruction::GetPropertyDynamic {
                        dest: prop_value,
                        object: source,
                        property: key_name.clone(),
                    });

                    // Handle default value if present
                    // Requirements: 7.3 - use default when value is undefined
                    let final_value = match &prop.value.kind {
                        BindingPatternKind::AssignmentPattern(assign) => {
                            self.emit_default_value_check(prop_value, &assign.right)?
                        }
                        _ => prop_value,
                    };

                    // Recursively bind the value pattern (supports nested destructuring)
                    // Requirements: 7.6 - nested destructuring
                    self.lower_binding_pattern(&prop.value, final_value)?;
                }

                // Handle rest element: const { a, ...rest } = obj
                // Requirements: 7.5 - rest properties in object destructuring
                if let Some(rest) = &obj_pattern.rest {
                    let rest_obj = self.builder.add_local("_rest_obj".to_string(), Type::Any);
                    self.builder.emit(TypedInstruction::ObjectRest {
                        dest: rest_obj,
                        source,
                        excluded_keys: extracted_keys,
                    });
                    self.lower_binding_pattern(&rest.argument, rest_obj)?;
                }

                Ok(source)
            }
            BindingPatternKind::ArrayPattern(arr_pattern) => {
                // Array destructuring: const [a, b] = arr
                // Requirements: 7.1 - extract elements by index
                
                let num_elements = arr_pattern.elements.len();
                
                for (i, elem) in arr_pattern.elements.iter().enumerate() {
                    if let Some(binding) = elem {
                        // Get element at index
                        let elem_value = self.builder.add_local(format!("_elem_{}", i), Type::Any);
                        self.builder.emit(TypedInstruction::GetPropertyDynamic {
                            dest: elem_value,
                            object: source,
                            property: i.to_string(),
                        });

                        // Handle default value if present
                        // Requirements: 7.3 - use default when value is undefined
                        let final_value = match &binding.kind {
                            BindingPatternKind::AssignmentPattern(assign) => {
                                self.emit_default_value_check(elem_value, &assign.right)?
                            }
                            _ => elem_value,
                        };

                        // Recursively bind the element pattern (supports nested destructuring)
                        // Requirements: 7.6 - nested destructuring
                        self.lower_binding_pattern(binding, final_value)?;
                    }
                }

                // Handle rest element: const [a, ...rest] = arr
                // Requirements: 7.4 - rest elements in array destructuring
                if let Some(rest) = &arr_pattern.rest {
                    let rest_arr = self.builder.add_local("_rest_arr".to_string(), Type::Any);
                    self.builder.emit(TypedInstruction::ArraySliceFrom {
                        dest: rest_arr,
                        source,
                        start_index: num_elements as u32,
                    });
                    self.lower_binding_pattern(&rest.argument, rest_arr)?;
                }

                Ok(source)
            }
            BindingPatternKind::AssignmentPattern(assign_pattern) => {
                // Default value pattern: const { a = 5 } = obj or const [x = 1] = arr
                // Requirements: 7.3 - use default when value is undefined
                
                // Check if source is undefined and use default if so
                let final_value = self.emit_default_value_check(source, &assign_pattern.right)?;

                // Bind the left pattern to the final value
                self.lower_binding_pattern(&assign_pattern.left, final_value)
            }
        }
    }

    /// Emit code to check if a value is undefined and use a default value if so
    /// Requirements: 7.3 - default values in destructuring
    fn emit_default_value_check(
        &mut self,
        source: LocalId,
        default_expr: &Expression,
    ) -> DxResult<LocalId> {
        // Check if source is undefined
        let is_undef = self.builder.add_local("_is_undef".to_string(), Type::Primitive(PrimitiveType::Bool));
        self.builder.emit(TypedInstruction::IsUndefined {
            dest: is_undef,
            src: source,
        });

        // Create result local
        let result = self.builder.add_local("_default_result".to_string(), Type::Any);

        // Create blocks for branching
        let use_default_block = self.builder.new_block();
        let use_source_block = self.builder.new_block();
        let merge_block = self.builder.new_block();

        // Branch: if undefined, use default; otherwise use source
        self.builder.set_terminator(Terminator::Branch {
            condition: is_undef,
            then_block: use_default_block,
            else_block: use_source_block,
        });

        // Use default block - evaluate default expression
        self.builder.switch_to_block(use_default_block);
        let mut ctx = self.make_expr_context();
        let default_val = lower_expr(&mut ctx, default_expr)?;
        self.builder.emit(TypedInstruction::Copy {
            dest: result,
            src: default_val,
        });
        self.builder.set_terminator(Terminator::Goto(merge_block));

        // Use source block - use the original value
        self.builder.switch_to_block(use_source_block);
        self.builder.emit(TypedInstruction::Copy {
            dest: result,
            src: source,
        });
        self.builder.set_terminator(Terminator::Goto(merge_block));

        // Continue from merge block
        self.builder.switch_to_block(merge_block);

        Ok(result)
    }

    fn lower_function_declaration(&mut self, func_decl: &Function) -> DxResult<Option<LocalId>> {
        // function name(params) { body }
        // Create a function object and bind it to the function name

        // Get function name
        let func_name = func_decl
            .id
            .as_ref()
            .map(|id| id.name.to_string())
            .unwrap_or_else(|| "_anon_func".to_string());

        // Find free variables that need to be captured using proper analysis
        let captured_vars = find_captured_variables_in_func(func_decl, &self.variables);

        // Create a new function ID for this function declaration
        let func_id = FunctionId(self.builder.id.0 + 3000); // Offset to avoid conflicts

        // Create the function object
        let dest = self.builder.add_local(
            func_name.clone(),
            Type::Function(FunctionSignature {
                params: func_decl.params.items.iter().map(|_| Type::Any).collect(),
                return_type: Box::new(Type::Any),
            }),
        );

        self.builder.emit(TypedInstruction::CreateFunction {
            dest,
            function_id: func_id,
            captured_vars,
            is_arrow: false, // Regular functions have their own `this`
        });

        // Bind the function name in the current scope
        self.variables.insert(func_name, dest);

        Ok(Some(dest))
    }

    fn lower_class_declaration(
        &mut self,
        class: &oxc_ast::ast::Class,
    ) -> DxResult<Option<LocalId>> {
        // class Name extends SuperClass { constructor() {} methods... }

        // Get class name
        let class_name = class
            .id
            .as_ref()
            .map(|id| id.name.to_string())
            .unwrap_or_else(|| "_anon_class".to_string());

        // Create a type ID for this class
        let type_id = TypeId(self.builder.id.0 + 4000); // Offset to avoid conflicts

        // Handle super class if present
        let super_class = if let Some(super_expr) = &class.super_class {
            let mut ctx = self.make_expr_context();
            Some(lower_expr(&mut ctx, super_expr)?)
        } else {
            None
        };

        // Find the constructor method
        let mut constructor_id: Option<FunctionId> = None;
        for element in &class.body.body {
            if let oxc_ast::ast::ClassElement::MethodDefinition(method) = element {
                if method.kind == oxc_ast::ast::MethodDefinitionKind::Constructor {
                    // Create a function ID for the constructor
                    constructor_id = Some(FunctionId(self.builder.id.0 + 5000));
                    break;
                }
            }
        }

        // Create the class constructor
        let dest = self.builder.add_local(
            class_name.clone(),
            Type::Function(FunctionSignature {
                params: vec![],
                return_type: Box::new(Type::Object(type_id)),
            }),
        );

        self.builder.emit(TypedInstruction::CreateClass {
            dest,
            class_id: type_id,
            constructor_id,
            super_class,
        });

        // Get the prototype for defining methods
        let prototype = self.builder.add_local("_prototype".to_string(), Type::Any);
        self.builder.emit(TypedInstruction::GetPrototype {
            dest: prototype,
            constructor: dest,
        });

        // Process class elements (methods, getters, setters, static methods)
        for element in &class.body.body {
            match element {
                oxc_ast::ast::ClassElement::MethodDefinition(method) => {
                    if method.kind == oxc_ast::ast::MethodDefinitionKind::Constructor {
                        // Constructor is handled separately
                        continue;
                    }

                    let method_name = self.extract_property_key_name(&method.key);
                    let func_id =
                        FunctionId(self.builder.id.0 + 6000 + self.builder.locals.len() as u32);
                    let is_static = method.r#static;

                    match method.kind {
                        oxc_ast::ast::MethodDefinitionKind::Method => {
                            self.builder.emit(TypedInstruction::DefineMethod {
                                prototype,
                                name: method_name,
                                function_id: func_id,
                                is_static,
                            });
                        }
                        oxc_ast::ast::MethodDefinitionKind::Get => {
                            self.builder.emit(TypedInstruction::DefineGetter {
                                prototype,
                                name: method_name,
                                function_id: func_id,
                                is_static,
                            });
                        }
                        oxc_ast::ast::MethodDefinitionKind::Set => {
                            self.builder.emit(TypedInstruction::DefineSetter {
                                prototype,
                                name: method_name,
                                function_id: func_id,
                                is_static,
                            });
                        }
                        _ => {}
                    }
                }
                oxc_ast::ast::ClassElement::PropertyDefinition(prop) => {
                    // Instance properties are initialized in the constructor
                    // Static properties are set on the class itself
                    if prop.r#static {
                        let prop_name = self.extract_property_key_name(&prop.key);
                        if let Some(value_expr) = &prop.value {
                            let mut ctx = self.make_expr_context();
                            let value = lower_expr(&mut ctx, value_expr)?;
                            self.builder.emit(TypedInstruction::SetPropertyDynamic {
                                object: dest,
                                property: prop_name,
                                value,
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        // Bind the class name in the current scope
        self.variables.insert(class_name, dest);

        Ok(Some(dest))
    }

    fn extract_property_key_name(&self, key: &oxc_ast::ast::PropertyKey) -> String {
        match key {
            oxc_ast::ast::PropertyKey::StaticIdentifier(ident) => ident.name.to_string(),
            oxc_ast::ast::PropertyKey::PrivateIdentifier(ident) => format!("#{}", ident.name),
            oxc_ast::ast::PropertyKey::NumericLiteral(lit) => lit.value.to_string(),
            oxc_ast::ast::PropertyKey::StringLiteral(lit) => lit.value.to_string(),
            _ => "__unknown__".to_string(),
        }
    }

    fn lower_return_statement(&mut self, ret: &ReturnStatement) -> DxResult<Option<LocalId>> {
        let value = if let Some(arg) = &ret.argument {
            let mut ctx = self.make_expr_context();
            Some(lower_expr(&mut ctx, arg)?)
        } else {
            None
        };

        // In async functions, resolve the promise instead of returning directly
        if let Some(async_ctx) = &self.async_context {
            let promise_local = async_ctx.promise_local;
            if let Some(val) = value {
                self.builder.emit(TypedInstruction::PromiseResolve {
                    promise: promise_local,
                    value: val,
                });
            } else {
                // Resolve with undefined
                let undef = self.builder.add_local("_undef".to_string(), Type::Any);
                self.builder.emit(TypedInstruction::Const {
                    dest: undef,
                    value: Constant::Undefined,
                });
                self.builder.emit(TypedInstruction::PromiseResolve {
                    promise: promise_local,
                    value: undef,
                });
            }
            // Return the promise
            self.builder.set_terminator(Terminator::Return(Some(promise_local)));
        } else {
            self.builder.set_terminator(Terminator::Return(value));
        }

        Ok(value)
    }

    fn lower_if_statement(&mut self, if_stmt: &IfStatement) -> DxResult<Option<LocalId>> {
        // if (condition) then else alternate
        let mut ctx = self.make_expr_context();
        let condition = lower_expr(&mut ctx, &if_stmt.test)?;

        // Create blocks
        let then_block = self.builder.new_block();
        let else_block = if if_stmt.alternate.is_some() {
            Some(self.builder.new_block())
        } else {
            None
        };
        let merge_block = self.builder.new_block();

        // Emit conditional branch based on condition
        self.builder.set_terminator(Terminator::Branch {
            condition,
            then_block,
            else_block: else_block.unwrap_or(merge_block),
        });

        // Lower then branch
        self.builder.switch_to_block(then_block);
        self.lower_statement(&if_stmt.consequent)?;
        self.builder.set_terminator(Terminator::Goto(merge_block));

        // Lower else branch if present
        if let Some(else_stmt) = &if_stmt.alternate {
            if let Some(else_block) = else_block {
                self.builder.switch_to_block(else_block);
                self.lower_statement(else_stmt)?;
                self.builder.set_terminator(Terminator::Goto(merge_block));
            }
        }

        // Continue at merge block
        self.builder.switch_to_block(merge_block);

        Ok(None)
    }

    fn lower_switch_statement(&mut self, switch: &SwitchStatement) -> DxResult<Option<LocalId>> {
        // switch (discriminant) { case x: ... default: ... }

        // Lower the discriminant
        let mut ctx = self.make_expr_context();
        let discriminant = lower_expr(&mut ctx, &switch.discriminant)?;

        // Create exit block
        let exit_block = self.builder.new_block();
        self.break_blocks.push(exit_block);

        // Create blocks for each case body and case test
        let mut case_body_blocks: Vec<BlockId> = Vec::new();
        let mut case_test_blocks: Vec<BlockId> = Vec::new();
        let mut default_body_block: Option<BlockId> = None;

        for case in switch.cases.iter() {
            let body_block = self.builder.new_block();
            let test_block = self.builder.new_block();
            if case.test.is_none() {
                default_body_block = Some(body_block);
            }
            case_body_blocks.push(body_block);
            case_test_blocks.push(test_block);
        }

        // If no default, default goes to exit
        let default_target = default_body_block.unwrap_or(exit_block);

        // Generate case matching logic with proper comparisons
        if !switch.cases.is_empty() {
            // Jump to first case test
            self.builder.set_terminator(Terminator::Goto(case_test_blocks[0]));

            // Generate test blocks for each case
            for (i, case) in switch.cases.iter().enumerate() {
                self.builder.switch_to_block(case_test_blocks[i]);

                if let Some(test_expr) = &case.test {
                    // Compare discriminant with case value
                    let mut ctx = self.make_expr_context();
                    let case_value = lower_expr(&mut ctx, test_expr)?;

                    // Strict equality comparison (===)
                    let matches = self
                        .builder
                        .add_local("_case_match".to_string(), Type::Primitive(PrimitiveType::Bool));
                    self.builder.emit(TypedInstruction::StrictEqual {
                        dest: matches,
                        left: discriminant,
                        right: case_value,
                    });

                    // If matches, go to body; otherwise go to next test or default
                    let next_test = if i + 1 < case_test_blocks.len() {
                        case_test_blocks[i + 1]
                    } else {
                        default_target
                    };

                    self.builder.set_terminator(Terminator::Branch {
                        condition: matches,
                        then_block: case_body_blocks[i],
                        else_block: next_test,
                    });
                } else {
                    // This is the default case - skip to next test or exit
                    // Default is handled after all other cases are tested
                    let next_test = if i + 1 < case_test_blocks.len() {
                        case_test_blocks[i + 1]
                    } else {
                        exit_block
                    };
                    self.builder.set_terminator(Terminator::Goto(next_test));
                }
            }
        } else {
            self.builder.set_terminator(Terminator::Goto(exit_block));
        }

        // Lower each case body
        for (i, case) in switch.cases.iter().enumerate() {
            self.builder.switch_to_block(case_body_blocks[i]);

            // Lower case body statements
            for stmt in &case.consequent {
                self.lower_statement(stmt)?;
            }

            // Fall through to next case body or exit (unless break is encountered)
            let next_block = if i + 1 < case_body_blocks.len() {
                case_body_blocks[i + 1]
            } else {
                exit_block
            };
            self.builder.set_terminator(Terminator::Goto(next_block));
        }

        // Continue at exit
        self.builder.switch_to_block(exit_block);
        self.break_blocks.pop();

        Ok(None)
    }

    fn lower_for_statement(&mut self, for_stmt: &ForStatement) -> DxResult<Option<LocalId>> {
        self.lower_for_statement_with_label(for_stmt, None)
    }

    fn lower_for_statement_with_label(&mut self, for_stmt: &ForStatement, label: Option<&str>) -> DxResult<Option<LocalId>> {
        // for (init; test; update) body

        // Lower init
        if let Some(init) = &for_stmt.init {
            match init {
                ForStatementInit::VariableDeclaration(var_decl) => {
                    self.lower_variable_declaration(var_decl)?;
                }
                // Handle expression initializers (e.g., for (i = 0; ...))
                _ => {
                    // ForStatementInit inherits from Expression for other variants
                    // Try to convert to expression and lower it
                    if let Some(expr) = init.as_expression() {
                        let mut ctx = self.make_expr_context();
                        lower_expr(&mut ctx, expr)?;
                    }
                }
            }
        }

        // Create blocks
        let test_block = self.builder.new_block();
        let body_block = self.builder.new_block();
        let update_block = self.builder.new_block();
        let exit_block = self.builder.new_block();

        // Save loop targets for break/continue
        self.break_blocks.push(exit_block);
        self.continue_blocks.push(update_block);

        // If this is a labeled loop, register the continue block for the label
        if let Some(label_name) = label {
            self.label_continue_blocks.insert(label_name.to_string(), update_block);
        }

        // Jump to test
        self.builder.set_terminator(Terminator::Goto(test_block));
        self.builder.switch_to_block(test_block);

        // Lower test and generate conditional branch
        if let Some(test) = &for_stmt.test {
            let mut ctx = self.make_expr_context();
            let condition = lower_expr(&mut ctx, test)?;

            // Branch: if condition is true, enter body; otherwise exit
            self.builder.set_terminator(Terminator::Branch {
                condition,
                then_block: body_block,
                else_block: exit_block,
            });
        } else {
            // No test - infinite loop (always enter body)
            self.builder.set_terminator(Terminator::Goto(body_block));
        }

        // Lower body
        self.builder.switch_to_block(body_block);
        self.lower_statement(&for_stmt.body)?;
        self.builder.set_terminator(Terminator::Goto(update_block));

        // Lower update
        self.builder.switch_to_block(update_block);
        if let Some(update) = &for_stmt.update {
            let mut ctx = self.make_expr_context();
            lower_expr(&mut ctx, update)?;
        }
        self.builder.set_terminator(Terminator::Goto(test_block));

        // Continue at exit
        self.builder.switch_to_block(exit_block);

        // Pop loop targets
        self.break_blocks.pop();
        self.continue_blocks.pop();

        Ok(None)
    }

    fn lower_for_in_statement(&mut self, for_in: &ForInStatement) -> DxResult<Option<LocalId>> {
        self.lower_for_in_statement_with_label(for_in, None)
    }

    fn lower_for_in_statement_with_label(&mut self, for_in: &ForInStatement, label: Option<&str>) -> DxResult<Option<LocalId>> {
        // for (var in obj) body
        // Iterate over enumerable property names

        // Lower the object expression
        let mut ctx = self.make_expr_context();
        let obj = lower_expr(&mut ctx, &for_in.right)?;

        // Get the enumerable keys from the object
        // We use a special "keys" property that the runtime will handle
        let keys = self.builder.add_local("_keys".to_string(), Type::Any);
        // Call Object.keys() equivalent - get enumerable own property names
        self.builder.emit(TypedInstruction::Call {
            dest: Some(keys),
            function: FunctionId(0xFFFF_0001), // Special function ID for Object.keys
            args: vec![obj],
        });

        // Get the length of the keys array
        let keys_length = self
            .builder
            .add_local("_keys_length".to_string(), Type::Primitive(PrimitiveType::I32));
        self.builder.emit(TypedInstruction::GetPropertyDynamic {
            dest: keys_length,
            object: keys,
            property: "length".to_string(),
        });

        // Create index variable
        let index = self
            .builder
            .add_local("_index".to_string(), Type::Primitive(PrimitiveType::I32));
        self.builder.emit(TypedInstruction::Const {
            dest: index,
            value: Constant::I32(0),
        });

        // Create blocks
        let test_block = self.builder.new_block();
        let body_block = self.builder.new_block();
        let update_block = self.builder.new_block();
        let exit_block = self.builder.new_block();

        // Save loop targets for break/continue
        self.break_blocks.push(exit_block);
        self.continue_blocks.push(update_block);

        // If this is a labeled loop, register the continue block for the label
        if let Some(label_name) = label {
            self.label_continue_blocks.insert(label_name.to_string(), update_block);
        }

        // Jump to test
        self.builder.set_terminator(Terminator::Goto(test_block));
        self.builder.switch_to_block(test_block);

        // Test: index < keys.length
        let condition = self
            .builder
            .add_local("_for_in_cond".to_string(), Type::Primitive(PrimitiveType::Bool));
        self.builder.emit(TypedInstruction::BinOp {
            dest: condition,
            op: BinOpKind::Lt,
            left: index,
            right: keys_length,
            op_type: PrimitiveType::I32,
        });
        self.builder.set_terminator(Terminator::Branch {
            condition,
            then_block: body_block,
            else_block: exit_block,
        });

        // Body block
        self.builder.switch_to_block(body_block);

        // Get current key from keys array
        let current_key = self.builder.add_local("_key".to_string(), Type::Any);
        self.builder.emit(TypedInstruction::GetPropertyComputed {
            dest: current_key,
            object: keys,
            key: index,
        });

        // Bind the loop variable
        if let ForStatementLeft::VariableDeclaration(var_decl) = &for_in.left {
            if let Some(declarator) = var_decl.declarations.first() {
                if let BindingPatternKind::BindingIdentifier(ident) = &declarator.id.kind {
                    let name = ident.name.to_string();
                    self.variables.insert(name, current_key);
                }
            }
        } else if let ForStatementLeft::AssignmentTargetIdentifier(ident) = &for_in.left {
            let name = ident.name.to_string();
            self.variables.insert(name, current_key);
        }

        // Lower body
        self.lower_statement(&for_in.body)?;
        self.builder.set_terminator(Terminator::Goto(update_block));

        // Update block: increment index
        self.builder.switch_to_block(update_block);
        let one = self.builder.add_local("_one".to_string(), Type::Primitive(PrimitiveType::I32));
        self.builder.emit(TypedInstruction::Const {
            dest: one,
            value: Constant::I32(1),
        });
        let new_index = self
            .builder
            .add_local("_new_index".to_string(), Type::Primitive(PrimitiveType::I32));
        self.builder.emit(TypedInstruction::BinOp {
            dest: new_index,
            op: BinOpKind::Add,
            left: index,
            right: one,
            op_type: PrimitiveType::I32,
        });
        self.builder.emit(TypedInstruction::Copy {
            dest: index,
            src: new_index,
        });
        self.builder.set_terminator(Terminator::Goto(test_block));

        // Exit block
        self.builder.switch_to_block(exit_block);

        // Pop loop targets
        self.break_blocks.pop();
        self.continue_blocks.pop();

        Ok(None)
    }

    fn lower_for_of_statement(&mut self, for_of: &ForOfStatement) -> DxResult<Option<LocalId>> {
        self.lower_for_of_statement_with_label(for_of, None)
    }

    fn lower_for_of_statement_with_label(&mut self, for_of: &ForOfStatement, label: Option<&str>) -> DxResult<Option<LocalId>> {
        // for (var of iterable) body
        // Iterate over iterable values

        // Lower the iterable expression
        let mut ctx = self.make_expr_context();
        let iterable = lower_expr(&mut ctx, &for_of.right)?;

        // Get the length of the iterable
        let iterable_length = self
            .builder
            .add_local("_iterable_length".to_string(), Type::Primitive(PrimitiveType::I32));
        self.builder.emit(TypedInstruction::GetPropertyDynamic {
            dest: iterable_length,
            object: iterable,
            property: "length".to_string(),
        });

        // Create index variable
        let index = self
            .builder
            .add_local("_index".to_string(), Type::Primitive(PrimitiveType::I32));
        self.builder.emit(TypedInstruction::Const {
            dest: index,
            value: Constant::I32(0),
        });

        // Create blocks
        let test_block = self.builder.new_block();
        let body_block = self.builder.new_block();
        let update_block = self.builder.new_block();
        let exit_block = self.builder.new_block();

        // Save loop targets for break/continue
        self.break_blocks.push(exit_block);
        self.continue_blocks.push(update_block);

        // If this is a labeled loop, register the continue block for the label
        if let Some(label_name) = label {
            self.label_continue_blocks.insert(label_name.to_string(), update_block);
        }

        // Jump to test
        self.builder.set_terminator(Terminator::Goto(test_block));
        self.builder.switch_to_block(test_block);

        // Test: index < iterable.length
        let condition = self
            .builder
            .add_local("_for_of_cond".to_string(), Type::Primitive(PrimitiveType::Bool));
        self.builder.emit(TypedInstruction::BinOp {
            dest: condition,
            op: BinOpKind::Lt,
            left: index,
            right: iterable_length,
            op_type: PrimitiveType::I32,
        });
        self.builder.set_terminator(Terminator::Branch {
            condition,
            then_block: body_block,
            else_block: exit_block,
        });

        // Body block
        self.builder.switch_to_block(body_block);

        // Get current value and bind to loop variable
        let current_value = self.builder.add_local("_value".to_string(), Type::Any);
        self.builder.emit(TypedInstruction::GetPropertyComputed {
            dest: current_value,
            object: iterable,
            key: index,
        });

        // Bind the loop variable
        if let ForStatementLeft::VariableDeclaration(var_decl) = &for_of.left {
            if let Some(declarator) = var_decl.declarations.first() {
                if let BindingPatternKind::BindingIdentifier(ident) = &declarator.id.kind {
                    let name = ident.name.to_string();
                    self.variables.insert(name, current_value);
                }
            }
        }

        // Lower body
        self.lower_statement(&for_of.body)?;
        self.builder.set_terminator(Terminator::Goto(update_block));

        // Update block: increment index
        self.builder.switch_to_block(update_block);
        let one = self.builder.add_local("_one".to_string(), Type::Primitive(PrimitiveType::I32));
        self.builder.emit(TypedInstruction::Const {
            dest: one,
            value: Constant::I32(1),
        });
        let new_index = self
            .builder
            .add_local("_new_index".to_string(), Type::Primitive(PrimitiveType::I32));
        self.builder.emit(TypedInstruction::BinOp {
            dest: new_index,
            op: BinOpKind::Add,
            left: index,
            right: one,
            op_type: PrimitiveType::I32,
        });
        self.builder.emit(TypedInstruction::Copy {
            dest: index,
            src: new_index,
        });
        self.builder.set_terminator(Terminator::Goto(test_block));

        // Exit block
        self.builder.switch_to_block(exit_block);

        // Pop loop targets
        self.break_blocks.pop();
        self.continue_blocks.pop();

        Ok(None)
    }

    fn lower_while_statement(&mut self, while_stmt: &WhileStatement) -> DxResult<Option<LocalId>> {
        self.lower_while_statement_with_label(while_stmt, None)
    }

    fn lower_while_statement_with_label(&mut self, while_stmt: &WhileStatement, label: Option<&str>) -> DxResult<Option<LocalId>> {
        // while (test) body

        let test_block = self.builder.new_block();
        let body_block = self.builder.new_block();
        let exit_block = self.builder.new_block();

        // Save loop targets
        self.break_blocks.push(exit_block);
        self.continue_blocks.push(test_block);

        // If this is a labeled loop, register the continue block for the label
        if let Some(label_name) = label {
            self.label_continue_blocks.insert(label_name.to_string(), test_block);
        }

        // Jump to test
        self.builder.set_terminator(Terminator::Goto(test_block));
        self.builder.switch_to_block(test_block);

        // Lower test and generate conditional branch
        let mut ctx = self.make_expr_context();
        let test = lower_expr(&mut ctx, &while_stmt.test)?;

        // Branch based on condition
        self.builder.set_terminator(Terminator::Branch {
            condition: test,
            then_block: body_block,
            else_block: exit_block,
        });

        // Lower body
        self.builder.switch_to_block(body_block);
        self.lower_statement(&while_stmt.body)?;
        self.builder.set_terminator(Terminator::Goto(test_block));

        // Continue at exit
        self.builder.switch_to_block(exit_block);

        // Pop loop targets
        self.break_blocks.pop();
        self.continue_blocks.pop();

        Ok(None)
    }

    fn lower_do_while_statement(
        &mut self,
        do_while: &DoWhileStatement,
    ) -> DxResult<Option<LocalId>> {
        self.lower_do_while_statement_with_label(do_while, None)
    }

    fn lower_do_while_statement_with_label(
        &mut self,
        do_while: &DoWhileStatement,
        label: Option<&str>,
    ) -> DxResult<Option<LocalId>> {
        // do body while (test)

        let body_block = self.builder.new_block();
        let test_block = self.builder.new_block();
        let exit_block = self.builder.new_block();

        // Save loop targets
        self.break_blocks.push(exit_block);
        self.continue_blocks.push(test_block);

        // If this is a labeled loop, register the continue block for the label
        if let Some(label_name) = label {
            self.label_continue_blocks.insert(label_name.to_string(), test_block);
        }

        // Jump to body (execute at least once)
        self.builder.set_terminator(Terminator::Goto(body_block));
        self.builder.switch_to_block(body_block);

        // Lower body
        self.lower_statement(&do_while.body)?;
        self.builder.set_terminator(Terminator::Goto(test_block));

        // Lower test and generate conditional branch
        self.builder.switch_to_block(test_block);
        let mut ctx = self.make_expr_context();
        let test = lower_expr(&mut ctx, &do_while.test)?;

        // Branch: if true, go back to body; if false, exit
        self.builder.set_terminator(Terminator::Branch {
            condition: test,
            then_block: body_block,
            else_block: exit_block,
        });

        // Continue at exit
        self.builder.switch_to_block(exit_block);

        // Pop loop targets
        self.break_blocks.pop();
        self.continue_blocks.pop();

        Ok(None)
    }

    fn lower_try_statement(&mut self, try_stmt: &TryStatement) -> DxResult<Option<LocalId>> {
        // try { block } catch (e) { handler } finally { finalizer }

        // Create blocks for try, catch, finally, and exit
        let try_block = self.builder.new_block();
        let catch_block = if try_stmt.handler.is_some() {
            Some(self.builder.new_block())
        } else {
            None
        };
        let finally_block = if try_stmt.finalizer.is_some() {
            Some(self.builder.new_block())
        } else {
            None
        };
        let exit_block = self.builder.new_block();

        // Set up exception handler before entering try block
        self.builder.emit(TypedInstruction::SetupExceptionHandler {
            catch_block: catch_block.unwrap_or(exit_block),
            finally_block,
        });

        // Jump to try block
        self.builder.set_terminator(Terminator::Goto(try_block));
        self.builder.switch_to_block(try_block);

        // Lower the try block body
        self.lower_block_statement(&try_stmt.block)?;

        // Clear exception handler after try block
        self.builder.emit(TypedInstruction::ClearExceptionHandler);

        // Jump to finally (if present) or exit
        let after_try_target = finally_block.unwrap_or(exit_block);
        self.builder.set_terminator(Terminator::Goto(after_try_target));

        // Lower catch handler if present
        if let Some(handler) = &try_stmt.handler {
            if let Some(catch_block_id) = catch_block {
                self.builder.switch_to_block(catch_block_id);

                // Bind the exception to the catch parameter
                if let Some(param) = &handler.param {
                    if let BindingPatternKind::BindingIdentifier(ident) = &param.pattern.kind {
                        let name = ident.name.to_string();
                        let exception_local = self.builder.add_local(name.clone(), Type::Any);
                        self.builder.emit(TypedInstruction::GetException {
                            dest: exception_local,
                        });
                        self.variables.insert(name, exception_local);
                    }
                }

                // Lower catch body
                self.lower_block_statement(&handler.body)?;

                // Jump to finally (if present) or exit
                let after_catch_target = finally_block.unwrap_or(exit_block);
                self.builder.set_terminator(Terminator::Goto(after_catch_target));
            }
        }

        // Lower finally block if present
        if let Some(finalizer) = &try_stmt.finalizer {
            if let Some(finally_block_id) = finally_block {
                self.builder.switch_to_block(finally_block_id);

                // Lower finally body
                self.lower_block_statement(finalizer)?;

                // Jump to exit
                self.builder.set_terminator(Terminator::Goto(exit_block));
            }
        }

        // Continue at exit block
        self.builder.switch_to_block(exit_block);

        Ok(None)
    }

    fn lower_throw_statement(&mut self, throw: &ThrowStatement) -> DxResult<Option<LocalId>> {
        // throw expr
        let mut ctx = self.make_expr_context();
        let value = lower_expr(&mut ctx, &throw.argument)?;

        // Emit throw instruction
        self.builder.emit(TypedInstruction::Throw { value });

        // After throw, control flow is transferred to exception handler
        self.builder.set_terminator(Terminator::Unreachable);

        Ok(None)
    }

    fn lower_break_statement(&mut self, brk: &BreakStatement) -> DxResult<Option<LocalId>> {
        // break [label]
        let target = if let Some(label) = &brk.label {
            // Break to labeled statement
            let label_name = label.name.to_string();
            self.label_break_blocks.get(&label_name).copied()
        } else {
            // Break to innermost loop/switch
            self.break_blocks.last().copied()
        };

        if let Some(target) = target {
            self.builder.set_terminator(Terminator::Goto(target));
        } else {
            self.builder.set_terminator(Terminator::Unreachable);
        }
        Ok(None)
    }

    fn lower_continue_statement(&mut self, cont: &ContinueStatement) -> DxResult<Option<LocalId>> {
        // continue [label]
        let target = if let Some(label) = &cont.label {
            // Continue to labeled statement
            let label_name = label.name.to_string();
            self.label_continue_blocks.get(&label_name).copied()
        } else {
            // Continue to innermost loop
            self.continue_blocks.last().copied()
        };

        if let Some(target) = target {
            self.builder.set_terminator(Terminator::Goto(target));
        } else {
            self.builder.set_terminator(Terminator::Unreachable);
        }
        Ok(None)
    }

    fn lower_labeled_statement(&mut self, labeled: &LabeledStatement) -> DxResult<Option<LocalId>> {
        // label: statement
        let label = labeled.label.name.to_string();
        self.labels.push(label.clone());

        // Create an exit block for this label (for break)
        let exit_block = self.builder.new_block();
        self.label_break_blocks.insert(label.clone(), exit_block);

        // Check if the body is a loop statement - if so, we need to set up continue target
        // The continue target will be set by the loop lowering functions
        let is_loop = matches!(
            &labeled.body,
            Statement::ForStatement(_)
                | Statement::ForInStatement(_)
                | Statement::ForOfStatement(_)
                | Statement::WhileStatement(_)
                | Statement::DoWhileStatement(_)
        );

        // For loops, we'll create a placeholder continue block that will be updated
        // by the loop lowering. For non-loops, continue is not valid.
        if is_loop {
            // The actual continue block will be set by the loop lowering
            // We need to pass the label to the loop so it can register its continue block
            // For now, we'll use a different approach: check if we're in a labeled context
            // and register the continue block from within the loop lowering
        }

        // Lower the body - loop lowering will check for active labels and register continue blocks
        let result = self.lower_labeled_loop_body(&labeled.body, &label)?;

        // Jump to exit block if we haven't already terminated
        self.builder.set_terminator(Terminator::Goto(exit_block));
        self.builder.switch_to_block(exit_block);

        // Clean up
        self.labels.pop();
        self.label_break_blocks.remove(&label);
        self.label_continue_blocks.remove(&label);

        Ok(result)
    }

    /// Lower a statement that may be a loop inside a labeled statement
    /// This allows us to register the continue block for labeled loops
    fn lower_labeled_loop_body(&mut self, stmt: &Statement, label: &str) -> DxResult<Option<LocalId>> {
        match stmt {
            Statement::ForStatement(for_stmt) => self.lower_for_statement_with_label(for_stmt, Some(label)),
            Statement::ForInStatement(for_in) => self.lower_for_in_statement_with_label(for_in, Some(label)),
            Statement::ForOfStatement(for_of) => self.lower_for_of_statement_with_label(for_of, Some(label)),
            Statement::WhileStatement(while_stmt) => self.lower_while_statement_with_label(while_stmt, Some(label)),
            Statement::DoWhileStatement(do_while) => self.lower_do_while_statement_with_label(do_while, Some(label)),
            // For non-loop statements, just lower normally
            _ => self.lower_statement(stmt),
        }
    }

    pub fn finish(self) -> FunctionBuilder {
        self.builder
    }
}
