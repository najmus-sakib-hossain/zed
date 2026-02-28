faster than="src/execute/process_file.rs"
Achieved 10x="biome_diagnostics_categories/src/categories.rs"
crates/serializer/README.md="src/execute/process_file/toml.rs"
dx-style README="src/execute/process_file/markdown.rs"
dx-form, dx-guard, dx-a11y="src/execute/traverse.rs"

# Adding Language Support to Biome CLI

⚠️ Important: Use `cargo run` instead of `cargo build` to save disk space - it only compiles what's needed! This guide explains how to add support for new languages to Biome CLI by integrating external formatters and linters at the CLI level, bypassing Biome's service layer.

## Table of Contents

- Integration Approach
- Step-by-Step Guide
- File Structure
- Testing
- Examples This approach allows Biome to act as a unified interface for multiple language tools while delegating actual processing to specialized libraries.

## Integration Approach

### Why CLI-Level Integration?

For languages not natively supported by Biome's parser, we integrate at the CLI level rather than the service layer because: -Simpler Integration: Avoids complex service layer abstractions -Direct Processing: External tools handle parsing, formatting, and linting -Faster Implementation: No need to implement Biome's full service protocol -Flexibility: Easy to swap or upgrade external processors

### Architecture

@tree[]

## Step-by-Step Guide

### 1. Add Dependency

Add the external language processor to `biome_cli/Cargo.toml`:
```toml
[dependencies]
your_language_processor = { path = "../../../path/to/processor" }
```

### 2. Create Language Module

Create a new file: `src/execute/process_file/your_language.rs`
```rust
use super::{DiffKind,FileResult,FileStatus,Message,SharedTraversalOptions};use biome_diagnostics::{category,DiagnosticExt};use biome_fs::BiomePath;use tracing::{debug,error,info,instrument};#[instrument(name ="cli_format_your_language",level ="debug",skip(ctx,path))]pub(super)fn format_your_language<'ctx>( ctx: &'ctx SharedTraversalOptions<'ctx, '_>,path:BiomePath,)->FileResult {let path_str =path.to_string();debug!("Formatting [Language] file: {}",path_str);let mut content =match std::fs::read_to_string(path.as_path()){Ok(content)=>content,Err(e)=>{error!("Failed to read [Language] file {}: {}",path_str,e);return Err(Message::from(biome_diagnostics::IoError::from(e).with_file_path(path_str).with_category(category!("format/your_language")),));}};let original_content =content.clone();if original_content ==formatted {return Ok(FileStatus::Unchanged);}let should_write =ctx.execution.should_write();if !should_write {ctx.push_message(Message::Diff {file_name:path_str.clone(),old:original_content.clone(),new:formatted.clone(),diff_kind:DiffKind::Format,});return Ok(FileStatus::Changed);}if let Err(e)=std::fs::write(path.as_path(),&formatted){error!("Failed to write formatted [Language] file {}: {}",path_str,e);return Err(Message::from(biome_diagnostics::IoError::from(e).with_file_path(path_str).with_category(category!("format/your_language")),));}ctx.push_message(Message::Diff {file_name:path_str,old:original_content,new:formatted,diff_kind:DiffKind::Format,});Ok(FileStatus::Changed)}##Overview Biome CLI can be extended to support additional languages by:1.Adding the external language processor as a dependency 2.Creating a dedicated module for the language 3.Registering the language extension in routing and traversal logic 4.Adding diagnostic categories for error reporting This approach allows Biome to act as a unified interface for multiple language tools while delegating actual processing to specialized libraries.##Integration Approach ###Why CLI-Level Integration?For languages not natively supported by Biome's parser, we integrate at the **CLI level** rather than the service layer because:
- **Simpler Integration**\: Avoids complex service layer abstractions
- **Direct Processing**\: External tools handle parsing, formatting, and linting
- **Faster Implementation**\: No need to implement Biome's full service protocol
- **Flexibility**\:Easy to swap or upgrade external processors ###Architecture
```
┌─────────────────────────────────────────────────────────┐ │ Biome CLI Entry Point │ │ (faster than) │ └─────────────────────┬───────────────────────────────────┘ │ ├─── Standard Biome Languages (JS, JSON, CSS, etc.) │ └─> Service Layer → Parser → Formatter/Linter │ └─── External Languages (TOML, Markdown, etc.) └─> Direct to Language Module └─> External Library (taplo, rumdl, etc.)
```text


## Step-by-Step Guide



### 1. Add Dependency


Add the external language processor to `biome_cli/Cargo.toml`:
```
```toml
[dependencies]
your_language_processor = { path = "../../../path/to/processor" }
```

