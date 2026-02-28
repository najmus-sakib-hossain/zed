//! Function and Class Compilation
//!
//! Handles:
//! - Function declarations
//! - Function expressions
//! - Arrow functions
//! - Async functions
//! - Closures and captured variables
//! - Class declarations
//! - Constructors, methods, properties
//! - Inheritance and super calls
//! - Private fields

use crate::compiler::mir::*;
use crate::compiler::statements::StatementLowerer;
use crate::error::DxResult;
use oxc_ast::ast::*;
use std::collections::HashMap;

/// Async function state for state machine transformation
#[derive(Debug, Clone)]
pub struct AsyncFunctionState {
    /// Current state index
    pub state: u32,
    /// Promise to resolve/reject
    pub promise_id: LocalId,
    /// Captured variables
    pub captures: Vec<LocalId>,
    /// Resume points (await expressions)
    pub resume_points: Vec<BlockId>,
}

/// Function compilation context
pub struct FunctionCompiler {
    /// Next function ID
    next_id: u32,
    /// Compiled functions
    functions: Vec<TypedFunction>,
    /// Closure capture information - reserved for closure variable capture
    #[allow(dead_code)]
    captures: HashMap<String, LocalId>,
    /// Async function states
    async_states: HashMap<FunctionId, AsyncFunctionState>,
}

impl Default for FunctionCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl FunctionCompiler {
    pub fn new() -> Self {
        Self {
            next_id: 1, // 0 is reserved for main
            functions: Vec::new(),
            captures: HashMap::new(),
            async_states: HashMap::new(),
        }
    }

    /// Compile a function declaration
    pub fn compile_function_decl(&mut self, func: &Function) -> DxResult<FunctionId> {
        let func_id = FunctionId(self.next_id);
        self.next_id += 1;

        let name = func
            .id
            .as_ref()
            .map(|id| id.name.to_string())
            .unwrap_or_else(|| format!("__anon_{}", func_id.0));

        // Check if this is an async function
        if func.r#async {
            return self.compile_async_function(func_id, &name, func);
        }

        let mut builder = FunctionBuilder::new(func_id, name);

        // Add parameters
        for (i, param) in func.params.items.iter().enumerate() {
            let param_name = self.extract_param_name(param, i);
            builder.add_param(param_name, Type::Any);
        }

        // Set return type (for now, always f64)
        builder.return_type = Type::Primitive(PrimitiveType::F64);

        // Compile function body
        if let Some(body) = &func.body {
            let mut lowerer = StatementLowerer::new(builder);
            for stmt in &body.statements {
                lowerer.lower_statement(stmt)?;
            }
            builder = lowerer.finish();
        }

        let typed_func = builder.build();
        self.functions.push(typed_func);

