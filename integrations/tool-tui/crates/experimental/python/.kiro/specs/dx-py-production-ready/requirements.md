
# Requirements Document

## Introduction

This specification defines the requirements to make DX-Py a production-ready Python toolchain that can genuinely compete with CPython (runtime), pytest (test runner), and uv (package manager). The current codebase has solid architecture but critical functionality gaps that prevent real-world usage.

## Glossary

- Runtime: The DX-Py Python interpreter that executes Python code
- Bytecode_Compiler: Component that compiles Python source to DPB bytecode
- Dispatcher: The bytecode execution loop in the interpreter
- JIT_Compiler: Just-In-Time compiler using Cranelift for native code generation
- Package_Manager: Component for dependency resolution and package installation
- Test_Runner: Component for discovering and executing Python tests
- PyPI: Python Package Index, the official package repository
- Wheel: Binary distribution format for Python packages
- Fixture: pytest concept for test setup/teardown and dependency injection

## Requirements

### Requirement 1: Fix Class System

User Story: As a Python developer, I want to define and instantiate classes with `__init__` methods, so that I can use object-oriented programming patterns.

#### Acceptance Criteria

- WHEN a class is defined with an `__init__` method, THE Runtime SHALL compile the method body to valid bytecode
- WHEN a class is instantiated with arguments, THE Runtime SHALL call `__init__` with the instance as `self` and pass the arguments
- WHEN a method accesses `self.attribute`, THE Runtime SHALL correctly resolve instance attributes
- WHEN a class inherits from another class, THE Runtime SHALL follow the C3 MRO for method resolution
- WHEN `super()` is called in a method, THE Runtime SHALL return a proxy that delegates to the parent class
- IF a class definition contains syntax errors, THEN THE Runtime SHALL report a clear error message with line number

### Requirement 2: Fix Exception Handling

User Story: As a Python developer, I want try/except/finally blocks to work correctly, so that I can handle errors gracefully.

#### Acceptance Criteria

- WHEN an exception is raised inside a try block, THE Runtime SHALL check each except handler in order
- WHEN an except handler matches the exception type, THE Runtime SHALL execute that handler's body
- WHEN a finally block exists, THE Runtime SHALL execute it regardless of whether an exception occurred
- WHEN an exception is not caught, THE Runtime SHALL propagate it up the call stack
- WHEN `raise` is used without an argument inside an except block, THE Runtime SHALL re-raise the current exception
- WHEN `raise ExceptionType from cause` is used, THE Runtime SHALL chain the exceptions correctly
- IF no except handler matches, THEN THE Runtime SHALL execute the finally block before propagating

### Requirement 3: Fix List Comprehensions

User Story: As a Python developer, I want list comprehensions to work correctly, so that I can write concise data transformations.

#### Acceptance Criteria

- WHEN a simple list comprehension `[expr for x in iterable]` is executed, THE Runtime SHALL produce a list with transformed elements
- WHEN a filtered comprehension `[expr for x in iterable if condition]` is executed, THE Runtime SHALL only include elements where condition is true
- WHEN a nested comprehension `[expr for x in iter1 for y in iter2]` is executed, THE Runtime SHALL iterate in the correct order (outer first)
- WHEN the iterable is a range object, THE Runtime SHALL correctly iterate over it
- WHEN the iterable is a list, THE Runtime SHALL correctly iterate over it
- IF the comprehension expression raises an exception, THEN THE Runtime SHALL propagate it correctly

### Requirement 4: Implement Dict and Set Comprehensions

User Story: As a Python developer, I want dict and set comprehensions to work, so that I can create dictionaries and sets concisely.

#### Acceptance Criteria

- WHEN a dict comprehension `{k: v for x in iterable}` is executed, THE Runtime SHALL produce a dictionary
- WHEN a set comprehension `{expr for x in iterable}` is executed, THE Runtime SHALL produce a set
- WHEN a filtered dict comprehension is executed, THE Runtime SHALL only include matching key-value pairs
- WHEN a filtered set comprehension is executed, THE Runtime SHALL only include matching elements
- IF duplicate keys are produced in a dict comprehension, THEN THE Runtime SHALL keep the last value (Python semantics)

