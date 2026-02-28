//! Type inference and checking

use crate::compiler::parser::ParsedAST;
use crate::error::DxResult;
use std::collections::HashMap;

/// Solved type information
pub struct TypedAST {
    /// Type annotations for all expressions
    pub types: HashMap<NodeId, Type>,
    /// Function signatures
    pub functions: Vec<TypedFunction>,
    /// Type definitions (interfaces, type aliases)
    pub type_definitions: Vec<TypeDefinition>,
}

/// A node ID for tracking types
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NodeId(pub u32);

/// Type representation
#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    /// Primitive number (f64)
    Number,
    /// String
    String,
    /// Boolean
    Boolean,
    /// Null
    Null,
    /// Undefined
    Undefined,
    /// Object with known shape
    Object(ObjectType),
    /// Array with element type
    Array(Box<Type>),
    /// Function type
    Function(FunctionType),
    /// Union type
    Union(Vec<Type>),
    /// Any (dynamic)
    Any,
    /// Unknown
    Unknown,
    /// Never (unreachable)
    Never,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectType {
    pub properties: Vec<Property>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Property {
    pub name: String,
    pub ty: Type,
    pub optional: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionType {
    pub params: Vec<Type>,
    pub return_type: Box<Type>,
}

#[derive(Clone, Debug)]
pub struct TypedFunction {
    pub name: String,
    pub params: Vec<TypedParam>,
    pub return_type: Type,
    pub body_node_id: NodeId,
}

#[derive(Clone, Debug)]
pub struct TypedParam {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub struct TypeDefinition {
    pub name: String,
    pub fields: Vec<TypedField>,
}

#[derive(Clone, Debug)]
pub struct TypedField {
    pub name: String,
    pub ty: Type,
}

/// Type solver
pub struct TypeSolver {
    /// Type cache
    #[allow(dead_code)]
    cache: HashMap<String, Type>,
}

impl TypeSolver {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Solve types for the AST
    pub fn solve(&mut self, _ast: &ParsedAST) -> DxResult<TypedAST> {
        // For now, return a basic typed AST
        // Full implementation would walk the AST and infer types
        Ok(TypedAST {
            types: HashMap::new(),
            functions: Vec::new(),
            type_definitions: Vec::new(),
        })
    }
}

impl Default for TypeSolver {
    fn default() -> Self {
        Self::new()
    }
}
