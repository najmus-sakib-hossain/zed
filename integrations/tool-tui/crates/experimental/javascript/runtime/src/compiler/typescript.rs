//! TypeScript Type System
//!
//! This module handles TypeScript type annotations, type checking,
//! and type inference to generate optimized code.

use crate::compiler::mir::{PrimitiveType, Type, TypeId};
use crate::error::DxResult;
use oxc_ast::ast::TSType;
use std::collections::HashMap;

/// TypeScript type analyzer
pub struct TypeScriptAnalyzer {
    /// Type aliases (type MyType = ...)
    type_aliases: HashMap<String, Type>,
    /// Interface definitions
    interfaces: HashMap<String, InterfaceType>,
    /// Generic type parameters in scope - reserved for generic type support
    #[allow(dead_code)]
    generic_params: Vec<String>,
    /// Object type definitions (for TSTypeLiteral)
    object_types: HashMap<TypeId, ObjectTypeDefinition>,
    /// Next available type ID
    next_type_id: u32,
}

/// Object type definition for TSTypeLiteral
#[derive(Debug, Clone)]
pub struct ObjectTypeDefinition {
    pub id: TypeId,
    pub properties: Vec<ObjectProperty>,
    pub index_signature: Option<IndexSignature>,
}

/// Property in an object type
#[derive(Debug, Clone)]
pub struct ObjectProperty {
    pub name: String,
    pub ty: Type,
    pub optional: bool,
    pub readonly: bool,
}

/// Index signature for object types (e.g., [key: string]: number)
#[derive(Debug, Clone)]
pub struct IndexSignature {
    pub key_type: Type,
    pub value_type: Type,
}

/// Enum value - can be auto-incremented, explicit number, or string
#[derive(Debug, Clone)]
pub enum EnumValue {
    Auto,
    Number(i64),
    String(String),
}

/// Compiled enum definition
#[derive(Debug, Clone)]
pub struct EnumDefinition {
    pub name: String,
    /// Forward mapping: member name -> value
    pub forward_mapping: HashMap<String, EnumValue>,
    /// Reverse mapping: numeric value -> member name (for numeric enums)
    pub reverse_mapping: HashMap<i64, String>,
}

/// Decorator information
#[derive(Debug, Clone)]
pub struct DecoratorInfo {
    pub name: String,
    pub arguments: Vec<DecoratorArgument>,
}

/// Decorator argument
#[derive(Debug, Clone)]
pub enum DecoratorArgument {
    String(String),
    Number(f64),
    Boolean(bool),
    Identifier(String),
    Array(Vec<DecoratorArgument>),
    Object(Vec<(String, DecoratorArgument)>),
}

/// Target of a decorator
#[derive(Debug, Clone)]
pub enum DecoratorTarget {
    Class(String),
    Method {
        class: String,
        method: String,
    },
    Property {
        class: String,
        property: String,
    },
    Parameter {
        class: String,
        method: String,
        index: usize,
    },
}

/// Applied decorator
#[derive(Debug, Clone)]
pub struct DecoratorApplication {
    pub decorator_name: String,
    pub arguments: Vec<DecoratorArgument>,
    pub target: DecoratorTarget,
}

/// Interface type definition
#[derive(Debug, Clone)]
pub struct InterfaceType {
    pub name: String,
    pub properties: Vec<PropertySignature>,
    pub methods: Vec<MethodSignature>,
    pub extends: Vec<String>,
}

/// Property signature in interface
#[derive(Debug, Clone)]
pub struct PropertySignature {
    pub name: String,
    pub ty: Type,
    pub optional: bool,
    pub readonly: bool,
}

/// Method signature in interface
#[derive(Debug, Clone)]
pub struct MethodSignature {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub return_type: Type,
    pub optional: bool,
}

