---
inclusion: auto
---

# DX Serializer Format Guide

## Overview

DX Serializer is an LLM-optimized data format with three representations:
- **Human Format (.sr)**: User-facing, editable format with aligned key-value pairs
- **LLM Format (.llm)**: Token-optimized format for AI context (52-73% savings vs JSON) - **Version 1.0 (Wrapped Dataframe)**
- **Machine Format (.machine)**: Binary zero-copy format for runtime performance

## LLM Format Version 1.0 Key Features

1. **Wrapped Dataframes**: Tables use `[headers](rows)` syntax for deterministic parsing
2. **Quoted Strings**: Multi-word values use quotes `"..."` (NOT underscores)
3. **Inline Objects**: Use parentheses `key(param=val)` for single-line objects
4. **Mental Model Alignment**: `[]` for arrays, `()` for objects, `[headers](rows)` for tables

## Configuration

In the root `dx` config file:

```dx
[serializer]
default = human
path = @/serializer

[markdown]
default = human
path = @/markdown
```

- `default`: Specifies the format used in user-facing files (human/llm/machine)
- `path`: Output directory for generated formats (relative to project root)

## Format Conversion

When you write a file in human format (.sr), the CLI automatically generates:
- `.llm` version in `@/serializer/` folder
- `.machine` version in `@/serializer/` folder

Even if config specifies `default = llm`, writing human format will convert correctly.

## Human Format (.sr) Syntax

### Context (Metadata)
```dx
author = essensefromexistence
version = 0.0.1
name = dx
description = Orchestrate dont just own your code
```

### Sections with Named Entries
```dx
[icons.academia]
body = <path fill="currentColor" d="..."/>
width = 0
height = 0

[icons.academia-square]
body = <path fill="currentColor" d="..."/>
width = 0
height = 0
```

Format: `[section_name.entry_id]` where `entry_id` is the first column value.

**Converts to LLM format as:**
```dx
icons[id body width height](
academia "<path fill='currentColor' d='...'/>" 0 0
academia-square "<path fill='currentColor' d='...'/>" 0 0
)
```

### Arrays
```dx
[editors]
items:
- neovim
- vscode
- cursor
```

### Nested Sections
```dx
[i18n.locales]
default = en-US
path = @/locales

[js.dependencies]
next = 16.0.1
react = 19.0.1
```

**Converts to LLM format as:**
```dx
i18n.locales(default=en-US path=@/locales)
js.dependencies(next=16.0.1 react=19.0.1)
```

Note: Dots in section names are preserved in LLM format.

## CLI Commands

```bash
# Convert and generate all formats
dx serializer <file.sr>

# Verbose output
dx serializer <file.sr> --verbose

# Process directory recursively
dx serializer <directory> --recursive
```

## Key Rules

1. **No alignment padding in human format** - Use natural spacing
2. **Section names use dot notation** - `[section.subsection]` or `[section.entry_id]`
3. **First column is the identifier** - Used in section names like `[icons.icon-name]`
4. **Automatic format generation** - Writing .sr files auto-generates .llm and .machine
5. **Path resolution** - `@/` prefix resolves to project root

## Icon Files Example

Icon files use the pattern `[icons.icon-name]` where icon-name is the icon identifier:

```dx
author = James Walsh
name = Academicons
total = 158

[icons.academia]
body = <path fill="currentColor" d="..."/>
width = 0
height = 0

[icons.acm]
body = <path fill="currentColor" d="..."/>
width = 512
height = 0
```

## LLM Format Syntax Examples

### Inline Objects (Parentheses)
```dx
driven(path=@/driven)
editors(default=neovim items=[neovim zed vscode cursor])
forge(container=null pipeline=null repository="https://dx.vercel.app/user/dx")
```

### Wrapped Dataframes (Tables)
```dx
dependencies[name version](
dx-package-1 0.0.1
dx-package-2 0.0.1
)

users[id name email](
1 Alice alice@example.com
2 Bob bob@example.com
)
```

### Quoted Strings for Multi-Word Values
```dx
description="Orchestrate dont just own your code"
title="Enhanced Developing Experience"

employees[id name dept](
1 "James Smith" Engineering
2 "Mary Johnson" "Research and Development"
)
```

### Arrays in Square Brackets
```dx
tags=[rust performance serialization]
editors=[neovim zed vscode cursor "firebase studio"]
```

## Common Patterns

### Config Files (Human Format)
```dx
[serializer]
default = human
path = @/serializer

[icon]
path = @/crates/icon/icons
variant = default
pack:
- Lucide
- Hugeicons
```

**Converts to LLM:**
```dx
serializer(default=human path=@/serializer)
icon(path=@/crates/icon/icons variant=default pack=[Lucide Hugeicons])
```

### Data Tables (Human Format)
```dx
[dependencies.package-1]
name = dx-package-1
version = 0.0.1

[dependencies.package-2]
name = dx-package-2
version = 0.0.1
```

**Converts to LLM:**
```dx
dependencies[name version](
dx-package-1 0.0.1
dx-package-2 0.0.1
)
```

## Critical LLM Format Rules

### DO ✅
1. **Use quotes `"..."` for multi-word strings** - Standard and predictable
2. **Use wrapped dataframes `[headers](rows)`** - Deterministic parsing
3. **Use `[]` for arrays, `()` for objects** - Mental model alignment
4. **Use parentheses for inline objects** - `key(param=val)`
5. **Let parser infer types** - Numbers, booleans, strings auto-detected

### DON'T ❌
1. **Never replace spaces with underscores** - Breaks tokenization (doubles token cost)
2. **Don't omit quotes for multi-word strings** - Causes parsing ambiguity
3. **Don't use old format without wrapped dataframes** - Parsing ambiguity
4. **Don't manually add counts** - Serializer calculates automatically

## Token Efficiency

**Why underscores fail:**
- `" dont"` → 1 token (space + word)
- `"_dont"` → 2 tokens (underscore, then word)
- Replacing spaces with underscores **doubles token cost**

**Wrapped dataframes eliminate ambiguity:**
- Parser knows exactly when table starts `(` and ends `)`
- No guessing based on column counts or blank lines
- Newlines inside `()` are free (1 token = 1 token)

**Real-world savings:**
- Package dependencies (50 items): 73% savings vs JSON
- User database (100 rows): 69% savings vs JSON
- API endpoints (25 items): 53% savings vs JSON
- Average: 62% token savings vs JSON

## Migration Notes

**Deprecated patterns:**
- `[formats]` section - Use `[serializer]` and `[markdown]` instead
- `serializer_source` key - Use `default` key instead
- Numbered sections `[icons:1]` - Use named sections `[icons.icon-name]` instead
- Alignment padding in keys - No longer needed
- Underscores for spaces - Use quoted strings instead
- Old table format without `()` wrapping - Use wrapped dataframes

**Current best practices (Human Format):**
- Use dot notation for all sections
- First column value becomes the entry identifier
- Let String grow dynamically (no capacity limits)
- Natural spacing without forced alignment

**Current best practices (LLM Format v1.0):**
- Wrapped dataframes: `[headers](rows)`
- Quoted strings: `"multi word value"`
- Inline objects: `key(param=val)`
- Arrays: `key=[item1 item2]`
