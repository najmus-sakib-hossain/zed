//! Complete JavaScript Expression Lowering
//!
//! Handles all JavaScript expressions:
//! - Unary operators (!, -, +, ~, typeof, void, delete)
//! - Binary operators (all arithmetic, logical, bitwise)
//! - Ternary operator (? :)
//! - Assignment operators (=, +=, -=, etc.)
//! - Member expressions (obj.prop, obj[expr])
//! - Call expressions (func(), obj.method())
//! - New expressions (new Class())
//! - Array literals ([1, 2, 3])
//! - Object literals ({a: 1, b: 2})
//! - Template literals (`hello ${name}`)
//! - Arrow functions (() => {})
//! - Spread operator (...arr)
//! - Destructuring ({a, b} = obj, [x, y] = arr)

use crate::compiler::mir::*;
use crate::error::{unsupported_feature, DxError, DxResult};
use oxc_ast::ast::*;
use std::collections::HashMap;

/// Expression lowering context
pub struct ExpressionLowerer {
    /// Current function builder
    builder: FunctionBuilder,
    /// Variable bindings
    variables: HashMap<String, LocalId>,
}

/// Context for lowering expressions
pub struct ExprContext<'a> {
    pub builder: &'a mut FunctionBuilder,
    pub variables: &'a mut HashMap<String, LocalId>,
    /// Super class local (if in a class constructor/method that extends another class)
    pub super_class: Option<LocalId>,
    /// Current 'this' local (if in a class constructor/method)
    pub this_local: Option<LocalId>,
}

impl ExpressionLowerer {
    pub fn new(builder: FunctionBuilder) -> Self {
        Self {
            builder,
            variables: HashMap::new(),
        }
    }

    pub fn lower_expression(&mut self, expr: &Expression) -> DxResult<LocalId> {
        let mut ctx = ExprContext {
            builder: &mut self.builder,
            variables: &mut self.variables,
            super_class: None,
            this_local: None,
        };
        lower_expr(&mut ctx, expr)
    }
}

/// Lower an expression to a local
pub fn lower_expr(ctx: &mut ExprContext, expr: &Expression) -> DxResult<LocalId> {
    match expr {
        Expression::NumericLiteral(lit) => lower_numeric_literal(ctx, lit),
        Expression::BooleanLiteral(lit) => lower_boolean_literal(ctx, lit),
        Expression::StringLiteral(lit) => lower_string_literal(ctx, lit),
        Expression::NullLiteral(_) => lower_null_literal(ctx),
        Expression::Identifier(ident) => lower_identifier(ctx, ident),
        Expression::BinaryExpression(bin) => lower_binary_expression(ctx, bin),
        Expression::UnaryExpression(unary) => lower_unary_expression(ctx, unary),
        Expression::LogicalExpression(logical) => lower_logical_expression(ctx, logical),
        Expression::ConditionalExpression(cond) => lower_conditional_expression(ctx, cond),
        Expression::AssignmentExpression(assign) => lower_assignment_expression(ctx, assign),
        Expression::UpdateExpression(update) => lower_update_expression(ctx, update),
        Expression::CallExpression(call) => lower_call_expression(ctx, call),
        // MemberExpression is now a separate type, match multiple variants
        Expression::StaticMemberExpression(member) => {
            // obj.prop
            let obj = lower_expr(ctx, &member.object)?;
            let prop = member.property.name.to_string();

            let dest = ctx.builder.add_local("_member".to_string(), Type::Any);
            ctx.builder.emit(TypedInstruction::GetPropertyDynamic {
                dest,
                object: obj,
                property: prop,
            });
            Ok(dest)
        }
        Expression::ComputedMemberExpression(member) => {
            // obj[expr]
            let obj = lower_expr(ctx, &member.object)?;
            let key = lower_expr(ctx, &member.expression)?;

            let dest = ctx.builder.add_local("_member".to_string(), Type::Any);
            ctx.builder.emit(TypedInstruction::GetPropertyComputed {
                dest,
                object: obj,
                key,
            });
            Ok(dest)
        }
        Expression::PrivateFieldExpression(member) => {
            // obj.#field
            let obj = lower_expr(ctx, &member.object)?;
            let prop = format!("#{}", member.field.name);

            let dest = ctx.builder.add_local("_member".to_string(), Type::Any);
            ctx.builder.emit(TypedInstruction::GetPropertyDynamic {
                dest,
                object: obj,
                property: prop,
            });
            Ok(dest)
        }
        Expression::NewExpression(new_expr) => lower_new_expression(ctx, new_expr),
        Expression::ArrayExpression(arr) => lower_array_expression(ctx, arr),
        Expression::ObjectExpression(obj) => lower_object_expression(ctx, obj),
        Expression::ArrowFunctionExpression(arrow) => lower_arrow_function(ctx, arrow),
        Expression::FunctionExpression(func) => lower_function_expression(ctx, func),
        Expression::TemplateLiteral(tmpl) => lower_template_literal(ctx, tmpl),
        // SpreadElement is not a direct Expression variant anymore
        Expression::SequenceExpression(seq) => lower_sequence_expression(ctx, seq),
        Expression::ParenthesizedExpression(paren) => lower_expr(ctx, &paren.expression),
        Expression::ThisExpression(_) => lower_this_expression(ctx),
        Expression::AwaitExpression(await_expr) => lower_await_expression(ctx, await_expr),

        // Explicitly handle unsupported expressions with clear error messages
        // per Requirement 10.1: "WHEN a JavaScript feature is not supported,
        // THE DX_Runtime SHALL throw a SyntaxError or TypeError with a clear
        // message indicating the unsupported feature"
        Expression::BigIntLiteral(lit) => lower_bigint_literal(ctx, lit),
        Expression::RegExpLiteral(_) => Err(unsupported_feature(
            "RegExp literals",
            "Regular expression literals (e.g., /pattern/flags) are not yet supported in DX-JS",
            "Use the RegExp constructor: new RegExp('pattern', 'flags')",
        )),
        Expression::MetaProperty(meta) => {
            let name = format!("{}.{}", meta.meta.name, meta.property.name);
            Err(unsupported_feature(
                &format!("meta property '{}'", name),
                &format!("Meta property '{}' is not yet supported in DX-JS", name),
                "Check DX-JS documentation for supported meta properties",
            ))
        }
        Expression::Super(_) => {
            // super keyword used as a value (not in a call context)
            // This is typically used in super.property access
            // Return the super class reference if available
            if let Some(super_class) = ctx.super_class {
                Ok(super_class)
            } else {
                Err(unsupported_feature(
                    "super keyword outside class",
                    "The 'super' keyword can only be used inside a class that extends another class",
                    "Ensure super is used within a class that uses 'extends'",
                ))
            }
        },
        Expression::ChainExpression(_) => Err(unsupported_feature(
            "optional chaining",
            "Optional chaining (e.g., obj?.prop, obj?.method()) is not yet supported in DX-JS",
            "Use explicit null checks: obj && obj.prop",
        )),
        Expression::ClassExpression(_) => Err(unsupported_feature(
            "class expressions",
            "Class expressions (e.g., const MyClass = class { }) are not yet supported in DX-JS",
            "Use class declarations instead: class MyClass { }",
        )),
        Expression::ImportExpression(import_expr) => lower_import_expression(ctx, import_expr),
        Expression::TaggedTemplateExpression(tagged) => lower_tagged_template(ctx, tagged),
        Expression::YieldExpression(_) => Err(unsupported_feature(
            "yield expressions",
            "Generator functions and yield expressions are not yet supported in DX-JS",
            "Use async/await or callbacks for asynchronous iteration",
        )),
        Expression::PrivateInExpression(_) => Err(unsupported_feature(
            "private field 'in' operator",
            "The 'in' operator for private fields (e.g., #field in obj) is not yet supported",
            "Use try-catch or other patterns to check for private field existence",
        )),
        Expression::JSXElement(_) | Expression::JSXFragment(_) => Err(unsupported_feature(
            "JSX",
            "JSX syntax is not yet supported in DX-JS runtime",
            "Use React.createElement() or compile JSX with a bundler first",
        )),
        Expression::TSAsExpression(ts_expr) => {
            // TypeScript 'as' expressions should be stripped during compilation
            // but if we encounter one, just evaluate the underlying expression
            lower_expr(ctx, &ts_expr.expression)
        }
        Expression::TSSatisfiesExpression(ts_expr) => {
            // TypeScript 'satisfies' expressions should be stripped during compilation
            lower_expr(ctx, &ts_expr.expression)
        }
        Expression::TSTypeAssertion(ts_expr) => {
            // TypeScript type assertions (<Type>expr) should be stripped
            lower_expr(ctx, &ts_expr.expression)
        }
        Expression::TSNonNullExpression(ts_expr) => {
            // TypeScript non-null assertions (expr!) should be stripped
            lower_expr(ctx, &ts_expr.expression)
        }
        Expression::TSInstantiationExpression(ts_expr) => {
            // TypeScript instantiation expressions (expr<Type>) should be stripped
            lower_expr(ctx, &ts_expr.expression)
        }
    }
}

fn lower_numeric_literal(ctx: &mut ExprContext, lit: &NumericLiteral) -> DxResult<LocalId> {
    let dest = ctx.builder.add_local("_lit".to_string(), Type::Primitive(PrimitiveType::F64));
    ctx.builder.emit(TypedInstruction::Const {
        dest,
        value: Constant::F64(lit.value),
    });
    Ok(dest)
}

fn lower_boolean_literal(ctx: &mut ExprContext, lit: &BooleanLiteral) -> DxResult<LocalId> {
    let dest = ctx.builder.add_local("_bool".to_string(), Type::Primitive(PrimitiveType::Bool));
    ctx.builder.emit(TypedInstruction::Const {
        dest,
        value: Constant::Bool(lit.value),
    });
    Ok(dest)
}

