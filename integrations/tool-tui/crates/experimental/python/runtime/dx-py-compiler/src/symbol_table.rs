//! Symbol Table for Python Name Resolution
//!
//! This module implements Python's scoping rules for variable resolution.
//! It tracks local, global, free (closure), and cell variables across
//! nested function and class scopes.

use dx_py_parser::{Arguments, Comprehension, Expression, Module, Statement};
use std::collections::{HashMap, HashSet};

/// Symbol binding type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Symbol {
    /// Local variable with index in locals array
    Local { index: u16 },
    /// Global variable (module-level)
    Global { name: String },
    /// Free variable (captured from enclosing scope)
    Free { index: u16 },
    /// Cell variable (captured by nested function)
    Cell { index: u16 },
    /// Builtin name
    Builtin { name: String },
}

/// Type of scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeType {
    /// Module-level scope
    Module,
    /// Function scope
    Function,
    /// Class scope
    Class,
    /// Comprehension scope (list/dict/set/generator)
    Comprehension,
    /// Lambda scope
    Lambda,
}

/// Flags for symbol properties
#[derive(Debug, Clone, Copy, Default)]
pub struct SymbolFlags {
    /// Explicitly declared global
    pub is_global: bool,
    /// Explicitly declared nonlocal
    pub is_nonlocal: bool,
    /// Assigned in this scope
    pub is_assigned: bool,
    /// Referenced (read) in this scope
    pub is_referenced: bool,
    /// Is a parameter
    pub is_parameter: bool,
    /// Is captured by nested scope (becomes cell)
    pub is_cell: bool,
    /// Is a free variable from enclosing scope
    pub is_free: bool,
}

/// A single scope in the symbol table
#[derive(Debug, Clone)]
pub struct Scope {
    /// Scope type
    pub scope_type: ScopeType,
    /// Scope name (function/class name or "<module>")
    pub name: String,
    /// Symbol name -> flags
    symbols: HashMap<String, SymbolFlags>,
    /// Local variable names in order
    pub locals: Vec<String>,
    /// Free variable names in order
    pub free_vars: Vec<String>,
    /// Cell variable names in order
    pub cell_vars: Vec<String>,
    /// Names explicitly declared global
    pub explicit_globals: HashSet<String>,
    /// Names explicitly declared nonlocal
    pub explicit_nonlocals: HashSet<String>,
    /// Child scopes (nested functions/classes)
    pub children: Vec<Scope>,
}

impl Scope {
    /// Create a new scope
    pub fn new(scope_type: ScopeType, name: String) -> Self {
        Self {
            scope_type,
            name,
            symbols: HashMap::new(),
            locals: Vec::new(),
            free_vars: Vec::new(),
            cell_vars: Vec::new(),
            explicit_globals: HashSet::new(),
            explicit_nonlocals: HashSet::new(),
            children: Vec::new(),
        }
    }

    /// Add or update a symbol
    pub fn add_symbol(&mut self, name: &str, flags: SymbolFlags) {
        self.symbols
            .entry(name.to_string())
            .and_modify(|f| {
                f.is_global |= flags.is_global;
                f.is_nonlocal |= flags.is_nonlocal;
                f.is_assigned |= flags.is_assigned;
                f.is_referenced |= flags.is_referenced;
                f.is_parameter |= flags.is_parameter;
            })
            .or_insert(flags);
    }

    /// Mark a name as assigned
    pub fn mark_assigned(&mut self, name: &str) {
        let flags = self.symbols.entry(name.to_string()).or_default();
        flags.is_assigned = true;
    }

    /// Mark a name as referenced
    pub fn mark_referenced(&mut self, name: &str) {
        let flags = self.symbols.entry(name.to_string()).or_default();
        flags.is_referenced = true;
    }

    /// Mark a name as global
    pub fn mark_global(&mut self, name: &str) {
        self.explicit_globals.insert(name.to_string());
        let flags = self.symbols.entry(name.to_string()).or_default();
        flags.is_global = true;
    }