### Requirement 5: Fix JSON Module

User Story: As a Python developer, I want to use `json.dumps()` and `json.loads()`, so that I can serialize and deserialize JSON data.

#### Acceptance Criteria

- WHEN `import json` is executed, THE Runtime SHALL load the json module with all standard functions
- WHEN `json.dumps(obj)` is called with a dict, THE Runtime SHALL return a valid JSON string
- WHEN `json.dumps(obj)` is called with a list, THE Runtime SHALL return a valid JSON array string
- WHEN `json.loads(string)` is called with valid JSON, THE Runtime SHALL return the corresponding Python object
- WHEN `json.dumps()` is called with `indent` parameter, THE Runtime SHALL format the output with indentation
- IF `json.loads()` receives invalid JSON, THEN THE Runtime SHALL raise a JSONDecodeError

### Requirement 6: Implement Generator Expressions and Functions

User Story: As a Python developer, I want to use generators, so that I can work with large datasets memory-efficiently.

#### Acceptance Criteria

- WHEN a generator expression `(expr for x in iterable)` is created, THE Runtime SHALL return a generator object
- WHEN `next()` is called on a generator, THE Runtime SHALL yield the next value
- WHEN a function contains `yield`, THE Runtime SHALL treat it as a generator function
- WHEN a generator is exhausted, THE Runtime SHALL raise StopIteration
- WHEN `yield from iterable` is used, THE Runtime SHALL delegate to the sub-iterator
- WHEN a generator is used in a for loop, THE Runtime SHALL iterate until StopIteration

### Requirement 7: Implement Working JIT Compilation

User Story: As a Python developer, I want hot functions to be JIT-compiled, so that I can get better performance for compute-intensive code.

#### Acceptance Criteria

- WHEN a function is called more than 100 times, THE JIT_Compiler SHALL compile it to native code
- WHEN JIT-compiled code is executed, THE Runtime SHALL produce the same results as interpreted execution
- WHEN a type guard fails in JIT code, THE Runtime SHALL deoptimize back to the interpreter
- WHEN JIT compilation fails, THE Runtime SHALL fall back to interpretation without crashing
- THE JIT_Compiler SHALL support integer arithmetic operations
- THE JIT_Compiler SHALL support floating-point arithmetic operations
- THE JIT_Compiler SHALL support function calls

### Requirement 8: Package Manager - Implement PyPI Downloads

User Story: As a Python developer, I want to install packages from PyPI, so that I can use third-party libraries.

#### Acceptance Criteria

- WHEN `dx-py add requests` is executed, THE Package_Manager SHALL download the package from PyPI
- WHEN downloading a package, THE Package_Manager SHALL verify the SHA256 hash
- WHEN a package has dependencies, THE Package_Manager SHALL resolve and download them recursively
- WHEN multiple versions satisfy constraints, THE Package_Manager SHALL select the highest compatible version
- WHEN a wheel is available for the current platform, THE Package_Manager SHALL prefer it over sdist
- IF a package is not found on PyPI, THEN THE Package_Manager SHALL report a clear error

### Requirement 9: Package Manager - Implement Wheel Installation

User Story: As a Python developer, I want wheels to be installed correctly, so that I can import and use packages.

#### Acceptance Criteria

- WHEN a wheel is downloaded, THE Package_Manager SHALL extract it to the correct location
- WHEN installing a wheel, THE Package_Manager SHALL create the package's.dist-info directory
- WHEN a wheel contains entry points, THE Package_Manager SHALL create the corresponding scripts
- WHEN a wheel contains data files, THE Package_Manager SHALL install them to the correct locations
- WHEN uninstalling a package, THE Package_Manager SHALL remove all installed files
- THE Package_Manager SHALL track installed packages in a RECORD file

### Requirement 10: Package Manager - Virtual Environment Support

User Story: As a Python developer, I want to create and use virtual environments, so that I can isolate project dependencies.

#### Acceptance Criteria

