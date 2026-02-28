# DX VS Code Extension

> **Holographic editing for DX Serializer files** - See Human format, save LLM format.

## Features

- **Holographic View**: Open `dx` file → see Human format → save as LLM format
- **Syntax Highlighting**: Color-coded keys, values, sections, arrays
- **Token Counter**: Real-time token count for any file
- **WASM Powered**: Rust serializer via WebAssembly for performance

## How It Works

When you open a `dx` file:
1. Extension reads LLM format from disk 2. Converts to Human format for display 3. On save, converts back to LLM format 4. Also saves Human format to `.dx/serializer/dx.human`

## Format Example

**LLM Format (on disk - `dx` file):** ```dsr config[name=dx,version=0.0.1,title="Enhanced Developing Experience",author=essensefromexistence]
workspace[paths=@/www,@/backend]
editors[items=neovim,zed,vscode,cursor,default=neovim]
forge[repository=https://github.com/user/repo,tools=cli,docs,tests]
js.dependencies[react=19.0.1,next=16.0.1]
```
**Human Format (in editor):** ```dx name  = dx version  = 0.0.1 title  = "Enhanced Developing Experience"
author  = essensefromexistence
[workspace]
paths:
- @/www
- @/backend
[editors]
items:
- neovim
- zed
- vscode
- cursor
default  = neovim
[forge]
repository  = https://github.com/user/repo tools:
- cli
- docs
- tests
[js.dependencies]
react  = 19.0.1 next  = 16.0.1 ```
## Human Format Rules
- **Scalars**: `key = value` (padded for alignment)
- **Arrays**: `key:` followed by `- item` lines
- **Sections**: `[section]` or `[section.subsection]`
- **Strings with spaces**: Use quotes: `title = "My Title"`
## Token Counter
Click the token count in the status bar to see breakdown by model:
+--------+--------+------+------+
| Model  | Tokens | Est. | Cost |
+========+========+======+======+
| Claude | Sonnet | 4    | 1    |
+--------+--------+------+------+
## Commands
+---------+-------------+
| Command | Description |
+=========+=============+
| `DX:    | Refresh     |
+---------+-------------+
## Installation
```bash

# From extension directory

npm install npm run compile npx vsce package --no-dependencies kiro --install-extension vscode-dx-0.0.1.vsix --force ```

## Security Limits
+-------+-------+
| Limit | Value |
+=======+=======+
| Max   | input |
+-------+-------+

## License

MIT

git clone https://github.com/jinghaihan/vscode-power-mode && cd vscode-power-mode && rm -rf .git && cd ..
