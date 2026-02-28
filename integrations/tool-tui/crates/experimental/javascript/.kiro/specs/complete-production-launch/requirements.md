
# Requirements Document: DX JavaScript Tooling Complete Production Launch

## Introduction

This specification addresses ALL remaining incomplete implementations, stubs, placeholders, and TODOs across the entire DX JavaScript tooling suite to achieve true production readiness for launch. The codebase has solid architecture but contains critical gaps that must be filled before competing with Bun, npm, pnpm, and yarn. This spec covers complete implementation of: -Runtime: Replace all placeholder codegen with real implementations -Package Manager: Complete installation pipeline with real extraction -Bundler: Production-ready bundling with all features working -Test Runner: Complete test execution with all features -Compatibility Layer: Working Node.js/Bun API implementations -Project Manager: Complete task execution

## Glossary

- Runtime: The dx-js-runtime crate that executes JavaScript/TypeScript code
- Codegen: Code generation module that compiles MIR to native code via Cranelift
- Package_Manager: The dx-pkg workspace that manages npm-compatible packages
- Bundler: The dx-bundle workspace that bundles JavaScript modules
- Test_Runner: The dx-test workspace for running JavaScript tests
- Compatibility_Layer: The dx-compat crates providing Node.js/Bun API compatibility
- Project_Manager: The project-manager crate for task orchestration
- MIR: Mid-level Intermediate Representation used in the compiler
- JIT: Just-In-Time compilation for native code generation
- DXP: DX Package format (binary package format)
- DXRP: DX Registry Protocol (binary registry protocol)
- DXL: DX Lock format (binary lockfile format)

## Requirements

### Requirement 1: Complete Runtime Codegen - Object Creation

User Story: As a developer, I want JavaScript objects, arrays, and functions to be properly created at runtime, so that my code works correctly.

#### Acceptance Criteria

- WHEN CreateFunction instruction is executed THEN the Codegen SHALL allocate a proper closure object with captured variables (not return function ID as placeholder)
- WHEN CreateArray instruction is executed THEN the Codegen SHALL allocate a proper array with all elements initialized (not return element count)
- WHEN CreateObject instruction is executed THEN the Codegen SHALL allocate a proper object with all properties set (not return property count)
- WHEN GetThis instruction is executed THEN the Codegen SHALL return the correct `this` binding from the call frame (not NaN)
- WHEN TypeOf instruction is executed THEN the Codegen SHALL return the correct type string based on runtime type checking (not hardcoded 1.0)
- WHEN CreateGenerator instruction is executed THEN the Codegen SHALL create a proper generator state machine (not return function ID)
- WHEN CreatePromise instruction is executed THEN the Codegen SHALL create a proper Promise object with state tracking (not return 1.0)
- WHEN CreateAsyncFunction instruction is executed THEN the Codegen SHALL create a proper async function wrapper (not return function ID)

### Requirement 2: Complete Runtime Codegen - Property Access

User Story: As a developer, I want property access and modification to work correctly, so that I can use objects and arrays.

#### Acceptance Criteria

- WHEN GetProperty instruction is executed THEN the Codegen SHALL retrieve the actual property value from the object (not return NaN)
- WHEN SetProperty instruction is executed THEN the Codegen SHALL set the property value on the object (not no-op)
- WHEN GetPropertyDynamic instruction is executed THEN the Codegen SHALL resolve the property name at runtime and retrieve the value
- WHEN SetPropertyDynamic instruction is executed THEN the Codegen SHALL resolve the property name at runtime and set the value
- WHEN GetPropertyComputed instruction is executed THEN the Codegen SHALL evaluate the computed key and retrieve the value
- WHEN SetPropertyComputed instruction is executed THEN the Codegen SHALL evaluate the computed key and set the value
- WHEN GetCaptured instruction is executed THEN the Codegen SHALL retrieve the captured variable from the closure environment
- WHEN SetCaptured instruction is executed THEN the Codegen SHALL update the captured variable in the closure environment

### Requirement 3: Complete Runtime Codegen - Exception Handling

User Story: As a developer, I want try-catch-finally to work correctly, so that I can handle errors.

#### Acceptance Criteria

- WHEN Throw instruction is executed THEN the Codegen SHALL unwind to the nearest exception handler (not just trap)
- WHEN SetupExceptionHandler instruction is executed THEN the Codegen SHALL register the catch/finally blocks for unwinding
- WHEN ClearExceptionHandler instruction is executed THEN the Codegen SHALL remove the exception handler from the stack
- WHEN GetException instruction is executed THEN the Codegen SHALL return the caught exception value (not NaN)
- WHEN an exception is thrown THEN the Runtime SHALL execute finally blocks before propagating
- WHEN no exception handler exists THEN the Runtime SHALL terminate with an uncaught exception error

