# Dx Workflow Guidelines

## Code Organization Rules - CRITICAL

### File Size Limits (PRAGMATIC ENFORCEMENT)

- **TARGET 500-800 lines** for optimal maintainability
- **SOFT LIMIT 1000 lines** - prefer splitting at this point
- **EXCEPTION**: Files with working, well-tested code may exceed 1000 lines if:
  - The code is cohesive and splitting would reduce clarity
  - The file has a single, well-defined responsibility
  - Tests pass and clippy warnings are addressed
- **HARD LIMIT 2000 lines** - files exceeding this MUST be split
- Check file size BEFORE making changes: `wc -l <file>`

### Module Structure Requirements

- **ONE responsibility per file**
- **Extract to separate modules** when:
  - File approaches 1000 lines
  - Multiple distinct concerns exist
  - Functions/structs can be logically grouped
- **Use mod.rs pattern** for module organization
- **Re-export public items** from mod.rs

### Before Adding Code - MANDATORY CHECKS

1. Check current file size: `wc -l <file>`
2. If file > 1000 lines, create new module FIRST
3. Move related code to new module
4. Update imports and re-exports
5. THEN add new functionality

### Example: Splitting Large Files
```rust
// BEFORE: app.rs (1200 lines) ❌
// AFTER: Split into:
// - app.rs (300 lines) - main struct & coordination
// - app_state.rs (200 lines) - state management
// - app_render.rs (250 lines) - rendering logic
// - app_handlers.rs (250 lines) - event handlers
// - app_helpers.rs (200 lines) - utility functions
```

## Terminal & Commands

- Default Shell: Git Bash (REQUIRED - not PowerShell or CMD)
- Command Syntax: ALWAYS use Unix-style bash commands
- Examples:
  - List files: `ls -la`
  - Remove files: `rm file.txt`
  - Remove directories: `rm -rf dir/`
  - Copy files: `cp source.txt dest.txt`
  - Create directories: `mkdir -p path/to/dir`
  - View files: `cat file.txt`
  - Find in files: `grep -r "pattern" .`
  - Count lines: `wc -l file.rs`
  - Find directories: `find . -type d -name "pattern"`
  - Remove multiple: `rm -rf dir1 dir2 dir3`

## Agent Output Policy

CRITICAL: Do NOT create markdown files to document your work unless explicitly requested.

### What NOT to Do

- Creating `COMPLETION_SUMMARY.md` files
- Creating `TASK_X_SUMMARY.md` files
- Creating progress reports or status documents
- Documenting what you did in markdown files
- Adding code to files that are already too large
- Writing verbose explanations of completed work

### What TO Do

- Execute the requested task IMMEDIATELY
- Provide ONE-LINE confirmation when done
- Create markdown files ONLY when user explicitly asks
- Focus on ACTION, not documentation
- Check file sizes and split modules proactively
- Maintain clean, organized code structure
- Trust the user can see changes in editor/git diff

### Example Responses

Bad: "I've completed the task. Let me create a summary document..."
Good: "Done."

Bad: "Here's what I did step by step..."
Good: "Removed 3 .dx folders from subdirs."

## Code Quality Standards

### Formatting & Linting

- Run `cargo fmt` after every change
- Run `cargo clippy` to catch issues
- Fix all clippy warnings before completion
- Use `cargo check` for fast compilation checks

### Production-Ready Code Checklist

- ✅ File under 2000 lines (target 500-800)
- ✅ Single responsibility per module
- ✅ Proper error handling (no unwrap in production)
- ✅ Clear function names and documentation
- ✅ No code duplication
- ✅ Formatted with rustfmt
- ✅ No clippy warnings
- ✅ Logical module organization
- ✅ Tests compile and pass

## Efficiency First

- Execute tasks as quickly as possible
- Minimize unnecessary output
- Skip documentation unless explicitly requested
- Trust that the user can see the changes in their editor/git diff
- Maintain code quality while being efficient
- Answer questions directly without preamble
- One-line confirmations for completed tasks
