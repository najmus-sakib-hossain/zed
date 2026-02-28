
# Driven Binary Format (.drv)

## Overview

The `.drv` format is a binary representation of AI development rules optimized for: -Zero-copy parsing -Minimal size (73% smaller than JSON) -Cryptographic verification (Ed25519) -Fast random access via string table

## Format Version

Current: v1.0

## Structure

@tree[]

## Header

+-------+--------+-------------+
| Field | Type   | Description |
+=======+========+=============+
| Magic | `\[u8; | 4\          |
+-------+--------+-------------+



### Flags

+-----+----------+-------------+
| Bit | Name     | Description |
+=====+==========+=============+
| 0   | `SIGNED` | File        |
+-----+----------+-------------+



## String Table

All strings are deduplicated and stored in a single table. References use `u32` indices.
```rust
struct StringTable { count: u32, offsets: [u32; count], // Byte offset of each string data: [u8], // UTF-8 string data }
```

### Benefits

- "className" appears 500 times? Stored once
- O(1) string lookup by index
- Zero-copy access via byte slices

## Sections

### Section Header

```rust
struct SectionHeader { section_type: u8, // 0x01=Persona, 0x02=Standards, etc.
length: u32, // Byte length of section data }
```

### Persona Section (0x01)

```rust
struct PersonaSection { name: StringId, // Index into string table role: StringId, identity: Option<StringId>, style: Option<StringId>, traits_count: u16, traits: [StringId; traits_count], principles_count: u16, principles: [StringId; principles_count], }
```

### Standards Section (0x02)

```rust
struct StandardsSection { rules_count: u32, rules: [StandardRule; rules_count], }
struct StandardRule { category: u8, // RuleCategory enum priority: u8, // 0-255 description: StringId, pattern: Option<StringId>, }
```

#### Rule Categories

+-------+----------+
| Value | Category |
+=======+==========+
| 0     | Style    |
+-------+----------+



### Context Section (0x03)

```rust
struct ContextSection { includes_count: u16, includes: [StringId; includes_count], // Glob patterns excludes_count: u16, excludes: [StringId; excludes_count], focus_count: u16, focus: [StringId; focus_count], }
```

### Workflow Section (0x04)

```rust
struct WorkflowSection { name: StringId, description: StringId, steps_count: u16, steps: [WorkflowStep; steps_count], }
struct WorkflowStep { name: StringId, description: StringId, tool: Option<StringId>, input: Option<StringId>, expected_output: Option<StringId>, }
```

## Signature

When the `SIGNED` flag is set, a 64-byte Ed25519 signature follows all sections:
```rust
struct Signature { signature: [u8; 64], // Ed25519 signature over header + all sections }
```

### Verification

```rust
// Public key embedded in client const VERIFYING_KEY: [u8; 32] = [...];
fn verify(data: &[u8]) -> bool { let header = &data[0..16];
let payload = &data[16..data.len() - 64];
let signature = &data[data.len() - 64..];
let key = VerifyingKey::from_bytes(&VERIFYING_KEY)?;
key.verify(&[header, payload].concat(), signature).is_ok()
}
```

## Size Comparison

+---------+-------+------+-----------+
| Content | JSON  | .drv | Reduction |
+=========+=======+======+===========+
| 50      | rules | 4.2  | KB        |
+---------+-------+------+-----------+



## Versioning

### Version Migration

When opening a file with an older version: -Check `version` field in header -If `version < CURRENT_VERSION`, apply migrations -Migrations are additive (new fields have defaults)

### Compatibility Matrix

+--------+------+---------+------+
| Reader | v1.0 | v1.1    | v2.0 |
+========+======+=========+======+
| v1.0   | ✅    | ⚠️*     | ❌    |
+--------+------+---------+------+



## Implementation Notes

### Zero-Copy Parsing

```rust
// Direct memory access via bytemuck let header: &DrvHeader = bytemuck::from_bytes(&data[0..16]);
// String access via slice let string = &string_table_data[start..end];
```

### Checksum Validation

```rust
use blake3;
fn validate_checksum(data: &[u8]) -> bool { let stored = u32::from_le_bytes(data[12..16].try_into().unwrap());
let computed = blake3::hash(&data[16..]);
u32::from_le_bytes(computed.as_bytes()[0..4].try_into().unwrap()) == stored }
```

## Example

### Input (Markdown)

```markdown


## Persona


You are an expert Rust engineer.


### Traits


- Precise
- Security-conscious


## Standards



### Style


- Use rustfmt
```

### Output (Binary, hex dump)

```
44 52 56 00 # Magic: "DRV\0"
01 00 # Version: 1 01 00 # Flags: SIGNED 02 00 00 00 # Section count: 2 AB CD EF 12 # Checksum


# String table


05 00 00 00 # 5 strings 00 00 00 00 # Offset 0: "expert Rust engineer"
14 00 00 00 # Offset 20: "Precise"
1B 00 00 00 # Offset 27: "Security-conscious"
2D 00 00 00 # Offset 45: "Use rustfmt"
38 00 00 00 # Offset 56: "" (empty)
65 78 70 65 72 74 20 52 75 73 74 20 65 6E 67 69 6E 65 65 72 # "expert Rust engineer"
50 72 65 63 69 73 65 # "Precise"
...


# Persona section


01 # Type: Persona XX XX XX XX # Length ...


# Standards section


02 # Type: Standards XX XX XX XX # Length ...


# Ed25519 signature (64 bytes)


XX XX XX XX XX XX XX XX ...
```

## Tools

### Inspect

```bash


# Show binary structure


driven inspect rules.drv


# Verify signature


driven verify rules.drv --key public.key ```


### Convert


```bash

# Binary to readable

driven convert rules.drv rules.md

# Readable to binary

driven convert rules.md rules.drv ```

## Security Considerations

- Always verify signatures before applying rules
- Validate checksums to detect corruption
- Bound string indices to prevent out-of-bounds reads
- Limit section sizes to prevent memory exhaustion

## Future Extensions

- v1.1: Add `Imports` section for rule composition
- v1.2: Add `Variables` for parameterized rules
- v2.0: Streaming format for large rule sets
