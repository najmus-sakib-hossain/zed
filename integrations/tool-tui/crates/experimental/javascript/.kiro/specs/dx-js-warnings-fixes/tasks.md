
# Implementation Plan: dx-js Warnings and JSON Parse Fixes

## Overview

This plan addresses 3 clippy errors, 57 clippy warnings, and the JSON.parse error handling issue in the dx-js runtime. Tasks are ordered to fix critical errors first, then warnings by module, then JSON.parse.

## Tasks

- Fix critical bit mask errors in tagged.rs
- 1.1 Add SUBTYPE_MASK constant and fix is_object(), is_array(), is_function() methods-Add `const SUBTYPE_MASK: u64 = 0xFFFF_FFFF_0000_0000` for heap object subtype checking
- Update is_object() to use `(self.0 & SUBTYPE_MASK) == TAG_OBJECT`
- Update is_array() to use `(self.0 & SUBTYPE_MASK) == TAG_ARRAY`
- Update is_function() to use `(self.0 & SUBTYPE_MASK) == TAG_FUNCTION`
- Update is_heap_object() to check tag range correctly
- Fix manual_range_contains warning on line 238
- Fix unnecessary_map_or warning on line 332
- Requirements: 1.1, 1.2, 1.3, 1.4
- Fix warnings in compiler modules
- 2.1 Fix builtins_registry.rs warnings-Add Default impl for BuiltinRegistry
- Remove `.to_string()` from print!/eprint! format args (lines 116, 128, 140)
- Requirements: 2.1, 2.2
- 2.2 Fix codegen.rs warnings-Replace `.get(0)` with `.first()` (lines 1680, 2470)
- Requirements: 2.3
- 2.3 Fix expressions.rs warnings-Collapse match into if let (lines 1449, 1589)
- Prefix unused recursion parameter with underscore (line 1535)
- Requirements: 3.1, 3.2, 3.3
- 2.4 Fix functions.rs warnings-Add Default impl for FunctionCompiler
- Requirements: 8.1
- 2.5 Fix json_import.rs warnings-Consolidate identical if blocks (lines 77-81, 110-114)
- Requirements: 5.1
- 2.6 Fix modules.rs warnings-Add Default impl for ModuleResolver
- Use strip_prefix instead of manual slicing (lines 164, 279)
- Consolidate identical if blocks (line 460)
- Use char array instead of closure (line 770)
- Fix only_used_in_recursion warning (line 186)
- Requirements: 4.1, 4.2, 4.3, 4.4
- 2.7 Fix optimizations.rs warnings-Remove empty match block (line 708)
- Requirements: 3.1
- 2.8 Fix typescript.rs warnings-Fix only_used_in_recursion warning (line 416)
- Requirements: 3.3
- Fix warnings in runtime modules
- 3.1 Fix async_runtime.rs warnings-Add Default impl for Promise
- Add Default impl for EventLoop
- Replace match with if let (lines 385, 416)
- Requirements: 6.1, 6.2, 6.3
- 3.2 Fix builtins.rs warnings-Consolidate identical if blocks (lines 267-271)
- Replace useless format! with.to_string() (line 234)
- Requirements: 7.1, 7.2
- 3.3 Fix builtins_instance.rs warnings-Add type ali complex callback types
- Requirements: 8.5
- 3.4 Fix child_process.rs warnings-Remove needless borrows (lines 12, 14)
- Requirements: 8.5
- 3.5 Fix crypto.rs warnings-Remove unnecessary cast (line 61)
- Requirements: 8.9
- 3.6 Fix datetime.rs warnings-Implement Display trait instead of inherent to_string
- Requirements: 8.5
- 3.7 Fix events.rs warnings-Replace or_insert_with(Vec::new) with or_default() (lines 26, 36)
- Requirements: 8.5
- 3.8 Fix nodejs.rs warnings-Use.flatten() instead of manual if let (line 95)
- Requirements: 8.7
- 3.9 Fix regexp.rs warnings-Use Option::map instead of manual if let (line 54)
- Requirements: 8.5
- 3.10 Fix streams.rs warnings-Add Default impl for ReadableStream
- Add Default impl for WritableStream
- Requirements: 8.2, 8.3
- Fix warnings in other modules
- 4.1 Fix crystallized/cache.rs warnings-Use std::io::Error::other() (line 56)
- Requirements: 8.5
- 4.2 Fix debugger/mod.rs warnings-Replace or_insert_with(Vec::new) with or_default() (line 42)
- Replace map_or(false,...) with is_some_and() (line 57)
- Requirements: 8.5, 8.6
- 4.3 Fix gc/gc_ref.rs warnings-Fix non-canonical clone impl (line 138)
- Remove unnecessary cast (line 345)
- Requirements: 8.8, 8.9
- 4.4 Fix simd/console.rs warnings-Add Default impl for BatchConsole
- Requirements: 8.4
- 4.5 Fix simd/mod.rs warnings-Use iterator instead of range loop (lines 421, 434)
- Requirements: 8.10
- 4.6 Fix snapshot/immortal.rs warnings-Remove needless borrow (line 28)
- Requirements: 8.5
- Checkpoint
- Verify all warnings are fixed
- Run `cargo clippy` and ensure no warnings or errors
- Ensure all tests pass, ask the user if questions arise.
- Fix JSON.parse error handling in codegen.rs
- 6.1 Update builtin_json_parse to set error state on parse failure-Extract line/column from serde_json error
- Set runtime error with SyntaxError message including position
- Requirements: 9.1, 9.2
- Write property tests for correctness properties
- 7.1 Write property test for heap object type identification-Property 1: Heap Object Type Identification
- Validates: Requirements 1.1, 1.2, 1.3
- 7.2 Write property test for JSON parse error handling-Property 2: JSON Parse Error Handling
- Validates: Requirements 9.1, 9.2
- 7.3 Write property test for JSON round-trip-Property 3: JSON Round-Trip
- Validates: Requirements 9.3
- Final checkpoint
- Ensure all tests pass
- Run `cargo test` and `cargo clippy`
- Ensure all tests pass, ask the user if questions arise.

## Notes

- All tasks are required for comprehensive validation
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties
- Unit tests validate specific examples and edge cases
