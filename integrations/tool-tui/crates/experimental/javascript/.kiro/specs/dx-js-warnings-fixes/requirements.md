
# Requirements Document

## Introduction

This document specifies the requirements for fixing build warnings and the JSON parsing issue in the dx-js runtime. The runtime currently has 57 clippy warnings and 3 errors related to bit mask operations, plus a JSON parsing issue where `JSON.parse` returns `undefined` instead of throwing proper errors.

## Glossary

- Runtime: The dx-js JavaScript runtime engine
- Clippy: Rust's linting tool that catches common mistakes and suggests improvements
- NaN-boxing: A technique for encoding JavaScript values in 64-bit floats using NaN bit patterns
- Tagged_Value: The NaN-boxed value representation used in the runtime
- JSON_Parser: The component responsible for parsing JSON strings into JavaScript values

## Requirements

### Requirement 1: Fix Bit Mask Errors in Tagged Values

User Story: As a developer, I want the tagged value implementation to have correct bit mask operations, so that type checking works correctly.

#### Acceptance Criteria

- WHEN checking if a value is an object THEN the Runtime SHALL use a correct bit mask that accounts for subtype bits
- WHEN checking if a value is an array THEN the Runtime SHALL use a correct bit mask that accounts for subtype bits
- WHEN checking if a value is a function THEN the Runtime SHALL use a correct bit mask that accounts for subtype bits
- THE Tagged_Value module SHALL compile without clippy errors related to bad_bit_mask

### Requirement 2: Fix Clippy Warnings in Builtins Registry

User Story: As a developer, I want clean code without unnecessary warnings, so that the codebase is maintainable.

#### Acceptance Criteria

- THE BuiltinRegistry SHALL implement the Default trait
- WHEN printing values THEN the Runtime SHALL not use unnecessary `.to_string()` calls in format macros
- WHEN accessing the first element of a slice THEN the Runtime SHALL use `.first()` instead of `.get(0)`

### Requirement 3: Fix Clippy Warnings in Expressions Module

User Story: As a developer, I want clean pattern matching code, so that the codebase is readable.

#### Acceptance Criteria

- WHEN matching a single pattern THEN the Runtime SHALL use `if let` instead of `match`
- WHEN patterns can be collapsed THEN the Runtime SHALL combine outer and inner patterns
- WHEN a parameter is only used in recursion THEN the Runtime SHALL prefix it with underscore or use it meaningfully

### Requirement 4: Fix Clippy Warnings in Modules

User Story: As a developer, I want clean string manipulation code, so that the codebase follows best practices.

#### Acceptance Criteria

- THE ModuleResolver SHALL implement the Default trait
- WHEN stripping prefixes THEN the Runtime SHALL use `strip_prefix` instead of manual slicing
- WHEN identical if blocks exist THEN the Runtime SHALL consolidate them
- WHEN searching for characters THEN the Runtime SHALL use array of chars instead of closures

### Requirement 5: Fix Clippy Warnings in JSON Import

User Story: As a developer, I want clean conditional code, so that the codebase is maintainable.

#### Acceptance Criteria

- WHEN if blocks have identical bodies THEN the Runtime SHALL consolidate them into a single branch

### Requirement 6: Fix Clippy Warnings in Async Runtime

User Story: As a developer, I want clean async code, so that the codebase is maintainable.

#### Acceptance Criteria

- THE Promise struct SHALL implement the Default trait
- THE EventLoop struct SHALL implement the Default trait
- WHEN matching single patterns THEN the Runtime SHALL use `if let` instead of `match`

### Requirement 7: Fix Clippy Warnings in Runtime Builtins

User Story: As a developer, I want clean builtin implementations, so that the codebase is maintainable.

#### Acceptance Criteria

- WHEN if blocks have identical bodies THEN the Runtime SHALL consolidate them
- WHEN using format! with static strings THEN the Runtime SHALL use `.to_string()` instead

### Requirement 8: Fix Clippy Warnings in Other Modules

User Story: As a developer, I want all modules to be warning-free, so that the codebase is clean.

#### Acceptance Criteria

- THE FunctionCompiler SHALL implement the Default trait
- THE ReadableStream SHALL implement the Default trait
- THE WritableStream SHALL implement the Default trait
- THE BatchConsole SHALL implement the Default trait
- WHEN using `or_insert_with(Vec::new)` THEN the Runtime SHALL use `or_default()` instead
- WHEN using `map_or(false,...)` THEN the Runtime SHALL use `is_some_and(...)` instead
- WHEN iterating with manual flatten THEN the Runtime SHALL use `.flatten()` iterator
- WHEN implementing Clone for Copy types THEN the Runtime SHALL use `*self`
- WHEN casting to the same type THEN the Runtime SHALL remove unnecessary casts
- WHEN using manual range contains THEN the Runtime SHALL use `RangeInclusive::contains`

### Requirement 9: Fix JSON.parse Error Handling

User Story: As a developer, I want JSON.parse to throw proper errors, so that I can handle parsing failures correctly.

#### Acceptance Criteria

- IF JSON.parse receives invalid JSON THEN the Runtime SHALL throw a SyntaxError (not return undefined)
- WHEN JSON.parse fails THEN the Runtime SHALL include position information in the error message
- FOR ALL valid JSON strings, JSON.parse SHALL return the correct JavaScript value
