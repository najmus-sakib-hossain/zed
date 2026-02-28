//! Typed Middle Intermediate Representation

use crate::compiler::type_solver::TypedAST;
use crate::error::DxResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Source span for tracking source locations with fine granularity
/// 
/// This tracks the exact position in source code including line and column
/// information for accurate error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SourceSpan {
    /// Start byte offset in source
    pub start: u32,
    /// End byte offset in source
    pub end: u32,
    /// Line number (1-indexed)
    pub line: u32,
    /// Column number (1-indexed)
    pub column: u32,
}

impl SourceSpan {
    /// Create a new source span
    pub fn new(start: u32, end: u32, line: u32, column: u32) -> Self {
        Self { start, end, line, column }
    }
    
    /// Create an empty/unknown span
    pub fn unknown() -> Self {
        Self { start: 0, end: 0, line: 0, column: 0 }
    }
    
    /// Check if this span has valid location information
    pub fn is_valid(&self) -> bool {
        self.line > 0 && self.column > 0
    }
}

/// A complete typed MIR program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedMIR {
    pub functions: Vec<TypedFunction>,
    pub globals: Vec<TypedGlobal>,
    pub entry_point: Option<FunctionId>,
    pub type_layouts: HashMap<TypeId, TypeLayout>,
    /// Source file name for this MIR
    #[serde(default)]
    pub source_file: String,
}

