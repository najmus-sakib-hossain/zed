
# DX Serializer API Reference

Complete Rust API documentation for dx-serializer.

## Core Functions

### parse()

Parse DX format into typed structures.
```rust
pub fn parse(input: &[u8]) -> Result<DxValue, DxError> ```
Parameters: -`input: &[u8]` — DX format bytes (UTF-8) Returns: -`Ok(DxValue)` — Parsed data structure -`Err(DxError)` — Parse error with position and message Example:
```rust
let data = parse(b"name:Alice^age:30")?;
```
Performance: ~1.9µs for typical documents (SIMD-accelerated).


### encode()


Encode Rust structures to DX format.
```rust
pub fn encode(value: &DxValue) -> Result<Vec<u8>, DxError> ```
Parameters: -`value: &DxValue` — Data to encode Returns: -`Ok(Vec<u8>)` — DX format bytes -`Err(DxError)` — Encoding error Example:
```rust
let mut obj = DxObject::new();
obj.insert("name".to_string(), DxValue::String("Alice".into()));
let encoded = encode(&DxValue::Object(obj))?;
```
Optimizations: -Automatic ditto compression for repeated values -Alias generation for repeated keys -Optimal operator selection (`:` vs `^` vs `>`)

### format_human()

Format DX data for human display (LSP).
```rust
pub fn format_human(value: &DxValue) -> Result<String, DxError> ```
Parameters: -`value: &DxValue` — Data to format Returns: -`Ok(String)` — Beautifully formatted output -`Err(DxError)` — Formatting error Example:
```rust
let formatted = format_human(&data)?;
println!("{}", formatted);
```
Output:
```
name : Alice age : 30 active : ✓ ```
Features: -Column alignment -Unicode symbols (✓/✗ for booleans) -Table box drawing -Configurable styling

## Data Types

### DxValue

Core enum representing all DX values.
```rust
pub enum DxValue { Object(DxObject), Array(DxArray), Table(DxTable), String(Cow<'static, str>), Int(i64), Float(f64), Bool(bool), Null, Anchor(u32), }
```
Methods:

#### as_str()

```rust
pub fn as_str(&self) -> Option<&str> ```
Returns `Some(&str)` if value is String, `None` otherwise.


#### as_int()


```rust
pub fn as_int(&self) -> Option<i64> ```
Returns `Some(i64)` if value is Int, `None` otherwise.

#### as_float()

```rust
pub fn as_float(&self) -> Option<f64> ```
Returns `Some(f64)` if value is Float, `None` otherwise.


#### as_bool()


```rust
pub fn as_bool(&self) -> Option<bool> ```
Returns `Some(bool)` if value is Bool, `None` otherwise.

#### type_name()

```rust
pub fn type_name(&self) -> &'static str ```
Returns human-readable type name. Example:
```rust
let val = DxValue::Int(42);
assert_eq!(val.type_name(), "int");
assert_eq!(val.as_int(), Some(42));
```


### DxObject


Ordered key-value map.
```rust
pub struct DxObject { pub fields: Vec<(String, DxValue)>, lookup: FxHashMap<String, usize>, }
```
Methods:


#### new()


```rust
pub fn new() -> Self ```
Creates empty object.

#### insert()

```rust
pub fn insert(&mut self, key: String, value: DxValue)
```
Inserts key-value pair (preserves order).

#### get()

```rust
pub fn get(&self, key: &str) -> Option<&DxValue> ```
Gets value by key (O(1) lookup).


#### get_mut()


```rust
pub fn get_mut(&mut self, key: &str) -> Option<&mut DxValue> ```
Gets mutable value reference.

#### contains_key()

```rust
pub fn contains_key(&self, key: &str) -> bool ```
Checks if key exists. Example:
```rust
let mut obj = DxObject::new();
obj.insert("name".into(), DxValue::String("Alice".into()));
assert!(obj.contains_key("name"));
assert_eq!(obj.get("name").unwrap().as_str(), Some("Alice"));
```


### DxArray


Dynamic array of values.
```rust
pub struct DxArray { pub elements: Vec<DxValue>, }
```
Methods:


#### new()


```rust
pub fn new() -> Self ```
Creates empty array.

#### push()

```rust
pub fn push(&mut self, value: DxValue)
```
Appends value to end.

#### get()

```rust
pub fn get(&self, index: usize) -> Option<&DxValue> ```
Gets value by index.


#### len()


```rust
pub fn len(&self) -> usize ```
Returns element count. Example:
```rust
let mut arr = DxArray::new();
arr.push(DxValue::Int(1));
arr.push(DxValue::Int(2));
assert_eq!(arr.len(), 2);
```

### DxTable

Schema-guided tabular data.
```rust
pub struct DxTable { pub schema: Schema, pub rows: Vec<Vec<DxValue>>, }
```
Fields: -`schema: Schema` — Column definitions with type hints -`rows: Vec<Vec<DxValue>>` — Data rows Example:
```rust
// Parse table let data = parse(b"users=id%i name%s age%i 1 Alice 30 2 Bob 25 ")?;
if let DxValue::Table(table) = data.get("users").unwrap() { for row in &table.rows { println!("{}: {} ({})", row[0].as_int().unwrap(), row[1].as_str().unwrap(), row[2].as_int().unwrap()
);
}
}
```

### Schema

Table column schema.
```rust
pub struct Schema { pub columns: Vec<Column>, }
pub struct Column { pub name: String, pub type_hint: TypeHint, }
pub enum TypeHint { Int, // %i Float, // %f String, // %s Bool, // %b Auto, // (inferred)
}
```
Methods:

#### parse_definition()