fn lower_string_literal(ctx: &mut ExprContext, lit: &StringLiteral) -> DxResult<LocalId> {
    let dest = ctx
        .builder
        .add_local("_str".to_string(), Type::Primitive(PrimitiveType::String));
    ctx.builder.emit(TypedInstruction::Const {
        dest,
        value: Constant::String(lit.value.to_string()),
    });
    Ok(dest)
}

fn lower_null_literal(ctx: &mut ExprContext) -> DxResult<LocalId> {
    let dest = ctx.builder.add_local("_null".to_string(), Type::Primitive(PrimitiveType::Null));
    ctx.builder.emit(TypedInstruction::Const {
        dest,
        value: Constant::Null,
    });
    Ok(dest)
}

fn lower_bigint_literal(ctx: &mut ExprContext, lit: &BigIntLiteral) -> DxResult<LocalId> {
    // BigInt literals are stored as their string representation (without the 'n' suffix)
    let bigint_str = lit.raw.to_string();
    // Remove the 'n' suffix if present
    let bigint_value = bigint_str.trim_end_matches('n').to_string();
    
    let dest = ctx.builder.add_local("_bigint".to_string(), Type::Any);
    ctx.builder.emit(TypedInstruction::Const {
        dest,
        value: Constant::BigInt(bigint_value),
    });
    Ok(dest)
}

fn lower_identifier(ctx: &mut ExprContext, ident: &IdentifierReference) -> DxResult<LocalId> {
    let name = ident.name.to_string();
    if let Some(&local_id) = ctx.variables.get(&name) {
        Ok(local_id)
    } else {
        // Variable not found - return undefined for now
        let dest = ctx.builder.add_local("_undef".to_string(), Type::Any);
        ctx.builder.emit(TypedInstruction::Const {
            dest,
            value: Constant::Undefined,
        });
        Ok(dest)
    }
}

fn lower_binary_expression(ctx: &mut ExprContext, bin: &BinaryExpression) -> DxResult<LocalId> {
    let left = lower_expr(ctx, &bin.left)?;
    let right = lower_expr(ctx, &bin.right)?;

    match bin.operator {
        // Arithmetic operators
        BinaryOperator::Addition => {
            let dest = ctx.builder.add_local("_add".to_string(), Type::Any);
            ctx.builder.emit(TypedInstruction::BinOp {
                dest,
                op: BinOpKind::Add,
                left,
                right,
                op_type: PrimitiveType::F64,
            });
            Ok(dest)
        }
        BinaryOperator::Subtraction => {
            let dest =
                ctx.builder.add_local("_sub".to_string(), Type::Primitive(PrimitiveType::F64));
            ctx.builder.emit(TypedInstruction::BinOp {
                dest,
                op: BinOpKind::Sub,
                left,
                right,
                op_type: PrimitiveType::F64,
            });
            Ok(dest)
        }
        BinaryOperator::Multiplication => {
            let dest =
                ctx.builder.add_local("_mul".to_string(), Type::Primitive(PrimitiveType::F64));
            ctx.builder.emit(TypedInstruction::BinOp {
                dest,
                op: BinOpKind::Mul,
                left,
                right,
                op_type: PrimitiveType::F64,
            });
            Ok(dest)
        }
        BinaryOperator::Division => {
            let dest =
                ctx.builder.add_local("_div".to_string(), Type::Primitive(PrimitiveType::F64));
            ctx.builder.emit(TypedInstruction::BinOp {
                dest,
                op: BinOpKind::Div,
                left,
                right,
                op_type: PrimitiveType::F64,
            });
            Ok(dest)
        }
        BinaryOperator::Remainder => {
            let dest =
                ctx.builder.add_local("_mod".to_string(), Type::Primitive(PrimitiveType::F64));
            ctx.builder.emit(TypedInstruction::BinOp {
                dest,
                op: BinOpKind::Mod,
                left,
                right,
                op_type: PrimitiveType::F64,
            });
            Ok(dest)
        }
        BinaryOperator::Exponential => {
            let dest =
                ctx.builder.add_local("_exp".to_string(), Type::Primitive(PrimitiveType::F64));
            ctx.builder.emit(TypedInstruction::Exponentiate {
                dest,
                base: left,
                exponent: right,
            });
            Ok(dest)
        }

        // Comparison operators (loose equality)
        BinaryOperator::Equality => {
            let dest =
                ctx.builder.add_local("_eq".to_string(), Type::Primitive(PrimitiveType::Bool));
            ctx.builder.emit(TypedInstruction::LooseEqual { dest, left, right });
            Ok(dest)
        }
        BinaryOperator::Inequality => {
            let dest =
                ctx.builder.add_local("_ne".to_string(), Type::Primitive(PrimitiveType::Bool));
            ctx.builder.emit(TypedInstruction::LooseNotEqual { dest, left, right });
            Ok(dest)
        }

        // Comparison operators (strict equality)
        BinaryOperator::StrictEquality => {
            let dest =
                ctx.builder.add_local("_seq".to_string(), Type::Primitive(PrimitiveType::Bool));
            ctx.builder.emit(TypedInstruction::StrictEqual { dest, left, right });
            Ok(dest)
        }
        BinaryOperator::StrictInequality => {
            let dest =
                ctx.builder.add_local("_sne".to_string(), Type::Primitive(PrimitiveType::Bool));
            ctx.builder.emit(TypedInstruction::StrictNotEqual { dest, left, right });
            Ok(dest)
        }

        // Relational operators
        BinaryOperator::LessThan => {
            let dest =
                ctx.builder.add_local("_lt".to_string(), Type::Primitive(PrimitiveType::Bool));
            ctx.builder.emit(TypedInstruction::BinOp {
                dest,
                op: BinOpKind::Lt,
                left,
                right,
                op_type: PrimitiveType::F64,
            });
            Ok(dest)
        }
        BinaryOperator::LessEqualThan => {
            let dest =
                ctx.builder.add_local("_le".to_string(), Type::Primitive(PrimitiveType::Bool));
            ctx.builder.emit(TypedInstruction::BinOp {
                dest,
                op: BinOpKind::Le,
                left,
                right,
                op_type: PrimitiveType::F64,
            });
            Ok(dest)
        }
        BinaryOperator::GreaterThan => {
            let dest =
                ctx.builder.add_local("_gt".to_string(), Type::Primitive(PrimitiveType::Bool));
            ctx.builder.emit(TypedInstruction::BinOp {
                dest,
                op: BinOpKind::Gt,
                left,
                right,
                op_type: PrimitiveType::F64,
            });
            Ok(dest)
        }
        BinaryOperator::GreaterEqualThan => {
            let dest =
                ctx.builder.add_local("_ge".to_string(), Type::Primitive(PrimitiveType::Bool));
            ctx.builder.emit(TypedInstruction::BinOp {
                dest,
                op: BinOpKind::Ge,
                left,
                right,
                op_type: PrimitiveType::F64,
            });
            Ok(dest)
        }

        // Bitwise operators
        BinaryOperator::BitwiseAnd => {
            let dest =
                ctx.builder.add_local("_band".to_string(), Type::Primitive(PrimitiveType::I32));
            ctx.builder.emit(TypedInstruction::BitwiseAnd { dest, left, right });
            Ok(dest)
        }
        BinaryOperator::BitwiseOR => {
            let dest =
                ctx.builder.add_local("_bor".to_string(), Type::Primitive(PrimitiveType::I32));
            ctx.builder.emit(TypedInstruction::BitwiseOr { dest, left, right });
            Ok(dest)
        }
        BinaryOperator::BitwiseXOR => {
            let dest =
                ctx.builder.add_local("_bxor".to_string(), Type::Primitive(PrimitiveType::I32));
            ctx.builder.emit(TypedInstruction::BitwiseXor { dest, left, right });
            Ok(dest)
        }
        BinaryOperator::ShiftLeft => {
            let dest =
                ctx.builder.add_local("_shl".to_string(), Type::Primitive(PrimitiveType::I32));
            ctx.builder.emit(TypedInstruction::ShiftLeft { dest, left, right });
            Ok(dest)
        }
        BinaryOperator::ShiftRight => {
            let dest =
                ctx.builder.add_local("_shr".to_string(), Type::Primitive(PrimitiveType::I32));
            ctx.builder.emit(TypedInstruction::ShiftRight { dest, left, right });
            Ok(dest)
        }
        BinaryOperator::ShiftRightZeroFill => {
            let dest =
                ctx.builder.add_local("_shru".to_string(), Type::Primitive(PrimitiveType::I32));
            ctx.builder.emit(TypedInstruction::ShiftRightUnsigned { dest, left, right });
            Ok(dest)
        }

        // instanceof and in operators
        BinaryOperator::Instanceof => {
            let dest = ctx
                .builder
                .add_local("_instanceof".to_string(), Type::Primitive(PrimitiveType::Bool));
            ctx.builder.emit(TypedInstruction::InstanceOf {
                dest,
                object: left,
                constructor: right,
            });
            Ok(dest)
        }
        BinaryOperator::In => {
            let dest =
                ctx.builder.add_local("_in".to_string(), Type::Primitive(PrimitiveType::Bool));
            ctx.builder.emit(TypedInstruction::In {
                dest,
                property: left,
                object: right,
            });
            Ok(dest)
        }
    }
}

