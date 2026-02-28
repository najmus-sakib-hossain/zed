# Implementation Plan: DX WWW Framework Structure

## Overview

This implementation plan creates the folder structure conventions, file-system routing, build pipeline, and developer experience tooling for the DX WWW Framework. The framework will support `.pg` (page) and `.cp` (component) files, compile them to `.dxob` binary format, and provide a complete development and production workflow.

## Tasks

- [x] 1. Create project structure and configuration system
  - [x] 1.1 Implement `DxConfig` struct and TOML parser
    - Create `crates/driven/www/src/config.rs`
    - Define configuration schema matching `dx.config.toml` structure
    - Implement validation for all config options
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6_
  
  - [x] 1.2 Write property test for configuration validation
    - **Property 15: Configuration validation**
    - **Validates: Requirements 7.7**
  
  - [x] 1.3 Implement project folder structure scanner
    - Create `crates/driven/www/src/project.rs`
    - Scan and validate standard directories (pages/, components/, api/, etc.)
    - Return structured representation of project layout
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7_

- [x] 2. Implement file-system router
  - [x] 2.1 Create route pattern matching system
    - Create `crates/driven/www/src/router/mod.rs`
    - Implement `FileSystemRouter` struct with route storage
    - Define `Route`, `DynamicRoute`, and `RoutePattern` types
    - _Requirements: 2.1, 2.2_
  
  - [x] 2.2 Implement static route mapping
    - Scan pages directory and build route table
    - Map file paths to URL paths (e.g., `pages/about.pg` → `/about`)
    - Handle index files (e.g., `pages/index.pg` → `/`)
    - _Requirements: 2.1, 2.2, 2.3_
  
  - [x] 2.3 Write property test for route mapping
    - **Property 1: File-system to route mapping preserves directory structure**
    - **Validates: Requirements 2.3, 2.7**
  
  - [x] 2.4 Implement dynamic route pattern recognition
    - Parse bracket notation `[param]` and catch-all `[...param]`
    - Create dynamic route patterns with parameter extraction
    - Implement route matching with parameter extraction
    - _Requirements: 2.4, 2.5_
  
  - [x] 2.5 Write property test for dynamic routes
    - **Property 2: Dynamic route pattern recognition**
    - **Validates: Requirements 2.4, 2.5, 4.2**
  
  - [x] 2.6 Implement layout resolution system
    - Create `crates/driven/www/src/router/layout.rs`
    - Scan for `_layout.pg` files in directory hierarchy
    - Build layout chains from root to leaf
    - _Requirements: 2.6, 10.1_
  
  - [x] 2.7 Write property tests for layout system
    - **Property 3: Layout application scope**
    - **Property 4: Layout chain composition**
    - **Validates: Requirements 2.6, 10.1, 10.3, 10.4**

- [x] 3. Checkpoint - Ensure routing tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 4. Implement component parser
  - [x] 4.1 Create component file parser
    - Create `crates/driven/www/src/parser/mod.rs`
    - Parse `.pg` and `.cp` files into structured components
    - Extract `<script>`, `<template>`, and `<style>` sections
    - _Requirements: 3.1, 3.2_
  
  - [x] 4.2 Implement multi-language script parser
    - Parse `lang` attribute from `<script>` tags
    - Support Rust, Python, JavaScript, Go language detection
    - Extract script source code for compilation
    - _Requirements: 3.5_
  
  - [x] 4.3 Implement template parser
    - Parse HTML template with Svelte-style directives
    - Support `{#if}`, `{#each}`, `{#await}` syntax
    - Parse React-style props and event bindings
    - _Requirements: 3.6, 3.7_
  
  - [x] 4.4 Implement style parser
    - Parse CSS with atomic class support
    - Detect scoped vs global styles
    - Extract style rules for compilation
    - _Requirements: 6.2_
  
  - [x] 4.5 Write property test for component naming conventions
    - **Property 28: Component naming convention enforcement**
    - **Validates: Requirements 3.3, 3.4**