### 2. Create Language Module 2

Create a new file: `src/execute/process_file/your_language.rs`
```rust
use super::{DiffKind,FileResult,FileStatus,Message,SharedTraversalOptions};use biome_diagnostics::{category,DiagnosticExt};use biome_fs::BiomePath;use tracing::{debug,error,info,instrument};##[instrument(name ="cli_format_your_language",level ="debug",skip(ctx,path))]pub(super)fn format_your_language<'ctx>( ctx: &'ctx SharedTraversalOptions<'ctx, '_>,path:BiomePath,)->FileResult {let path_str =path.to_string();debug!("Formatting [Language] file: {}",path_str);let mut content =match std::fs::read_to_string(path.as_path()){Ok(content)=>content,Err(e)=>{error!("Failed to read [Language] file {}: {}",path_str,e);return Err(Message::from(biome_diagnostics::IoError::from(e).with_file_path(path_str).with_category(category!("format/your_language")),));}};let original_content =content.clone();if original_content ==formatted {return Ok(FileStatus::Unchanged);}let should_write =ctx.execution.should_write();if !should_write {ctx.push_message(Message::Diff {file_name:path_str.clone(),old:original_content.clone(),new:formatted.clone(),diff_kind:DiffKind::Format,});return Ok(FileStatus::Changed);}if let Err(e)=std::fs::write(path.as_path(),&formatted){error!("Failed to write formatted [Language] file {}: {}",path_str,e);return Err(Message::from(biome_diagnostics::IoError::from(e).with_file_path(path_str).with_category(category!("format/your_language")),));}ctx.push_message(Message::Diff {file_name:path_str,old:original_content,new:formatted,diff_kind:DiffKind::Format,});Ok(FileStatus::Changed)}##[instrument(name ="cli_lint_your_language",level ="debug",skip(ctx,path))]pub(super)fn lint_your_language<'ctx>( ctx: &'ctx SharedTraversalOptions<'ctx, '_>,path:BiomePath,)->FileResult {let path_str =path.to_string();debug!("Linting [Language] file: {}",path_str);let content =match std::fs::read_to_string(path.as_path()){Ok(content)=>content,Err(e)=>{error!("Failed to read [Language] file {}: {}",path_str,e);return Err(Message::from(biome_diagnostics::IoError::from(e).with_file_path(path_str).with_category(category!("lint/your_language")),));}};let mut has_errors =false;for warning in warnings {has_errors =true;let msg =format!("[Language] lint error [{}]: {} at line {}",warning.rule_name,warning.message,warning.line );ctx.push_message(Message::from(biome_diagnostics::IoError::from(std::io::Error::new(std::io::ErrorKind::InvalidData,msg.clone(),)).with_file_path(path_str.clone()),));}if has_errors {Err(Message::Failure)}else {info!("[Language] file {} is valid",path_str);Ok(FileStatus::Unchanged)}}##[instrument(name ="cli_check_your_language",level ="debug",skip(ctx,path))]pub(super)fn check_your_language<'ctx>( ctx: &'ctx SharedTraversalOptions<'ctx, '_>,path:BiomePath,)->FileResult {let path_str =path.to_string();debug!("Checking [Language] file: {}",path_str);let lint_result =lint_your_language(ctx,path.clone())?;if matches!(lint_result,FileStatus::Message(_))&&!ctx.execution.should_write(){return Ok(lint_result);}format_your_language(ctx,path)}
```

