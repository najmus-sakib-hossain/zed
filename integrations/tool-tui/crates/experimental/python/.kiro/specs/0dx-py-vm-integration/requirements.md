
# Requirements Document

## Introduction

DX-Py has excellent architecture with a tiered JIT compiler, SIMD-accelerated operations, lock-free GC, and comprehensive test infrastructure. However, the runtime interpreter doesn't execute Python code properly - the components exist but aren't wired together correctly. This specification defines the requirements to integrate the existing components into a working Python runtime that can: -Execute basic Python programs (variables, functions, loops, conditionals) -Run the test-runner against real pytest test suites -Serve as a drop-in replacement for CPython in common workflows The goal is NOT to rewrite the runtime, but to wire together the existing excellent components.

## Glossary

- DX-Py: The Rust-based Python toolchain (runtime, package-manager, test-runner)
- Interpreter: The bytecode execution engine in dx-py-interpreter
- Dispatcher: The component that decodes and executes bytecode instructions
- Frame: Execution context containing local variables, stack, and instruction pointer
- Code_Object: Compiled representation of a Python function or module
- DPB: DX-Py Bytecode format
- VM: Virtual Machine
- the complete execution environment

## Requirements

### Requirement 1: User-Defined Functions

User Story: As a Python developer, I want to define and call my own functions, so that I can run real Python programs.

#### Acceptance Criteria

- WHEN MAKE_FUNCTION opcode is executed, THE Interpreter SHALL create a callable PyFunction from the Code_Object on the stack
- WHEN a PyFunction is called via CALL opcode, THE Interpreter SHALL create a new Frame with arguments bound to local variables
- WHEN RETURN opcode is executed, THE Interpreter SHALL pop the current Frame and push the return value to the caller's stack
- WHEN a function has positional arguments, THE Interpreter SHALL bind them the function's varnames
- WHEN a function has default argument values, THE Interpreter SHALL use defaults for missing arguments
- FOR ALL valid function definitions, defining then calling SHALL return the expected value (round-trip property)

### Requirement 2: Closures and Nested Functions

User Story: As a Python developer, I want nested functions that capture variables from enclosing scopes, so that I can use closures and factory patterns.

#### Acceptance Criteria

- WHEN MAKE_CLOSURE opcode is executed, THE Interpreter SHALL create a PyFunction with captured cell variables
- WHEN LOAD_DEREF opcode is executed, THE Interpreter SHALL load the value from the cell at the specified index
- WHEN STORE_DEREF opcode is executed, THE Interpreter SHALL store the value into the cell at the specified index
- WHEN an enclosing function returns, THE Interpreter SHALL preserve cells referenced by returned closures
- FOR ALL closures, accessing captured variables SHALL return the current value in the cell

### Requirement 3: Class Definitions and Instances

User Story: As a Python developer, I want to define classes and create instances, so that I can use object-oriented programming.

#### Acceptance Criteria

- WHEN BUILD_CLASS opcode is executed, THE Interpreter SHALL create a PyClass from name, bases, and namespace dict
- WHEN a class is called, THE Interpreter SHALL create a PyInstance and call init if defined
- WHEN LOAD_ATTR is executed on an instance, THE Interpreter SHALL check instance dict, then class dict, then base classes
- WHEN STORE_ATTR is executed on an instance, THE Interpreter SHALL store the attribute in the instance dict
- WHEN a method is accessed on an instance, THE Interpreter SHALL return a bound method with self pre-filled
- WHEN LOAD_METHOD/CALL_METHOD opcodes are executed, THE Interpreter SHALL efficiently call methods without creating bound method objects

### Requirement 4: Module Import System

User Story: As a Python developer, I want to import modules from files, so that I can organize code across multiple files.

#### Acceptance Criteria

- WHEN IMPORT_NAME opcode is executed, THE Interpreter SHALL locate and load the module using the import system
- WHEN a module is first imported, THE Interpreter SHALL compile and execute it in a new namespace
- WHEN a module has already been imported, THE Interpreter SHALL return the cached module from sys.modules
- WHEN IMPORT_FROM opcode is executed, THE Interpreter SHALL retrieve the named attribute from the module
- WHEN a relative import is used, THE Interpreter SHALL resolve it relative to the current package
- FOR ALL importable modules, importing then accessing attributes SHALL return the expected values

### Requirement 5: List Comprehensions and Generator Expressions