- [x] 5. Implement build pipeline core
  - [x] 5.1 Create build pipeline orchestrator
    - Create `crates/driven/www/src/build/mod.rs`
    - Implement `BuildPipeline` struct with compilation workflow
    - Define `BuildOutput` and `BinaryObject` types
    - _Requirements: 6.1_
  
  - [x] 5.2 Implement script compiler
    - Create `crates/driven/www/src/build/script.rs`
    - Compile Rust scripts to WASM or native
    - Support multi-language compilation (Python, JS, Go)
    - Generate export/import tables
    - _Requirements: 6.1, 6.6_
  
  - [x] 5.3 Write property test for syntax validation
    - **Property 13: Syntax validation before compilation**
    - **Validates: Requirements 6.6**
  
  - [x] 5.4 Implement template compiler
    - Create `crates/driven/www/src/build/template.rs`
    - Compile HTML templates to binary DOM instructions
    - Generate element tree and attribute bindings
    - _Requirements: 6.1_
  
  - [x] 5.5 Integrate dx-style for CSS compilation
    - Create `crates/driven/www/src/build/style.rs`
    - Call dx-style to compile atomic CSS to binary format
    - Handle scoped styles and global styles
    - _Requirements: 6.2_
  
  - [x] 5.6 Write property test for CSS compilation
    - **Property 9: CSS compilation to binary format**
    - **Validates: Requirements 6.2**
  
  - [x] 5.7 Implement binary object generator
    - Create `crates/driven/www/src/build/binary.rs`
    - Generate `.dxob` files with proper header and sections
    - Implement DXOB format specification
    - _Requirements: 6.1_
  
  - [x] 5.8 Write property test for compilation output
    - **Property 8: Source to binary compilation**
    - **Validates: Requirements 6.1, 9.1**

- [x] 6. Implement build optimization and caching
  - [x] 6.1 Create build cache system
    - Create `crates/driven/www/src/build/cache.rs`
    - Implement content-based caching for compiled artifacts
    - Track file dependencies for cache invalidation
    - _Requirements: 6.5_
  
  - [x] 6.2 Implement incremental compilation
    - Detect changed files and identify affected dependents
    - Recompile only affected files
    - Preserve unchanged binary objects
    - _Requirements: 6.5_
  
  - [x] 6.3 Write property test for incremental compilation
    - **Property 12: Incremental compilation efficiency**
    - **Validates: Requirements 6.5**
  
  - [x] 6.4 Implement binary optimizer
    - Create `crates/driven/www/src/build/optimize.rs`
    - Minify binary objects for production
    - Tree-shake unused code
    - _Requirements: 6.3, 9.6_
  
  - [x] 6.5 Write property tests for optimization
    - **Property 10: Build optimization reduces size**
    - **Property 20: Tree-shaking removes unused code**
    - **Validates: Requirements 6.3, 9.6**
  
  - [x] 6.6 Implement dependency bundler
    - Bundle external dependencies into binary objects
    - Resolve dependency versions and conflicts
    - _Requirements: 6.7_
  
  - [x] 6.7 Write property test for dependency bundling
    - **Property 14: Dependency bundling completeness**
    - **Validates: Requirements 6.7**

- [x] 7. Checkpoint - Ensure build pipeline tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 8. Implement route manifest generation
  - [x] 8.1 Create manifest generator
    - Create `crates/driven/www/src/build/manifest.rs`
    - Generate `manifest.json` with all routes and assets
    - Include route metadata, layouts, and data loaders
    - _Requirements: 6.4, 9.2_
  
  - [x] 8.2 Write property test for manifest completeness
    - **Property 11: Route manifest completeness**
    - **Validates: Requirements 6.4, 9.2**

