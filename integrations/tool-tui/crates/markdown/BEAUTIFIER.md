# DX Markdown Beautifier

Beautiful, human-readable markdown with auto-formatting, linting, and LLM optimization.

## Features

### 1. **Auto-Formatting**

- Removes trailing whitespace
- Collapses multiple blank lines
- Ensures consistent file endings
- Proper spacing between sections

### 2. **Beautiful Tables**

- Plus-sign ASCII tables with proper alignment
- Responsive column widths based on content
- Right-aligned numbers, left-aligned text
- Double-line Unicode borders for headers (optional)

### 3. **Linting**

- Detects trailing whitespace
- Finds excessive blank lines
- Checks for inconsistent heading styles
- Reports issues with line numbers

### 4. **Complete Workflow**
```
.md files → .dx/markdown/*.human (beautified) → .md files (LLM-optimized)
```

## Usage

### Command Line
```bash
cargo run --example beautify_markdown -- /path/to/docs ```
### Programmatic API
```rust
use markdown::{MarkdownBeautifier, MarkdownWorkflow, WorkflowConfig};
// Simple beautification let beautifier = MarkdownBeautifier::new();
let beautified = beautifier.beautify(content)?;
// Complete workflow let config = WorkflowConfig::new("./docs");
let workflow = MarkdownWorkflow::new(config)?;
let result = workflow.run()?;
println!("Saved {:.1}% tokens", result.savings_percent());
```
### Linting & Auto-Fix
```rust
use markdown::{lint_markdown, autofix_markdown};
// Check for issues let issues = lint_markdown(content);
for issue in issues {  println!("{}", issue);
}
// Auto-fix common issues let fixed = autofix_markdown(content);
```
## Table Examples
### Before (Standard Markdown)
```markdown
+-------+-------+--------+
| Name  | Score | Status |
+=======+=======+========+
| Alice | 95    | Pass   |
+-------+-------+--------+
```
### After (Beautified with Plus-Sign ASCII)
```
+-------+-------+--------+
| Name  | Score | Status |
+=======+=======+========+
| Alice | 95    | Pass   |
+-------+-------+--------+
```
### After (Beautified with Unicode Double-Line)
```
╔═══════╦═══════╦════════╗ ║ Name  ║ Score ║ Status ║ ╠═══════╬═══════╬════════╣ ║ Alice ║  95 ║ Pass  ║ ║ Bob  ║  87 ║ Pass  ║ ╚═══════╩═══════╩════════╝ ```

## Configuration
```rust
use markdown::{BeautifierConfig, MarkdownBeautifier};
let config = BeautifierConfig {  use_ascii_tables: true,  // Use + tables (false = Unicode)
    max_line_width: 100,  // Maximum line width  indent_size: 2,  // Indent for nested lists  section_spacing: true,  // Blank lines between sections };
let beautifier = MarkdownBeautifier::with_config(config);
```

## Workflow Configuration
```rust
use markdown::WorkflowConfig;
let mut config = WorkflowConfig::new("./docs");
config.autofix = true;  // Auto-fix lint issues config.show_lint = true;  // Show lint warnings config.human_dir = ".dx/markdown".into();  // Output directory ```
## File Structure
```
project/ ├── README.md  # Original markdown ├── docs/ │  └── guide.md  # Original markdown └── .dx/  └── markdown/  ├── README.human  # Beautified, human-readable  └── docs/  └── guide.human  # Beautified, human-readable ```
After running the workflow:
- `.human` files contain beautified, human-readable markdown
- `.md` files contain LLM-optimized, token-efficient markdown

## Benefits

1. **For Humans**: Beautiful, readable markdown with perfect formatting 2. **For LLMs**: Token-optimized format (15-65% reduction)
3. **For Teams**: Consistent formatting across all documentation 4. **For CI/CD**: Automated linting and formatting in pipelines

## Integration with dx-serializer

The beautifier integrates with dx-serializer's token counter to provide:
- Accurate token counts before/after optimization
- Token savings percentage
- Support for multiple LLM models (GPT-4, Claude, etc.)

## Best Practices

1. **Commit `.human` files**: These are the source of truth for documentation 2. **Ignore `.md` files in git**: These are generated from `.human` files 3. **Run beautifier in CI**: Ensure consistent formatting 4. **Use auto-fix**: Let the tool handle formatting automatically

## Performance

- Processes 1000+ markdown files in seconds
- Parallel processing for large repositories
- Incremental updates (only changed files)
- Memory-efficient streaming for large files
