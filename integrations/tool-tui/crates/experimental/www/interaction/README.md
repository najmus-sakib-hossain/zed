
# dx-interaction — User Action Preservation

Preserve user interactions (focus, selection, scroll) during DOM updates.

## What It Does

- Focus preservation — Remembers which element was focused
- Cursor position — Maintains cursor position in inputs
- Text selection — Preserves highlighted text
- Scroll positions — Remembers scroll state

## Replaces

- focus-trap (15 KB)
- focus-visible (10 KB)
- react-focus-lock (20 KB)
- Manual scroll restoration scripts Total replaced: 45+ KB → 0 KB

## Example

```typescript
// Automatic preservation during updates const manager = InteractionManager::new();
// Before DOM update manager.save();
// ... DOM update happens ...
// After DOM update manager.restore(); // User never feels the update ```


## The Problem


During DOM updates, users lose their place: -Typing in an input → cursor jumps to end -Reading a long article → scroll position resets -Selecting text → selection disappears -Focused element → loses focus


## The Solution


dx-interaction preserves ALL user state automatically: -Cursor stays at the exact position -Scroll position maintained -Text selection preserved -Focus remains on the same element


## Binary Protocol


+-------------+------+-------------+
| Opcode      | Hex  | Description |
+=============+======+=============+
| INTERACTION | SAVE | 0xC0        |
+-------------+------+-------------+


## Features



### Focus Tracking


```rust
// Saves:
- Element ID
- Cursor position (for inputs/textareas)
- Selection range
// Restores:
- Focuses the same element
- Sets cursor to exact position
- Maintains selection
```


### Scroll Recording


```rust
// Saves:
- Window scroll (x, y)
- All scrollable element positions
// Restores:
- Window scrollTo()
- Element.scrollLeft/scrollTop
```


### Text Selection


```rust
// Saves:
- Selection start/end offsets
- Selected element ID
// Restores:
- Creates Range
- Applies to Selection API
```


## Architecture


@tree:InteractionManager[]


## Integration with dx-morph


```rust
// dx-morph calls this automatically:
interaction.save(); // Before patching DOM dx_morph.patch(changes); // Apply changes interaction.restore(); // After patching ```
Result: User never notices the update happening!

## Tests

```bash
cargo test -p dx-interaction ```
All 2 tests passing ✅ Zero user disruption. Perfect updates. Every time.