- [x] 9. Implement API routes system
  - [x] 9.1 Create API route scanner
    - Create `crates/driven/www/src/api/mod.rs`
    - Scan `api/` directory for route files
    - Map API files to endpoints (e.g., `api/users.rs` → `/api/users`)
    - Support dynamic API routes with parameters
    - _Requirements: 4.1, 4.2_
  
  - [x] 9.2 Implement API route handler compilation
    - Compile API route handlers to WASM or native
    - Support HTTP method routing (GET, POST, PUT, DELETE, PATCH)
    - Generate request/response serialization code
    - _Requirements: 4.3, 4.4, 4.5_
  
  - [x] 9.3 Write property tests for API routes
    - **Property 18: API route serialization**
    - **Property 26: API error response handling**
    - **Validates: Requirements 4.4, 12.5**

- [x] 10. Implement data loading system
  - [x] 10.1 Create data loader interface
    - Create `crates/driven/www/src/data/mod.rs`
    - Define `DataLoader` trait and execution interface
    - Implement data loader discovery from page components
    - _Requirements: 5.1_
  
  - [x] 10.2 Implement data loader execution
    - Execute data loaders before page rendering
    - Pass route parameters to data loaders
    - Pass loader results as props to components
    - _Requirements: 5.1, 5.2_
  
  - [x] 10.3 Write property tests for data loading
    - **Property 5: Data loader execution order**
    - **Property 6: Data loader error handling**
    - **Validates: Requirements 5.1, 5.2, 5.3**
  
  - [x] 10.4 Implement data loader caching
    - Create `crates/driven/www/src/data/cache.rs`
    - Cache data loader results by route and parameters
    - Implement cache invalidation strategies
    - _Requirements: 5.5_
  
  - [x] 10.5 Write property test for caching
    - **Property 7: Data loader caching consistency**
    - **Validates: Requirements 5.5**

- [x] 11. Implement development server
  - [x] 11.1 Create dev server core
    - Create `crates/driven/www/src/dev/mod.rs`
    - Implement HTTP server with request routing
    - Serve compiled binary objects and static assets
    - _Requirements: 8.1_
  
  - [x] 11.2 Implement file watcher
    - Create `crates/driven/www/src/dev/watcher.rs`
    - Watch source files for changes
    - Trigger incremental recompilation on changes
    - _Requirements: 8.1, 8.2_
  
  - [x] 11.3 Implement hot reload system
    - Create `crates/driven/www/src/dev/hot_reload.rs`
    - Establish WebSocket connections with clients
    - Send component updates without full page refresh
    - _Requirements: 8.2_
  
  - [x] 11.4 Write property test for hot reload
    - **Property 16: Hot reload without full refresh**
    - **Validates: Requirements 8.2**
  
  - [x] 11.5 Implement error overlay
    - Create `crates/driven/www/src/dev/error_overlay.rs`
    - Display compilation errors in browser
    - Show file location, line number, and code context
    - Display runtime errors with stack traces
    - _Requirements: 8.3, 12.1, 12.2_
  
  - [x] 11.6 Write property tests for error display
    - **Property 17: Compilation error display**
    - **Property 25: Runtime error overlay display**
    - **Property 27: Development error logging**
    - **Validates: Requirements 8.3, 12.1, 12.2, 12.6**

- [x] 12. Checkpoint - Ensure dev server tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 13. Implement static asset handling
  - [x] 13.1 Create static asset server
    - Create `crates/driven/www/src/assets/mod.rs`
    - Serve files from `public/` directory
    - Preserve directory structure in URL paths
    - _Requirements: 11.1, 11.2_
  
  - [x] 13.2 Write property test for asset path preservation
    - **Property 21: Static asset path preservation**
    - **Validates: Requirements 11.1**
  
  - [x] 13.3 Implement asset optimization
    - Create `crates/driven/www/src/assets/optimize.rs`
    - Optimize images (compress, resize, format conversion)
    - Generate content hashes for cache busting
    - _Requirements: 11.3, 11.4_
  
  - [x] 13.4 Write property tests for asset optimization
    - **Property 22: Asset optimization reduces size**
    - **Property 23: Content hashing for cache busting**
    - **Validates: Requirements 11.3, 11.4**
  
  - [x] 13.5 Implement asset URL resolution
    - Resolve asset imports in components
    - Handle content-hashed filenames
    - Support public path configuration
    - _Requirements: 11.5_
  
  - [x] 13.6 Write property test for URL resolution
    - **Property 24: Asset URL resolution**
    - **Validates: Requirements 11.5**