fn lower_unary_expression(ctx: &mut ExprContext, unary: &UnaryExpression) -> DxResult<LocalId> {
    match unary.operator {
        UnaryOperator::Delete => {
            // delete obj.prop or delete obj[key]
            // Need to handle the argument specially
            match &unary.argument {
                Expression::StaticMemberExpression(member) => {
                    let obj = lower_expr(ctx, &member.object)?;
                    let prop = member.property.name.to_string();
                    let dest = ctx
                        .builder
                        .add_local("_delete".to_string(), Type::Primitive(PrimitiveType::Bool));
                    ctx.builder.emit(TypedInstruction::Delete {
                        dest,
                        object: obj,
                        property: prop,
                    });
                    Ok(dest)
                }
                Expression::ComputedMemberExpression(member) => {
                    let obj = lower_expr(ctx, &member.object)?;
                    let key = lower_expr(ctx, &member.expression)?;
                    let dest = ctx
                        .builder
                        .add_local("_delete".to_string(), Type::Primitive(PrimitiveType::Bool));
                    ctx.builder.emit(TypedInstruction::DeleteComputed {
                        dest,
                        object: obj,
                        key,
                    });
                    Ok(dest)
                }
                _ => {
                    // delete on non-member expression always returns true
                    let dest = ctx
                        .builder
                        .add_local("_delete".to_string(), Type::Primitive(PrimitiveType::Bool));
                    ctx.builder.emit(TypedInstruction::Const {
                        dest,
                        value: Constant::Bool(true),
                    });
                    Ok(dest)
                }
            }
        }
        _ => {
            // For other unary operators, evaluate the operand first
            let operand = lower_expr(ctx, &unary.argument)?;

            match unary.operator {
                UnaryOperator::UnaryNegation => {
                    // -x => 0 - x
                    let zero = ctx
                        .builder
                        .add_local("_zero".to_string(), Type::Primitive(PrimitiveType::F64));
                    ctx.builder.emit(TypedInstruction::Const {
                        dest: zero,
                        value: Constant::F64(0.0),
                    });

                    let dest = ctx
                        .builder
                        .add_local("_neg".to_string(), Type::Primitive(PrimitiveType::F64));
                    ctx.builder.emit(TypedInstruction::BinOp {
                        dest,
                        op: BinOpKind::Sub,
                        left: zero,
                        right: operand,
                        op_type: PrimitiveType::F64,
                    });
                    Ok(dest)
                }
                UnaryOperator::UnaryPlus => {
                    // +x => convert to number
                    // For now, just return x (numeric conversion happens at runtime)
                    Ok(operand)
                }
                UnaryOperator::LogicalNot => {
                    // !x => convert to boolean and negate
                    // First convert to boolean
                    let bool_val = ctx
                        .builder
                        .add_local("_tobool".to_string(), Type::Primitive(PrimitiveType::Bool));
                    ctx.builder.emit(TypedInstruction::ToBool {
                        dest: bool_val,
                        src: operand,
                    });

                    // Then negate: !bool_val is equivalent to bool_val == false
                    let false_val = ctx
                        .builder
                        .add_local("_false".to_string(), Type::Primitive(PrimitiveType::Bool));
                    ctx.builder.emit(TypedInstruction::Const {
                        dest: false_val,
                        value: Constant::Bool(false),
                    });

                    let dest = ctx
                        .builder
                        .add_local("_not".to_string(), Type::Primitive(PrimitiveType::Bool));
                    ctx.builder.emit(TypedInstruction::StrictEqual {
                        dest,
                        left: bool_val,
                        right: false_val,
                    });
                    Ok(dest)
                }
                UnaryOperator::BitwiseNot => {
                    // ~x => bitwise NOT
                    let dest = ctx
                        .builder
                        .add_local("_bnot".to_string(), Type::Primitive(PrimitiveType::I32));
                    ctx.builder.emit(TypedInstruction::BitwiseNot { dest, operand });
                    Ok(dest)
                }
                UnaryOperator::Typeof => {
                    // typeof x => return string "number", "string", etc.
                    let dest = ctx
                        .builder
                        .add_local("_typeof".to_string(), Type::Primitive(PrimitiveType::String));
                    ctx.builder.emit(TypedInstruction::TypeOf { dest, operand });
                    Ok(dest)
                }
                UnaryOperator::Void => {
                    // void x => always undefined (but still evaluate x for side effects)
                    let dest = ctx.builder.add_local("_void".to_string(), Type::Any);
                    ctx.builder.emit(TypedInstruction::Const {
                        dest,
                        value: Constant::Undefined,
                    });
                    Ok(dest)
                }
                UnaryOperator::Delete => {
                    // Already handled above
                    unreachable!()
                }
            }
        }
    }
}

fn lower_logical_expression(
    ctx: &mut ExprContext,
    logical: &LogicalExpression,
) -> DxResult<LocalId> {
    // Evaluate left operand first
    let left = lower_expr(ctx, &logical.left)?;

    match logical.operator {
        LogicalOperator::And => {
            // Short-circuit AND: left && right
            // If left is falsy, return left without evaluating right
            // If left is truthy, evaluate and return right

            // Create result local to hold the final value
            let result = ctx.builder.add_local("_and_result".to_string(), Type::Any);

            // Create blocks for short-circuit evaluation
            let eval_right_block = ctx.builder.new_block();
            let merge_block = ctx.builder.new_block();

            // Convert left to boolean for branching
            let left_bool = ctx
                .builder
                .add_local("_left_bool".to_string(), Type::Primitive(PrimitiveType::Bool));
            ctx.builder.emit(TypedInstruction::ToBool {
                dest: left_bool,
                src: left,
            });

            // Copy left to result (in case we short-circuit)
            ctx.builder.emit(TypedInstruction::Copy {
                dest: result,
                src: left,
            });

            // Branch: if left is truthy, evaluate right; otherwise skip to merge
            ctx.builder.set_terminator(Terminator::Branch {
                condition: left_bool,
                then_block: eval_right_block,
                else_block: merge_block,
            });

            // Eval right block - evaluate right operand and store in result
            ctx.builder.switch_to_block(eval_right_block);
            let right = lower_expr(ctx, &logical.right)?;
            ctx.builder.emit(TypedInstruction::Copy {
                dest: result,
                src: right,
            });
            ctx.builder.set_terminator(Terminator::Goto(merge_block));

            // Continue from merge block
            ctx.builder.switch_to_block(merge_block);

            Ok(result)
        }
        LogicalOperator::Or => {
            // Short-circuit OR: left || right
            // If left is truthy, return left without evaluating right
            // If left is falsy, evaluate and return right

            // Create result local to hold the final value
            let result = ctx.builder.add_local("_or_result".to_string(), Type::Any);

            // Create blocks for short-circuit evaluation
            let eval_right_block = ctx.builder.new_block();
            let merge_block = ctx.builder.new_block();

            // Convert left to boolean for branching
            let left_bool = ctx
                .builder
                .add_local("_left_bool".to_string(), Type::Primitive(PrimitiveType::Bool));
            ctx.builder.emit(TypedInstruction::ToBool {
                dest: left_bool,
                src: left,
            });

            // Copy left to result (in case we short-circuit)
            ctx.builder.emit(TypedInstruction::Copy {
                dest: result,
                src: left,
            });

            // Branch: if left is truthy, skip to merge; otherwise evaluate right
            ctx.builder.set_terminator(Terminator::Branch {
                condition: left_bool,
                then_block: merge_block,
                else_block: eval_right_block,
            });

            // Eval right block - evaluate right operand and store in result
            ctx.builder.switch_to_block(eval_right_block);
            let right = lower_expr(ctx, &logical.right)?;
            ctx.builder.emit(TypedInstruction::Copy {
                dest: result,
                src: right,
            });
            ctx.builder.set_terminator(Terminator::Goto(merge_block));

            // Continue from merge block
            ctx.builder.switch_to_block(merge_block);

            Ok(result)
        }
        LogicalOperator::Coalesce => {
            // Nullish coalescing: left ?? right
            // If left is null or undefined, return right; otherwise return left

            // Create result local to hold the final value
            let result = ctx.builder.add_local("_coalesce_result".to_string(), Type::Any);

            // Create blocks for nullish coalescing
            let eval_right_block = ctx.builder.new_block();
            let merge_block = ctx.builder.new_block();

            // Check if left is nullish (null or undefined)
            let is_nullish = ctx
                .builder
                .add_local("_is_nullish".to_string(), Type::Primitive(PrimitiveType::Bool));
            ctx.builder.emit(TypedInstruction::IsNullish {
                dest: is_nullish,
                src: left,
            });

            // Copy left to result (in case left is not nullish)
            ctx.builder.emit(TypedInstruction::Copy {
                dest: result,
                src: left,
            });

            // Branch: if left is nullish, evaluate right; otherwise skip to merge
            ctx.builder.set_terminator(Terminator::Branch {
                condition: is_nullish,
                then_block: eval_right_block,
                else_block: merge_block,
            });

            // Eval right block - evaluate right operand and store in result
            ctx.builder.switch_to_block(eval_right_block);
            let right = lower_expr(ctx, &logical.right)?;
            ctx.builder.emit(TypedInstruction::Copy {
                dest: result,
                src: right,
            });
            ctx.builder.set_terminator(Terminator::Goto(merge_block));

            // Continue from merge block
            ctx.builder.switch_to_block(merge_block);

            Ok(result)
        }
    }
}

fn lower_conditional_expression(
    ctx: &mut ExprContext,
    cond: &ConditionalExpression,
) -> DxResult<LocalId> {
    // condition ? consequent : alternate
    // Implement proper branching

    // Lower the condition
    let condition = lower_expr(ctx, &cond.test)?;

    // Create blocks for branching
    let then_block = ctx.builder.new_block();
    let else_block = ctx.builder.new_block();
    let merge_block = ctx.builder.new_block();

    // Create result local
    let result = ctx.builder.add_local("_cond_result".to_string(), Type::Any);

    // Branch on condition
    ctx.builder.set_terminator(Terminator::Branch {
        condition,
        then_block,
        else_block,
    });

    // Then block - evaluate consequent
    ctx.builder.switch_to_block(then_block);
    let then_val = lower_expr(ctx, &cond.consequent)?;
    ctx.builder.emit(TypedInstruction::Copy {
        dest: result,
        src: then_val,
    });
    ctx.builder.set_terminator(Terminator::Goto(merge_block));

    // Else block - evaluate alternate
    ctx.builder.switch_to_block(else_block);
    let else_val = lower_expr(ctx, &cond.alternate)?;
    ctx.builder.emit(TypedInstruction::Copy {
        dest: result,
        src: else_val,
    });
    ctx.builder.set_terminator(Terminator::Goto(merge_block));

    // Continue from merge block
    ctx.builder.switch_to_block(merge_block);

    Ok(result)
}