        Ok(func_id)
    }

    /// Compile an async function
    ///
    /// Async functions are transformed into a state machine that:
    /// 1. Creates a Promise immediately
    /// 2. Starts executing the function body
    /// 3. At each await, suspends and stores state
    /// 4. Resumes when the awaited promise settles
    /// 5. Resolves/rejects the returned promise when complete
    fn compile_async_function(
        &mut self,
        func_id: FunctionId,
        name: &str,
        func: &Function,
    ) -> DxResult<FunctionId> {
        let async_name = format!("async_{}", name);
        let mut builder = FunctionBuilder::new(func_id, async_name);

        // Add parameters
        for (i, param) in func.params.items.iter().enumerate() {
            let param_name = self.extract_param_name(param, i);
            builder.add_param(param_name, Type::Any);
        }

        // Return type is always Promise for async functions
        builder.return_type = Type::Any; // Promise type

        // Create the promise that will be returned
        let promise_local = builder.add_local("__promise__".to_string(), Type::Any);
        builder.emit(TypedInstruction::CreatePromise {
            dest: promise_local,
        });

        // Create async state tracking
        let state = AsyncFunctionState {
            state: 0,
            promise_id: promise_local,
            captures: Vec::new(),
            resume_points: Vec::new(),
        };
        self.async_states.insert(func_id, state);

        // Compile function body with await handling
        if let Some(body) = &func.body {
            let mut lowerer = StatementLowerer::new(builder);
            lowerer.set_async_context(func_id, promise_local);

            for stmt in &body.statements {
                lowerer.lower_statement(stmt)?;
            }
            builder = lowerer.finish();
        }

        // Return the promise
        builder.set_terminator(Terminator::Return(Some(promise_local)));

        let typed_func = builder.build();
        self.functions.push(typed_func);

        Ok(func_id)
    }

    /// Compile an arrow function
    pub fn compile_arrow_function(
        &mut self,
        arrow: &ArrowFunctionExpression,
    ) -> DxResult<FunctionId> {
        let func_id = FunctionId(self.next_id);
        self.next_id += 1;

        let name = format!("__arrow_{}", func_id.0);

        // Check if this is an async arrow function
        if arrow.r#async {
            return self.compile_async_arrow_function(func_id, &name, arrow);
        }

        let mut builder = FunctionBuilder::new(func_id, name);

        // Add parameters
        for (i, param) in arrow.params.items.iter().enumerate() {
            let param_name = self.extract_param_name(param, i);
            builder.add_param(param_name, Type::Any);
        }

        // Compile body
        if arrow.expression {
            // Expression body: () => expr
            // We need to return the expression value
            // For now, just create empty function
        } else {
            // Statement body: () => { ... }
            // arrow.body is a FunctionBody, not Statement
            let body = &arrow.body;
            let mut lowerer = StatementLowerer::new(builder);
            for stmt in &body.statements {
                lowerer.lower_statement(stmt)?;
            }
            builder = lowerer.finish();
        }

        let typed_func = builder.build();
        self.functions.push(typed_func);

        Ok(func_id)
    }

    /// Compile an async arrow function
    fn compile_async_arrow_function(
        &mut self,
        func_id: FunctionId,
        name: &str,
        arrow: &ArrowFunctionExpression,
    ) -> DxResult<FunctionId> {
        let async_name = format!("async_{}", name);
        let mut builder = FunctionBuilder::new(func_id, async_name);

        // Add parameters
        for (i, param) in arrow.params.items.iter().enumerate() {
            let param_name = self.extract_param_name(param, i);
            builder.add_param(param_name, Type::Any);
        }

        // Return type is always Promise for async functions
        builder.return_type = Type::Any;

        // Create the promise that will be returned
        let promise_local = builder.add_local("__promise__".to_string(), Type::Any);
        builder.emit(TypedInstruction::CreatePromise {
            dest: promise_local,
        });

        // Create async state tracking
        let state = AsyncFunctionState {
            state: 0,
            promise_id: promise_local,
            captures: Vec::new(),
            resume_points: Vec::new(),
        };
        self.async_states.insert(func_id, state);

        // Compile body
        if arrow.expression {
            // Expression body: async () => expr
            // The expression result should resolve the promise
            // For now, just return the promise
        } else {
            // Statement body: async () => { ... }
            let body = &arrow.body;
            let mut lowerer = StatementLowerer::new(builder);
            lowerer.set_async_context(func_id, promise_local);

            for stmt in &body.statements {
                lowerer.lower_statement(stmt)?;
            }
            builder = lowerer.finish();
        }

        // Return the promise
        builder.set_terminator(Terminator::Return(Some(promise_local)));

        let typed_func = builder.build();
        self.functions.push(typed_func);

        Ok(func_id)
    }

    /// Extract parameter name from binding pattern
    fn extract_param_name(&self, param: &FormalParameter, index: usize) -> String {
        match &param.pattern.kind {
            BindingPatternKind::BindingIdentifier(ident) => ident.name.to_string(),
            _ => format!("__param_{}", index),
        }
    }

    /// Get all compiled functions
    pub fn get_functions(self) -> Vec<TypedFunction> {
        self.functions
    }

    /// Get async function state
    pub fn get_async_state(&self, func_id: FunctionId) -> Option<&AsyncFunctionState> {
        self.async_states.get(&func_id)
    }
}

/// Class compilation context
pub struct ClassCompiler {
    /// Next class ID
    next_id: u32,
    /// Compiled classes (as type layouts)
    classes: Vec<TypeLayout>,
    /// Class methods (as functions)
    methods: Vec<TypedFunction>,
    /// Static initialization functions (one per class with static blocks)
    static_initializers: Vec<TypedFunction>,
    /// Next function ID for methods
    next_func_id: u32,
}