    /// Mark a name as nonlocal
    pub fn mark_nonlocal(&mut self, name: &str) {
        self.explicit_nonlocals.insert(name.to_string());
        let flags = self.symbols.entry(name.to_string()).or_default();
        flags.is_nonlocal = true;
    }

    /// Mark a name as a parameter
    pub fn mark_parameter(&mut self, name: &str) {
        let flags = self.symbols.entry(name.to_string()).or_default();
        flags.is_parameter = true;
        flags.is_assigned = true;
    }

    /// Get symbol flags for a name
    pub fn get_symbol(&self, name: &str) -> Option<&SymbolFlags> {
        self.symbols.get(name)
    }

    /// Check if a name is local to this scope
    pub fn is_local(&self, name: &str) -> bool {
        if let Some(flags) = self.symbols.get(name) {
            !flags.is_global && !flags.is_nonlocal && (flags.is_assigned || flags.is_parameter)
        } else {
            false
        }
    }

    /// Get the local variable index for a name
    pub fn get_local_index(&self, name: &str) -> Option<u16> {
        self.locals.iter().position(|n| n == name).map(|i| i as u16)
    }

    /// Get the free variable index for a name
    pub fn get_free_index(&self, name: &str) -> Option<u16> {
        self.free_vars.iter().position(|n| n == name).map(|i| i as u16)
    }

    /// Get the cell variable index for a name
    pub fn get_cell_index(&self, name: &str) -> Option<u16> {
        self.cell_vars.iter().position(|n| n == name).map(|i| i as u16)
    }
}

/// Symbol table for an entire module
#[derive(Debug)]
pub struct SymbolTable {
    /// Stack of scopes during analysis
    scope_stack: Vec<Scope>,
    /// Completed root scope (module level)
    pub root: Option<Scope>,
    /// Set of Python builtins
    builtins: HashSet<String>,
}

impl SymbolTable {
    /// Create a new symbol table
    pub fn new() -> Self {
        Self {
            scope_stack: Vec::new(),
            root: None,
            builtins: Self::default_builtins(),
        }
    }

