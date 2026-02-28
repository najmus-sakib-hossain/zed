//! AST to Python source code printer
//!
//! This module provides functionality to convert an AST back to Python source code.
//! This is primarily used for round-trip testing to verify parser correctness.

use crate::ast::*;

/// Pretty printer for Python AST
pub struct Printer {
    indent_level: usize,
    indent_str: String,
}

impl Default for Printer {
    fn default() -> Self {
        Self::new()
    }
}

impl Printer {
    /// Create a new printer with default settings
    pub fn new() -> Self {
        Self {
            indent_level: 0,
            indent_str: "    ".to_string(),
        }
    }

    /// Get the current indentation string
    fn indent(&self) -> String {
        self.indent_str.repeat(self.indent_level)
    }

    /// Print a module to source code
    pub fn print_module(&mut self, module: &Module) -> String {
        let mut output = String::new();
        for stmt in &module.body {
            output.push_str(&self.print_statement(stmt));
        }
        output
    }

    /// Print a statement to source code
    pub fn print_statement(&mut self, stmt: &Statement) -> String {
        match stmt {
            Statement::FunctionDef {
                name,
                args,
                body,
                decorators,
                returns,
                is_async,
                ..
            } => {
                let mut output = String::new();
                for decorator in decorators {
                    output.push_str(&format!(
                        "{}@{}\n",
                        self.indent(),
                        self.print_expression(decorator)
                    ));
                }
                let async_prefix = if *is_async { "async " } else { "" };
                output.push_str(&format!(
                    "{}{}def {}({}){}:\n",
                    self.indent(),
                    async_prefix,
                    name,
                    self.print_arguments(args),
                    returns
                        .as_ref()
                        .map(|r| format!(" -> {}", self.print_expression(r)))
                        .unwrap_or_default()
                ));
                self.indent_level += 1;
                for s in body {
                    output.push_str(&self.print_statement(s));
                }
                self.indent_level -= 1;
                output
            }

            Statement::ClassDef {
                name,
                bases,
                keywords,
                body,
                decorators,
                ..
            } => {
                let mut output = String::new();
                for decorator in decorators {
                    output.push_str(&format!(
                        "{}@{}\n",
                        self.indent(),
                        self.print_expression(decorator)
                    ));
                }
                let bases_str = if bases.is_empty() && keywords.is_empty() {
                    String::new()
                } else {
                    let mut parts: Vec<String> =
                        bases.iter().map(|b| self.print_expression(b)).collect();
                    for kw in keywords {
                        if let Some(arg) = &kw.arg {
                            parts.push(format!("{}={}", arg, self.print_expression(&kw.value)));
                        } else {
                            parts.push(format!("**{}", self.print_expression(&kw.value)));
                        }
                    }
                    format!("({})", parts.join(", "))
                };
                output.push_str(&format!("{}class {}{}:\n", self.indent(), name, bases_str));
                self.indent_level += 1;
                for s in body {
                    output.push_str(&self.print_statement(s));
                }
                self.indent_level -= 1;
                output
            }

            Statement::Return { value, .. } => match value {
                Some(v) => format!("{}return {}\n", self.indent(), self.print_expression(v)),
                None => format!("{}return\n", self.indent()),
            },

            Statement::Delete { targets, .. } => {
                let targets_str =
                    targets.iter().map(|t| self.print_expression(t)).collect::<Vec<_>>().join(", ");
                format!("{}del {}\n", self.indent(), targets_str)
            }

            Statement::Assign { targets, value, .. } => {
                let targets_str = targets
                    .iter()
                    .map(|t| self.print_expression(t))
                    .collect::<Vec<_>>()
                    .join(" = ");
                format!("{}{} = {}\n", self.indent(), targets_str, self.print_expression(value))
            }

            Statement::AugAssign {
                target, op, value, ..
            } => {
                format!(
                    "{}{} {}= {}\n",
                    self.indent(),
                    self.print_expression(target),
                    self.print_binop(op),
                    self.print_expression(value)
                )
            }

            Statement::AnnAssign {
                target,
                annotation,
                value,
                ..
            } => {
                let value_str = value
                    .as_ref()
                    .map(|v| format!(" = {}", self.print_expression(v)))
                    .unwrap_or_default();
                format!(
                    "{}{}: {}{}\n",
                    self.indent(),
                    self.print_expression(target),
                    self.print_expression(annotation),
                    value_str
                )
            }

            Statement::For {
                target,
                iter,
                body,
                orelse,
                is_async,
                ..
            } => {
                let async_prefix = if *is_async { "async " } else { "" };
                let mut output = format!(
                    "{}{}for {} in {}:\n",
                    self.indent(),
                    async_prefix,
                    self.print_expression(target),
                    self.print_expression(iter)
                );
                self.indent_level += 1;
                for s in body {
                    output.push_str(&self.print_statement(s));
                }
                self.indent_level -= 1;
                if !orelse.is_empty() {
                    output.push_str(&format!("{}else:\n", self.indent()));
                    self.indent_level += 1;
                    for s in orelse {
                        output.push_str(&self.print_statement(s));
                    }
                    self.indent_level -= 1;
                }
                output
            }

            Statement::While {
                test, body, orelse, ..
            } => {
                let mut output =
                    format!("{}while {}:\n", self.indent(), self.print_expression(test));
                self.indent_level += 1;
                for s in body {
                    output.push_str(&self.print_statement(s));
                }
                self.indent_level -= 1;
                if !orelse.is_empty() {
                    output.push_str(&format!("{}else:\n", self.indent()));
                    self.indent_level += 1;
                    for s in orelse {
                        output.push_str(&self.print_statement(s));
                    }
                    self.indent_level -= 1;
                }
                output
            }

            Statement::If {
                test, body, orelse, ..
            } => {
                let mut output = format!("{}if {}:\n", self.indent(), self.print_expression(test));
                self.indent_level += 1;
                for s in body {
                    output.push_str(&self.print_statement(s));
                }
                self.indent_level -= 1;
                if !orelse.is_empty() {
                    // Check if orelse is a single If statement (elif)
                    if orelse.len() == 1 {
                        if let Statement::If {
                            test: elif_test,
                            body: elif_body,
                            orelse: elif_orelse,
                            ..
                        } = &orelse[0]
                        {
                            output.push_str(&format!(
                                "{}elif {}:\n",
                                self.indent(),
                                self.print_expression(elif_test)
                            ));
                            self.indent_level += 1;
                            for s in elif_body {
                                output.push_str(&self.print_statement(s));
                            }
                            self.indent_level -= 1;
                            if !elif_orelse.is_empty() {
                                output.push_str(&self.print_else_block(elif_orelse));
                            }
                            return output;
                        }
                    }
                    output.push_str(&format!("{}else:\n", self.indent()));
                    self.indent_level += 1;
                    for s in orelse {
                        output.push_str(&self.print_statement(s));
                    }
                    self.indent_level -= 1;
                }
                output
            }

            Statement::With {
                items,
                body,
                is_async,
                ..
            } => {
                let async_prefix = if *is_async { "async " } else { "" };
                let items_str = items
                    .iter()
                    .map(|item| {
                        let vars_str = item
                            .optional_vars
                            .as_ref()
                            .map(|v| format!(" as {}", self.print_expression(v)))
                            .unwrap_or_default();
                        format!("{}{}", self.print_expression(&item.context_expr), vars_str)
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let mut output = format!("{}{}with {}:\n", self.indent(), async_prefix, items_str);
                self.indent_level += 1;
                for s in body {
                    output.push_str(&self.print_statement(s));
                }
                self.indent_level -= 1;
                output
            }

            Statement::Match { subject, cases, .. } => {
                let mut output =
                    format!("{}match {}:\n", self.indent(), self.print_expression(subject));
                self.indent_level += 1;
                for case in cases {
                    output.push_str(&self.print_match_case(case));
                }
                self.indent_level -= 1;
                output
            }

            Statement::Raise { exc, cause, .. } => match (exc, cause) {
                (None, _) => format!("{}raise\n", self.indent()),
                (Some(e), None) => format!("{}raise {}\n", self.indent(), self.print_expression(e)),
                (Some(e), Some(c)) => format!(
                    "{}raise {} from {}\n",
                    self.indent(),
                    self.print_expression(e),
                    self.print_expression(c)
                ),
            },

            Statement::Try {
                body,
                handlers,
                orelse,
                finalbody,
                ..
            } => {
                let mut output = format!("{}try:\n", self.indent());
                self.indent_level += 1;
                for s in body {
                    output.push_str(&self.print_statement(s));
                }
                self.indent_level -= 1;
                for handler in handlers {
                    output.push_str(&self.print_except_handler(handler));
                }
                if !orelse.is_empty() {
                    output.push_str(&format!("{}else:\n", self.indent()));
                    self.indent_level += 1;
                    for s in orelse {
                        output.push_str(&self.print_statement(s));
                    }
                    self.indent_level -= 1;
                }
                if !finalbody.is_empty() {
                    output.push_str(&format!("{}finally:\n", self.indent()));
                    self.indent_level += 1;
                    for s in finalbody {
                        output.push_str(&self.print_statement(s));
                    }
                    self.indent_level -= 1;
                }
                output
            }

            Statement::Assert { test, msg, .. } => match msg {
                Some(m) => format!(
                    "{}assert {}, {}\n",
                    self.indent(),
                    self.print_expression(test),
                    self.print_expression(m)
                ),
                None => format!("{}assert {}\n", self.indent(), self.print_expression(test)),
            },

            Statement::Import { names, .. } => {
                let names_str =
                    names.iter().map(|a| self.print_alias(a)).collect::<Vec<_>>().join(", ");
                format!("{}import {}\n", self.indent(), names_str)
            }

            Statement::ImportFrom {
                module,
                names,
                level,
                ..
            } => {
                let dots = ".".repeat(*level);
                let module_str = module.as_ref().map(|m| m.as_str()).unwrap_or("");
                let names_str =
                    names.iter().map(|a| self.print_alias(a)).collect::<Vec<_>>().join(", ");
                format!("{}from {}{} import {}\n", self.indent(), dots, module_str, names_str)
            }

            Statement::Global { names, .. } => {
                format!("{}global {}\n", self.indent(), names.join(", "))
            }

            Statement::Nonlocal { names, .. } => {
                format!("{}nonlocal {}\n", self.indent(), names.join(", "))
            }

            Statement::Expr { value, .. } => {
                format!("{}{}\n", self.indent(), self.print_expression(value))
            }

            Statement::Pass { .. } => format!("{}pass\n", self.indent()),
            Statement::Break { .. } => format!("{}break\n", self.indent()),
            Statement::Continue { .. } => format!("{}continue\n", self.indent()),
        }
    }

    fn print_else_block(&mut self, orelse: &[Statement]) -> String {
        if orelse.is_empty() {
            return String::new();
        }
        // Check if orelse is a single If statement (elif)
        if orelse.len() == 1 {
            if let Statement::If {
                test,
                body,
                orelse: inner_orelse,
                ..
            } = &orelse[0]
            {
                let mut output =
                    format!("{}elif {}:\n", self.indent(), self.print_expression(test));
                self.indent_level += 1;
                for s in body {
                    output.push_str(&self.print_statement(s));
                }
                self.indent_level -= 1;
                output.push_str(&self.print_else_block(inner_orelse));
                return output;
            }
        }
        let mut output = format!("{}else:\n", self.indent());
        self.indent_level += 1;
        for s in orelse {
            output.push_str(&self.print_statement(s));
        }
        self.indent_level -= 1;
        output
    }

    fn print_except_handler(&mut self, handler: &ExceptHandler) -> String {
        let type_str = handler
            .typ
            .as_ref()
            .map(|t| format!(" {}", self.print_expression(t)))
            .unwrap_or_default();
        let name_str = handler.name.as_ref().map(|n| format!(" as {}", n)).unwrap_or_default();
        let mut output = format!("{}except{}{}:\n", self.indent(), type_str, name_str);
        self.indent_level += 1;
        for s in &handler.body {
            output.push_str(&self.print_statement(s));
        }
        self.indent_level -= 1;
        output
    }

    fn print_match_case(&mut self, case: &MatchCase) -> String {
        let guard_str = case
            .guard
            .as_ref()
            .map(|g| format!(" if {}", self.print_expression(g)))
            .unwrap_or_default();
        let mut output =
            format!("{}case {}{}:\n", self.indent(), self.print_pattern(&case.pattern), guard_str);
        self.indent_level += 1;
        for s in &case.body {
            output.push_str(&self.print_statement(s));
        }
        self.indent_level -= 1;
        output
    }

    fn print_pattern(&self, pattern: &Pattern) -> String {
        match pattern {
            Pattern::MatchValue { value, .. } => self.print_expression(value),
            Pattern::MatchSingleton { value, .. } => self.print_constant(value),
            Pattern::MatchSequence { patterns, .. } => {
                let patterns_str =
                    patterns.iter().map(|p| self.print_pattern(p)).collect::<Vec<_>>().join(", ");
                format!("[{}]", patterns_str)
            }
            Pattern::MatchMapping {
                keys,
                patterns,
                rest,
                ..
            } => {
                let mut parts: Vec<String> = keys
                    .iter()
                    .zip(patterns.iter())
                    .map(|(k, p)| {
                        format!("{}: {}", self.print_expression(k), self.print_pattern(p))
                    })
                    .collect();
                if let Some(r) = rest {
                    parts.push(format!("**{}", r));
                }
                format!("{{{}}}", parts.join(", "))
            }
            Pattern::MatchClass {
                cls,
                patterns,
                kwd_attrs,
                kwd_patterns,
                ..
            } => {
                let mut parts: Vec<String> =
                    patterns.iter().map(|p| self.print_pattern(p)).collect();
                for (attr, pat) in kwd_attrs.iter().zip(kwd_patterns.iter()) {
                    parts.push(format!("{}={}", attr, self.print_pattern(pat)));
                }
                format!("{}({})", self.print_expression(cls), parts.join(", "))
            }
            Pattern::MatchStar { name, .. } => match name {
                Some(n) => format!("*{}", n),
                None => "*_".to_string(),
            },
            Pattern::MatchAs { pattern, name, .. } => match (pattern, name) {
                (None, None) => "_".to_string(),
                (None, Some(n)) => n.clone(),
                (Some(p), Some(n)) => format!("{} as {}", self.print_pattern(p), n),
                (Some(p), None) => self.print_pattern(p),
            },
            Pattern::MatchOr { patterns, .. } => {
                patterns.iter().map(|p| self.print_pattern(p)).collect::<Vec<_>>().join(" | ")
            }
        }
    }

    /// Print an expression to source code
    pub fn print_expression(&self, expr: &Expression) -> String {
        match expr {
            Expression::BoolOp { op, values, .. } => {
                let op_str = match op {
                    BoolOp::And => " and ",
                    BoolOp::Or => " or ",
                };
                values.iter().map(|v| self.print_expression(v)).collect::<Vec<_>>().join(op_str)
            }

            Expression::NamedExpr { target, value, .. } => {
                format!("({} := {})", self.print_expression(target), self.print_expression(value))
            }

            Expression::BinOp {
                left, op, right, ..
            } => {
                format!(
                    "({} {} {})",
                    self.print_expression(left),
                    self.print_binop(op),
                    self.print_expression(right)
                )
            }

            Expression::UnaryOp { op, operand, .. } => {
                let op_str = match op {
                    UnaryOp::Invert => "~",
                    UnaryOp::Not => "not ",
                    UnaryOp::UAdd => "+",
                    UnaryOp::USub => "-",
                };
                format!("{}{}", op_str, self.print_expression(operand))
            }

            Expression::Lambda { args, body, .. } => {
                format!("lambda {}: {}", self.print_arguments(args), self.print_expression(body))
            }

            Expression::IfExp {
                test, body, orelse, ..
            } => {
                format!(
                    "({} if {} else {})",
                    self.print_expression(body),
                    self.print_expression(test),
                    self.print_expression(orelse)
                )
            }

            Expression::Dict { keys, values, .. } => {
                let pairs: Vec<String> = keys
                    .iter()
                    .zip(values.iter())
                    .map(|(k, v)| match k {
                        Some(key) => {
                            format!("{}: {}", self.print_expression(key), self.print_expression(v))
                        }
                        None => format!("**{}", self.print_expression(v)),
                    })
                    .collect();
                format!("{{{}}}", pairs.join(", "))
            }

            Expression::Set { elts, .. } => {
                let elts_str =
                    elts.iter().map(|e| self.print_expression(e)).collect::<Vec<_>>().join(", ");
                format!("{{{}}}", elts_str)
            }

            Expression::ListComp {
                elt, generators, ..
            } => {
                format!(
                    "[{} {}]",
                    self.print_expression(elt),
                    self.print_comprehensions(generators)
                )
            }

            Expression::SetComp {
                elt, generators, ..
            } => {
                format!(
                    "{{{} {}}}",
                    self.print_expression(elt),
                    self.print_comprehensions(generators)
                )
            }

            Expression::DictComp {
                key,
                value,
                generators,
                ..
            } => {
                format!(
                    "{{{}: {} {}}}",
                    self.print_expression(key),
                    self.print_expression(value),
                    self.print_comprehensions(generators)
                )
            }

            Expression::GeneratorExp {
                elt, generators, ..
            } => {
                format!(
                    "({} {})",
                    self.print_expression(elt),
                    self.print_comprehensions(generators)
                )
            }

            Expression::Await { value, .. } => {
                format!("await {}", self.print_expression(value))
            }

            Expression::Yield { value, .. } => match value {
                Some(v) => format!("yield {}", self.print_expression(v)),
                None => "yield".to_string(),
            },

            Expression::YieldFrom { value, .. } => {
                format!("yield from {}", self.print_expression(value))
            }

            Expression::Compare {
                left,
                ops,
                comparators,
                ..
            } => {
                let mut result = self.print_expression(left);
                for (op, comp) in ops.iter().zip(comparators.iter()) {
                    result.push_str(&format!(
                        " {} {}",
                        self.print_cmpop(op),
                        self.print_expression(comp)
                    ));
                }
                result
            }

            Expression::Call {
                func,
                args,
                keywords,
                ..
            } => {
                let mut parts: Vec<String> =
                    args.iter().map(|a| self.print_expression(a)).collect();
                for kw in keywords {
                    if let Some(arg) = &kw.arg {
                        parts.push(format!("{}={}", arg, self.print_expression(&kw.value)));
                    } else {
                        parts.push(format!("**{}", self.print_expression(&kw.value)));
                    }
                }
                format!("{}({})", self.print_expression(func), parts.join(", "))
            }

            Expression::FormattedValue {
                value,
                conversion,
                format_spec,
                ..
            } => {
                let conv_str = conversion.map(|c| format!("!{}", c)).unwrap_or_default();
                let spec_str = format_spec
                    .as_ref()
                    .map(|s| format!(":{}", self.print_expression(s)))
                    .unwrap_or_default();
                format!("{{{}{}{}}}", self.print_expression(value), conv_str, spec_str)
            }

            Expression::JoinedStr { values, .. } => {
                let parts: Vec<String> = values
                    .iter()
                    .map(|v| match v {
                        Expression::Constant {
                            value: Constant::Str(s),
                            ..
                        } => s.clone(),
                        _ => self.print_expression(v),
                    })
                    .collect();
                format!("f\"{}\"", parts.join(""))
            }

            Expression::Constant { value, .. } => self.print_constant(value),

            Expression::Attribute { value, attr, .. } => {
                format!("{}.{}", self.print_expression(value), attr)
            }

            Expression::Subscript { value, slice, .. } => {
                format!("{}[{}]", self.print_expression(value), self.print_expression(slice))
            }

            Expression::Starred { value, .. } => {
                format!("*{}", self.print_expression(value))
            }

            Expression::Name { id, .. } => id.clone(),

            Expression::List { elts, .. } => {
                let elts_str =
                    elts.iter().map(|e| self.print_expression(e)).collect::<Vec<_>>().join(", ");
                format!("[{}]", elts_str)
            }

            Expression::Tuple { elts, .. } => {
                if elts.is_empty() {
                    "()".to_string()
                } else if elts.len() == 1 {
                    format!("({},)", self.print_expression(&elts[0]))
                } else {
                    let elts_str = elts
                        .iter()
                        .map(|e| self.print_expression(e))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("({})", elts_str)
                }
            }

            Expression::Slice {
                lower, upper, step, ..
            } => {
                let lower_str =
                    lower.as_ref().map(|l| self.print_expression(l)).unwrap_or_default();
                let upper_str =
                    upper.as_ref().map(|u| self.print_expression(u)).unwrap_or_default();
                match step {
                    Some(s) => format!("{}:{}:{}", lower_str, upper_str, self.print_expression(s)),
                    None => format!("{}:{}", lower_str, upper_str),
                }
            }
        }
    }

    fn print_constant(&self, constant: &Constant) -> String {
        match constant {
            Constant::None => "None".to_string(),
            Constant::Bool(true) => "True".to_string(),
            Constant::Bool(false) => "False".to_string(),
            Constant::Int(n) => n.to_string(),
            Constant::Float(n) => {
                let s = n.to_string();
                if s.contains('.') {
                    s
                } else {
                    format!("{}.0", s)
                }
            }
            Constant::Complex { real, imag } => format!("{}+{}j", real, imag),
            Constant::Str(s) => format!(
                "\"{}\"",
                s.replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\t', "\\t")
            ),
            Constant::Bytes(b) => format!("b\"{}\"", String::from_utf8_lossy(b)),
            Constant::Ellipsis => "...".to_string(),
        }
    }

    fn print_binop(&self, op: &BinOp) -> String {
        match op {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mult => "*",
            BinOp::MatMult => "@",
            BinOp::Div => "/",
            BinOp::Mod => "%",
            BinOp::Pow => "**",
            BinOp::LShift => "<<",
            BinOp::RShift => ">>",
            BinOp::BitOr => "|",
            BinOp::BitXor => "^",
            BinOp::BitAnd => "&",
            BinOp::FloorDiv => "//",
        }
        .to_string()
    }

    fn print_cmpop(&self, op: &CmpOp) -> String {
        match op {
            CmpOp::Eq => "==",
            CmpOp::NotEq => "!=",
            CmpOp::Lt => "<",
            CmpOp::LtE => "<=",
            CmpOp::Gt => ">",
            CmpOp::GtE => ">=",
            CmpOp::Is => "is",
            CmpOp::IsNot => "is not",
            CmpOp::In => "in",
            CmpOp::NotIn => "not in",
        }
        .to_string()
    }

    fn print_arguments(&self, args: &Arguments) -> String {
        let mut parts = Vec::new();

        // Positional-only args
        for arg in &args.posonlyargs {
            parts.push(self.print_arg(arg));
        }
        if !args.posonlyargs.is_empty() {
            parts.push("/".to_string());
        }

        // Regular args with defaults
        let num_defaults = args.defaults.len();
        let num_args = args.args.len();
        for (i, arg) in args.args.iter().enumerate() {
            let default_idx = i as isize - (num_args as isize - num_defaults as isize);
            if default_idx >= 0 {
                parts.push(format!(
                    "{}={}",
                    self.print_arg(arg),
                    self.print_expression(&args.defaults[default_idx as usize])
                ));
            } else {
                parts.push(self.print_arg(arg));
            }
        }

        // *args
        if let Some(vararg) = &args.vararg {
            parts.push(format!("*{}", self.print_arg(vararg)));
        } else if !args.kwonlyargs.is_empty() {
            parts.push("*".to_string());
        }

        // Keyword-only args
        for (i, arg) in args.kwonlyargs.iter().enumerate() {
            if let Some(Some(default)) = args.kw_defaults.get(i) {
                parts.push(format!("{}={}", self.print_arg(arg), self.print_expression(default)));
            } else {
                parts.push(self.print_arg(arg));
            }
        }

        // **kwargs
        if let Some(kwarg) = &args.kwarg {
            parts.push(format!("**{}", self.print_arg(kwarg)));
        }

        parts.join(", ")
    }

    fn print_arg(&self, arg: &Arg) -> String {
        match &arg.annotation {
            Some(ann) => format!("{}: {}", arg.arg, self.print_expression(ann)),
            None => arg.arg.clone(),
        }
    }

    fn print_alias(&self, alias: &Alias) -> String {
        match &alias.asname {
            Some(asname) => format!("{} as {}", alias.name, asname),
            None => alias.name.clone(),
        }
    }

    fn print_comprehensions(&self, generators: &[Comprehension]) -> String {
        generators
            .iter()
            .map(|gen| {
                let async_str = if gen.is_async { "async " } else { "" };
                let ifs_str = gen
                    .ifs
                    .iter()
                    .map(|i| format!(" if {}", self.print_expression(i)))
                    .collect::<Vec<_>>()
                    .join("");
                format!(
                    "{}for {} in {}{}",
                    async_str,
                    self.print_expression(&gen.target),
                    self.print_expression(&gen.iter),
                    ifs_str
                )
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Print a module to Python source code
pub fn print_module(module: &Module) -> String {
    let mut printer = Printer::new();
    printer.print_module(module)
}

/// Print an expression to Python source code
pub fn print_expression(expr: &Expression) -> String {
    let printer = Printer::new();
    printer.print_expression(expr)
}

/// Print a statement to Python source code
pub fn print_statement(stmt: &Statement) -> String {
    let mut printer = Printer::new();
    printer.print_statement(stmt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_module;

    #[test]
    fn test_print_simple_function() {
        let source = "def foo(x):\n    return x\n";
        let module = parse_module(source).unwrap();
        let printed = print_module(&module);
        assert!(printed.contains("def foo(x):"));
        assert!(printed.contains("return x"));
    }

    #[test]
    fn test_print_class() {
        let source = "class Foo:\n    pass\n";
        let module = parse_module(source).unwrap();
        let printed = print_module(&module);
        assert!(printed.contains("class Foo:"));
        assert!(printed.contains("pass"));
    }

    #[test]
    fn test_print_if_statement() {
        let source = "if x:\n    y = 1\nelse:\n    y = 2\n";
        let module = parse_module(source).unwrap();
        let printed = print_module(&module);
        assert!(printed.contains("if x:"));
        assert!(printed.contains("else:"));
    }
}