impl ClassCompiler {
    pub fn new(starting_func_id: u32) -> Self {
        Self {
            next_id: 0,
            classes: Vec::new(),
            methods: Vec::new(),
            static_initializers: Vec::new(),
            next_func_id: starting_func_id,
        }
    }

    /// Compile a class declaration
    pub fn compile_class(&mut self, class: &Class) -> DxResult<TypeId> {
        let type_id = TypeId(self.next_id);
        self.next_id += 1;

        let class_name = class
            .id
            .as_ref()
            .map(|id| id.name.to_string())
            .unwrap_or_else(|| format!("__class_{}", type_id.0));

        let mut fields = Vec::new();
        let mut offset = 0u32;
        let mut static_block_stmts: Vec<&Statement> = Vec::new();

        // Process class elements
        for element in &class.body.body {
            match element {
                ClassElement::PropertyDefinition(prop) => {
                    let field_name = self.extract_property_name(&prop.key);
                    fields.push(FieldLayout {
                        name: field_name,
                        offset,
                        ty: Type::Any,
                    });
                    offset += 8; // 8 bytes per field (pointer size)
                }
                ClassElement::MethodDefinition(method) => {
                    // Compile method as a function
                    let method_name =
                        format!("{}::{}", class_name, self.extract_property_name(&method.key));
                    let func_id = FunctionId(self.next_func_id);
                    self.next_func_id += 1;

                    // method.value is already a Function
                    let func_value = &*method.value;
                    {
                        let mut builder = FunctionBuilder::new(func_id, method_name);

                        // Add 'this' parameter
                        builder.add_param("this".to_string(), Type::Object(type_id));

                        // Add other parameters
                        for (i, param) in func_value.params.items.iter().enumerate() {
                            let param_name = self.extract_param_name(param, i);
                            builder.add_param(param_name, Type::Any);
                        }

                        // Compile method body
                        if let Some(body) = &func_value.body {
                            let mut lowerer = StatementLowerer::new(builder);
                            for stmt in &body.statements {
                                lowerer.lower_statement(stmt)?;
                            }
                            builder = lowerer.finish();
                        }

                        self.methods.push(builder.build());
                    }
                }
                ClassElement::StaticBlock(static_block) => {
                    // Collect static block statements to compile later
                    // Static blocks are executed in source order during class initialization
                    for stmt in &static_block.body {
                        static_block_stmts.push(stmt);
                    }
                }
                _ => {
                    // Other element types (accessors, etc.)
                }
            }
        }

        // Compile static blocks if any exist
        if !static_block_stmts.is_empty() {
            let static_init_name = format!("{}::__static_init__", class_name);
            let func_id = FunctionId(self.next_func_id);
            self.next_func_id += 1;

            let mut builder = FunctionBuilder::new(func_id, static_init_name);

            // Add 'this' parameter bound to the class constructor
            builder.add_param("this".to_string(), Type::Object(type_id));

            // Compile all static block statements in source order
            let mut lowerer = StatementLowerer::new(builder);
            for stmt in static_block_stmts {
                lowerer.lower_statement(stmt)?;
            }
            builder = lowerer.finish();

            self.static_initializers.push(builder.build());
        }

        // Create type layout
        let layout = TypeLayout {
            size: offset,
            alignment: 8,
            fields,
        };

        self.classes.push(layout);

        Ok(type_id)
    }

    /// Extract property name from property key
    fn extract_property_name(&self, key: &PropertyKey) -> String {
        match key {
            PropertyKey::StaticIdentifier(ident) => ident.name.to_string(),
            PropertyKey::PrivateIdentifier(ident) => format!("#{}", ident.name),
            PropertyKey::NumericLiteral(lit) => lit.value.to_string(),
            PropertyKey::StringLiteral(lit) => lit.value.to_string(),
            _ => "__unknown__".to_string(),
        }
    }

    fn extract_param_name(&self, param: &FormalParameter, index: usize) -> String {
        match &param.pattern.kind {
            BindingPatternKind::BindingIdentifier(ident) => ident.name.to_string(),
            _ => format!("__param_{}", index),
        }
    }

    /// Get compiled classes, methods, and static initializers
    pub fn get_classes(self) -> (Vec<TypeLayout>, Vec<TypedFunction>, Vec<TypedFunction>) {
        (self.classes, self.methods, self.static_initializers)
    }
}