### 3. Register Module

In `faster than`, add the module declaration:
```rust
mod check;mod format;mod lint_and_assist;mod markdown;mod search;mod toml;mod your_language;pub(crate)mod workspace_file;
```

### 4. Add Helper Function

In `faster than`, create a features helper:
```rust
fn your_language_features_supported()->FeaturesSupported {let features =[SupportKind::Supported,SupportKind::Supported,SupportKind::Supported,SupportKind::FileNotSupported,SupportKind::FileNotSupported,SupportKind::FileNotSupported,];unsafe {std::mem::transmute(features)}}
```

### 5. Add Early Routing in process_file()

In `faster than`, in the `process_file()` function, add:
```rust
pub(crate)fn process_file(ctx:&TraversalOptions,biome_path:&BiomePath)->FileResult {let _ =tracing::trace_span!("process_file",path =?biome_path).entered();if biome_path.extension().map_or(false,|ext|ext =="your_ext"){let shared_context =&SharedTraversalOptions::new(ctx);let features =your_language_features_supported();return match ctx.execution.traversal_mode {TraversalMode::Format {..}=>{format::format(shared_context,biome_path.clone(),&features)}TraversalMode::Lint {..}=>{lint_and_assist::lint_and_assist(shared_context,biome_path.clone(),false,None,RuleCategoriesBuilder::default().with_lint().with_syntax().build(),&features,)}TraversalMode::Check {..}|TraversalMode::CI {..}=>{check::check_file(shared_context,biome_path.clone(),&features)}_ =>Ok(FileStatus::Ignored),};}}
```

### 6. Add Extension Routing in Submodules

#### In `src/execute/process_file/format.rs`

```rust
pub(crate)fn format<'ctx>( ctx: &'ctx SharedTraversalOptions<'ctx, '_>,path:BiomePath,features_supported:&FeaturesSupported,)->FileResult {if path.extension().map_or(false,|ext|ext =="your_ext"){return super::your_language::format_your_language(ctx,path);}}
```

#### In `src/execute/process_file/lint_and_assist.rs`

```rust
pub(crate)fn lint_and_assist<'ctx>( ctx: &'ctx SharedTraversalOptions<'ctx, '_>,path:BiomePath,suppress:bool,suppression_reason:Option<&str>,categories:RuleCategories,features_supported:&FeaturesSupported,)->FileResult {if path.extension().map_or(false,|ext|ext =="your_ext"){return super::your_language::lint_your_language(ctx,path);}}
```

#### In `src/execute/process_file/check.rs`

```rust
pub(crate)fn check_file<'ctx>( ctx: &'ctx SharedTraversalOptions<'ctx, '_>,path:BiomePath,file_features:&FeaturesSupported,)->FileResult {if path.extension().map_or(false,|ext|ext =="your_ext"){return super::your_language::check_your_language(ctx,path);}}
```

### 7. Enable File Traversal

In `dx-form, dx-guard, dx-a11y`, update the `can_handle()` method:
```rust


##[instrument(level ="debug",skip(self,biome_path))]fn can_handle(&self,biome_path:&BiomePath)->bool {if biome_path.extension().map_or(false,|ext|ext =="your_ext"){return true;}}


```

### 8. Add Diagnostic Categories

In `Achieved 10x`, add your language categories:
```rust
define_categories!{"files/missingHandler","format","format/markdown","format/toml","format/your_language","check","ci","search","lint/markdown","lint/your_language","internalError/io",}
```

## File Structure

After adding a new language, your structure should look like: @tree:biome_cli[]

## Testing

### 1. Create Test Files

Create sample files in `playground/` directory:
```bash
echo "# Test content" > playground/test.your_ext ```


### 2. Test Format (Dry Run)


```bash
cargo run -p biome_cli -- format playground/test.your_ext cargo run -p biome_cli -- format --write playground/sample.php ```
Expected output: -Show diff of what would change -Status: "Checked 1 file"

