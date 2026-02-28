# DX CLI Design System

## Core Principles

The DX CLI uses a consistent visual design system with a **continuous left border** (`│`) on structural lines, with prompts appearing without the border.

## Left Border Rules

### CRITICAL: `│` (Vertical Bar) Placement

**Lines WITH `│` border:**
- Section headers (e.g., `│ ◇ Development Setup`)
- Box sections (title and content lines)
- Blank lines between sections (just `│`)
- Blank lines after prompts (just `│`)
- Intro/outro lines

**Lines WITHOUT `│` border:**
- Active prompts (e.g., `♦ What's your name?  █John`)
- Completed prompts (e.g., `✓ What's your name?  John`)
- Info messages (e.g., `● Checking system compatibility...`)
- Success messages (e.g., `✓ Cargo: installed`)
- Warning messages (e.g., `⚠ Docker: not found`)

### Rule Summary
**Prompts appear without the `│` border. Structural elements (headers, boxes, blank lines) have the `│` border.**

## Prompt States

### Active Input State
```
♦ What's your name?  █John
│
```
- NO left border `│` on the prompt line
- Diamond symbol `♦` at start
- ONE space after `♦` before text
- Cursor `█` shows typing position
- Blank line with `│` after prompt

### Completed Prompt State
```
✓ What's your name?  John
│
```
- NO left border `│` on the prompt line
- Checkmark `✓` at start
- ONE space after `✓` before text
- Shows the entered value
- Blank line with `│` after prompt

### Section Headers
```
│
│ ◇ Development Setup
│
```
- Left border `│` IS present on section header
- ONE space between `│` and `◇`
- ONE space after `◇` before text
- Blank line with `│` before section header
- Blank line with `│` after section header

## Spacing Rules

1. **Blank lines between sections** - shown as `│` alone
2. **Blank line before section headers** - shown as `│` alone
3. **Blank line after section headers** - shown as `│` alone
4. **Blank line after each prompt** - shown as `│` alone
5. **NO blank lines between prompts within a section** - prompts flow directly with `│` separator

## Box Sections

### Correct Format
```
│
│ Getting Started  ────────────────────────╮
│                                           │
│  Let's set up your DX environment.        │
├───────────────────────────────────────────╯
│
```

### Rules
- Blank line with `│` before box
- Title line: `│` then ONE space then title text (no symbol)
- Title line has decorative border on right
- Content is left-aligned with padding
- Bottom border connects properly
- Blank line with `│` after box

## Symbols Reference

- `│` - Left border (ALWAYS present on every line)
- `♦` - Active prompt indicator
- `✓` - Completed successfully
- `◇` - Section header
- `●` - Info/processing
- `⚠` - Warning
- `✗` - Error
- `█` - Cursor during input

## Complete Example

```
┌─ Welcome to DX CLI! 🚀
│
│ Getting Started  ────────────────────────╮
│                                           │
│  Let's set up your DX environment.        │
├───────────────────────────────────────────╯
│
│ ◇ Basic Information
│
♦ What's your name?  █John
│
✓ What's your name?  John
│
✓ What's your email?  john@example.com
│
♦ Choose a username  █test
│
✓ Choose a username  test
│
│ ◇ Personalization
│
♦ Choose your avatar emoji  ...
│
✓ Choose your avatar emoji  😀
│
│ ◇ Development Setup
│
✓ Preferred code editor  Visual Studio Code
│
│ ◇ System Health Check
│
● Checking system compatibility...
│
✓ Cargo: installed
│
✓ Git: installed
│
│ Setup Complete! 🎉  ────────────────────╮
│                                          │
│  Name: John                              │
│  Email: john@example.com                 │
├──────────────────────────────────────────╯
│
✓ Your DX environment is ready!
│
● Run 'dx --help' to see available commands
│
└─ Happy coding! 🚀
```

## Implementation Notes

- Prompt components render WITHOUT the `│` prefix
- Section headers render WITH the `│` prefix
- Use `eprintln!("│");` for blank lines between sections and after prompts
- All log messages (info, success, warning, error) render WITHOUT `│` prefix
- Box sections have `│` on all lines
- The `│` border creates structure while prompts remain clean