    /// Get the default Python builtins
    fn default_builtins() -> HashSet<String> {
        [
            "abs",
            "all",
            "any",
            "ascii",
            "bin",
            "bool",
            "breakpoint",
            "bytearray",
            "bytes",
            "callable",
            "chr",
            "classmethod",
            "compile",
            "complex",
            "delattr",
            "dict",
            "dir",
            "divmod",
            "enumerate",
            "eval",
            "exec",
            "filter",
            "float",
            "format",
            "frozenset",
            "getattr",
            "globals",
            "hasattr",
            "hash",
            "help",
            "hex",
            "id",
            "input",
            "int",
            "isinstance",
            "issubclass",
            "iter",
            "len",
            "list",
            "locals",
            "map",
            "max",
            "memoryview",
            "min",
            "next",
            "object",
            "oct",
            "open",
            "ord",
            "pow",
            "print",
            "property",
            "range",
            "repr",
            "reversed",
            "round",
            "set",
            "setattr",
            "slice",
            "sorted",
            "staticmethod",
            "str",
            "sum",
            "super",
            "tuple",
            "type",
            "vars",
            "zip",
            "__import__",
            // Exceptions
            "BaseException",
            "Exception",
            "ArithmeticError",
            "AssertionError",
            "AttributeError",
            "BlockingIOError",
            "BrokenPipeError",
            "BufferError",
            "BytesWarning",
            "ChildProcessError",
            "ConnectionAbortedError",
            "ConnectionError",
            "ConnectionRefusedError",
            "ConnectionResetError",
            "DeprecationWarning",
            "EOFError",
            "EnvironmentError",
            "FileExistsError",
            "FileNotFoundError",
            "FloatingPointError",
            "FutureWarning",
            "GeneratorExit",
            "IOError",
            "ImportError",
            "ImportWarning",
            "IndentationError",
            "IndexError",
            "InterruptedError",
            "IsADirectoryError",
            "KeyError",
            "KeyboardInterrupt",
            "LookupError",
            "MemoryError",
            "ModuleNotFoundError",
            "NameError",
            "NotADirectoryError",
            "NotImplemented",
            "NotImplementedError",
            "OSError",
            "OverflowError",
            "PendingDeprecationWarning",
            "PermissionError",
            "ProcessLookupError",
            "RecursionError",
            "ReferenceError",
            "ResourceWarning",
            "RuntimeError",
            "RuntimeWarning",
            "StopAsyncIteration",
            "StopIteration",
            "SyntaxError",
            "SyntaxWarning",
            "SystemError",
            "SystemExit",
            "TabError",
            "TimeoutError",
            "TypeError",
            "UnboundLocalError",
            "UnicodeDecodeError",
            "UnicodeEncodeError",
            "UnicodeError",
            "UnicodeTranslateError",
            "UnicodeWarning",
            "UserWarning",
            "ValueError",
            "Warning",
            "ZeroDivisionError",
            // Constants
            "True",
            "False",
            "None",
            "Ellipsis",
            "__debug__",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    /// Check if a name is a builtin
    pub fn is_builtin(&self, name: &str) -> bool {
        self.builtins.contains(name)
    }

    /// Enter a new scope
    pub fn enter_scope(&mut self, scope_type: ScopeType, name: String) {
        self.scope_stack.push(Scope::new(scope_type, name));
    }

    /// Exit the current scope
    pub fn exit_scope(&mut self) -> Option<Scope> {
        self.scope_stack.pop()
    }

    /// Get the current scope
    pub fn current_scope(&self) -> Option<&Scope> {
        self.scope_stack.last()
    }

    /// Get the current scope mutably
    pub fn current_scope_mut(&mut self) -> Option<&mut Scope> {
        self.scope_stack.last_mut()
    }

    /// Analyze a module and build the symbol table
    pub fn analyze_module(&mut self, module: &Module) -> Result<(), SymbolError> {
        self.enter_scope(ScopeType::Module, "<module>".to_string());

        // First pass: collect all definitions
        for stmt in &module.body {
            self.analyze_statement(stmt)?;
        }

        // Second pass: resolve free variables
        self.resolve_free_variables()?;

        // Finalize
        self.root = self.exit_scope();
        Ok(())
    }

    /// Analyze a statement
    fn analyze_statement(&mut self, stmt: &Statement) -> Result<(), SymbolError> {
        match stmt {
            Statement::FunctionDef {
                name,
                args,
                body,
                decorators,
                ..
            } => {
                // Function name is defined in current scope
                self.current_scope_mut().unwrap().mark_assigned(name);

                // Analyze decorators in current scope
                for dec in decorators {
                    self.analyze_expression(dec)?;
                }

                // Enter function scope
                self.enter_scope(ScopeType::Function, name.clone());

                // Add parameters
                self.analyze_arguments(args)?;

                // Analyze body
                for s in body {
                    self.analyze_statement(s)?;
                }

                // Exit and add as child
                let func_scope = self.exit_scope().unwrap();
                self.current_scope_mut().unwrap().children.push(func_scope);
            }

            Statement::ClassDef {
                name,
                bases,
                keywords,
                body,
                decorators,
                ..
            } => {
                // Class name is defined in current scope
                self.current_scope_mut().unwrap().mark_assigned(name);

                // Analyze decorators and bases in current scope
                for dec in decorators {
                    self.analyze_expression(dec)?;
                }
                for base in bases {
                    self.analyze_expression(base)?;
                }
                for kw in keywords {
                    self.analyze_expression(&kw.value)?;
                }

                // Enter class scope
                self.enter_scope(ScopeType::Class, name.clone());

                // Analyze body
                for s in body {
                    self.analyze_statement(s)?;
                }

                // Exit and add as child
                let class_scope = self.exit_scope().unwrap();
                self.current_scope_mut().unwrap().children.push(class_scope);
            }

            Statement::Assign { targets, value, .. } => {
                self.analyze_expression(value)?;
                for target in targets {
                    self.analyze_assignment_target(target)?;
                }
            }

            Statement::AugAssign { target, value, .. } => {
                self.analyze_expression(target)?;
                self.analyze_expression(value)?;
                self.analyze_assignment_target(target)?;
            }

            Statement::AnnAssign {
                target,
                annotation,
                value,
                ..
            } => {
                self.analyze_expression(annotation)?;
                if let Some(v) = value {
                    self.analyze_expression(v)?;
                }
                self.analyze_assignment_target(target)?;
            }

            Statement::For {
                target,
                iter,
                body,
                orelse,
                ..
            } => {
                self.analyze_expression(iter)?;
                self.analyze_assignment_target(target)?;
                for s in body {
                    self.analyze_statement(s)?;
                }
                for s in orelse {
                    self.analyze_statement(s)?;
                }
            }

            Statement::While {
                test, body, orelse, ..
            } => {
                self.analyze_expression(test)?;
                for s in body {
                    self.analyze_statement(s)?;
                }
                for s in orelse {
                    self.analyze_statement(s)?;
                }
            }

            Statement::If {
                test, body, orelse, ..
            } => {
                self.analyze_expression(test)?;
                for s in body {
                    self.analyze_statement(s)?;
                }
                for s in orelse {
                    self.analyze_statement(s)?;
                }
            }

            Statement::With { items, body, .. } => {
                for item in items {
                    self.analyze_expression(&item.context_expr)?;
                    if let Some(vars) = &item.optional_vars {
                        self.analyze_assignment_target(vars)?;
                    }
                }
                for s in body {
                    self.analyze_statement(s)?;
                }
            }

            Statement::Try {
                body,
                handlers,
                orelse,
                finalbody,
                ..
            } => {
                for s in body {
                    self.analyze_statement(s)?;
                }
                for handler in handlers {
                    if let Some(typ) = &handler.typ {
                        self.analyze_expression(typ)?;
                    }
                    if let Some(name) = &handler.name {
                        self.current_scope_mut().unwrap().mark_assigned(name);
                    }
                    for s in &handler.body {
                        self.analyze_statement(s)?;
                    }
                }
                for s in orelse {
                    self.analyze_statement(s)?;
                }
                for s in finalbody {
                    self.analyze_statement(s)?;
                }
            }

            Statement::Raise { exc, cause, .. } => {
                if let Some(e) = exc {
                    self.analyze_expression(e)?;
                }
                if let Some(c) = cause {
                    self.analyze_expression(c)?;
                }
            }

            Statement::Assert { test, msg, .. } => {
                self.analyze_expression(test)?;
                if let Some(m) = msg {
                    self.analyze_expression(m)?;
                }
            }

            Statement::Import { names, .. } => {
                for alias in names {
                    let name = alias.asname.as_ref().unwrap_or(&alias.name);
                    // For "import a.b.c", only "a" is bound
                    let bound_name = name.split('.').next().unwrap_or(name);
                    self.current_scope_mut().unwrap().mark_assigned(bound_name);
                }
            }

            Statement::ImportFrom { names, .. } => {
                for alias in names {
                    if alias.name == "*" {
                        // import * doesn't bind specific names at compile time
                        continue;
                    }
                    let name = alias.asname.as_ref().unwrap_or(&alias.name);
                    self.current_scope_mut().unwrap().mark_assigned(name);
                }
            }

            Statement::Global { names, .. } => {
                for name in names {
                    self.current_scope_mut().unwrap().mark_global(name);
                }
            }

            Statement::Nonlocal { names, .. } => {
                for name in names {
                    self.current_scope_mut().unwrap().mark_nonlocal(name);
                }
            }

            Statement::Expr { value, .. } => {
                self.analyze_expression(value)?;
            }

            Statement::Return { value, .. } => {
                if let Some(v) = value {
                    self.analyze_expression(v)?;
                }
            }

            Statement::Delete { targets, .. } => {
                for target in targets {
                    self.analyze_expression(target)?;
                }
            }

            Statement::Match { subject, cases, .. } => {
                self.analyze_expression(subject)?;
                for case in cases {
                    self.analyze_pattern(&case.pattern)?;
                    if let Some(guard) = &case.guard {
                        self.analyze_expression(guard)?;
                    }
                    for s in &case.body {
                        self.analyze_statement(s)?;
                    }
                }
            }

            Statement::Pass { .. } | Statement::Break { .. } | Statement::Continue { .. } => {}
        }
        Ok(())
    }

    /// Analyze function arguments
    fn analyze_arguments(&mut self, args: &Arguments) -> Result<(), SymbolError> {
        // Positional-only args
        for arg in &args.posonlyargs {
            self.current_scope_mut().unwrap().mark_parameter(&arg.arg);
        }
        // Regular args
        for arg in &args.args {
            self.current_scope_mut().unwrap().mark_parameter(&arg.arg);
        }
        // *args
        if let Some(vararg) = &args.vararg {
            self.current_scope_mut().unwrap().mark_parameter(&vararg.arg);
        }
        // Keyword-only args
        for arg in &args.kwonlyargs {
            self.current_scope_mut().unwrap().mark_parameter(&arg.arg);
        }
        // **kwargs
        if let Some(kwarg) = &args.kwarg {
            self.current_scope_mut().unwrap().mark_parameter(&kwarg.arg);
        }
        // Default values are evaluated in enclosing scope
        // (handled during compilation, not symbol analysis)
        Ok(())
    }

    /// Analyze an expression
    fn analyze_expression(&mut self, expr: &Expression) -> Result<(), SymbolError> {
        match expr {
            Expression::Name { id, .. } => {
                self.current_scope_mut().unwrap().mark_referenced(id);
            }

            Expression::BoolOp { values, .. } => {
                for v in values {
                    self.analyze_expression(v)?;
                }
            }

            Expression::NamedExpr { target, value, .. } => {
                self.analyze_expression(value)?;
                self.analyze_assignment_target(target)?;
            }

            Expression::BinOp { left, right, .. } => {
                self.analyze_expression(left)?;
                self.analyze_expression(right)?;
            }

            Expression::UnaryOp { operand, .. } => {
                self.analyze_expression(operand)?;
            }

            Expression::Lambda { args, body, .. } => {
                self.enter_scope(ScopeType::Lambda, "<lambda>".to_string());
                self.analyze_arguments(args)?;
                self.analyze_expression(body)?;
                let lambda_scope = self.exit_scope().unwrap();
                self.current_scope_mut().unwrap().children.push(lambda_scope);
            }

            Expression::IfExp {
                test, body, orelse, ..
            } => {
                self.analyze_expression(test)?;
                self.analyze_expression(body)?;
                self.analyze_expression(orelse)?;
            }

            Expression::Dict { keys, values, .. } => {
                for k in keys.iter().flatten() {
                    self.analyze_expression(k)?;
                }
                for v in values {
                    self.analyze_expression(v)?;
                }
            }

            Expression::Set { elts, .. }
            | Expression::List { elts, .. }
            | Expression::Tuple { elts, .. } => {
                for e in elts {
                    self.analyze_expression(e)?;
                }
            }

            Expression::ListComp {
                elt, generators, ..
            } => {
                self.analyze_comprehension(elt, generators)?;
            }

            Expression::SetComp {
                elt, generators, ..
            } => {
                self.analyze_comprehension(elt, generators)?;
            }

            Expression::DictComp {
                key,
                value,
                generators,
                ..
            } => {
                // First generator's iter is in enclosing scope
                if let Some(first) = generators.first() {
                    self.analyze_expression(&first.iter)?;
                }

                self.enter_scope(ScopeType::Comprehension, "<dictcomp>".to_string());

                // First generator's target
                if let Some(first) = generators.first() {
                    self.analyze_assignment_target(&first.target)?;
                    for cond in &first.ifs {
                        self.analyze_expression(cond)?;
                    }
                }

                // Remaining generators
                for gen in generators.iter().skip(1) {
                    self.analyze_expression(&gen.iter)?;
                    self.analyze_assignment_target(&gen.target)?;
                    for cond in &gen.ifs {
                        self.analyze_expression(cond)?;
                    }
                }

                self.analyze_expression(key)?;
                self.analyze_expression(value)?;

                let comp_scope = self.exit_scope().unwrap();
                self.current_scope_mut().unwrap().children.push(comp_scope);
            }

            Expression::GeneratorExp {
                elt, generators, ..
            } => {
                self.analyze_comprehension(elt, generators)?;
            }

            Expression::Await { value, .. } => {
                self.analyze_expression(value)?;
            }

            Expression::Yield { value, .. } => {
                if let Some(v) = value {
                    self.analyze_expression(v)?;
                }
            }

            Expression::YieldFrom { value, .. } => {
                self.analyze_expression(value)?;
            }

            Expression::Compare {
                left, comparators, ..
            } => {
                self.analyze_expression(left)?;
                for c in comparators {
                    self.analyze_expression(c)?;
                }
            }

            Expression::Call {
                func,
                args,
                keywords,
                ..
            } => {
                self.analyze_expression(func)?;
                for a in args {
                    self.analyze_expression(a)?;
                }
                for kw in keywords {
                    self.analyze_expression(&kw.value)?;
                }
            }

            Expression::FormattedValue {
                value, format_spec, ..
            } => {
                self.analyze_expression(value)?;
                if let Some(spec) = format_spec {
                    self.analyze_expression(spec)?;
                }
            }

            Expression::JoinedStr { values, .. } => {
                for v in values {
                    self.analyze_expression(v)?;
                }
            }

            Expression::Attribute { value, .. } => {
                self.analyze_expression(value)?;
            }

            Expression::Subscript { value, slice, .. } => {
                self.analyze_expression(value)?;
                self.analyze_expression(slice)?;
            }

            Expression::Starred { value, .. } => {
                self.analyze_expression(value)?;
            }

            Expression::Slice {
                lower, upper, step, ..
            } => {
                if let Some(l) = lower {
                    self.analyze_expression(l)?;
                }
                if let Some(u) = upper {
                    self.analyze_expression(u)?;
                }
                if let Some(s) = step {
                    self.analyze_expression(s)?;
                }
            }

            Expression::Constant { .. } => {}
        }
        Ok(())
    }

    /// Analyze a comprehension
    fn analyze_comprehension(
        &mut self,
        elt: &Expression,
        generators: &[Comprehension],
    ) -> Result<(), SymbolError> {
        // First generator's iter is evaluated in enclosing scope
        if let Some(first) = generators.first() {
            self.analyze_expression(&first.iter)?;
        }

        self.enter_scope(ScopeType::Comprehension, "<comprehension>".to_string());

        // First generator's target and conditions
        if let Some(first) = generators.first() {
            self.analyze_assignment_target(&first.target)?;
            for cond in &first.ifs {
                self.analyze_expression(cond)?;
            }
        }

        // Remaining generators
        for gen in generators.iter().skip(1) {
            self.analyze_expression(&gen.iter)?;
            self.analyze_assignment_target(&gen.target)?;
            for cond in &gen.ifs {
                self.analyze_expression(cond)?;
            }
        }

        self.analyze_expression(elt)?;

        let comp_scope = self.exit_scope().unwrap();
        self.current_scope_mut().unwrap().children.push(comp_scope);
        Ok(())
    }

    /// Analyze an assignment target
    fn analyze_assignment_target(&mut self, target: &Expression) -> Result<(), SymbolError> {
        match target {
            Expression::Name { id, .. } => {
                self.current_scope_mut().unwrap().mark_assigned(id);
            }
            Expression::Tuple { elts, .. } | Expression::List { elts, .. } => {
                for e in elts {
                    self.analyze_assignment_target(e)?;
                }
            }
            Expression::Starred { value, .. } => {
                self.analyze_assignment_target(value)?;
            }
            Expression::Attribute { value, .. } => {
                self.analyze_expression(value)?;
            }
            Expression::Subscript { value, slice, .. } => {
                self.analyze_expression(value)?;
                self.analyze_expression(slice)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Analyze a pattern (for match statements)
    fn analyze_pattern(&mut self, pattern: &dx_py_parser::Pattern) -> Result<(), SymbolError> {
        use dx_py_parser::Pattern;
        match pattern {
            Pattern::MatchValue { value, .. } => {
                self.analyze_expression(value)?;
            }
            Pattern::MatchSequence { patterns, .. } => {
                for p in patterns {
                    self.analyze_pattern(p)?;
                }
            }
            Pattern::MatchMapping {
                keys,
                patterns,
                rest,
                ..
            } => {
                for k in keys {
                    self.analyze_expression(k)?;
                }
                for p in patterns {
                    self.analyze_pattern(p)?;
                }
                if let Some(name) = rest {
                    self.current_scope_mut().unwrap().mark_assigned(name);
                }
            }
            Pattern::MatchClass {
                cls,
                patterns,
                kwd_patterns,
                ..
            } => {
                self.analyze_expression(cls)?;
                for p in patterns {
                    self.analyze_pattern(p)?;
                }
                for p in kwd_patterns {
                    self.analyze_pattern(p)?;
                }
            }
            Pattern::MatchStar { name, .. } => {
                if let Some(n) = name {
                    self.current_scope_mut().unwrap().mark_assigned(n);
                }
            }
            Pattern::MatchAs { pattern, name, .. } => {
                if let Some(p) = pattern {
                    self.analyze_pattern(p)?;
                }
                if let Some(n) = name {
                    self.current_scope_mut().unwrap().mark_assigned(n);
                }
            }
            Pattern::MatchOr { patterns, .. } => {
                for p in patterns {
                    self.analyze_pattern(p)?;
                }
            }
            Pattern::MatchSingleton { .. } => {}
        }
        Ok(())
    }

    /// Resolve free variables after first pass
    fn resolve_free_variables(&mut self) -> Result<(), SymbolError> {
        if let Some(scope) = self.scope_stack.last_mut() {
            Self::resolve_scope_variables(scope, &[], &self.builtins);
        }
        Ok(())
    }

    /// Recursively resolve variables for a scope
    fn resolve_scope_variables(
        scope: &mut Scope,
        enclosing_locals: &[&HashSet<String>],
        _builtins: &HashSet<String>,
    ) {
        // Build set of local names
        // At module level, assigned names are globals, not locals
        let mut local_names: HashSet<String> = HashSet::new();
        if scope.scope_type != ScopeType::Module {
            for (name, flags) in &scope.symbols {
                if !flags.is_global && !flags.is_nonlocal && (flags.is_assigned || flags.is_parameter) {
                    local_names.insert(name.clone());
                }
            }
        }

        // Determine which names are free (referenced but not local/global)
        let mut free_names: HashSet<String> = HashSet::new();
        for (name, flags) in &scope.symbols {
            if flags.is_referenced && !flags.is_global && !local_names.contains(name) {
                // Check if it's in an enclosing scope
                for enc_locals in enclosing_locals.iter().rev() {
                    if enc_locals.contains(name) {
                        free_names.insert(name.clone());
                        break;
                    }
                }
            }
        }

        // Build locals list (parameters first, then other locals)
        let mut params: Vec<String> = Vec::new();
        let mut other_locals: Vec<String> = Vec::new();
        for (name, flags) in &scope.symbols {
            if local_names.contains(name) {
                if flags.is_parameter {
                    params.push(name.clone());
                } else {
                    other_locals.push(name.clone());
                }
            }
        }
        scope.locals = params;
        scope.locals.extend(other_locals);

        // Set free vars
        scope.free_vars = free_names.into_iter().collect();

        // Build enclosing locals for children
        let mut child_enclosing: Vec<&HashSet<String>> = enclosing_locals.to_vec();
        child_enclosing.push(&local_names);

        // Process children and collect their free vars
        let mut child_free: HashSet<String> = HashSet::new();
        for child in &mut scope.children {
            Self::resolve_scope_variables(child, &child_enclosing, _builtins);
            for name in &child.free_vars {
                if local_names.contains(name) {
                    child_free.insert(name.clone());
                }
            }
        }

        // Names that children need become cell vars
        scope.cell_vars = child_free.into_iter().collect();
    }

    /// Resolve a name in the current scope
    pub fn resolve_name(&self, name: &str) -> Option<Symbol> {
        let scope = self.current_scope()?;

        // Check explicit global
        if scope.explicit_globals.contains(name) {
            return Some(Symbol::Global {
                name: name.to_string(),
            });
        }

        // Check cell var
        if let Some(idx) = scope.get_cell_index(name) {
            return Some(Symbol::Cell { index: idx });
        }

        // Check local
        if let Some(idx) = scope.get_local_index(name) {
            return Some(Symbol::Local { index: idx });
        }

        // Check free var
        if let Some(idx) = scope.get_free_index(name) {
            return Some(Symbol::Free { index: idx });
        }

        // Check builtin
        if self.is_builtin(name) {
            return Some(Symbol::Builtin {
                name: name.to_string(),
            });
        }

        // Default to global
        Some(Symbol::Global {
            name: name.to_string(),
        })
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Symbol table analysis error
#[derive(Debug, thiserror::Error)]
pub enum SymbolError {
    #[error("name '{0}' is used prior to global declaration")]
    GlobalAfterUse(String),

    #[error("name '{0}' is used prior to nonlocal declaration")]
    NonlocalAfterUse(String),

    #[error("no binding for nonlocal '{0}' found")]
    NonlocalNotFound(String),

    #[error("name '{0}' is both global and nonlocal")]
    GlobalAndNonlocal(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use dx_py_parser::parse_module;

    #[test]
    fn test_simple_assignment() {
        let source = "x = 1";
        let module = parse_module(source).unwrap();
        let mut st = SymbolTable::new();
        st.analyze_module(&module).unwrap();

        let root = st.root.unwrap();
        assert!(root.is_local("x"));
    }

    #[test]
    fn test_function_scope() {
        let source = r#"
def foo(a, b):
    x = a + b
    return x
"#;
        let module = parse_module(source).unwrap();
        let mut st = SymbolTable::new();
        st.analyze_module(&module).unwrap();

        let root = st.root.unwrap();
        assert!(root.is_local("foo"));
        assert_eq!(root.children.len(), 1);

        let func_scope = &root.children[0];
        assert_eq!(func_scope.name, "foo");
        assert!(func_scope.locals.contains(&"a".to_string()));
        assert!(func_scope.locals.contains(&"b".to_string()));
        assert!(func_scope.locals.contains(&"x".to_string()));
    }

    #[test]
    fn test_global_declaration() {
        let source = r#"
x = 1
def foo():
    global x
    x = 2
"#;
        let module = parse_module(source).unwrap();
        let mut st = SymbolTable::new();
        st.analyze_module(&module).unwrap();

        let root = st.root.unwrap();
        let func_scope = &root.children[0];
        assert!(func_scope.explicit_globals.contains("x"));
        assert!(!func_scope.is_local("x"));
    }

    #[test]
    fn test_closure() {
        let source = r#"
def outer():
    x = 1
    def inner():
        return x
    return inner
"#;
        let module = parse_module(source).unwrap();
        let mut st = SymbolTable::new();
        st.analyze_module(&module).unwrap();

        let root = st.root.unwrap();
        let outer_scope = &root.children[0];
        assert!(outer_scope.cell_vars.contains(&"x".to_string()));

        let inner_scope = &outer_scope.children[0];
        assert!(inner_scope.free_vars.contains(&"x".to_string()));
    }
}