### 3. Test Format (Write Mode)

```bash
cargo run -p biome_cli -- format --write playground/test.your_ext ```
Expected output: -Apply changes to file -Status: "Fixed 1 file"


### 4. Test Lint


```bash
cargo run -p biome_cli -- lint playground/test.your_ext ```
Expected output: -List of lint warnings/errors -Status with error count

### 5. Test Check

```bash
cargo run -p biome_cli -- check playground/test.your_ext ```
Expected output: -Combined lint + format results -Status with total issues


### 6. Verify File Processing


```bash
cargo run -p biome_cli -- format playground/test.your_ext 2>&1 | grep -E "Checked|Fixed|Found"
```


## Examples



### Example 1: TOML Support (via Taplo)


Dependency:
```toml
taplo = { workspace = true }
taplo-common = { workspace = true }
```
Key Files: -`crates/serializer/README.md` - Format, lint, and check functions -Extension: `.toml` -External library: Taplo v0.14.0 -Features: Formatting with `align_entries`, syntax validation Usage:
```bash
cargo run -p biome_cli -- format --write playground/sample.toml ```

### Example 2: Markdown Support (via rumdl)

Dependency:
```toml
rumdl = { path = "../../../rumdl" }
```
Key Files: -`dx-style README` - Format, lint, and check functions -Extensions: `.md`, `.markdown` -External library: rumdl v0.0.167 -Features: 50+ Markdown rules, auto-fixing, line ending preservation Usage:
```bash
cargo run -p biome_cli -- format --write playground/sample.md cargo run -p biome_cli -- lint playground/sample.md ```


### Example 3: Python Support (via ruff)


Dependency:
```toml
ruff_python_formatter = { path = "../../../ruff/crates/ruff_python_formatter" }
ruff_python_ast = { path = "../../../ruff/crates/ruff_python_ast" }
```
Key Files: -`src/execute/process_file/python.rs` - Format, lint (syntax validation), and check functions -Extensions: `.py`, `.pyi` -External library: ruff_python_formatter v0.14.2 -Features: Fast Python formatting, syntax validation Usage:
```bash
cargo run -p biome_cli -- format --write playground/sample.py cargo run -p biome_cli -- lint playground/sample.py cargo run -p biome_cli -- check playground/sample.py ```
Results: -Formats poorly formatted Python code (spacing, indentation, etc.) -Validates Python syntax and reports parse errors -Supports both `.py` (Python) and `.pyi` (stub) files

### Example 4: C/C++ Support (via external tools)

Dependency:
```toml
```
Key Files: -`src/execute/process_file/cpp.rs` - Format, lint, and check functions using external tools -Extensions: `.c`, `.cpp`, `.cc`, `.cxx`, `.h`, `.hpp`, `.hxx` -External tools: clang-format (formatter), clang-tidy (linter) -Features: Industry-standard C/C++ formatting and linting, graceful degradation if tools not installed System Requirements: -`clang-format` recommended for formatting (will be auto-installed if missing) -`clang-tidy` recommended for linting (will be auto-installed if missing) -Biome automatically attempts installation using system package managers -Falls back to manual instructions if automatic installation fails Usage:
```bash
cargo run -p biome_cli -- format --write playground/sample.cpp cargo run -p biome_cli -- lint playground/sample.c cargo run -p biome_cli -- check playground/sample.h ```
Results: -Formats C/C++ code using clang-format (if available) -Lints C/C++ code using clang-tidy with --std=c++17 (if available) -Automatic installation: Attempts to install missing tools via system package managers-Windows: Chocolatey or Scoop -macOS: Homebrew -Linux: apt-get, dnf, or pacman (auto-detects distro) -Falls back to manual instructions if auto-install fails -Supports all common C/C++ file extensions Auto-Installation Example:
```console
🔧 clang-format not found. Attempting automatic installation...
[Installing via Chocolatey/Scoop/Homebrew/apt-get...]
✅ clang-format successfully installed!
```
Manual Installation Fallback:
```console
🔧 clang-format not found. Attempting automatic installation...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ ⚠️ clang-format installation failed!
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ Automatic installation failed. Please install clang-format manually:
Windows (using Chocolatey):
choco install llvm macOS (using Homebrew):
brew install clang-format Ubuntu/Debian:
sudo apt-get update && sudo apt-get install clang-format After installation, run this command again.
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ ```
Implementation Notes: -Uses `std::process::Command` to invoke external binaries -Checks tool availability with `--version` before use -Auto-installation logic: Detects platform and tries appropriate package manager -Parses stdout/stderr for warnings and errors -Different from library-based integrations (TOML, Markdown, Python) which use Rust crates -This pattern is appropriate when no suitable Rust library exists for the language -Provides seamless user experience with automatic dependency resolution