### Requirement 4: Complete Runtime Builtins - Console Timing

User Story: As a developer, I want console.time/timeEnd to work, so that I can measure performance.

#### Acceptance Criteria

- WHEN console.time(label) is called THEN the Runtime SHALL store the current timestamp with the given label
- WHEN console.timeEnd(label) is called THEN the Runtime SHALL calculate and log the elapsed time since console.time(label)
- WHEN console.timeLog(label) is called THEN the Runtime SHALL log the current elapsed time without stopping the timer
- WHEN console.time is called with an existing label THEN the Runtime SHALL warn and restart the timer
- WHEN console.timeEnd is called with a non-existent label THEN the Runtime SHALL warn that the timer does not exist

### Requirement 5: Complete Runtime - Static Class Blocks

User Story: As a developer, I want static class blocks to work, so that I can use modern JavaScript class features.

#### Acceptance Criteria

- WHEN a class contains static blocks THEN the Compiler SHALL execute them during class initialization
- WHEN multiple static blocks exist THEN the Compiler SHALL execute them in source order
- WHEN a static block references `this` THEN the Compiler SHALL bind it to the class constructor
- WHEN a static block throws THEN the Runtime SHALL propagate the exception and prevent class creation

### Requirement 6: Complete Runtime - Free Variable Analysis

User Story: As a developer, I want closures to capture variables correctly, so that my functions work as expected.

#### Acceptance Criteria

- WHEN a closure is created THEN the Compiler SHALL perform proper free variable analysis (not capture all variables)
- WHEN a variable is referenced in a nested function THEN the Compiler SHALL identify it as a free variable
- WHEN a variable is only used locally THEN the Compiler SHALL NOT capture it in the closure
- WHEN a captured variable is modified THEN the Runtime SHALL update the shared reference

### Requirement 7: Complete Runtime - Module Compilation

User Story: As a developer, I want modules to be properly compiled, so that imports and exports work.

#### Acceptance Criteria

- WHEN a module is loaded THEN the Compiler SHALL perform actual compilation (not create placeholder Module)
- WHEN parsing imports THEN the Compiler SHALL use proper AST-based parsing (not string search with find())
- WHEN resolving imports THEN the Resolver SHALL follow Node.js resolution algorithm
- WHEN a module has circular dependencies THEN the Compiler SHALL handle them correctly

### Requirement 8: Complete Package Manager - Installation Pipeline

User Story: As a developer, I want `dx install` to install packages, so that I can use dependencies.

#### Acceptance Criteria

- WHEN install is called THEN the Package_Manager SHALL resolve dependencies using the LocalResolver (not return empty vec)
- WHEN packages are fetched THEN the Package_Manager SHALL download actual tarballs from npm registry
- WHEN packages are extracted THEN the Package_Manager SHALL decompress and extract tar.gz files (not create stubs)
- WHEN extraction completes THEN the Package_Manager SHALL preserve file permissions and symlinks
- WHEN hardlinks are supported THEN the Package_Manager SHALL use them for instant extraction from cache
- IF extraction fails THEN the Package_Manager SHALL provide a descriptive error message

### Requirement 9: Complete Package Manager - DXP Format

User Story: As a developer, I want the binary package format to work, so that installations are fast.

#### Acceptance Criteria

- WHEN DxpBuilder.build() is called THEN the Package_Manager SHALL create a valid DXP package (not panic with todo!())
- WHEN a DXP package is read THEN the Package_Manager SHALL extract all files correctly
- WHEN dependencies are stored THEN the Package_Manager SHALL parse them from the package (not return empty vec)
- WHEN package names are stored THEN the Package_Manager SHALL store actual names (not hash placeholders)

### Requirement 10: Complete Package Manager - Registry Protocol

User Story: As a developer, I want the registry to work correctly, so that I can download packages.

#### Acceptance Criteria

- WHEN delta updates are available THEN the Registry SHALL apply them (not skip with TODO comment)
- WHEN fetching metadata THEN the Registry SHALL parse all fields including dependencies
- WHEN the registry index is updated THEN the Registry SHALL implement full binary serialization
- THE Registry SHALL support both npm registry (HTTP+JSON) and DX binary registry (DXRP) protocols

### Requirement 11: Complete Package Manager - Speculative Fetching

User Story: As a developer, I want predictive fetching to speed up installations.

#### Acceptance Criteria

