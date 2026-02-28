# DX Format System - 3 Formats, Same Data

## Overview

Both `dx serializer` and `dx markdown` use a revolutionary 3-format system:

1. **Human Format** (.sr/.md) - Beautiful, readable on real disk
2. **LLM Format** (.llm) - Token-optimized in .dx/ folder (52-73% smaller)
3. **Machine Format** (.machine) - Binary rkyv in .dx/ folder (fastest in world)

## Serializer (DX ∞)

### Usage
```bash
# Process single file
dx serializer config.sr

# Process directory recursively
dx serializer .
dx serializer src/
```

### Output Structure
```
config.sr                    # Human format (beautiful, on real disk)
.dx/serializer/
├── config.llm               # LLM-optimized format
└── config.machine           # Binary rkyv format
```

### Token Savings
- 73.4% smaller than JSON
- 37.2% smaller than TOON
- ~1.9µs parse speed

## Markdown Compiler

### Usage
```bash
# Process single file
dx markdown README.md

# Process directory recursively
dx markdown .
dx markdown docs/
```

### Output Structure
```
README.md                    # Human format (beautiful, on real disk)
.dx/markdown/
├── README.llm               # LLM-optimized format
└── README.machine           # Binary rkyv format
```

### Token Savings
- 10-80% depending on content
- No-line-gap format for LLMs
- Preserves all semantic meaning

## Demo Command

```bash
# See the format system in action
dx demo-formats
dx formats
```

## Why 3 Formats?

1. **Human** (.sr/.md on disk) - Developers need readable code
2. **LLM** (.llm in .dx/) - AI needs token-efficient context
3. **Machine** (.machine in .dx/) - Runtime needs blazing speed

All three represent the exact same data, just optimized for different consumers.

## File Locations

**Human-readable files** stay on the real disk where you edit them:
- `config.sr`, `README.md`, `docs/guide.md`

**LLM and Machine formats** go into `.dx/` folder (gitignored):
- `.dx/serializer/*.llm` and `.dx/serializer/*.machine`
- `.dx/markdown/*.llm` and `.dx/markdown/*.machine`