impl TypeScriptAnalyzer {
    /// Create new TypeScript analyzer
    pub fn new() -> Self {
        Self {
            type_aliases: HashMap::new(),
            interfaces: HashMap::new(),
            generic_params: Vec::new(),
            object_types: HashMap::new(),
            next_type_id: 1, // Start at 1, 0 is reserved for unknown
        }
    }

    /// Allocate a new type ID
    fn allocate_type_id(&mut self) -> TypeId {
        let id = TypeId(self.next_type_id);
        self.next_type_id += 1;
        id
    }

    /// Convert TypeScript type annotation to MIR Type
    pub fn convert_ts_type(&mut self, ts_type: &TSType) -> DxResult<Type> {
        match ts_type {
            TSType::TSAnyKeyword(_) => Ok(Type::Any),
            TSType::TSBooleanKeyword(_) => Ok(Type::Primitive(PrimitiveType::Bool)),
            TSType::TSNumberKeyword(_) => Ok(Type::Primitive(PrimitiveType::F64)),
            TSType::TSStringKeyword(_) => Ok(Type::Primitive(PrimitiveType::String)),
            TSType::TSNullKeyword(_) => Ok(Type::Primitive(PrimitiveType::Null)),
            TSType::TSUndefinedKeyword(_) => Ok(Type::Primitive(PrimitiveType::Undefined)),
            TSType::TSVoidKeyword(_) => Ok(Type::Primitive(PrimitiveType::Undefined)),

            TSType::TSArrayType(array) => {
                let element_type = self.convert_ts_type(&array.element_type)?;
                Ok(Type::Array(Box::new(element_type)))
            }

            TSType::TSTupleType(_tuple) => {
                // Tuples not yet supported in MIR, use Array
                Ok(Type::Array(Box::new(Type::Any)))
            }

            TSType::TSUnionType(_union) => {
                // Unions not yet supported in MIR, use Any
                Ok(Type::Any)
            }

            TSType::TSTypeLiteral(literal) => {
                // Object type with specific properties - create proper object type
                self.convert_ts_type_literal(literal)
            }

            TSType::TSTypeReference(type_ref) => {
                // Look up type alias or interface
                let type_name = type_ref.type_name.to_string();

                // Check if it's a built-in generic type
                match type_name.as_str() {
                    "Array" => {
                        if let Some(type_params) = &type_ref.type_parameters {
                            if let Some(first_param) = type_params.params.first() {
                                let element_type = self.convert_ts_type(first_param)?;
                                return Ok(Type::Array(Box::new(element_type)));
                            }
                        }
                        Ok(Type::Array(Box::new(Type::Any)))
                    }
                    "Promise" => {
                        // Promise not yet in MIR Type, use Any
                        Ok(Type::Any)
                    }
                    _ => {
                        // Check type aliases
                        if let Some(alias_type) = self.type_aliases.get(&type_name) {
                            Ok(alias_type.clone())
                        } else {
                            // Unknown type, default to Any
                            Ok(Type::Any)
                        }
                    }
                }
            }

            TSType::TSFunctionType(func) => {
                // Parse function type with params and return type
                self.convert_ts_function_type(func)
            }

            _ => {
                // Unsupported TypeScript type, default to Any
                Ok(Type::Any)
            }
        }
    }