- [x] 14. Implement CLI commands
  - [x] 14.1 Create CLI command parser
    - Create `crates/driven/www/src/cli/mod.rs`
    - Define command enum and argument parsing
    - Implement command dispatch
    - _Requirements: 8.4, 8.5, 8.6, 8.7_
  
  - [x] 14.2 Implement `dx new` command
    - Create new project with standard folder structure
    - Generate `dx.config.toml` with defaults
    - Support project templates
    - _Requirements: 8.4_
  
  - [x] 14.3 Implement `dx dev` command
    - Start development server
    - Enable hot reload and file watching
    - Open browser automatically (if configured)
    - _Requirements: 8.5_
  
  - [x] 14.4 Implement `dx build` command
    - Run production build pipeline
    - Generate optimized binary objects
    - Output to configured directory
    - _Requirements: 8.6_
  
  - [x] 14.5 Implement `dx generate` command
    - Generate new pages, components, API routes, layouts
    - Use templates with proper naming conventions
    - Update project structure
    - _Requirements: 8.7_
  
  - [x] 14.6 Write unit tests for CLI commands
    - Test command parsing and execution
    - Test project generation
    - Test file generation
    - _Requirements: 8.4, 8.5, 8.6, 8.7_

- [x] 15. Implement production build system
  - [x] 15.1 Create production build orchestrator
    - Create `crates/driven/www/src/production/mod.rs`
    - Run full optimization pipeline
    - Generate deployment-ready output
    - _Requirements: 9.1, 9.2_
  
  - [x] 15.2 Implement source map generation
    - Create `crates/driven/www/src/production/sourcemap.rs`
    - Generate source maps for binary objects
    - Map compiled code to original source locations
    - _Requirements: 9.5_
  
  - [x] 15.3 Write property test for source maps
    - **Property 19: Source map generation**
    - **Validates: Requirements 9.5**
  
  - [x] 15.4 Implement deployment target support
    - Support static, server, and edge deployment targets
    - Generate target-specific output formats
    - Create deployment configuration files
    - _Requirements: 9.7_

- [x] 16. Implement error handling system
  - [x] 16.1 Create custom error pages support
    - Support `pages/_error.pg` for custom error pages
    - Support `pages/_404.pg` for custom 404 pages
    - Pass error information to error pages
    - _Requirements: 12.3, 12.4_
  
  - [x] 16.2 Implement error boundary system
    - Create error boundaries for component trees
    - Catch and handle rendering errors
    - Support error recovery strategies
    - _Requirements: 12.1, 12.2_

- [x] 17. Integration and final wiring
  - [x] 17.1 Wire all components together
    - Connect router, build pipeline, dev server, and CLI
    - Ensure proper data flow between components
    - Add integration points for all subsystems
    - _Requirements: All_
  
  - [x] 17.2 Create main entry point
    - Create `crates/driven/www/src/main.rs`
    - Initialize framework and dispatch CLI commands
    - Handle global error cases
    - _Requirements: All_
  
  - [x] 17.3 Write integration tests
    - Test complete workflows (create → develop → build)
    - Test multi-language support end-to-end
    - Test hot reload and error handling
    - _Requirements: All_

- [x] 18. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Each task references specific requirements for traceability
- Property tests validate universal correctness properties
- Unit tests validate specific examples and edge cases
- The implementation follows the DX project structure conventions
- All code should be in `crates/driven/www/` directory
- Use Rust 2024 edition with workspace dependencies
- Follow DX code style guidelines (max 500 lines per file)
