---
inclusion: auto
---

# Icon CLI Integration for Zed

## Overview

The Zed codebase has an integrated `icon` CLI tool for downloading and managing SVG icons from 250+ icon packs (100,000+ icons total).

## Icon CLI Command

The `icon` command is available in your PATH and provides:

```bash
icon search <query>              # Search for icons
icon export <query> <dir>        # Export icons to directory
icon desktop <icon:pack>         # Export to desktop assets
icon packs                       # List available icon packs
```

## Asset Structure

Icons are stored in: `assets/icons/`

**Naming Convention:**
- Use snake_case for filenames: `arrow_down.svg`, `ai_anthropic.svg`
- Underscores separate words: `git_branch_plus.svg`
- Prefixes for categories: `ai_*`, `file_*`, `tool_*`, `debug_*`

**Current Categories:**
- `ai_*` - AI provider icons (anthropic, openai, claude, etc.)
- `file_*` - File type icons (code, doc, markdown, etc.)
- `tool_*` - Tool action icons (copy, delete, search, etc.)
- `debug_*` - Debugger icons (breakpoint, continue, step, etc.)
- `editor_*` - Editor brand icons (atom, vscode, sublime, etc.)
- Generic icons - No prefix (arrow, check, close, menu, etc.)

## Icon Registration Process

When adding new icons to Zed, follow this workflow:

### 1. Download Icon Using CLI

```bash
# Search for the icon first
icon search <query> --limit 10

# Export to assets/icons/ with proper naming
icon export <query> assets/icons --pack <pack_name> --limit 1
```

**Important:** Rename the exported file to match Zed's snake_case convention:
- `lucide_search.svg` → `search.svg`
- `solar_home-2.svg` → `home_2.svg`

### 2. Register in IconName Enum

Add the icon to `crates/icons/src/icons.rs`:

```rust
#[derive(
    Debug, PartialEq, Eq, Copy, Clone, EnumIter, EnumString, IntoStaticStr, Serialize, Deserialize,
)]
#[strum(serialize_all = "snake_case")]
pub enum IconName {
    // ... existing icons ...
    YourNewIcon,  // PascalCase in enum
}
```

**Naming Rules:**
- Enum variant: PascalCase (e.g., `ArrowDown`, `AiAnthropic`)
- File name: snake_case (e.g., `arrow_down.svg`, `ai_anthropic.svg`)
- The `#[strum(serialize_all = "snake_case")]` attribute automatically converts PascalCase to snake_case

### 3. Icon Path Resolution

The `IconName::path()` method automatically resolves:
```rust
IconName::ArrowDown.path() // → "assets/icons/arrow_down.svg"
```

## Recommended Icon Packs

**For UI Icons:**
- `lucide` - Modern, clean, consistent (recommended for most UI)
- `heroicons` - Tailwind UI icons (good for web-style UI)
- `feather` - Simple, minimal icons

**For Brand/Logo Icons:**
- `simple-icons` - Brand logos (GitHub, Twitter, etc.)
- `devicon` - Developer tool logos
- `logos` - Technology logos

**For Specialized Icons:**
- `octicon` - GitHub-style icons
- `carbon` - IBM Carbon design system
- `material-symbols` - Google Material icons

## Example Workflow

```bash
# 1. Search for an icon
icon search database --pack lucide --limit 5

# 2. Export to assets
icon export database assets/icons --pack lucide --limit 1

# 3. Rename file (if needed)
mv assets/icons/lucide_database.svg assets/icons/database.svg

# 4. Add to IconName enum in crates/icons/src/icons.rs
# Add: Database,

# 5. Use in code
use icons::IconName;
let icon = IconName::Database;
```

## Icon CLI Tips

1. **Always search first** to find the right icon and pack
2. **Use --pack filter** to get consistent icon style
3. **Check existing icons** before adding duplicates
4. **Follow naming conventions** strictly for consistency
5. **Prefer lucide pack** for new UI icons (Zed's primary icon set)

## Common Icon Patterns

```bash
# Add a new AI provider icon
icon search anthropic --pack simple-icons
icon export anthropic assets/icons --pack simple-icons --limit 1
mv assets/icons/simple-icons_anthropic.svg assets/icons/ai_anthropic.svg

# Add a new tool icon
icon search hammer --pack lucide
icon export hammer assets/icons --pack lucide --limit 1
mv assets/icons/lucide_hammer.svg assets/icons/tool_hammer.svg

# Add a new file type icon
icon search rust --pack devicon
icon export rust assets/icons --pack devicon --limit 1
mv assets/icons/devicon_rust.svg assets/icons/file_rust.svg
```

## Building the Icon Index

If the icon CLI shows "Index not found", build it:

```bash
cd crates/media/icon
cargo run --release --bin build_index
```

## Icon Quality Guidelines

- **Size:** Icons should be scalable SVG (no fixed dimensions)
- **Style:** Prefer outline/stroke icons over filled
- **Consistency:** Use icons from the same pack for visual harmony
- **Simplicity:** Avoid overly complex icons (keep under 2KB)
- **Accessibility:** Ensure icons work at small sizes (16x16px)

## Related Files

- Icon assets: `assets/icons/`
- Icon enum: `crates/icons/src/icons.rs`
- Icon CLI docs: `ICONS.md`
- Icon CLI binary: `icon` (in PATH)

## When to Add New Icons

Add new icons when:
- Adding a new AI provider integration
- Adding a new file type support
- Adding a new tool/action to the UI
- Adding a new editor/brand integration
- Replacing placeholder icons with proper ones

Do NOT add icons for:
- Temporary features
- Experimental UI (use existing icons)
- Personal preferences (maintain consistency)