    /// Convert TSTypeLiteral to proper object type
    fn convert_ts_type_literal(&mut self, literal: &oxc_ast::ast::TSTypeLiteral) -> DxResult<Type> {
        let type_id = self.allocate_type_id();
        let mut properties = Vec::new();
        let mut index_signature = None;

        for member in &literal.members {
            match member {
                oxc_ast::ast::TSSignature::TSPropertySignature(prop) => {
                    let name = match &prop.key {
                        oxc_ast::ast::PropertyKey::StaticIdentifier(id) => id.name.to_string(),
                        oxc_ast::ast::PropertyKey::StringLiteral(s) => s.value.to_string(),
                        _ => continue, // Skip computed properties for now
                    };

                    let ty = if let Some(type_ann) = &prop.type_annotation {
                        self.convert_ts_type(&type_ann.type_annotation)?
                    } else {
                        Type::Any
                    };

                    properties.push(ObjectProperty {
                        name,
                        ty,
                        optional: prop.optional,
                        readonly: prop.readonly,
                    });
                }
                oxc_ast::ast::TSSignature::TSIndexSignature(idx) => {
                    // Handle index signature: [key: string]: ValueType
                    if let Some(param) = idx.parameters.first() {
                        let key_type =
                            self.convert_ts_type(&param.type_annotation.type_annotation)?;

                        let value_type =
                            self.convert_ts_type(&idx.type_annotation.type_annotation)?;

                        index_signature = Some(IndexSignature {
                            key_type,
                            value_type,
                        });
                    }
                }
                _ => {} // Skip method signatures for now
            }
        }

        // Store the object type definition
        self.object_types.insert(
            type_id,
            ObjectTypeDefinition {
                id: type_id,
                properties,
                index_signature,
            },
        );

        Ok(Type::Object(type_id))
    }

    /// Convert TSFunctionType to proper function type with params and return type
    fn convert_ts_function_type(&mut self, func: &oxc_ast::ast::TSFunctionType) -> DxResult<Type> {
        use crate::compiler::mir::FunctionSignature;

        // Parse parameter types
        let mut params = Vec::new();
        for param in &func.params.items {
            let param_type = if let Some(type_ann) = &param.pattern.type_annotation {
                self.convert_ts_type(&type_ann.type_annotation)?
            } else {
                Type::Any
            };
            params.push(param_type);
        }

        // Parse return type
        let return_type = self.convert_ts_type(&func.return_type.type_annotation)?;

        Ok(Type::Function(FunctionSignature {
            params,
            return_type: Box::new(return_type),
        }))
    }

    /// Get object type definition by ID
    pub fn get_object_type(&self, type_id: TypeId) -> Option<&ObjectTypeDefinition> {
        self.object_types.get(&type_id)
    }

    /// Generate enum object from TSEnumDeclaration
    /// TypeScript enums compile to objects with both name->value and value->name mappings
    pub fn generate_enum(&self, name: &str, members: &[(String, EnumValue)]) -> EnumDefinition {
        let mut forward_mapping = HashMap::new();
        let mut reverse_mapping = HashMap::new();
        let mut current_value: i64 = 0;

        for (member_name, value) in members {
            let actual_value = match value {
                EnumValue::Auto => {
                    let v = current_value;
                    current_value += 1;
                    EnumValue::Number(v)
                }
                EnumValue::Number(n) => {
                    current_value = *n + 1;
                    EnumValue::Number(*n)
                }
                EnumValue::String(s) => EnumValue::String(s.clone()),
            };

            forward_mapping.insert(member_name.clone(), actual_value.clone());

            // Reverse mapping only for numeric enums
            if let EnumValue::Number(n) = &actual_value {
                reverse_mapping.insert(*n, member_name.clone());
            }
        }

        EnumDefinition {
            name: name.to_string(),
            forward_mapping,
            reverse_mapping,
        }
    }

    /// Apply decorators to a class or method
    /// Decorators are applied in reverse order (bottom-up)
    pub fn apply_decorators(
        &self,
        decorators: &[DecoratorInfo],
        target: DecoratorTarget,
    ) -> Vec<DecoratorApplication> {
        decorators
            .iter()
            .rev()
            .map(|dec| DecoratorApplication {
                decorator_name: dec.name.clone(),
                arguments: dec.arguments.clone(),
                target: target.clone(),
            })
            .collect()
    }

    /// Register type alias
    pub fn register_type_alias(&mut self, name: String, ty: Type) {
        self.type_aliases.insert(name, ty);
    }

    /// Register interface
    pub fn register_interface(&mut self, interface: InterfaceType) {
        self.interfaces.insert(interface.name.clone(), interface);
    }