- WHEN a package is requested THEN the Fetcher SHALL pre-fetch predicted dependencies (not just log "Would pre-fetch")
- WHEN Markov predictions are available THEN the Fetcher SHALL use them to prioritize downloads
- WHEN pre-fetching THEN the Fetcher SHALL run in background without blocking main installation

### Requirement 12: Complete Bundler - Source Maps

User Story: As a developer, I want source maps to work, so that I can debug bundled code.

#### Acceptance Criteria

- WHEN a module is compiled THEN the Bundler SHALL generate source maps (not return None)
- WHEN source maps are generated THEN the Bundler SHALL include correct line/column mappings
- WHEN bundling multiple modules THEN the Bundler SHALL merge source maps correctly

### Requirement 13: Complete Test Runner - Missing Features

User Story: As a developer, I want all test runner features to work, so that I can test my code effectively.

#### Acceptance Criteria

- WHEN watch mode is enabled THEN the Test_Runner SHALL re-run tests on file changes
- WHEN coverage is requested THEN the Test_Runner SHALL generate coverage reports
- WHEN snapshot testing is used THEN the Test_Runner SHALL compare against stored snapshots
- WHEN mocks are needed THEN the Test_Runner SHALL provide mock/spy utilities

### Requirement 14: Complete Compatibility Layer - Node.js APIs

User Story: As a developer, I want Node.js APIs to work, so that I can run existing Node.js code.

#### Acceptance Criteria

- WHEN hex encoding is needed THEN the Compatibility_Layer SHALL provide proper hex encode/decode (not placeholder)
- WHEN base64 encoding is needed THEN the Compatibility_Layer SHALL provide proper base64 encode/decode
- WHEN crypto APIs are used THEN the Compatibility_Layer SHALL provide working implementations
- WHEN fs APIs are used THEN the Compatibility_Layer SHALL provide working file system operations

### Requirement 15: Complete Project Manager - Task Execution

User Story: As a developer, I want the project manager to execute tasks, so that I can automate workflows.

#### Acceptance Criteria

- WHEN a task is executed THEN the Project_Manager SHALL run the command (not return placeholder output)
- WHEN a task fails THEN the Project_Manager SHALL capture and report the error
- WHEN tasks have dependencies THEN the Project_Manager SHALL execute them in correct order

### Requirement 16: Complete Error Handling

User Story: As a developer, I want proper error messages, so that I can debug issues.

#### Acceptance Criteria

- WHEN Object methods receive non-objects THEN the Runtime SHALL throw TypeError (not return empty array)
- WHEN JSON.parse receives invalid JSON THEN the Runtime SHALL throw SyntaxError with line/column information
- WHEN package installation fails THEN the Package_Manager SHALL explain why (not say "Coming soon")
- ALL error messages SHALL include actionable information for fixing the issue

### Requirement 17: Complete Variable Initialization

User Story: As a developer, I want all variable initializers to work, so that my declarations are correct.

#### Acceptance Criteria

- WHEN variable declarations have complex initializers THEN the Compiler SHALL handle all init types (not skip with TODO)
- WHEN destructuring is used THEN the Compiler SHALL properly extract values
- WHEN default values are provided THEN the Compiler SHALL use them when source is undefined

### Requirement 18: Remove All Stub Implementations

User Story: As a maintainer, I want no stub code in production, so that the codebase is reliable.

#### Acceptance Criteria

- THE codebase SHALL NOT contain any `todo!()` macros in production code paths
- THE codebase SHALL NOT contain any `unimplemented!()` macros in production code paths
- THE codebase SHALL NOT contain any placeholder return values (NaN, 0.0, 1.0, empty vec) for real operations
- THE codebase SHALL NOT contain any "Coming soon" messages in user-facing output
- ALL TODO comments SHALL be resolved or converted to tracked issues

### Requirement 19: Complete Benchmark Verification

User Story: As a user, I want verified benchmarks, so that I can trust the performance claims.

#### Acceptance Criteria

- THE benchmarks SHALL run against actual implementations (not placeholder code)
- THE benchmark results SHALL be reproducible by anyone
- THE performance claims in documentation SHALL match actual benchmark results
- IF a benchmark cannot be verified THEN the claim SHALL be removed from documentation

### Requirement 20: Complete Cross-Component Integration

User Story: As a developer, I want all components to work together, so that I have a complete toolchain.

#### Acceptance Criteria

- WHEN dx-js runs a script THEN it SHALL use the complete runtime with all features
- WHEN dx-pkg installs packages THEN they SHALL be usable by dx-js and dx-bundle
- WHEN dx-bundle bundles code THEN it SHALL use the same parser and resolver as dx-js
- WHEN dx-test runs tests THEN it SHALL use the same runtime as dx-js