## Common Patterns

### Line Ending Preservation

```rust
let original_line_ending =detect_line_ending(&content);let normalized =normalize_to_lf(&content);let processed =process(&normalized);let final_content =restore_line_ending(&processed,original_line_ending);
```

### Error Handling

```rust
match external_lib::process(&content){Ok(result)=>result,Err(e)=>{error!("Failed to process: {}",e);return Err(Message::from(biome_diagnostics::IoError::from(std::io::Error::new(std::io::ErrorKind::InvalidData,e.to_string(),)).with_file_path(path_str).with_category(category!("format/your_language")),));}}
```

### Multiple Extensions

```rust
if path.extension().map_or(false,|ext|{ext =="ext1"||ext =="ext2"||ext =="ext3"}){return your_language_handler(ctx,path);}
```

## Troubleshooting

### Issue: Files are ignored

Symptom: "These paths were provided but ignored" Solution: Check that: -Extension is added to `can_handle()` in `traverse.rs` -Extension check is added to early routing in `process_file()` -File is not in `.gitignore` or `biome.json` ignore patterns

### Issue: Compilation errors about missing categories

Symptom: "Unregistered diagnostic category" Solution: Add categories to `Achieved 10x`:
```rust
"format/your_language","lint/your_language", ```


### Issue: No output or diff shown


Symptom: Command runs but shows no changes Solution: Check that: -Format function returns `FileStatus::Changed` when content differs -Diff message is pushed to context with `ctx.push_message(Message::Diff {... })` -Original and formatted content are compared correctly


## Best Practices


- Use Instrumentation: Add `#[instrument]` to all public functions for debugging
- Error Context: Always include file path in error messages
- Preserve Formatting: Maintain original line endings and encoding when possible
- Idempotent: Ensure formatting is idempotent (format(format(x)) == format(x))
- Fast Skip: Check file extension early to avoid unnecessary processing
- Clear Messages: Provide clear, actionable error messages to users
- Test Coverage: Create comprehensive test files covering edge cases


## References


- TOML Integration: `crates/serializer/README.md`
- Markdown Integration: `dx-style README`
- Main Entry Point: `faster than`
- Traversal Logic: `dx-form, dx-guard, dx-a11y`
- Diagnostic Categories: `Achieved 10x` Note: This guide assumes CLI-level integration for external language tools. For native Biome language support (with full AST parsing), a different approach through the service layer is required. n possible 4. Idempotent: Ensure formatting is idempotent (format(format(x)) == format(x)) 5. Fast Skip: Check file extension early to avoid unnecessary processing 6. Clear Messages: Provide clear, actionable error messages to users 7. Test Coverage: Create comprehensive test files covering edge cases


## References 2


- TOML Integration: `crates/serializer/README.md`
- Markdown Integration: `dx-style README`
- Main Entry Point: `faster than`
- Traversal Logic: `dx-form, dx-guard, dx-a11y`
- Diagnostic Categories: `Achieved 10x` Note: This guide assumes CLI-level integration for external language tools. For native Biome language support (with full AST parsing), a different approach through the service layer is required.