User Story: As a Python developer, I want to use list comprehensions, so that I can write concise data transformations.

#### Acceptance Criteria

- WHEN a list comprehension is compiled, THE Compiler SHALL generate inline loop bytecode (not a nested function)
- WHEN LIST_APPEND opcode is executed inside a comprehension, THE Interpreter SHALL append the value to the list being built
- WHEN a comprehension has an 'if' clause, THE Interpreter SHALL only include elements that pass the condition
- WHEN a comprehension has multiple 'for' clauses, THE Interpreter SHALL nest the loops correctly
- FOR ALL list comprehensions, the result SHALL equal the equivalent explicit loop

### Requirement 6: Exception Handling Integration

User Story: As a Python developer, I want try/except/finally to work correctly, so that I can handle errors gracefully.

#### Acceptance Criteria

- WHEN SETUP_EXCEPT opcode is executed, THE Interpreter SHALL push an exception handler block
- WHEN an exception is raised, THE Interpreter SHALL unwind to the nearest matching handler
- WHEN an except clause catches an exception, THE Interpreter SHALL bind it to the target variable if specified
- WHEN a finally block exists, THE Interpreter SHALL execute it whether or not an exception occurred
- WHEN an exception propagates past all handlers, THE Interpreter SHALL print a traceback with file/line info

### Requirement 7: Context Managers (with statement)

User Story: As a Python developer, I want 'with' statements to work, so that I can use context managers for resource management.

#### Acceptance Criteria

- WHEN BEFORE_WITH opcode is executed, THE Interpreter SHALL call enter on the context manager
- WHEN the with block exits normally, THE Interpreter SHALL call exit with None arguments
- WHEN an exception occurs in the with block, THE Interpreter SHALL call exit with exception info
- IF exit returns True, THEN THE Interpreter SHALL suppress the exception
- WHEN 'as' is used, THE Interpreter SHALL bind the return value of enter to the target

### Requirement 8: Decorators

User Story: As a Python developer, I want decorators to work, so that I can use @pytest.fixture and similar patterns.

#### Acceptance Criteria

- WHEN a function has @decorator syntax, THE Compiler SHALL generate code to call decorator(function)
- WHEN multiple decorators are stacked, THE Interpreter SHALL apply them bottom-to-top
- WHEN a class has @decorator syntax, THE Compiler SHALL generate code to call decorator(class)
- FOR ALL decorated functions, the result SHALL be the return value of the outermost decorator

### Requirement 9: Builtin Functions Completeness

User Story: As a Python developer, I want common builtin functions to work, so that I can write idiomatic Python.

#### Acceptance Criteria

- THE Interpreter SHALL provide isinstance() and issubclass() for type checking
- THE Interpreter SHALL provide getattr(), setattr(), hasattr(), delattr() for attribute access
- THE Interpreter SHALL provide super() for method resolution in inheritance
- THE Interpreter SHALL provide property() for descriptor-based attributes
- THE Interpreter SHALL provide staticmethod() and classmethod() for method types
- THE Interpreter SHALL provide enumerate(), zip(), map(), filter() for iteration
- THE Interpreter SHALL provide sorted(), reversed(), min(), max(), sum() for sequences
- THE Interpreter SHALL provide open() for file I/O

### Requirement 10: Test Runner Integration

User Story: As a Python developer, I want to run pytest test suites with DX-Py, so that I can benefit from faster test execution.

#### Acceptance Criteria

- WHEN the test-runner discovers tests, THE Runtime SHALL be able to import and execute the test files
- WHEN a test uses @pytest.fixture, THE Runtime SHALL correctly apply the decorator
- WHEN a test uses assert statements, THE Runtime SHALL provide detailed failure messages
- WHEN a test uses pytest.raises(), THE Runtime SHALL correctly handle the context manager
- WHEN running tests in parallel, THE Runtime SHALL isolate test execution correctly

### Requirement 11: Real-World Package Compatibility

User Story: As a Python developer, I want to import and use common packages, so that DX-Py is useful for real projects.

#### Acceptance Criteria

- THE Runtime SHALL be able to import and use the 'json' standard library module
- THE Runtime SHALL be able to import and use the 'os' and 'sys' standard library modules
- THE Runtime SHALL be able to import and use the 'pathlib' standard library module
- THE Runtime SHALL be able to import and use the 'collections' standard library module
- WHEN a pure-Python package is installed, THE Runtime SHALL be able to import and use it