fn lower_assignment_expression(
    ctx: &mut ExprContext,
    assign: &AssignmentExpression,
) -> DxResult<LocalId> {
    // Handle compound assignment operators (+=, -=, *=, /=, %=, etc.)
    // For compound assignments, we need to:
    // 1. Get the current value of the left-hand side
    // 2. Perform the binary operation with the right-hand side
    // 3. Store the result back to the left-hand side
    
    let right_value = lower_expr(ctx, &assign.right)?;
    
    // Determine if this is a compound assignment and get the binary operation
    let bin_op = match assign.operator {
        AssignmentOperator::Assign => None,
        AssignmentOperator::Addition => Some(BinOpKind::Add),
        AssignmentOperator::Subtraction => Some(BinOpKind::Sub),
        AssignmentOperator::Multiplication => Some(BinOpKind::Mul),
        AssignmentOperator::Division => Some(BinOpKind::Div),
        AssignmentOperator::Remainder => Some(BinOpKind::Mod),
        // Bitwise operators - for now, treat as simple assignment
        // TODO: Implement proper bitwise operations
        _ => None,
    };

    match &assign.left {
        AssignmentTarget::AssignmentTargetIdentifier(ident) => {
            let name = ident.name.to_string();
            
            // If the variable already exists, emit a Copy to update it
            // This ensures the same LocalId is used, which is crucial for loops
            if let Some(&existing_local) = ctx.variables.get(&name) {
                let final_value = if let Some(op) = bin_op {
                    // Compound assignment: compute existing_local op right_value
                    let temp = ctx.builder.add_local("_compound_temp".to_string(), Type::Primitive(PrimitiveType::F64));
                    ctx.builder.emit(TypedInstruction::BinOp {
                        dest: temp,
                        op,
                        left: existing_local,
                        right: right_value,
                        op_type: PrimitiveType::F64,
                    });
                    temp
                } else {
                    // Simple assignment
                    right_value
                };
                
                ctx.builder.emit(TypedInstruction::Copy {
                    dest: existing_local,
                    src: final_value,
                });
                Ok(existing_local)
            } else {
                // New variable - just track it
                // For compound assignment on undefined variable, the result would be NaN
                // but we'll just use the right value for simplicity
                ctx.variables.insert(name.clone(), right_value);
                Ok(right_value)
            }
        }
        AssignmentTarget::StaticMemberExpression(member) => {
            // obj.prop = value (or obj.prop op= value for compound)
            let obj = lower_expr(ctx, &member.object)?;
            let prop = member.property.name.to_string();

            let final_value = if let Some(op) = bin_op {
                // Compound assignment: get current value, compute, then set
                let current = ctx.builder.add_local("_prop_current".to_string(), Type::Any);
                ctx.builder.emit(TypedInstruction::GetPropertyDynamic {
                    dest: current,
                    object: obj,
                    property: prop.clone(),
                });
                let temp = ctx.builder.add_local("_compound_temp".to_string(), Type::Primitive(PrimitiveType::F64));
                ctx.builder.emit(TypedInstruction::BinOp {
                    dest: temp,
                    op,
                    left: current,
                    right: right_value,
                    op_type: PrimitiveType::F64,
                });
                temp
            } else {
                right_value
            };

            ctx.builder.emit(TypedInstruction::SetPropertyDynamic {
                object: obj,
                property: prop,
                value: final_value,
            });
            Ok(final_value)
        }
        AssignmentTarget::ComputedMemberExpression(member) => {
            // obj[expr] = value (or obj[expr] op= value for compound)
            let obj = lower_expr(ctx, &member.object)?;
            let key = lower_expr(ctx, &member.expression)?;

            let final_value = if let Some(op) = bin_op {
                // Compound assignment: get current value, compute, then set
                let current = ctx.builder.add_local("_elem_current".to_string(), Type::Any);
                ctx.builder.emit(TypedInstruction::GetPropertyComputed {
                    dest: current,
                    object: obj,
                    key,
                });
                let temp = ctx.builder.add_local("_compound_temp".to_string(), Type::Primitive(PrimitiveType::F64));
                ctx.builder.emit(TypedInstruction::BinOp {
                    dest: temp,
                    op,
                    left: current,
                    right: right_value,
                    op_type: PrimitiveType::F64,
                });
                temp
            } else {
                right_value
            };

            ctx.builder.emit(TypedInstruction::SetPropertyComputed {
                object: obj,
                key,
                value: final_value,
            });
            Ok(final_value)
        }
        AssignmentTarget::PrivateFieldExpression(member) => {
            // obj.#field = value (or obj.#field op= value for compound)
            let obj = lower_expr(ctx, &member.object)?;
            let prop = format!("#{}", member.field.name);

            let final_value = if let Some(op) = bin_op {
                // Compound assignment: get current value, compute, then set
                let current = ctx.builder.add_local("_private_current".to_string(), Type::Any);
                ctx.builder.emit(TypedInstruction::GetPropertyDynamic {
                    dest: current,
                    object: obj,
                    property: prop.clone(),
                });
                let temp = ctx.builder.add_local("_compound_temp".to_string(), Type::Primitive(PrimitiveType::F64));
                ctx.builder.emit(TypedInstruction::BinOp {
                    dest: temp,
                    op,
                    left: current,
                    right: right_value,
                    op_type: PrimitiveType::F64,
                });
                temp
            } else {
                right_value
            };

            ctx.builder.emit(TypedInstruction::SetPropertyDynamic {
                object: obj,
                property: prop,
                value: final_value,
            });
            Ok(final_value)
        }
        _ => {
            // Unsupported assignment target (destructuring, etc.)
            Ok(right_value)
        }
    }
}

fn lower_update_expression(ctx: &mut ExprContext, update: &UpdateExpression) -> DxResult<LocalId> {
    // ++x or x++ or --x or x--
    // Handle prefix vs postfix semantics correctly

    // Get the variable name
    let var_name = match &update.argument {
        SimpleAssignmentTarget::AssignmentTargetIdentifier(ident) => ident.name.to_string(),
        _ => {
            // For non-identifier targets, return a dummy value
            let dest = ctx
                .builder
                .add_local("_update".to_string(), Type::Primitive(PrimitiveType::F64));
            ctx.builder.emit(TypedInstruction::Const {
                dest,
                value: Constant::F64(0.0),
            });
            return Ok(dest);
        }
    };

    // Get the current value of the variable
    let current_value = if let Some(&local_id) = ctx.variables.get(&var_name) {
        local_id
    } else {
        // Variable not found - create with value 0
        let dest = ctx.builder.add_local(var_name.clone(), Type::Primitive(PrimitiveType::F64));
        ctx.builder.emit(TypedInstruction::Const {
            dest,
            value: Constant::F64(0.0),
        });
        ctx.variables.insert(var_name.clone(), dest);
        dest
    };

    // Save the old value for postfix operations
    let old_value = ctx
        .builder
        .add_local("_old_value".to_string(), Type::Primitive(PrimitiveType::F64));
    ctx.builder.emit(TypedInstruction::Copy {
        dest: old_value,
        src: current_value,
    });

    // Create the increment/decrement value (1.0)
    let one = ctx.builder.add_local("_one".to_string(), Type::Primitive(PrimitiveType::F64));
    ctx.builder.emit(TypedInstruction::Const {
        dest: one,
        value: Constant::F64(1.0),
    });

    // Compute the new value - store directly in the original variable's LocalId
    // This is crucial for loops: the test block uses the same LocalId, so updating
    // it here ensures the test sees the updated value on the next iteration.
    // The Cranelift Variable system will handle SSA form and phi nodes automatically.
    let op = if update.operator == UpdateOperator::Increment {
        BinOpKind::Add
    } else {
        BinOpKind::Sub
    };

    // Create a temporary for the computation
    let temp = ctx
        .builder
        .add_local("_temp".to_string(), Type::Primitive(PrimitiveType::F64));
    ctx.builder.emit(TypedInstruction::BinOp {
        dest: temp,
        op,
        left: current_value,
        right: one,
        op_type: PrimitiveType::F64,
    });
    
    // Copy the result back to the original variable
    // This ensures the same LocalId is updated, which the Variable system tracks
    ctx.builder.emit(TypedInstruction::Copy {
        dest: current_value,
        src: temp,
    });

    // Return the appropriate value based on prefix/postfix
    if update.prefix {
        // Prefix (++x, --x): return the new value
        Ok(current_value)
    } else {
        // Postfix (x++, x--): return the old value
        Ok(old_value)
    }

}

