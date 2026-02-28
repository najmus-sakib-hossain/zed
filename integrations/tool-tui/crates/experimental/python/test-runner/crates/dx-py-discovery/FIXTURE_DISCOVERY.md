
# Fixture Discovery

This module implements pytest fixture discovery using tree-sitter AST parsing. It can discover fixtures from Python source files without importing them, achieving fast discovery times.

## Features

- Parse `@pytest.fixture` decorators: Identifies fixture functions in Python source
- Extract fixture metadata:-Scope (function, class, module, session)
- Autouse flag
- Parameters (params)
- Dependencies (from function parameters)
- Generator detection (yield-based teardown)
- Build fixture registry: Stores all discovered fixtures for efficient lookup

## Usage

### Basic Discovery

```rust
use dx_py_discovery::FixtureDiscovery;
use std::path::Path;
// Create a fixture discovery scanner let mut discovery = FixtureDiscovery::new()?;
// Discover fixtures from a file let fixtures = discovery.discover_file(Path::new("conftest.py"))?;
for fixture in fixtures { println!("Found fixture: {} (scope: {:?})", fixture.name, fixture.scope);
}
```

### Building a Fixture Registry

```rust
use dx_py_fixture::FixtureRegistry;
use dx_py_discovery::FixtureDiscovery;
// Create registry and discovery let mut registry = FixtureRegistry::new();
let mut discovery = FixtureDiscovery::new()?;
// Discover and register fixtures from conftest.py let conftest_fixtures = discovery.discover_file(Path::new("conftest.py"))?;
registry.register_all(conftest_fixtures);
// Discover and register fixtures from test files let test_fixtures = discovery.discover_file(Path::new("test_example.py"))?;
registry.register_all(test_fixtures);
// Validate dependencies registry.validate_dependencies()?;
registry.detect_circular_dependencies()?;
// Look up fixtures if let Some(fixture) = registry.get("sample_data") { println!("Fixture scope: {:?}", fixture.scope);
println!("Dependencies: {:?}", fixture.dependencies);
}
```

## Supported Fixture Patterns

### Simple Fixture

```python
@pytest.fixture def sample_data():
return {"key": "value"}
```

### Fixture with Scope

```python
@pytest.fixture(scope="module")
def module_data():
return {"module": "test"}
@pytest.fixture(scope="session")
def session_config():
return {"debug": False}
```

### Autouse Fixture

```python
@pytest.fixture(autouse=True)
def auto_setup():
print("Automatic setup")
```

### Fixture with Dependencies

```python
@pytest.fixture def database():
return Database()
@pytest.fixture def user_service(database):
return UserService(database)
```

### Generator Fixture (with Teardown)

```python
@pytest.fixture def temp_file():
file = create_temp_file()
yield file cleanup(file)
```

### Multiple Attributes

```python
@pytest.fixture(scope="module", autouse=True)
def module_setup():
setup_module()
yield teardown_module()
```

## Implementation Details

### AST Parsing

The discovery uses tree-sitter to parse Python source code into an AST. It walks the tree looking for `decorated_definition` nodes that contain `@pytest.fixture` decorators.

### Decorator Argument Parsing

The parser extracts keyword arguments from the decorator call: -`scope`: Parsed as string and converted to `FixtureScope` enum -`autouse`: Parsed as boolean -`params`: Parsed as list (currently returns empty vec, needs enhancement)

### Dependency Detection

Function parameters are extracted from the `parameters` node in the AST. These become the fixture's dependencies.

### Generator Detection

The parser recursively searches for `yield` or `yield_statement` nodes to determine if a fixture is a generator (has teardown code).

## Performance

Fixture discovery using tree-sitter is extremely fast: -No Python import required -No code execution -Pure AST parsing -Typical discovery time: < 1ms per file

## Limitations

### Current Limitations

- Params parsing: The `params` parameter is not fully parsed yet. This requires more sophisticated list/tuple parsing.
- Indirect fixtures: `@pytest.fixture(indirect=True)` is not yet supported.
- Fixture factories: Dynamic fixture creation is not detected.

### Future Enhancements

- Full params parsing with type detection
- Indirect fixture support
- Factory fixture detection
- Fixture name inference from function name
- Better error messages with line numbers

## Testing

The module includes comprehensive tests:
```bash


# Run unit tests


cargo test -p dx-py-discovery fixture_discovery


# Run integration tests


cargo test -p dx-py-discovery --test fixture_discovery_integration ```


## Requirements


This implementation satisfies Requirement 11.1 from the DX-Py Production Ready specification: WHEN a test function has a parameter matching a fixture name, THE Test_Runner SHALL inject the fixture value The fixture discovery is the first step in the fixture injection pipeline: -Discovery (this module): Find and parse fixture definitions -Registration: Store fixtures in registry -Resolution: Match test parameters to fixtures -Injection: Execute fixtures and pass values to tests


## Related Modules


- `dx-py-fixture`: Fixture management, caching, and execution
- `dx-py-discovery`: Test discovery and scanning
- `dx-py-executor`: Test execution with fixture injection
