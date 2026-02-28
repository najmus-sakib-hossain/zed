
# dx-form — Binary Validation Engine

Replace React Hook Form + Zod with compile-time binary validators.

## What It Does

- Compile-time schema parsing — Define schemas in `.dx` files
- Binary validation — Bitmask errors for instant feedback
- Zero allocations — Validation happens in-place
- Sub-microsecond speed — < 1 µs per field

## Replaces

- react-hook-form (31 KB)
- zod (54 KB)
- yup (45 KB)
- @hookform/resolvers (8 KB) Total replaced: 138 KB → 0 KB

## Example

```typescript
// app.dx schema User { email: email, age: number(min=18, max=120), password: string(minLength=8)
}
function RegisterForm() { const [form, setForm] = state({ email: "", age: "", password: "" });
// Validation happens on every keystroke (< 1 µs)
const errors = validate(User, form);
return ( <form> <input value={form.email}
error={errors.email}
/> </form> );
}
```

## Performance

+--------+-------+------+------+---+-----+---------+-------------+
| Metric | React | Hook | Form | + | Zod | dx-form | Improvement |
+========+=======+======+======+===+=====+=========+=============+
| Bundle | size  | 85   | KB   | 0 | KB  | ∞×      | Validation  |
+--------+-------+------+------+---+-----+---------+-------------+



## Binary Protocol

+----------+-------+---------+-------------+
| Opcode   | Value | Payload | Description |
+==========+=======+=========+=============+
| VALIDATE | FIELD | 0x60    | field       |
+----------+-------+---------+-------------+



## Error Bitmask

```rust
REQUIRED = 1 << 0 // 0x0001 EMAIL_INVALID = 1 << 1 // 0x0002 MIN_LENGTH = 1 << 2 // 0x0004 MAX_LENGTH = 1 << 3 // 0x0008 MIN_VALUE = 1 << 4 // 0x0010 MAX_VALUE = 1 << 5 // 0x0020 PATTERN_MISMATCH = 1 << 6 // 0x0040 URL_INVALID = 1 << 7 // 0x0080 NUMBER_INVALID = 1 << 8 // 0x0100 DATE_INVALID = 1 << 9 // 0x0200 ```


## Internal Architecture


- Schema Parser (dx-compiler) — Parses `schema {}` blocks
- Validator Generator (dx-compiler) — Generates Rust code
- Runtime Validator (dx-client) — Executes in WASM
- Error Decoder (dx-client) — Converts bitmask to messages