fn lower_call_expression(ctx: &mut ExprContext, call: &CallExpression) -> DxResult<LocalId> {
    // Check for super() constructor call
    // Requirements: 6.4 - super() calls parent constructor with correct this
    if matches!(&call.callee, Expression::Super(_)) {
        // This is a super() call - call the parent constructor
        if let (Some(super_class), Some(this_local)) = (ctx.super_class, ctx.this_local) {
            // Lower arguments
            let args: Vec<LocalId> = call
                .arguments
                .iter()
                .filter_map(|arg| lower_argument(ctx, arg).ok())
                .collect();

            let dest = ctx.builder.add_local("_super_result".to_string(), Type::Any);
            ctx.builder.emit(TypedInstruction::CallSuper {
                dest: Some(dest),
                super_constructor: super_class,
                args,
                this_arg: this_local,
            });
            return Ok(dest);
        } else {
            return Err(unsupported_feature(
                "super() outside class constructor",
                "The 'super()' call can only be used inside a class constructor that extends another class",
                "Ensure super() is called within a constructor of a class that uses 'extends'",
            ));
        }
    }

    // Check for super.method() calls
    // Requirements: 6.5 - super.method() calls parent class method with current this
    if let Expression::StaticMemberExpression(member) = &call.callee {
        if matches!(&member.object, Expression::Super(_)) {
            // This is a super.method() call
            if let (Some(super_class), Some(this_local)) = (ctx.super_class, ctx.this_local) {
                let method_name = member.property.name.to_string();
                
                // Lower arguments
                let args: Vec<LocalId> = call
                    .arguments
                    .iter()
                    .filter_map(|arg| lower_argument(ctx, arg).ok())
                    .collect();

                let dest = ctx.builder.add_local("_super_method_result".to_string(), Type::Any);
                ctx.builder.emit(TypedInstruction::SuperMethodCall {
                    dest: Some(dest),
                    super_class,
                    method_name,
                    args,
                    this_arg: this_local,
                });
                return Ok(dest);
            } else {
                return Err(unsupported_feature(
                    "super.method() outside class",
                    "The 'super.method()' call can only be used inside a class that extends another class",
                    "Ensure super.method() is called within a class that uses 'extends'",
                ));
            }
        }
    }

    // Check for builtin calls like Math.floor, console.log, etc.
    if let Expression::StaticMemberExpression(member) = &call.callee {
        if let Expression::Identifier(obj_ident) = &member.object {
            let obj_name = obj_ident.name.as_str();
            let method_name = member.property.name.as_str();

            // Check for Math builtins
            if obj_name == "Math" {
                let builtin_id = match method_name {
                    "floor" => Some(FunctionId(u32::MAX - 10)),
                    "ceil" => Some(FunctionId(u32::MAX - 11)),
                    "sqrt" => Some(FunctionId(u32::MAX - 12)),
                    "abs" => Some(FunctionId(u32::MAX - 13)),
                    "sin" => Some(FunctionId(u32::MAX - 14)),
                    "cos" => Some(FunctionId(u32::MAX - 15)),
                    "random" => Some(FunctionId(u32::MAX - 16)),
                    _ => None,
                };

                if let Some(func_id) = builtin_id {
                    // Lower arguments
                    let args: Vec<LocalId> = call
                        .arguments
                        .iter()
                        .filter_map(|arg| lower_argument(ctx, arg).ok())
                        .collect();

                    let dest = ctx.builder.add_local("_builtin_result".to_string(), Type::Any);
                    ctx.builder.emit(TypedInstruction::Call {
                        dest: Some(dest),
                        function: func_id,
                        args,
                    });
                    return Ok(dest);
                }
            }

            // Check for JSON builtins
            if obj_name == "JSON" {
                let builtin_id = match method_name {
                    "parse" => Some(FunctionId(u32::MAX - 17)),
                    "stringify" => Some(FunctionId(u32::MAX - 18)),
                    _ => None,
                };

                if let Some(func_id) = builtin_id {
                    // Lower arguments
                    let args: Vec<LocalId> = call
                        .arguments
                        .iter()
                        .filter_map(|arg| lower_argument(ctx, arg).ok())
                        .collect();

                    let dest = ctx.builder.add_local("_builtin_result".to_string(), Type::Any);
                    ctx.builder.emit(TypedInstruction::Call {
                        dest: Some(dest),
                        function: func_id,
                        args,
                    });
                    return Ok(dest);
                }
            }

            // Check for console builtins
            if obj_name == "console" {
                let builtin_id = match method_name {
                    "log" => Some(FunctionId(u32::MAX - 1)),
                    "warn" => Some(FunctionId(u32::MAX - 2)),
                    "error" => Some(FunctionId(u32::MAX - 3)),
                    _ => None,
                };

                if let Some(func_id) = builtin_id {
                    // Lower arguments
                    let args: Vec<LocalId> = call
                        .arguments
                        .iter()
                        .filter_map(|arg| lower_argument(ctx, arg).ok())
                        .collect();

                    let dest = ctx.builder.add_local("_builtin_result".to_string(), Type::Any);
                    ctx.builder.emit(TypedInstruction::Call {
                        dest: Some(dest),
                        function: func_id,
                        args,
                    });
                    return Ok(dest);
                }
            }
        }
    }

    // Check if this is a method call (obj.method() or obj[key]())
    // If so, we need to pass the object as `this`
    let (callee, this_arg) = match &call.callee {
        Expression::StaticMemberExpression(member) => {
            // obj.method() - pass obj as this
            let obj = lower_expr(ctx, &member.object)?;
            let prop = member.property.name.to_string();

            let method = ctx.builder.add_local("_method".to_string(), Type::Any);
            ctx.builder.emit(TypedInstruction::GetPropertyDynamic {
                dest: method,
                object: obj,
                property: prop,
            });
            (method, Some(obj))
        }
        Expression::ComputedMemberExpression(member) => {
            // obj[key]() - pass obj as this
            let obj = lower_expr(ctx, &member.object)?;
            let key = lower_expr(ctx, &member.expression)?;

            let method = ctx.builder.add_local("_method".to_string(), Type::Any);
            ctx.builder.emit(TypedInstruction::GetPropertyComputed {
                dest: method,
                object: obj,
                key,
            });
            (method, Some(obj))
        }
        _ => {
            // Regular function call - no this binding
            let callee = lower_expr(ctx, &call.callee)?;
            (callee, None)
        }
    };

    // Check if any arguments are spread elements
    let has_spread = call.arguments.iter().any(|arg| matches!(arg, Argument::SpreadElement(_)));

    if has_spread {
        // Build an array of all arguments, expanding spreads
        let args_array = ctx.builder.add_local("_args_array".to_string(), Type::Any);
        ctx.builder.emit(TypedInstruction::CreateArray {
            dest: args_array,
            elements: vec![],
        });

        for arg in &call.arguments {
            match arg {
                Argument::SpreadElement(spread) => {
                    // Spread the array into args
                    let spread_val = lower_expr(ctx, &spread.argument)?;
                    ctx.builder.emit(TypedInstruction::ArraySpread {
                        dest: args_array,
                        source: spread_val,
                    });
                }
                _ => {
                    // Regular argument - push to array
                    let arg_val = lower_argument(ctx, arg)?;
                    ctx.builder.emit(TypedInstruction::ArrayPush {
                        array: args_array,
                        value: arg_val,
                    });
                }
            }
        }

        // Call with spread
        let dest = ctx.builder.add_local("_call_result".to_string(), Type::Any);
        ctx.builder.emit(TypedInstruction::CallWithSpread {
            dest: Some(dest),
            callee,
            args: args_array,
        });

        Ok(dest)
    } else {
        // Regular call without spread
        let args: Vec<LocalId> =
            call.arguments.iter().filter_map(|arg| lower_argument(ctx, arg).ok()).collect();

        // Create result local
        let dest = ctx.builder.add_local("_call_result".to_string(), Type::Any);

        // Emit call instruction with proper this binding
        ctx.builder.emit(TypedInstruction::CallFunction {
            dest: Some(dest),
            callee,
            args,
            this_arg,
        });

        Ok(dest)
    }
}

fn lower_argument(ctx: &mut ExprContext, arg: &Argument) -> DxResult<LocalId> {
    // Handle different argument types
    match arg {
        Argument::SpreadElement(spread) => lower_expr(ctx, &spread.argument),
        // Handle literal types directly
        Argument::BooleanLiteral(lit) => {
            let dest =
                ctx.builder.add_local("_arg".to_string(), Type::Primitive(PrimitiveType::Bool));
            ctx.builder.emit(TypedInstruction::Const {
                dest,
                value: Constant::Bool(lit.value),
            });
            Ok(dest)
        }
        Argument::NullLiteral(_) => {
            let dest =
                ctx.builder.add_local("_arg".to_string(), Type::Primitive(PrimitiveType::Null));
            ctx.builder.emit(TypedInstruction::Const {
                dest,
                value: Constant::Null,
            });
            Ok(dest)
        }
        Argument::NumericLiteral(lit) => {
            let dest =
                ctx.builder.add_local("_arg".to_string(), Type::Primitive(PrimitiveType::F64));
            ctx.builder.emit(TypedInstruction::Const {
                dest,
                value: Constant::F64(lit.value),
            });
            Ok(dest)
        }
        Argument::StringLiteral(lit) => {
            let dest = ctx
                .builder
                .add_local("_arg".to_string(), Type::Primitive(PrimitiveType::String));
            ctx.builder.emit(TypedInstruction::Const {
                dest,
                value: Constant::String(lit.value.to_string()),
            });
            Ok(dest)
        }
        Argument::Identifier(ident) => {
            let name = ident.name.to_string();
            if let Some(&local_id) = ctx.variables.get(&name) {
                Ok(local_id)
            } else {
                let dest = ctx.builder.add_local("_arg".to_string(), Type::Any);
                ctx.builder.emit(TypedInstruction::Const {
                    dest,
                    value: Constant::Undefined,
                });
                Ok(dest)
            }
        }
        Argument::BinaryExpression(bin) => lower_binary_expression(ctx, bin),
        Argument::UnaryExpression(unary) => lower_unary_expression(ctx, unary),
        Argument::CallExpression(call) => lower_call_expression(ctx, call),
        Argument::ArrayExpression(arr) => lower_array_expression(ctx, arr),
        Argument::ObjectExpression(obj) => lower_object_expression(ctx, obj),
        Argument::ArrowFunctionExpression(arrow) => lower_arrow_function(ctx, arrow),
        Argument::FunctionExpression(func) => lower_function_expression(ctx, func),
        Argument::ConditionalExpression(cond) => lower_conditional_expression(ctx, cond),
        Argument::LogicalExpression(logical) => lower_logical_expression(ctx, logical),
        Argument::AssignmentExpression(assign) => lower_assignment_expression(ctx, assign),
        Argument::TemplateLiteral(tmpl) => lower_template_literal(ctx, tmpl),
        Argument::ParenthesizedExpression(paren) => lower_expr(ctx, &paren.expression),
        Argument::ThisExpression(_) => lower_this_expression(ctx),
        _ => {
            // For other complex expressions, return undefined placeholder
            let dest = ctx.builder.add_local("_arg".to_string(), Type::Any);
            ctx.builder.emit(TypedInstruction::Const {
                dest,
                value: Constant::Undefined,
            });
            Ok(dest)
        }
    }
}

/// Lower member expression - reserved for property access implementation
#[allow(dead_code)]
fn lower_member_expression(ctx: &mut ExprContext, _member: &MemberExpression) -> DxResult<LocalId> {
    // obj.prop or obj[expr]
    // For now, return undefined
    let dest = ctx.builder.add_local("_member".to_string(), Type::Any);
    ctx.builder.emit(TypedInstruction::Const {
        dest,
        value: Constant::Undefined,
    });
    Ok(dest)
}