    /// Infer type from expression
    pub fn infer_type(&self, expr: &oxc_ast::ast::Expression) -> Type {
        use oxc_ast::ast::Expression;

        match expr {
            Expression::BooleanLiteral(_) => Type::Primitive(PrimitiveType::Bool),
            Expression::NumericLiteral(_) => Type::Primitive(PrimitiveType::F64),
            Expression::StringLiteral(_) => Type::Primitive(PrimitiveType::String),
            Expression::NullLiteral(_) => Type::Primitive(PrimitiveType::Null),
            Expression::Identifier(_) => Type::Any, // Would need symbol table
            Expression::ArrayExpression(_) => Type::Array(Box::new(Type::Any)),
            Expression::ObjectExpression(_) => Type::Object(TypeId(0)),
            Expression::ArrowFunctionExpression(_) => {
                use crate::compiler::mir::FunctionSignature;
                Type::Function(FunctionSignature {
                    params: vec![],
                    return_type: Box::new(Type::Any),
                })
            }
            Expression::FunctionExpression(_) => {
                use crate::compiler::mir::FunctionSignature;
                Type::Function(FunctionSignature {
                    params: vec![],
                    return_type: Box::new(Type::Any),
                })
            }
            _ => Type::Any,
        }
    }

    /// Type check: Verify that value_type is assignable to target_type
    pub fn is_assignable(&self, value_type: &Type, target_type: &Type) -> bool {
        Self::is_assignable_impl(value_type, target_type)
    }

    fn is_assignable_impl(value_type: &Type, target_type: &Type) -> bool {
        match (value_type, target_type) {
            // Any is assignable to and from anything
            (Type::Any, _) | (_, Type::Any) => true,

            // Exact type match
            (Type::Primitive(a), Type::Primitive(b)) => a == b,

            // Array covariance
            (Type::Array(a), Type::Array(b)) => Self::is_assignable_impl(a, b),

            // Union type handling - not yet supported in MIR
            // (removed for now)

            // Object types
            (Type::Object(_), Type::Object(_)) => true,

            _ => false,
        }
    }

    /// Generate optimized code based on type information
    pub fn optimize_for_type(&self, ty: &Type) -> OptimizationHint {
        match ty {
            Type::Primitive(PrimitiveType::I32) => OptimizationHint::UseI32,
            Type::Primitive(PrimitiveType::F64) => OptimizationHint::UseF64,
            Type::Primitive(PrimitiveType::Bool) => OptimizationHint::UseBool,
            Type::Array(_) => OptimizationHint::UseTypedArray,
            Type::Function { .. } => OptimizationHint::Monomorphize,
            _ => OptimizationHint::None,
        }
    }
}

/// Optimization hints derived from type information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationHint {
    None,
    UseI32,
    UseF64,
    UseBool,
    UseTypedArray,
    Monomorphize,
}

impl Default for TypeScriptAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_type_checking() {
        let analyzer = TypeScriptAnalyzer::new();

        let i32_type = Type::Primitive(PrimitiveType::I32);
        let f64_type = Type::Primitive(PrimitiveType::F64);
        let bool_type = Type::Primitive(PrimitiveType::Bool);

        // Strict type checking: primitives must match exactly
        assert!(analyzer.is_assignable(&i32_type, &i32_type));
        assert!(analyzer.is_assignable(&f64_type, &f64_type));
        assert!(!analyzer.is_assignable(&i32_type, &f64_type));
        assert!(!analyzer.is_assignable(&f64_type, &i32_type));
        assert!(!analyzer.is_assignable(&bool_type, &i32_type));
    }

    #[test]
    fn test_any_type() {
        let analyzer = TypeScriptAnalyzer::new();

        let any_type = Type::Any;
        let i32_type = Type::Primitive(PrimitiveType::I32);

        assert!(analyzer.is_assignable(&any_type, &i32_type));
        assert!(analyzer.is_assignable(&i32_type, &any_type));
    }
}
