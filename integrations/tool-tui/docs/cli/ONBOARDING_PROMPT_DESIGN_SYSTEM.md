# DX CLI Onboarding Prompt Design System

## Clarification Question (Resolved)

Active text input prompts are now standardized as:

```text
♦ Project name?
│ dx-project█
```

This keeps the cursor in a normal terminal typing position.

---

## Purpose

This document defines the visual and interaction rules for DX CLI onboarding prompts so every prompt component looks consistent, predictable, and testable.

## Core Principles

1. **Left-side structure is always preserved**.
2. **No accidental blank lines** between prompt blocks.
3. **State-specific rendering is strict** (active vs submit vs cancel vs error).
4. **Box sections are the visual anchor** and must keep exact border alignment.
5. **Prompt outputs must be uniform across all onboarding components**.

---

## Canonical Layout

### Suite Header

```text
┌─ DX CLI Prompt Test Suite 🧪
│
│ Running All Tests (1-36)  ─────────────────────────────────╮
│                                                            │
│  Testing all prompt components sequentially                │
│                                                            │
├────────────────────────────────────────────────────────────╯
│
```

### Test Block

```text
│ ◇ Test: Text
│
✓ What's your name?  sumon
│
│ ◇ Test: Input
│
♦ Project name?
│ dx-project█
```

---

## Global Rendering Rules

## 1) Border and Alignment

- The vertical border `│` is the baseline visual spine.
- Section headings (`◇ Test: ...`) are prefixed with `│ `.
- Content inside a box section is aligned to the same visual width.
- Top-right and bottom-right box corners must use the **same dim color style** as all box borders.
- For action/status lines:
   - `♦` and `✓` replace the border position (no leading `│`).
   - `◇` test-step lines keep the border prefix: `│ ◇ Test: ...`.

## 2) Spacing

- Exactly one blank border line between major blocks unless explicitly suppressed.
- Never print duplicate border lines around a single prompt state transition.
- Do not add trailing empty lines after active prompt render unless required by a component spec.
- Never leave an orphan single border line (`│`) after an active prompt line.

## 3) Symbols

- **Active action symbol:** `♦`
- **Section symbol:** `◇`
- **Success symbol:** `✓`
- **Error symbol:** `▲` (or existing theme error symbol if standardized)
- **Cancel symbol:** `■` (or existing cancel symbol if standardized)
- Symbols are semantic; they must not change arbitrarily per component.

## 4) Color

- Use theme styles from prompt theme only (`primary`, `success`, `warning`, `error`, `dim`).
- Do not mix bright and dim variants for box borders.
- Status symbols must use status color (success/error/warning).

---

## Prompt State Rules

## Active State (Input Pending)

### Text/Input-like prompts

- Format is **two-line**:

```text
♦ Question text?
│ current_value█
```

- Cursor block `█` shows edit position.
- Cursor must visually move as the user types, using standard terminal end-of-input behavior.
- Placeholder appears in dim style after cursor when empty.
- Keep `?` for question-style prompts.

### Selection prompts (select/multiselect/tree/matrix/search)

- Title line begins with `♦`.
- On title line, do **not** prefix with `│` when `♦` is used.
- Option lines are grouped directly below with consistent indentation and include `│` border.
- Navigation help line appears once, at the bottom of the block.

### Editor prompts (json/markdown/table/list)

- Title line `♦` + message.
- Content area is contiguous, no extra empty spacer rows.
- Validation status/help appears once at bottom.

## Submit State (Completed)

- Must be single summary line:

```text
✓ Question text?  submitted_value
```

- Followed by one border line `│` if continuing to next prompt group.
- No cursor block in submit state.

## Cancel State

- Uses cancel symbol and explicit `cancelled` text.
- Must keep alignment with other status lines.

## Error State

- Uses error symbol + error message.
- Error message appears directly under related prompt context, not detached.

---

## Section and Box Rules

## Section Header

- Test section line:

```text
│ ◇ Test: Input
```

- Always one border spacer line below section header before first prompt.

## Box Section

- Header line uses title + horizontal run + right corner.
- Content lines are enclosed and aligned.
- Bottom line closes with matching corner style.

---

## Input-Specific Rules (Critical)

1. Active input appears on a new line under question:
   - `♦ Project name?`
   - `│ dx-project█`
2. Prompt text should be phrased as question where appropriate (include `?`).
3. No inline active input rendering like:
   - `♦ Project name  dx-project█` ❌
4. Submit keeps inline summary:
   - `✓ Project name?  dx-project` ✅
5. On action/status lines, apply symbol-specific border rules:
   - `♦ Select enabled features` ✅
   - `✓ Saved to response.json` ✅
   - `│ ◇ Test: Spinner` ✅
   - `◇ Test: Spinner` ❌
   - `│ ♦ Select enabled features` ❌

---

## Consistency Checklist (Acceptance Criteria)

Use this before approving prompt UI changes:

- [ ] Left border spine is consistent across all blocks.
- [ ] No double blank lines between prompts.
- [ ] No orphan trailing border line after active prompt rows.
- [ ] Active text/input prompts render as two lines.
- [ ] Cursor visibly advances as user types (`value█`).
- [ ] Submit lines are one-line summaries with `✓`.
- [ ] Box borders and corners have uniform dim color.
- [ ] Section labels use `│ ◇ Test: ...` format.
- [ ] No prompt-specific ad hoc formatting drift.
- [ ] Output remains readable in Windows terminals.

---

## Component Coverage Expectations

These rules apply to all onboarding-tested prompts:

- text, input, password, confirm
- select, multiselect, autocomplete
- email, phone_input, url
- number, slider, range_slider, rating, toggle, tags
- date_picker, time_picker, calendar
- color_picker, color_picker_advanced, emoji_picker
- credit_card, matrix_select, search_filter, tree_select, file_browser
- json_editor, markdown_editor, code_snippet, table_editor, list_editor
- kanban, wizard, progress, spinner

Temporary test-suite exceptions:

- credit_card: disabled temporarily
- emoji_picker: disabled temporarily (professional symbol-only CLI mode)

---

## Progress and Spinner Spacing

- Progress completion line must be followed by one `│` separator line.
- Spinner test section must start on a new line:

```text
♦ System check complete
│
│ ◇ Test: Spinner
│
♦ Environment ready
│
```

- Avoid collapsed output like `♦ System check complete` immediately followed by `│ ◇ Test: Spinner` on the same visual block.

---

## Change Control

Any future visual change to onboarding prompts should:

1. Update this file first.
2. Include before/after terminal snapshots.
3. Validate against the checklist above.
4. Keep symbols and spacing backward-consistent unless intentionally versioned.