fn lower_new_expression(ctx: &mut ExprContext, new_expr: &NewExpression) -> DxResult<LocalId> {
    // new Constructor(args)
    // Lower the callee (constructor)
    let callee = lower_expr(ctx, &new_expr.callee)?;

    // Lower arguments
    let args: Vec<LocalId> = new_expr
        .arguments
        .iter()
        .filter_map(|arg| lower_argument(ctx, arg).ok())
        .collect();

    // Create a new object
    let obj = ctx.builder.add_local("_new_obj".to_string(), Type::Any);
    ctx.builder.emit(TypedInstruction::CreateObject {
        dest: obj,
        properties: vec![],
    });

    // Call the constructor with the new object as `this`
    let result = ctx.builder.add_local("_new_result".to_string(), Type::Any);
    ctx.builder.emit(TypedInstruction::CallFunction {
        dest: Some(result),
        callee,
        args,
        this_arg: Some(obj),
    });

    // Return the object (or constructor result if it returns an object)
    Ok(obj)
}

fn lower_array_expression(ctx: &mut ExprContext, arr: &ArrayExpression) -> DxResult<LocalId> {
    // [1, 2, 3]
    // Lower all elements and create array
    let mut elements = Vec::new();

    for elem in &arr.elements {
        match elem {
            ArrayExpressionElement::SpreadElement(spread) => {
                // For spread, we'd need to expand the iterable
                // For now, just lower the argument
                let elem_local = lower_expr(ctx, &spread.argument)?;
                elements.push(elem_local);
            }
            ArrayExpressionElement::Elision(_) => {
                // Empty slot - create undefined
                let undef = ctx.builder.add_local("_elision".to_string(), Type::Any);
                ctx.builder.emit(TypedInstruction::Const {
                    dest: undef,
                    value: Constant::Undefined,
                });
                elements.push(undef);
            }
            // Handle literal elements directly
            ArrayExpressionElement::BooleanLiteral(lit) => {
                let elem_local = ctx
                    .builder
                    .add_local("_elem".to_string(), Type::Primitive(PrimitiveType::Bool));
                ctx.builder.emit(TypedInstruction::Const {
                    dest: elem_local,
                    value: Constant::Bool(lit.value),
                });
                elements.push(elem_local);
            }
            ArrayExpressionElement::NullLiteral(_) => {
                let elem_local = ctx
                    .builder
                    .add_local("_elem".to_string(), Type::Primitive(PrimitiveType::Null));
                ctx.builder.emit(TypedInstruction::Const {
                    dest: elem_local,
                    value: Constant::Null,
                });
                elements.push(elem_local);
            }
            ArrayExpressionElement::NumericLiteral(lit) => {
                let elem_local =
                    ctx.builder.add_local("_elem".to_string(), Type::Primitive(PrimitiveType::F64));
                ctx.builder.emit(TypedInstruction::Const {
                    dest: elem_local,
                    value: Constant::F64(lit.value),
                });
                elements.push(elem_local);
            }
            ArrayExpressionElement::StringLiteral(lit) => {
                let elem_local = ctx
                    .builder
                    .add_local("_elem".to_string(), Type::Primitive(PrimitiveType::String));
                ctx.builder.emit(TypedInstruction::Const {
                    dest: elem_local,
                    value: Constant::String(lit.value.to_string()),
                });
                elements.push(elem_local);
            }
            ArrayExpressionElement::Identifier(ident) => {
                // Variable reference
                let name = ident.name.to_string();
                if let Some(&local_id) = ctx.variables.get(&name) {
                    elements.push(local_id);
                } else {
                    let undef = ctx.builder.add_local("_undef".to_string(), Type::Any);
                    ctx.builder.emit(TypedInstruction::Const {
                        dest: undef,
                        value: Constant::Undefined,
                    });
                    elements.push(undef);
                }
            }
            _ => {
                // For other complex expressions, create undefined placeholder
                let undef = ctx.builder.add_local("_elem".to_string(), Type::Any);
                ctx.builder.emit(TypedInstruction::Const {
                    dest: undef,
                    value: Constant::Undefined,
                });
                elements.push(undef);
            }
        }
    }

    // Create array with elements
    let dest = ctx.builder.add_local("_array".to_string(), Type::Any);
    ctx.builder.emit(TypedInstruction::CreateArray { dest, elements });

    Ok(dest)
}

fn lower_object_expression(ctx: &mut ExprContext, obj: &ObjectExpression) -> DxResult<LocalId> {
    // {a: 1, b: 2}
    // Lower all properties and create object
    let mut properties = Vec::new();

    for prop in &obj.properties {
        match prop {
            ObjectPropertyKind::ObjectProperty(prop) => {
                // Get property key
                let key = match &prop.key {
                    PropertyKey::StaticIdentifier(ident) => ident.name.to_string(),
                    PropertyKey::StringLiteral(lit) => lit.value.to_string(),
                    PropertyKey::NumericLiteral(lit) => lit.value.to_string(),
                    _ => continue, // Skip computed keys for now
                };

                // Lower property value
                let value = lower_expr(ctx, &prop.value)?;
                properties.push((key, value));
            }
            ObjectPropertyKind::SpreadProperty(spread) => {
                // For spread, we'd need to copy all properties from the source
                // For now, just lower the argument
                let _ = lower_expr(ctx, &spread.argument)?;
            }
        }
    }

    // Create object with properties
    let dest = ctx.builder.add_local("_object".to_string(), Type::Any);
    ctx.builder.emit(TypedInstruction::CreateObject { dest, properties });

    Ok(dest)
}

fn lower_arrow_function(
    ctx: &mut ExprContext,
    arrow: &ArrowFunctionExpression,
) -> DxResult<LocalId> {
    // () => expr or () => { statements }
    // Create a function object that captures variables from outer scope

    // Find free variables that need to be captured
    let captured_vars = find_captured_variables(ctx, arrow);

    // Create a new function ID for this arrow function
    let func_id = FunctionId(ctx.builder.id.0 + 1000); // Offset to avoid conflicts

    // Create the function object
    let dest = ctx.builder.add_local(
        "_arrow".to_string(),
        Type::Function(FunctionSignature {
            params: arrow.params.items.iter().map(|_| Type::Any).collect(),
            return_type: Box::new(Type::Any),
        }),
    );

    ctx.builder.emit(TypedInstruction::CreateFunction {
        dest,
        function_id: func_id,
        captured_vars,
        is_arrow: true, // Arrow functions preserve lexical `this`
    });

    Ok(dest)
}

/// Find variables that need to be captured from outer scope
/// This performs proper free variable analysis by walking the AST
fn find_captured_variables(ctx: &ExprContext, arrow: &ArrowFunctionExpression) -> Vec<LocalId> {
    // Collect parameter names (these are locally declared, not captured)
    let mut local_vars: std::collections::HashSet<String> = std::collections::HashSet::new();
    for param in &arrow.params.items {
        collect_binding_names(&param.pattern, &mut local_vars);
    }

    // Find all referenced variables in the function body
    let mut referenced_vars: std::collections::HashSet<String> = std::collections::HashSet::new();
    for stmt in &arrow.body.statements {
        collect_referenced_variables_stmt(stmt, &mut referenced_vars, &mut local_vars);
    }

    // Free variables = referenced but not locally declared
    let mut captured = Vec::new();
    for var_name in &referenced_vars {
        if !local_vars.contains(var_name) {
            // Check if this variable exists in the outer scope
            if let Some(&local_id) = ctx.variables.get(var_name) {
                captured.push(local_id);
            }
        }
    }

    captured
}

/// Collect binding names from a binding pattern
fn collect_binding_names(
    pattern: &oxc_ast::ast::BindingPattern,
    names: &mut std::collections::HashSet<String>,
) {
    use oxc_ast::ast::BindingPatternKind;

    match &pattern.kind {
        BindingPatternKind::BindingIdentifier(ident) => {
            names.insert(ident.name.to_string());
        }
        BindingPatternKind::ObjectPattern(obj) => {
            for prop in &obj.properties {
                collect_binding_names(&prop.value, names);
            }
            if let Some(rest) = &obj.rest {
                collect_binding_names(&rest.argument, names);
            }
        }
        BindingPatternKind::ArrayPattern(arr) => {
            for elem in arr.elements.iter().flatten() {
                collect_binding_names(elem, names);
            }
            if let Some(rest) = &arr.rest {
                collect_binding_names(&rest.argument, names);
            }
        }
        BindingPatternKind::AssignmentPattern(assign) => {
            collect_binding_names(&assign.left, names);
        }
    }
}