- WHEN `dx-py init` is executed, THE Package_Manager SHALL create a virtual environment
- WHEN a virtual environment exists, THE Package_Manager SHALL install packages into it
- WHEN `dx-py run` is executed, THE Package_Manager SHALL use the virtual environment's Python
- THE Package_Manager SHALL create a pyvenv.cfg file with correct configuration
- THE Package_Manager SHALL create activation scripts for bash, PowerShell, and cmd

### Requirement 11: Test Runner - Full Fixture Support

User Story: As a Python developer, I want pytest fixtures to work correctly, so that I can write maintainable tests.

#### Acceptance Criteria

- WHEN a test function has a parameter matching a fixture name, THE Test_Runner SHALL inject the fixture value
- WHEN a fixture has `scope="module"`, THE Test_Runner SHALL create it once per module
- WHEN a fixture has `scope="session"`, THE Test_Runner SHALL create it once per test session
- WHEN a fixture uses `yield`, THE Test_Runner SHALL execute teardown code after the test
- WHEN a fixture depends on another fixture, THE Test_Runner SHALL resolve the dependency chain
- WHEN `autouse=True` is set, THE Test_Runner SHALL automatically use the fixture for all tests in scope

### Requirement 12: Test Runner - Parametrized Tests

User Story: As a Python developer, I want `@pytest.mark.parametrize` to work, so that I can run tests with multiple inputs.

#### Acceptance Criteria

- WHEN a test is decorated with `@pytest.mark.parametrize`, THE Test_Runner SHALL run it once per parameter set
- WHEN multiple parametrize decorators are stacked, THE Test_Runner SHALL create the cartesian product
- WHEN a parameter set has an id, THE Test_Runner SHALL use it in the test name
- WHEN a parametrized test fails, THE Test_Runner SHALL report which parameter set failed
- THE Test_Runner SHALL support parametrizing with lists, tuples, and pytest.param objects

### Requirement 13: Implement Async/Await Support

User Story: As a Python developer, I want to use async/await syntax, so that I can write concurrent code.

#### Acceptance Criteria

- WHEN an `async def` function is called, THE Runtime SHALL return a coroutine object
- WHEN `await` is used on a coroutine, THE Runtime SHALL suspend execution until the coroutine completes
- WHEN `asyncio.run()` is called, THE Runtime SHALL execute the coroutine to completion
- WHEN `asyncio.gather()` is called, THE Runtime SHALL run multiple coroutines concurrently
- WHEN an async for loop is used, THE Runtime SHALL iterate over an async iterator
- WHEN an async with statement is used, THE Runtime SHALL call `__aenter__` and `__aexit__`

### Requirement 14: Improve Standard Library Coverage

User Story: As a Python developer, I want common stdlib modules to work, so that I can write practical applications.

#### Acceptance Criteria

- THE Runtime SHALL implement `os.path` functions (join, exists, dirname, basename)
- THE Runtime SHALL implement `pathlib.Path` with basic operations
- THE Runtime SHALL implement `re` module for regular expressions
- THE Runtime SHALL implement `datetime` module for date/time operations
- THE Runtime SHALL implement `collections` module (defaultdict, Counter, deque)
- THE Runtime SHALL implement `itertools` module (chain, zip_longest, groupby)
- THE Runtime SHALL implement `functools` module (partial, reduce, lru_cache)

### Requirement 15: CLI Expression Improvements

User Story: As a Python developer, I want the `-c` flag to support multiple statements, so that I can run quick scripts from the command line.

#### Acceptance Criteria

- WHEN `-c "stmt1; stmt2"` is used, THE Runtime SHALL execute both statements in order
- WHEN `-c` contains newlines, THE Runtime SHALL parse them as separate statements
- WHEN `-c` contains a syntax error, THE Runtime SHALL report the error with position information
- THE Runtime SHALL support compound statements (if, for, while) in `-c` mode

## Priority Order

- Critical (Blocks basic usage): Requirements 1, 2, 3, 5, 15
- High (Core functionality): Requirements 4, 6, 8, 9, 11, 12
- Medium (Competitive features): Requirements 7, 10, 13, 14