```rust
pub fn parse_definition(input: &[u8]) -> Result<Self, DxError> ```
Parses schema string like `"id%i name%s age%i"`. Example:
```rust
let schema = Schema::parse_definition(b"id%i name%s")?;
assert_eq!(schema.columns.len(), 2);
assert_eq!(schema.columns[0].name, "id");
assert_eq!(schema.columns[0].type_hint, TypeHint::Int);
```


## Error Handling



### DxError


Comprehensive error type.
```rust
pub enum DxError { UnexpectedEof { pos: usize }, InvalidSyntax { pos: usize, msg: String }, InvalidUtf8 { pos: usize }, InvalidNumber { pos: usize, value: String }, SchemaError { msg: String }, TypeMismatch { expected: String, found: String, pos: usize }, DuplicateKey { key: String, pos: usize }, UnknownAlias { alias: String, pos: usize }, UnknownAnchor { id: u32, pos: usize }, TableSchemaRequired { pos: usize }, IoError(std::io::Error), Utf8Error(std::str::Utf8Error), }
```
Display: All errors include position and context. Example:
```rust
match parse(b"invalid:") { Err(DxError::UnexpectedEof { pos }) => { eprintln!("Parse error at position {}", pos);
}
Err(DxError::InvalidSyntax { pos, msg }) => { eprintln!("Syntax error at {}: {}", pos, msg);
}
_ => {}
}
```


## Configuration



### EncoderConfig


Configure encoder behavior.
```rust
pub struct EncoderConfig { pub use_aliases: bool, // Generate $alias shortcuts pub use_ditto: bool, // Use " for repeated values pub alias_min_length: usize, // Minimum chars for alias }
```
Default:
```rust
EncoderConfig { use_aliases: true, use_ditto: true, alias_min_length: 8, }
```
Usage:
```rust
let config = EncoderConfig { use_ditto: true, ..Default::default()
};
let encoder = Encoder::new(config);
let bytes = encoder.encode(&data)?;
```


### FormatterConfig


Configure human formatter.
```rust
pub struct FormatterConfig { pub column_padding: usize, // Spaces between columns pub use_unicode: bool, // ✓/✗ vs true/false pub add_dividers: bool, // Table separators }
```
Default:
```rust
FormatterConfig { column_padding: 4, use_unicode: true, add_dividers: true, }
```
Usage:
```rust
let config = FormatterConfig { use_unicode: false, // ASCII only ..Default::default()
};
let formatter = HumanFormatter::new(config);
let output = formatter.format(&data)?;
```


## Advanced Usage



### Zero-Copy Parsing


```rust
use std::fs;
fn parse_file(path: &str) -> Result<DxValue, DxError> { let bytes = fs::read(path)?;
// Parser operates directly on byte slice // No string allocation, no UTF-8 validation until needed parse(&bytes)
}
```
Performance: ~70% less memory than JSON parsers.


### Streaming Large Files


For files > 100MB, use streaming parser:
```rust
use std::fs::File;
use std::io::BufReader;
fn parse_stream(path: &str) -> Result<(), DxError> { let file = File::open(path)?;
let reader = BufReader::new(file);
// TODO: Implement streaming parser // Coming in v0.2.0 Ok(())
}
```


### Custom Type Conversion


Implement `From<T>` for your types:
```rust
struct User { name: String, age: u32, }
impl From<User> for DxValue { fn from(user: User) -> Self { let mut obj = DxObject::new();
obj.insert("name".into(), DxValue::String(user.name.into()));
obj.insert("age".into(), DxValue::Int(user.age as i64));
DxValue::Object(obj)
}
}
```


## Performance Tips



### 1. Use Type Hints


Slow (type inference):
```dx
count:42 ```
Fast (explicit hint):
```dx
count%i:42 ```
Type hints enable zero-copy vacuum parsing (4-5x speedup).


### 2. Reuse Buffers


```rust
let mut buffer = Vec::with_capacity(1024);
for item in items { buffer.clear();
buffer.extend_from_slice(&encode(&item)?);
// Write buffer...
}
```
Avoids repeated allocations.


### 3. Batch Operations


Slow:
```rust
for item in items { let encoded = encode(&item)?;
write(&encoded)?;
}
```
Fast:
```rust
let mut batch = DxArray::new();
for item in items { batch.push(item);
}
let encoded = encode(&DxValue::Array(batch))?;
write(&encoded)?;
```
Amortizes encoding overhead.


## Testing



### Unit Tests


```rust

#[cfg(test)]

mod tests { use super::*;

#[test]

fn test_parse_basic() { let data = parse(b"name:Alice").unwrap();
let obj = data.as_object().unwrap();
assert_eq!(obj.get("name").unwrap().as_str(), Some("Alice"));
}
}
```
Run: `cargo test`


### Benchmarks


```rust

#[cfg(test)]

mod benches { use criterion::{black_box, Criterion};
fn bench_parse(c: &mut Criterion) { let input = b"name:Alice^age:30^active:+";
c.bench_function("parse_simple", |b| { b.iter(|| parse(black_box(input)))
});
}
}
```
Run: `cargo bench`


## Integration Examples



### Axum Web Server


```rust
use axum::{Json, extract::Path};
async fn get_user(Path(id): Path<u32>) -> Json<DxValue> { let data = parse(&fetch_user_dx(id)).unwrap();
Json(data)
}
```


### Serde Integration (v0.2.0)


```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]

struct User { name: String, age: u32, }
let user = User { name: "Alice".into(), age: 30 };
let bytes = dx_serializer::to_bytes(&user)?;
let parsed: User = dx_serializer::from_bytes(&bytes)?;
```
See Also: -Syntax Guide (SYNTAX.md) -Performance Guide (PERFORMANCE.md) -Examples (../examples/)
