# DX CLI Prompt Design System

A comprehensive guide to the DX CLI's prompt design system, inspired by modern CLI tools like OpenClaw and Vercel's CLI.

## Table of Contents

- [Overview](#overview)
- [Design Principles](#design-principles)
- [Visual Elements](#visual-elements)
- [Component Specifications](#component-specifications)
- [Layout Rules](#layout-rules)
- [Examples](#examples)

---

## Overview

The DX CLI prompt design system provides a consistent, beautiful, and intuitive user experience for interactive command-line interfaces. It uses Unicode symbols, dim borders, and careful spacing to create a professional appearance while maintaining excellent readability.

### Key Features

- **Unicode symbols** for modern terminal aesthetics
- **Consistent spacing** with vertical bar (`│`) continuity
- **Dim borders** that don't distract from content
- **Clear visual hierarchy** between active and completed states
- **Minimal design** that focuses on user input

---

## Design Principles

### 1. Visual Continuity
The left border (`│`) creates a continuous vertical line throughout the interaction, providing visual structure and flow.

### 2. State Differentiation
- **Active prompts** use the diamond symbol (`♦`) in primary color
- **Completed prompts** use the checkmark symbol (`✓`) in success color
- **Log messages** use contextual symbols (`●`, `⚠`, `✕`)

### 3. Minimal Distraction
- All borders and structural elements use dim/gray color
- Only interactive elements and symbols use color
- Spacing is generous but not excessive

### 4. Consistent Alignment
- Symbols align at the same column position
- The `♦` symbol aligns with the `│` bar position
- All content maintains consistent left padding

---

## Visual Elements

### Symbols

| Symbol | Unicode | Usage | Color |
|--------|---------|-------|-------|
| `┌` | U+250C | Top-left corner (intro) | Dim |
| `│` | U+2502 | Vertical bar (continuity) | Dim |
| `├` | U+251C | Left T-junction (box bottom) | Dim |
| `╮` | U+256E | Top-right corner (box) | Dim |
| `╯` | U+256F | Bottom-right corner (box) | Dim |
| `─` | U+2500 | Horizontal line | Dim |
| `♦` | U+2666 | Active prompt indicator | Primary (cyan) |
| `✓` | U+2713 | Completed prompt | Success (green) |
| `●` | U+25CF | Info message | Blue |
| `⚠` | U+26A0 | Warning message | Yellow |
| `✕` | U+2715 | Error message | Red |
| `◻` | U+25FB | Checkbox (unchecked) | Dim/Primary |
| `◼` | U+25FC | Checkbox (checked) | Success/Primary |
| `○` | U+25CB | Radio button (inactive) | Dim |
| `●` | U+25CF | Radio button (active) | Primary |

### Color Palette

- **Primary**: Cyan - Used for active elements and focus
- **Success**: Green - Used for completed actions and confirmations
- **Warning**: Yellow - Used for warnings and cautions
- **Error**: Red - Used for errors and failures
- **Dim**: Gray - Used for all structural elements and borders

---

## Component Specifications

### 1. Intro Line

The intro line starts the prompt sequence with a welcoming message.

**Format:**
```
┌─ {title}
│
```

**Rules:**
- Starts with `┌─` (corner + horizontal line)
- Title text is in default color (not dimmed)
- Followed by a blank line with `│` bar
- No horizontal line extension after title

**Example:**
```
┌─ Welcome to DX - Your AGI-like AI Agent
│
```

---

### 2. Box Section

Box sections display important information or instructions in a bordered container.

**Format:**
```
♦  {title}  ─────────────────────────────────────────────────────────╮
│                                                                     │
│  {content line 1}                                                   │
│  {content line 2}                                                   │
│                                                                     │
├─────────────────────────────────────────────────────────────────────╯
│
```

**Rules:**
- Title line starts with `♦` symbol (primary color)
- Title is followed by horizontal lines (`─`) to match content width
- Top-right corner uses `╮`
- Content is padded with 2 spaces from left border
- Empty lines before and after content (inside box)
- Bottom border uses `├` and `╯`
- All borders are dim colored
- Followed by a blank line with `│` bar

**Example:**
```
♦  Setup  ───────────────────────────────────────────────────────────╮
│                                                                     │
│  Let's set up your AI chat experience with multiple providers.     │
│                                                                     │
├─────────────────────────────────────────────────────────────────────╯
│
```

---

### 3. Active Prompts

Active prompts are interactive elements waiting for user input.

#### Multi-Select Prompt

**Format:**
```
♦  {prompt title}
│
│  ◻  {option 1}  {hint 1}
│  ◻  {option 2}  {hint 2}
│  ◼  {option 3}  {hint 3}  (selected)
```

**Rules:**
- Title line starts with `♦` symbol (no leading `│`)
- Title is bold
- Blank line with `│` after title
- Each option line starts with `│` + 2 spaces
- Checkbox symbol + 2 spaces + label
- Hints are dimmed
- Selected items use `◼` (filled square)
- Active cursor item uses primary color
- No blank line after last option

#### Single-Select Prompt

**Format:**
```
♦  {prompt title}
│
│  ○  {option 1}  {hint 1}
│  ●  {option 2}  {hint 2}  (selected)
│  ○  {option 3}  {hint 3}
```

**Rules:**
- Same as multi-select but uses radio buttons (`○` / `●`)
- Only one option can be selected at a time

#### Confirm Prompt

**Format:**
```
♦  {prompt title}
│
│  Yes  /  No
```

**Rules:**
- Shows both Yes and No options separated by ` / `
- Active option is in primary color
- Inactive option is dimmed

---

### 4. Completed Prompts

Completed prompts show the user's selection in a compact format.

**Format:**
```
✓ {prompt title}  {selected value(s)}
│
```

**Rules:**
- Starts with `✓` symbol (success color, no leading `│`)
- Title is in default color (not bold)
- Selected value(s) are dimmed
- Multiple selections are comma-separated
- Followed by a blank line with `│` bar
- No extra spacing or padding

**Examples:**
```
✓ Select AI providers to configure:  OpenAI (GPT-4, GPT-3.5), Anthropic (Claude)
│
✓ Choose your default AI model:  GPT-4
│
✓ Would you like to start the DX agent daemon now?  Yes
│
```

---

### 5. Log Messages

Log messages provide feedback and information to the user.

#### Info Message

**Format:**
```
● {message text}
│
```

**Rules:**
- Starts with `●` symbol (blue, no leading spaces)
- Message text in default color
- Followed by a blank line with `│` bar

#### Warning Message

**Format:**
```
⚠ {message text}
│
```

**Rules:**
- Starts with `⚠` symbol (yellow, no leading spaces)
- Message text in default color
- Followed by a blank line with `│` bar

#### Success Message

**Format:**
```
✓ {message text}
│
```

**Rules:**
- Starts with `✓` symbol (green, no leading spaces)
- Message text in default color
- Followed by a blank line with `│` bar

#### Error Message

**Format:**
```
✕ {message text}
│
```

**Rules:**
- Starts with `✕` symbol (red, no leading spaces)
- Message text in default color
- Followed by a blank line with `│` bar

---

## Layout Rules

### Spacing Guidelines

1. **Vertical Spacing**
   - Always use `│` bar for blank lines between sections
   - One blank line after intro
   - One blank line after box sections
   - One blank line after completed prompts
   - One blank line after log messages
   - One blank line after prompt title (before options)

2. **Horizontal Spacing**
   - 2 spaces between `│` bar and content
   - 2 spaces between checkbox/radio and label
   - 1 space between symbol and text in log messages
   - No leading spaces before symbols in completed prompts or log messages

3. **Alignment**
   - `♦` symbol aligns with `│` bar column position
   - All `│` bars align vertically
   - Content inside boxes is left-aligned with consistent padding

### Width Calculation

- Box width is calculated based on the longest content line
- Horizontal lines (`─`) fill to match content width
- Right borders (`│`) align at the calculated width
- Minimum practical width: 60 characters
- Maximum recommended width: 100 characters

---

## Examples

### Complete Onboarding Flow

```
┌─ Welcome to DX - Your AGI-like AI Agent
│
♦  Setup  ───────────────────────────────────────────────────────────╮
│                                                                     │
│  Let's set up your AI chat experience with multiple providers.     │
│                                                                     │
├─────────────────────────────────────────────────────────────────────╯
│
♦  Select AI providers to configure:
│
│  ◻  OpenAI (GPT-4, GPT-3.5)  Most popular, great for general tasks
│  ◼  Anthropic (Claude)  Excellent for analysis and writing
│  ◻  Google (Gemini)  Fast and cost-effective
│  ◼  Ollama (Local models)  Run models locally for privacy
│  ◻  Custom API endpoint  Connect to any OpenAI-compatible API

[User selects and submits]

✓ Select AI providers to configure:  Anthropic (Claude), Ollama (Local models)
│
♦  Select integrations to set up:
│
│  ◻  GitHub  Code repositories and PR management
│  ◻  Discord  Chat and community management
│  ◻  Telegram  Messaging and notifications

[User selects none]

✓ Select integrations to set up:  none
│
● No integrations selected. You can add them later with 'dx connect <integration>'
│
♦  Would you like to start the DX agent daemon now?
│
│  Yes  /  No

[User selects Yes]

✓ Would you like to start the DX agent daemon now?  Yes
│
✓ Setup complete! Run 'dx run "hello"' to start chatting.
```

---

## Implementation Notes

### Terminal Compatibility

- All Unicode symbols are widely supported in modern terminals
- Git Bash (MINGW64) on Windows fully supports these symbols
- Fallback to ASCII symbols is available but not recommended

### Color Support

- Uses ANSI color codes for terminal output
- Gracefully degrades in terminals without color support
- Dim color is achieved through ANSI dim attribute

### Accessibility

- High contrast between active and inactive elements
- Clear visual hierarchy
- Symbols supplement text, not replace it
- Screen reader friendly (symbols have semantic meaning)

---

## Best Practices

### Do's ✓

- Always maintain the `│` bar continuity
- Use consistent spacing throughout
- Keep borders and structure dim
- Use color sparingly for emphasis
- Provide clear visual feedback for user actions
- Test in actual terminal environments

### Don'ts ✗

- Don't break the vertical `│` bar flow unnecessarily
- Don't use bright colors for structural elements
- Don't add extra blank lines without `│` bars
- Don't mix different symbol styles
- Don't use bold text excessively
- Don't create boxes wider than 100 characters

---

## Maintenance

This design system should be updated when:
- New prompt types are added
- Symbol usage changes
- Color scheme is modified
- Layout rules are adjusted
- Terminal compatibility requirements change

Last updated: 2026-02-04