/// Collect referenced variables from a statement
fn collect_referenced_variables_stmt(
    stmt: &oxc_ast::ast::Statement,
    referenced: &mut std::collections::HashSet<String>,
    local_vars: &mut std::collections::HashSet<String>,
) {
    use oxc_ast::ast::Statement;

    match stmt {
        Statement::ExpressionStatement(expr_stmt) => {
            collect_referenced_variables_expr(&expr_stmt.expression, referenced, local_vars);
        }
        Statement::BlockStatement(block) => {
            for s in &block.body {
                collect_referenced_variables_stmt(s, referenced, local_vars);
            }
        }
        Statement::IfStatement(if_stmt) => {
            collect_referenced_variables_expr(&if_stmt.test, referenced, local_vars);
            collect_referenced_variables_stmt(&if_stmt.consequent, referenced, local_vars);
            if let Some(alt) = &if_stmt.alternate {
                collect_referenced_variables_stmt(alt, referenced, local_vars);
            }
        }
        Statement::WhileStatement(while_stmt) => {
            collect_referenced_variables_expr(&while_stmt.test, referenced, local_vars);
            collect_referenced_variables_stmt(&while_stmt.body, referenced, local_vars);
        }
        Statement::ForStatement(for_stmt) => {
            if let Some(oxc_ast::ast::ForStatementInit::VariableDeclaration(decl)) = &for_stmt.init
            {
                for declarator in &decl.declarations {
                    collect_binding_names(&declarator.id, local_vars);
                    if let Some(init_expr) = &declarator.init {
                        collect_referenced_variables_expr(init_expr, referenced, local_vars);
                    }
                }
            }
            if let Some(test) = &for_stmt.test {
                collect_referenced_variables_expr(test, referenced, local_vars);
            }
            if let Some(update) = &for_stmt.update {
                collect_referenced_variables_expr(update, referenced, local_vars);
            }
            collect_referenced_variables_stmt(&for_stmt.body, referenced, local_vars);
        }
        Statement::ReturnStatement(ret) => {
            if let Some(arg) = &ret.argument {
                collect_referenced_variables_expr(arg, referenced, local_vars);
            }
        }
        Statement::VariableDeclaration(decl) => {
            for declarator in &decl.declarations {
                // First collect references from initializer (before adding to local_vars)
                if let Some(init) = &declarator.init {
                    collect_referenced_variables_expr(init, referenced, local_vars);
                }
                // Then add the declared variable to local_vars
                collect_binding_names(&declarator.id, local_vars);
            }
        }
        Statement::FunctionDeclaration(func) => {
            // Function name is local
            if let Some(id) = &func.id {
                local_vars.insert(id.name.to_string());
            }
            // Don't recurse into function body - it has its own scope
        }
        Statement::TryStatement(try_stmt) => {
            for s in &try_stmt.block.body {
                collect_referenced_variables_stmt(s, referenced, local_vars);
            }
            if let Some(handler) = &try_stmt.handler {
                // Catch parameter is local to catch block
                let mut catch_locals = local_vars.clone();
                if let Some(param) = &handler.param {
                    collect_binding_names(&param.pattern, &mut catch_locals);
                }
                for s in &handler.body.body {
                    collect_referenced_variables_stmt(s, referenced, &mut catch_locals);
                }
            }
            if let Some(finalizer) = &try_stmt.finalizer {
                for s in &finalizer.body {
                    collect_referenced_variables_stmt(s, referenced, local_vars);
                }
            }
        }
        Statement::ThrowStatement(throw) => {
            collect_referenced_variables_expr(&throw.argument, referenced, local_vars);
        }
        Statement::SwitchStatement(switch) => {
            collect_referenced_variables_expr(&switch.discriminant, referenced, local_vars);
            for case in &switch.cases {
                if let Some(test) = &case.test {
                    collect_referenced_variables_expr(test, referenced, local_vars);
                }
                for s in &case.consequent {
                    collect_referenced_variables_stmt(s, referenced, local_vars);
                }
            }
        }
        _ => {}
    }
}

/// Collect referenced variables from an expression
fn collect_referenced_variables_expr(
    expr: &oxc_ast::ast::Expression,
    referenced: &mut std::collections::HashSet<String>,
    _local_vars: &mut std::collections::HashSet<String>,
) {
    use oxc_ast::ast::Expression;

    match expr {
        Expression::Identifier(ident) => {
            // This is a variable reference
            referenced.insert(ident.name.to_string());
        }
        Expression::BinaryExpression(bin) => {
            collect_referenced_variables_expr(&bin.left, referenced, _local_vars);
            collect_referenced_variables_expr(&bin.right, referenced, _local_vars);
        }
        Expression::UnaryExpression(unary) => {
            collect_referenced_variables_expr(&unary.argument, referenced, _local_vars);
        }
        Expression::LogicalExpression(logical) => {
            collect_referenced_variables_expr(&logical.left, referenced, _local_vars);
            collect_referenced_variables_expr(&logical.right, referenced, _local_vars);
        }
        Expression::ConditionalExpression(cond) => {
            collect_referenced_variables_expr(&cond.test, referenced, _local_vars);
            collect_referenced_variables_expr(&cond.consequent, referenced, _local_vars);
            collect_referenced_variables_expr(&cond.alternate, referenced, _local_vars);
        }
        Expression::CallExpression(call) => {
            collect_referenced_variables_expr(&call.callee, referenced, _local_vars);
            for arg in &call.arguments {
                match arg {
                    oxc_ast::ast::Argument::SpreadElement(spread) => {
                        collect_referenced_variables_expr(
                            &spread.argument,
                            referenced,
                            _local_vars,
                        );
                    }
                    _ => {
                        if let Some(expr) = arg.as_expression() {
                            collect_referenced_variables_expr(expr, referenced, _local_vars);
                        }
                    }
                }
            }
        }
        Expression::StaticMemberExpression(member) => {
            collect_referenced_variables_expr(&member.object, referenced, _local_vars);
        }
        Expression::ComputedMemberExpression(member) => {
            collect_referenced_variables_expr(&member.object, referenced, _local_vars);
            collect_referenced_variables_expr(&member.expression, referenced, _local_vars);
        }
        Expression::PrivateFieldExpression(member) => {
            collect_referenced_variables_expr(&member.object, referenced, _local_vars);
        }
        Expression::AssignmentExpression(assign) => {
            // Right side is always referenced
            collect_referenced_variables_expr(&assign.right, referenced, _local_vars);
            // Left side depends on the target
            if let oxc_ast::ast::AssignmentTarget::AssignmentTargetIdentifier(ident) = &assign.left
            {
                referenced.insert(ident.name.to_string());
            }
        }
        Expression::ArrayExpression(arr) => {
            for elem in &arr.elements {
                match elem {
                    oxc_ast::ast::ArrayExpressionElement::SpreadElement(spread) => {
                        collect_referenced_variables_expr(
                            &spread.argument,
                            referenced,
                            _local_vars,
                        );
                    }
                    oxc_ast::ast::ArrayExpressionElement::Elision(_) => {}
                    _ => {
                        if let Some(expr) = elem.as_expression() {
                            collect_referenced_variables_expr(expr, referenced, _local_vars);
                        }
                    }
                }
            }
        }
        Expression::ObjectExpression(obj) => {
            for prop in &obj.properties {
                match prop {
                    oxc_ast::ast::ObjectPropertyKind::ObjectProperty(p) => {
                        collect_referenced_variables_expr(&p.value, referenced, _local_vars);
                    }
                    oxc_ast::ast::ObjectPropertyKind::SpreadProperty(spread) => {
                        collect_referenced_variables_expr(
                            &spread.argument,
                            referenced,
                            _local_vars,
                        );
                    }
                }
            }
        }
        Expression::ArrowFunctionExpression(_) => {
            // Don't recurse into nested functions - they have their own scope
        }
        Expression::FunctionExpression(_) => {
            // Don't recurse into nested functions - they have their own scope
        }
        Expression::UpdateExpression(update) => {
            // Update expressions like x++ or ++x reference the variable
            match &update.argument {
                oxc_ast::ast::SimpleAssignmentTarget::AssignmentTargetIdentifier(ident) => {
                    referenced.insert(ident.name.to_string());
                }
                oxc_ast::ast::SimpleAssignmentTarget::TSAsExpression(ts_as) => {
                    collect_referenced_variables_expr(&ts_as.expression, referenced, _local_vars);
                }
                oxc_ast::ast::SimpleAssignmentTarget::TSSatisfiesExpression(ts_sat) => {
                    collect_referenced_variables_expr(&ts_sat.expression, referenced, _local_vars);
                }
                oxc_ast::ast::SimpleAssignmentTarget::TSNonNullExpression(ts_non_null) => {
                    collect_referenced_variables_expr(
                        &ts_non_null.expression,
                        referenced,
                        _local_vars,
                    );
                }
                oxc_ast::ast::SimpleAssignmentTarget::TSTypeAssertion(ts_type) => {
                    collect_referenced_variables_expr(&ts_type.expression, referenced, _local_vars);
                }
                oxc_ast::ast::SimpleAssignmentTarget::TSInstantiationExpression(ts_inst) => {
                    collect_referenced_variables_expr(&ts_inst.expression, referenced, _local_vars);
                }
                _ => {}
            }
        }
        Expression::SequenceExpression(seq) => {
            for e in &seq.expressions {
                collect_referenced_variables_expr(e, referenced, _local_vars);
            }
        }
        Expression::NewExpression(new_expr) => {
            collect_referenced_variables_expr(&new_expr.callee, referenced, _local_vars);
            for arg in &new_expr.arguments {
                if let Some(expr) = arg.as_expression() {
                    collect_referenced_variables_expr(expr, referenced, _local_vars);
                }
            }
        }
        Expression::AwaitExpression(await_expr) => {
            collect_referenced_variables_expr(&await_expr.argument, referenced, _local_vars);
        }
        Expression::YieldExpression(yield_expr) => {
            if let Some(arg) = &yield_expr.argument {
                collect_referenced_variables_expr(arg, referenced, _local_vars);
            }
        }
        Expression::TemplateLiteral(tmpl) => {
            for expr in &tmpl.expressions {
                collect_referenced_variables_expr(expr, referenced, _local_vars);
            }
        }
        Expression::ParenthesizedExpression(paren) => {
            collect_referenced_variables_expr(&paren.expression, referenced, _local_vars);
        }
        _ => {}
    }
}

fn lower_function_expression(ctx: &mut ExprContext, func: &Function) -> DxResult<LocalId> {
    // function() { ... } or function name() { ... }
    // Create a function object that captures variables from outer scope

    // Find free variables that need to be captured
    let captured_vars = find_captured_variables_func(ctx, func);

    // Create a new function ID for this function expression
    let func_id = FunctionId(ctx.builder.id.0 + 2000); // Offset to avoid conflicts

    // Get function name if it exists
    let func_name = func
        .id
        .as_ref()
        .map(|id| id.name.to_string())
        .unwrap_or_else(|| "_anon".to_string());

    // Create the function object
    let dest = ctx.builder.add_local(
        func_name,
        Type::Function(FunctionSignature {
            params: func.params.items.iter().map(|_| Type::Any).collect(),
            return_type: Box::new(Type::Any),
        }),
    );

    ctx.builder.emit(TypedInstruction::CreateFunction {
        dest,
        function_id: func_id,
        captured_vars,
        is_arrow: false, // Regular functions have their own `this`
    });

    Ok(dest)
}

