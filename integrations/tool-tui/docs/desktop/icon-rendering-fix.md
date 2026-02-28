# Desktop Icon Rendering Fix (GPUI)

## Problem

In the desktop app, icons rendered in the bottom icon grid, but the top-left header buttons (workspace/Open/Commit) did not show their icons.

## Root Cause

Two separate issues caused the mismatch:

1. **Asset path inconsistency**
   - Failing paths used `icons/lucide/{name}.svg`.
   - Actual embedded static assets are stored as `assets/icons/{name}.svg` and must be referenced as `icons/{name}.svg`.

2. **Asset source override in GPUI startup**
   - `Application::with_assets(...)` is assignment-style.
   - Calling it twice meant the second asset source replaced the first one.
   - Result: one set of icons resolved, the other did not.

## Solution Applied

1. **Normalized icon paths**
   - Updated icon renderers to use `icons/{name}.svg` for static UI icons.

2. **Merged asset sources**
   - Added `AppAssets` in `apps/desktop/src/assets.rs`.
   - `AppAssets` checks dynamic SVG assets first, then falls back to embedded static assets.
   - Startup now calls `with_assets(...)` once with the merged source.

3. **Chat parity updates**
   - Made chat the default startup view.
   - Ensured chat header workspace trigger uses the same left-side icon treatment as other header controls.

## Validation

- `cargo check` for `apps/desktop` passes.
- Header icons now render consistently with the icon grid approach and chat header controls.