/// A function with all types resolved
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedFunction {
    pub id: FunctionId,
    pub name: String,
    pub params: Vec<TypedParam>,
    pub return_type: Type,
    pub blocks: Vec<TypedBlock>,
    pub locals: Vec<TypedLocal>,
    pub is_pure: bool,
    /// Source span for the function definition
    #[serde(default)]
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedParam {
    pub name: String,
    pub ty: Type,
    pub index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedLocal {
    pub name: String,
    pub ty: Type,
    pub index: u32,
}

/// A basic block with typed instructions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedBlock {
    pub id: BlockId,
    pub instructions: Vec<TypedInstruction>,
    pub terminator: Terminator,
    /// Source spans for each instruction (parallel to instructions vec)
    #[serde(default)]
    pub instruction_spans: Vec<SourceSpan>,
    /// Source span for the terminator
    #[serde(default)]
    pub terminator_span: SourceSpan,
}

/// A typed instruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypedInstruction {
    /// Constant value
    Const { dest: LocalId, value: Constant },

    /// Binary operation
    BinOp {
        dest: LocalId,
        op: BinOpKind,
        left: LocalId,
        right: LocalId,
        op_type: PrimitiveType,
    },

    /// Property access with known offset
    GetProperty {
        dest: LocalId,
        object: LocalId,
        offset: u32,
        prop_type: Type,
    },

    /// Property write
    SetProperty {
        object: LocalId,
        offset: u32,
        value: LocalId,
    },

    /// Function call
    Call {
        dest: Option<LocalId>,
        function: FunctionId,
        args: Vec<LocalId>,
    },

    /// Allocate object
    Allocate { dest: LocalId, layout: TypeId },

    /// Copy value
    Copy { dest: LocalId, src: LocalId },

    /// Create a function object (closure)
    CreateFunction {
        dest: LocalId,
        function_id: FunctionId,
        captured_vars: Vec<LocalId>,
        is_arrow: bool,
    },

    /// Call a function object (closure)
    CallFunction {
        dest: Option<LocalId>,
        callee: LocalId,
        args: Vec<LocalId>,
        this_arg: Option<LocalId>,
    },

    /// Get captured variable from closure environment
    GetCaptured { dest: LocalId, env_index: u32 },

    /// Set captured variable in closure environment
    SetCaptured { env_index: u32, value: LocalId },

    /// Get property by name (dynamic)
    GetPropertyDynamic {
        dest: LocalId,
        object: LocalId,
        property: String,
    },

    /// Set property by name (dynamic)
    SetPropertyDynamic {
        object: LocalId,
        property: String,
        value: LocalId,
    },

    /// Get property by computed key
    GetPropertyComputed {
        dest: LocalId,
        object: LocalId,
        key: LocalId,
    },

    /// Set property by computed key
    SetPropertyComputed {
        object: LocalId,
        key: LocalId,
        value: LocalId,
    },

    /// Create array
    CreateArray {
        dest: LocalId,
        elements: Vec<LocalId>,
    },

    /// Create object
    CreateObject {
        dest: LocalId,
        properties: Vec<(String, LocalId)>,
    },

    /// Throw an exception
    Throw { value: LocalId },

    /// Set up exception handler (try block entry)
    SetupExceptionHandler {
        catch_block: BlockId,
        finally_block: Option<BlockId>,
    },

    /// Clear exception handler (try block exit)
    ClearExceptionHandler,

    /// Get the caught exception value
    GetException { dest: LocalId },

    /// Get the current `this` binding
    GetThis { dest: LocalId },

    /// TypeOf operator - get the type string of a value
    TypeOf { dest: LocalId, operand: LocalId },

    /// Spread array elements into another array
    ArraySpread { dest: LocalId, source: LocalId },

    /// Push a value onto an array
    ArrayPush { array: LocalId, value: LocalId },

    /// Call with spread arguments
    CallWithSpread {
        dest: Option<LocalId>,
        callee: LocalId,
        args: LocalId, // Array of arguments
    },

    /// Generator yield - suspend execution and return value
    GeneratorYield {
        dest: LocalId,
        value: LocalId,
        resume_block: BlockId,
    },

    /// Generator return - complete the generator
    GeneratorReturn { value: Option<LocalId> },

    /// Create a generator object from a generator function
    CreateGenerator {
        dest: LocalId,
        function_id: FunctionId,
        captured_vars: Vec<LocalId>,
    },

    /// Get the next value from a generator
    GeneratorNext {
        dest: LocalId,
        generator: LocalId,
        send_value: Option<LocalId>,
    },

    /// Create a Promise
    CreatePromise { dest: LocalId },

    /// Resolve a Promise with a value
    PromiseResolve { promise: LocalId, value: LocalId },

    /// Reject a Promise with a reason
    PromiseReject { promise: LocalId, reason: LocalId },

    /// Await a Promise - suspend until resolved
    Await {
        dest: LocalId,
        promise: LocalId,
        resume_block: BlockId,
        reject_block: BlockId,
    },

    /// Create an async function wrapper
    CreateAsyncFunction {
        dest: LocalId,
        function_id: FunctionId,
        captured_vars: Vec<LocalId>,
    },

    /// Convert a value to boolean (for short-circuit evaluation)
    ToBool { dest: LocalId, src: LocalId },

    /// Check if a value is nullish (null or undefined)
    IsNullish { dest: LocalId, src: LocalId },

    /// Bitwise NOT operator (~)
    BitwiseNot { dest: LocalId, operand: LocalId },

    /// Bitwise AND operator (&)
    BitwiseAnd {
        dest: LocalId,
        left: LocalId,
        right: LocalId,
    },

    /// Bitwise OR operator (|)
    BitwiseOr {
        dest: LocalId,
        left: LocalId,
        right: LocalId,
    },

    /// Bitwise XOR operator (^)
    BitwiseXor {
        dest: LocalId,
        left: LocalId,
        right: LocalId,
    },

    /// Left shift operator (<<)
    ShiftLeft {
        dest: LocalId,
        left: LocalId,
        right: LocalId,
    },

    /// Right shift operator (>>)
    ShiftRight {
        dest: LocalId,
        left: LocalId,
        right: LocalId,
    },

    /// Unsigned right shift operator (>>>)
    ShiftRightUnsigned {
        dest: LocalId,
        left: LocalId,
        right: LocalId,
    },

    /// Exponentiation operator (**)
    Exponentiate {
        dest: LocalId,
        base: LocalId,
        exponent: LocalId,
    },

    /// Strict equality check (===)
    StrictEqual {
        dest: LocalId,
        left: LocalId,
        right: LocalId,
    },

    /// Strict inequality check (!==)
    StrictNotEqual {
        dest: LocalId,
        left: LocalId,
        right: LocalId,
    },

    /// Loose equality check (==)
    LooseEqual {
        dest: LocalId,
        left: LocalId,
        right: LocalId,
    },

    /// Loose inequality check (!=)
    LooseNotEqual {
        dest: LocalId,
        left: LocalId,
        right: LocalId,
    },

    /// instanceof operator
    InstanceOf {
        dest: LocalId,
        object: LocalId,
        constructor: LocalId,
    },

    /// in operator
    In {
        dest: LocalId,
        property: LocalId,
        object: LocalId,
    },

    /// delete operator
    Delete {
        dest: LocalId,
        object: LocalId,
        property: String,
    },

    /// delete with computed property
    DeleteComputed {
        dest: LocalId,
        object: LocalId,
        key: LocalId,
    },

    /// Create a class constructor function
    CreateClass {
        dest: LocalId,
        class_id: TypeId,
        constructor_id: Option<FunctionId>,
        super_class: Option<LocalId>,
    },

    /// Get the prototype of a class/constructor
    GetPrototype { dest: LocalId, constructor: LocalId },

    /// Set the prototype of an object
    SetPrototype { object: LocalId, prototype: LocalId },

    /// Call super constructor
    CallSuper {
        dest: Option<LocalId>,
        super_constructor: LocalId,
        args: Vec<LocalId>,
        this_arg: LocalId,
    },

    /// Call a method on the super class
    /// Requirements: 6.5 - super.method() calls parent class method with current this
    SuperMethodCall {
        dest: Option<LocalId>,
        super_class: LocalId,
        method_name: String,
        args: Vec<LocalId>,
        this_arg: LocalId,
    },

    /// Define a method on a prototype
    DefineMethod {
        prototype: LocalId,
        name: String,
        function_id: FunctionId,
        is_static: bool,
    },

    /// Define a getter on a prototype
    DefineGetter {
        prototype: LocalId,
        name: String,
        function_id: FunctionId,
        is_static: bool,
    },

    /// Define a setter on a prototype
    DefineSetter {
        prototype: LocalId,
        name: String,
        function_id: FunctionId,
        is_static: bool,
    },

    /// Dynamic import expression - import(specifier)
    /// Returns a Promise that resolves to the module namespace
    DynamicImport {
        dest: LocalId,
        specifier: LocalId,
    },

    /// Array slice operation for rest elements in destructuring
    /// Creates a new array from source[start_index..] 
    /// Requirements: 7.4 - rest elements in array destructuring
    ArraySliceFrom {
        dest: LocalId,
        source: LocalId,
        start_index: u32,
    },

    /// Object rest operation for rest properties in destructuring
    /// Creates a new object with all properties except the excluded keys
    /// Requirements: 7.5 - rest properties in object destructuring
    ObjectRest {
        dest: LocalId,
        source: LocalId,
        excluded_keys: Vec<String>,
    },

    /// Check if value is undefined (for default value handling)
    /// Requirements: 7.3 - default values in destructuring
    IsUndefined {
        dest: LocalId,
        src: LocalId,
    },

    /// Throw TypeError for destructuring null/undefined
    /// Requirements: 7.7 - destructuring null/undefined error
    ThrowDestructuringError {
        source: LocalId,
    },

    /// Build a template literal by concatenating quasis and expressions
    /// Requirements: 8.1, 8.2 - template literal interpolation with multiline support
    BuildTemplateLiteral {
        dest: LocalId,
        /// Static string parts (quasis) - always one more than expressions
        quasis: Vec<String>,
        /// Evaluated expression values
        expressions: Vec<LocalId>,
    },

    /// Call a tagged template function
    /// Requirements: 8.3 - tagged template invocation
    CallTaggedTemplate {
        dest: LocalId,
        /// The tag function to call
        tag: LocalId,
        /// Static string parts (quasis) as an array
        quasis: Vec<String>,
        /// Raw string parts (for String.raw)
        raw_quasis: Vec<String>,
        /// Evaluated expression values
        expressions: Vec<LocalId>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Terminator {
    Return(Option<LocalId>),
    Goto(BlockId),
    Branch {
        condition: LocalId,
        then_block: BlockId,
        else_block: BlockId,
    },
    Unreachable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Type {
    Primitive(PrimitiveType),
    Object(TypeId),
    Array(Box<Type>),
    Function(FunctionSignature),
    Any,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimitiveType {
    I32,
    I64,
    F64,
    Bool,
    String,
    Null,
    Undefined,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeLayout {
    pub size: u32,
    pub alignment: u32,
    pub fields: Vec<FieldLayout>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldLayout {
    pub name: String,
    pub offset: u32,
    pub ty: Type,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSignature {
    pub params: Vec<Type>,
    pub return_type: Box<Type>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Constant {
    I32(i32),
    I64(i64),
    F64(f64),
    Bool(bool),
    String(String),
    /// BigInt literal stored as string representation
    BigInt(String),
    Null,
    Undefined,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BinOpKind {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

// ID types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FunctionId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BlockId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LocalId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypeId(pub u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedGlobal {
    pub name: String,
    pub ty: Type,
}

/// Builder for a single function
pub struct FunctionBuilder {
    pub id: FunctionId,
    pub name: String,
    pub params: Vec<TypedParam>,
    pub return_type: Type,
    pub blocks: Vec<TypedBlock>,
    pub locals: Vec<TypedLocal>,
    pub current_block: BlockId,
    next_local_id: u32,
    next_block_id: u32,
}

impl FunctionBuilder {
    pub fn new(id: FunctionId, name: String) -> Self {
        Self {
            id,
            name,
            params: Vec::new(),
            return_type: Type::Primitive(PrimitiveType::F64),
            blocks: vec![TypedBlock {
                id: BlockId(0),
                instructions: Vec::new(),
                terminator: Terminator::Return(None),
                instruction_spans: Vec::new(),
                terminator_span: SourceSpan::unknown(),
            }],
            locals: Vec::new(),
            current_block: BlockId(0),
            next_local_id: 0,
            next_block_id: 1,
        }
    }

    pub fn add_local(&mut self, name: String, ty: Type) -> LocalId {
        let id = LocalId(self.next_local_id);
        self.next_local_id += 1;
        self.locals.push(TypedLocal {
            name,
            ty,
            index: id.0,
        });
        id
    }

    pub fn add_param(&mut self, name: String, ty: Type) -> LocalId {
        let id = LocalId(self.next_local_id);
        self.next_local_id += 1;
        self.params.push(TypedParam {
            name: name.clone(),
            ty: ty.clone(),
            index: id.0,
        });
        self.locals.push(TypedLocal {
            name,
            ty,
            index: id.0,
        });
        id
    }

    pub fn emit(&mut self, inst: TypedInstruction) {
        if let Some(block) = self.blocks.iter_mut().find(|b| b.id == self.current_block) {
            block.instructions.push(inst);
        }
    }

    pub fn new_block(&mut self) -> BlockId {
        let id = BlockId(self.next_block_id);
        self.next_block_id += 1;
        self.blocks.push(TypedBlock {
            id,
            instructions: Vec::new(),
            terminator: Terminator::Unreachable,
            instruction_spans: Vec::new(),
            terminator_span: SourceSpan::unknown(),
        });
        id
    }

    pub fn set_terminator(&mut self, term: Terminator) {
        if let Some(block) = self.blocks.iter_mut().find(|b| b.id == self.current_block) {
            block.terminator = term;
        }
    }

    pub fn switch_to_block(&mut self, id: BlockId) {
        self.current_block = id;
    }

    pub fn build(self) -> TypedFunction {
        TypedFunction {
            id: self.id,
            name: self.name,
            params: self.params,
            return_type: self.return_type,
            blocks: self.blocks,
            locals: self.locals,
            is_pure: false,
            span: SourceSpan::unknown(),
        }
    }
}

/// Lower typed AST to MIR
pub fn lower_to_mir(_typed_ast: &TypedAST) -> DxResult<TypedMIR> {
    // Create a simple entry point for now
    let entry_function = TypedFunction {
        id: FunctionId(0),
        name: "__dx_main__".to_string(),
        params: vec![],
        return_type: Type::Primitive(PrimitiveType::I64),
        blocks: vec![TypedBlock {
            id: BlockId(0),
            instructions: vec![],
            terminator: Terminator::Return(None),
            instruction_spans: Vec::new(),
            terminator_span: SourceSpan::unknown(),
        }],
        locals: vec![],
        is_pure: true,
        span: SourceSpan::unknown(),
    };

    Ok(TypedMIR {
        functions: vec![entry_function],
        globals: vec![],
        entry_point: Some(FunctionId(0)),
        type_layouts: HashMap::new(),
        source_file: String::new(),
    })
}