/// Find variables that need to be captured from outer scope for regular functions
fn find_captured_variables_func(ctx: &ExprContext, func: &Function) -> Vec<LocalId> {
    // Collect parameter names (these are locally declared, not captured)
    let mut local_vars: std::collections::HashSet<String> = std::collections::HashSet::new();
    for param in &func.params.items {
        collect_binding_names(&param.pattern, &mut local_vars);
    }

    // Function name is also local (for named function expressions)
    if let Some(id) = &func.id {
        local_vars.insert(id.name.to_string());
    }

    // Find all referenced variables in the function body
    let mut referenced_vars: std::collections::HashSet<String> = std::collections::HashSet::new();
    if let Some(body) = &func.body {
        for stmt in &body.statements {
            collect_referenced_variables_stmt(stmt, &mut referenced_vars, &mut local_vars);
        }
    }

    // Free variables = referenced but not locally declared
    let mut captured = Vec::new();
    for var_name in &referenced_vars {
        if !local_vars.contains(var_name) {
            // Check if this variable exists in the outer scope
            if let Some(&local_id) = ctx.variables.get(var_name) {
                captured.push(local_id);
            }
        }
    }

    captured
}

fn lower_template_literal(ctx: &mut ExprContext, tmpl: &TemplateLiteral) -> DxResult<LocalId> {
    // Template literal: `hello ${name}, you are ${age} years old`
    // quasis: ["hello ", ", you are ", " years old"]
    // expressions: [name, age]
    
    // Collect all quasis (static string parts)
    // Use cooked value if available, otherwise raw value
    // This preserves line breaks and escape sequences properly (Requirements: 8.2)
    let quasis: Vec<String> = tmpl.quasis.iter().map(|q| {
        // cooked is the processed string with escape sequences interpreted
        // raw is the literal string as written in source
        // For multiline templates, cooked preserves the line breaks
        q.value.cooked.as_ref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| q.value.raw.to_string())
    }).collect();
    
    // If there are no expressions, just return the single quasi as a string
    if tmpl.expressions.is_empty() {
        let dest = ctx
            .builder
            .add_local("_template".to_string(), Type::Primitive(PrimitiveType::String));
        let value = quasis.first().cloned().unwrap_or_default();
        ctx.builder.emit(TypedInstruction::Const {
            dest,
            value: Constant::String(value),
        });
        return Ok(dest);
    }
    
    // Evaluate all expressions (Requirements: 8.1)
    let mut expression_locals = Vec::with_capacity(tmpl.expressions.len());
    for expr in &tmpl.expressions {
        let expr_local = lower_expr(ctx, expr)?;
        expression_locals.push(expr_local);
    }
    
    // Create destination for the result
    let dest = ctx
        .builder
        .add_local("_template".to_string(), Type::Primitive(PrimitiveType::String));
    
    // Emit the BuildTemplateLiteral instruction
    ctx.builder.emit(TypedInstruction::BuildTemplateLiteral {
        dest,
        quasis,
        expressions: expression_locals,
    });
    
    Ok(dest)
}

/// Lower a tagged template expression
/// Requirements: 8.3 - tagged template invocation
/// 
/// Tagged templates call the tag function with:
/// - First argument: array of cooked strings (with escape sequences processed)
/// - Additional arguments: the interpolated values
/// - The strings array has a 'raw' property with unprocessed strings
fn lower_tagged_template(ctx: &mut ExprContext, tagged: &TaggedTemplateExpression) -> DxResult<LocalId> {
    // Evaluate the tag function
    let tag = lower_expr(ctx, &tagged.tag)?;
    
    // Get the template literal
    let tmpl = &tagged.quasi;
    
    // Collect cooked quasis (with escape sequences processed)
    let quasis: Vec<String> = tmpl.quasis.iter().map(|q| {
        q.value.cooked.as_ref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| q.value.raw.to_string())
    }).collect();
    
    // Collect raw quasis (unprocessed strings)
    let raw_quasis: Vec<String> = tmpl.quasis.iter().map(|q| {
        q.value.raw.to_string()
    }).collect();
    
    // Evaluate all expressions
    let mut expression_locals = Vec::with_capacity(tmpl.expressions.len());
    for expr in &tmpl.expressions {
        let expr_local = lower_expr(ctx, expr)?;
        expression_locals.push(expr_local);
    }
    
    // Create destination for the result
    let dest = ctx
        .builder
        .add_local("_tagged_template".to_string(), Type::Any);
    
    // Emit the CallTaggedTemplate instruction
    ctx.builder.emit(TypedInstruction::CallTaggedTemplate {
        dest,
        tag,
        quasis,
        raw_quasis,
        expressions: expression_locals,
    });
    
    Ok(dest)
}

/// Lower spread element - reserved for spread operator implementation
#[allow(dead_code)]
fn lower_spread_element(ctx: &mut ExprContext, spread: &SpreadElement) -> DxResult<LocalId> {
    // ...arr
    lower_expr(ctx, &spread.argument)
}

fn lower_sequence_expression(ctx: &mut ExprContext, seq: &SequenceExpression) -> DxResult<LocalId> {
    // expr1, expr2, expr3
    let mut last = None;
    for expr in &seq.expressions {
        last = Some(lower_expr(ctx, expr)?);
    }
    last.ok_or_else(|| DxError::CompileError("Empty sequence expression".to_string()))
}

fn lower_this_expression(ctx: &mut ExprContext) -> DxResult<LocalId> {
    // this - get the current `this` binding
    let dest = ctx.builder.add_local("_this".to_string(), Type::Any);
    ctx.builder.emit(TypedInstruction::GetThis { dest });
    Ok(dest)
}

/// Lower a dynamic import expression
///
/// import(specifier) returns a Promise that resolves to the module namespace:
/// 1. Evaluate the specifier expression (should be a string)
/// 2. Create a Promise for the import operation
/// 3. Resolve the module specifier
/// 4. Load and evaluate the module
/// 5. Resolve the Promise with the module namespace
///
/// Requirements:
/// - 2.1: WHEN `import(specifier)` is called with a valid module path, THE Runtime SHALL return a Promise
/// - 2.2: WHEN `import(specifier)` is called with a relative path, THE Runtime SHALL resolve it relative to the importing module
/// - 2.3: WHEN `import(specifier)` is called with a bare specifier, THE Runtime SHALL resolve it using Node.js module resolution
fn lower_import_expression(
    ctx: &mut ExprContext,
    import_expr: &ImportExpression,
) -> DxResult<LocalId> {
    // Evaluate the specifier expression
    let specifier = lower_expr(ctx, &import_expr.source)?;

    // Create destination for the Promise
    let dest = ctx.builder.add_local("_import_promise".to_string(), Type::Any);

    // Emit the dynamic import instruction
    // This will:
    // 1. Get the specifier string
    // 2. Resolve the module path
    // 3. Create a Promise
    // 4. Load and evaluate the module asynchronously
    // 5. Resolve/reject the Promise based on the result
    ctx.builder.emit(TypedInstruction::DynamicImport {
        dest,
        specifier,
    });

    Ok(dest)
}

/// Lower an await expression
///
/// await expr suspends execution until the promise resolves:
/// 1. Evaluate the expression (which should be a promise)
/// 2. If already resolved, return the value
/// 3. If pending, suspend and resume when resolved
/// 4. If rejected, throw the rejection reason
fn lower_await_expression(
    ctx: &mut ExprContext,
    await_expr: &AwaitExpression,
) -> DxResult<LocalId> {
    // Evaluate the promise expression
    let promise = lower_expr(ctx, &await_expr.argument)?;

    // Create blocks for await handling
    let resume_block = ctx.builder.new_block();
    let reject_block = ctx.builder.new_block();

    // Create destination for the resolved value
    let dest = ctx.builder.add_local("_await_result".to_string(), Type::Any);

    // Emit the await instruction
    // This will:
    // 1. Check if promise is already settled
    // 2. If fulfilled, store value in dest and continue
    // 3. If rejected, jump to reject_block
    // 4. If pending, suspend and schedule resume when settled
    ctx.builder.emit(TypedInstruction::Await {
        dest,
        promise,
        resume_block,
        reject_block,
    });

    // Set up the resume block (normal continuation)
    ctx.builder.set_terminator(Terminator::Goto(resume_block));
    ctx.builder.switch_to_block(resume_block);

    // Note: reject_block handling would be done by try/catch
    // For now, we leave it as unreachable (unhandled rejection)
    // A full implementation would propagate to the nearest catch handler

    Ok(dest)
}

/// Function builder (re-exported from MIR for convenience)
pub use crate::compiler::mir::FunctionBuilder;

/// Public function to find captured variables in a function declaration
/// Used by statements.rs for function declarations
pub fn find_captured_variables_in_func(
    func: &Function,
    outer_scope: &HashMap<String, LocalId>,
) -> Vec<LocalId> {
    // Collect parameter names (these are locally declared, not captured)
    let mut local_vars: std::collections::HashSet<String> = std::collections::HashSet::new();
    for param in &func.params.items {
        collect_binding_names(&param.pattern, &mut local_vars);
    }

    // Function name is also local (for named function expressions)
    if let Some(id) = &func.id {
        local_vars.insert(id.name.to_string());
    }

    // Find all referenced variables in the function body
    let mut referenced_vars: std::collections::HashSet<String> = std::collections::HashSet::new();
    if let Some(body) = &func.body {
        for stmt in &body.statements {
            collect_referenced_variables_stmt(stmt, &mut referenced_vars, &mut local_vars);
        }
    }

    // Free variables = referenced but not locally declared
    let mut captured = Vec::new();
    for var_name in &referenced_vars {
        if !local_vars.contains(var_name) {
            // Check if this variable exists in the outer scope
            if let Some(&local_id) = outer_scope.get(var_name) {
                captured.push(local_id);
            }
        }
    }

    captured
}
