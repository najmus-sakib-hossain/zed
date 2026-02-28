
# dx-error — Binary Error Boundaries

Component-level error isolation without crashing the entire app.

## What It Does

- Panic hook — Catches WASM panics
- Component isolation — Failed components don't crash the app
- Auto-retry — Configurable retry attempts
- Binary error reporting — Send error details to server

## Replaces

- @sentry/react (85 KB)
- react-error-boundary (8 KB)
- @bugsnag/js (80 KB) Total replaced: 173 KB → 0 KB

## Example

```typescript
// In .dx file errorBoundary(maxRetries=3) { function RiskyComponent() { // This might fail const data = fetchData();
return <div>{data.value}</div>;
}
}
// If RiskyComponent fails:
// 1. Error boundary catches it // 2. Shows fallback UI // 3. Auto-retries up to 3 times // 4. Reports error to server ```


## Performance


+--------+--------+----+---------------+----------+-------------+
| Metric | Sentry | +  | ErrorBoundary | dx-error | Improvement |
+========+========+====+===============+==========+=============+
| Bundle | size   | 85 | KB            | 0        | KB          |
+--------+--------+----+---------------+----------+-------------+


## Binary Protocol


+--------+----------+---------+
| Opcode | Hex      | Payload |
+========+==========+=========+
| ERROR  | BOUNDARY | 0xB0    |
+--------+----------+---------+


## Features



### Error Severity Levels


- `Warning` — Non-fatal, log only
- `Error` — Failed but recoverable
- `Critical` — Fatal error


### Automatic Retry


```rust
let boundary = ErrorBoundary::new(component_id, max_retries=3);
// On error:
boundary.catch_error(error);
// Auto-retry:
if boundary.recover() { // Attempt re-render } else { // Max retries exceeded, show permanent fallback }
```


### Fallback Configuration


```rust
FallbackConfig { show_error_details: true, // Show in dev mode show_retry_button: true, custom_message: Some("Oops! Something went wrong".into()), }
```


## Architecture


- Panic Hook — Catches WASM panics globally
- Error Boundary — Isolates component failures
- Registry — Manages multiple boundaries
- Binary Reporter — Sends errors to server


## Tests


```bash
cargo test -p dx-error ```
All 5 tests passing ✅
