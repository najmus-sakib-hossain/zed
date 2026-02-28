
# DX-Py Runtime

A Python runtime written in Rust, focused on correctness and developer experience.

## Status: In Development

DX-Py is actively being developed. While not yet production-ready, significant progress has been made on core Python features.

## Implemented Features

- Core Type Methods: String, list, and dict methods
- List Comprehensions: Basic, filtered, and nested
- Exception Handling: try/except/finally with type matching
- Class System: Classes, inheritance, super()
- JSON Module: dumps() and loads()
- Module System: Import caching and basic stdlib

## Installation

```bash
cd runtime cargo build --release


# Binary at target/release/dx-py


```

## Quick Start

```bash


# Execute expression


dx-py -c "1 + 2 * 3"


# Output: 7



# Run REPL


dx-py -i


# Show runtime info


dx-py info


# Compile to bytecode


dx-py compile script.py -o script.dpb


# Disassemble bytecode


dx-py disasm script.dpb ```


## CLI Options


+--------+-------------+
| Option | Description |
+========+=============+
| `-c    | <expr>`     |
+--------+-------------+


## Supported Features



### String Methods


- `upper()`, `lower()`, `strip()`, `lstrip()`, `rstrip()`
- `split()`, `join()`, `replace()`, `find()`
- `startswith()`, `endswith()`


### List Methods


- `append()`, `extend()`, `insert()`
- `remove()`, `pop()`, `clear()`
- `sort()`, `reverse()`, `index()`, `count()`


### Dict Methods


- `keys()`, `values()`, `items()`
- `get()`, `pop()`, `update()`, `clear()`


### Language Features


- List comprehensions (basic, filtered, nested)
- Exception handling (try/except/finally)
- Classes with inheritance and super()
- JSON module (dumps/loads)


## Architecture


@tree:runtime[]


## Testing


```bash

# Run all tests

cargo test --workspace

# Run specific crate

cargo test -p dx-py-core cargo test -p dx-py-interpreter

# Run with release optimizations

cargo test --release ```

### Expressions

- Arithmetic: `+`, `-`, `*`, `/`, `//`, `%`, `**`
- Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`
- Boolean: `and`, `or`, `not`
- Bitwise: `&`, `|`, `^`, `~`, `<<`, `>>`

### Built-in Functions

- `print`, `len`, `type`, `range`
- `int`, `float`, `str`, `bool`
- `abs`, `min`, `max`, `sum`
- `list`, `dict`, `tuple`, `set`

### Data Types

- `int`, `float`, `str`, `bool`
- `list`, `tuple`, `dict`, `set`
- `None`, `True`, `False`

## Known Limitations

- Dict/set comprehensions not yet supported
- Generator expressions not yet supported
- Async/await not yet implemented
- Some stdlib modules are stubs only
- Native extension loading is experimental

## License

MIT
