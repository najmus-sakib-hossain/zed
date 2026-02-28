
# Fixture Injection Implementation

## Overview

Task 16.2 implements the core fixture injection mechanism for the DX-Py test runner. This enables pytest-compatible fixture injection where test function parameters are automatically matched to fixture names and resolved with their dependency chains.

## Requirements

This implementation satisfies: -Requirement 11.1: Test functions with parameters matching fixture names receive injected fixture values -Requirement 11.5: Fixtures with dependencies have their dependency chains resolved correctly

## Architecture

### Components

- Test Parameter Extraction (`dx-py-discovery/src/scanner.rs`)
- Extracts parameter names from test function signatures
- Filters out `self` and `cls` parameters
- Stores parameters in `TestCase` struct
- Fixture Matching (`dx-py-fixture/src/lib.rs`)
- Matches test parameters to registered fixture names
- Includes autouse fixtures automatically
- Returns only fixtures that match test parameters
- Dependency Resolution (`dx-py-fixture/src/lib.rs`)
- Resolves transitive fixture dependencies
- Performs topological sort for correct execution order
- Handles complex dependency graphs

### Data Flow

```
Test Function ↓ Parameter Extraction (scanner)
↓ Parameter List ["fixture_a", "fixture_b"]
↓ Fixture Matching (manager)
↓ Matched Fixtures + Autouse Fixtures ↓ Dependency Resolution (manager)
↓ Ordered Fixture List [dep1, dep2, fixture_a, fixture_b]
↓ Fixture Execution (executor)
```

## API

### New Methods

#### `TestCase::with_parameters(parameters: Vec<String>)`

Builder method to set test parameters.
```rust
let test = TestCase::new("test_example", "test.py", 10)
.with_parameters(vec!["fixture_a".to_string(), "fixture_b".to_string()]);
```

#### `FixtureManager::resolve_fixtures_for_test(test_parameters: &[String])`

Core fixture injection method that: -Matches test parameters to fixture names -Adds autouse fixtures -Resolves dependency chains -Returns fixtures in dependency order ```rust let resolved = manager.resolve_fixtures_for_test(&test.parameters)?;
```


## Examples



### Simple Fixture Injection


```python

# conftest.py

@pytest.fixture def sample_data():
return {"key": "value"}

# test_example.py

def test_with_fixture(sample_data):
assert sample_data["key"] == "value"
```
The test runner will: -Extract parameter `sample_data` from test function -Match it to the registered fixture -Inject the fixture value when executing the test


### Fixture with Dependencies


```python

# conftest.py

@pytest.fixture def config():
return {"db_url": "localhost"}
@pytest.fixture def db(config):
return Database(config["db_url"])
@pytest.fixture def api(db):
return API(db)

# test_api.py

def test_api_endpoint(api):
response = api.get("/users")
assert response.status == 200 ```
The test runner will: -Extract parameter `api` from test function -Resolve dependency chain: `api` → `db` → `config` -Execute fixtures in order: `config`, `db`, `api` -Inject `api` value to test

### Autouse Fixtures

```python


# conftest.py


@pytest.fixture(autouse=True)
def setup_logging():
logging.basicConfig(level=logging.INFO)
@pytest.fixture def data():
return [1, 2, 3]


# test_example.py


def test_with_data(data):


# setup_logging is automatically injected


assert len(data) == 3 ```
The test runner will: -Extract parameter `data` from test function -Add autouse fixture `setup_logging` automatically -Execute both fixtures before the test


## Testing



### Unit Tests


- `test_fixture_injection_simple`: Basic parameter matching
- `test_fixture_injection_with_dependencies`: Dependency chain resolution
- `test_fixture_injection_multiple_params`: Multiple fixture parameters
- `test_fixture_injection_non_fixture_params`: Mixed fixture/non-fixture parameters
- `test_fixture_injection_with_autouse`: Autouse fixture inclusion
- `test_fixture_injection_empty_params`: Tests with no parameters
- `test_fixture_injection_chain_resolution`: Complex dependency chains


### Integration Tests


- `test_complete_fixture_injection_flow`: End-to-end fixture injection
- `test_fixture_injection_with_scopes`: Different fixture scopes
- `test_fixture_injection_complex_dependency_graph`: Complex dependency graphs


### Property Tests


Property-based tests for fixture injection will be added in task 16.6.


## Implementation Details



### Parameter Extraction


The scanner uses tree-sitter to parse Python AST and extract function parameters:
```rust
fn get_function_parameters(&self, node: Node, source: &[u8]) -> Vec<String> { let mut params = Vec::new();
for child in node.children(&mut node.walk()) { if child.kind() == "parameters" { for param in child.children(&mut child.walk()) { match param.kind() { "identifier" => { let param_name = self.node_text(param, source);
if param_name != "self" && param_name != "cls" { params.push(param_name);
}
}
// Handle typed_parameter, default_parameter, etc.
...
}
}
}
}
params }
```


### Fixture Matching


The fixture manager matches parameters to fixtures and adds autouse fixtures:
```rust
pub fn resolve_fixtures_for_test( &self, test_parameters: &[String], ) -> Result<Vec<ResolvedFixture>, FixtureError> { let mut fixture_names = Vec::new();
// Match test parameters to fixture names for param in test_parameters { if self.fixtures.contains_key(param) { fixture_names.push(param.clone());
}
}
// Add autouse fixtures for fixture in self.get_autouse_fixtures(FixtureScope::Function) { if !fixture_names.contains(&fixture.name) { fixture_names.push(fixture.name.clone());
}
}
// Resolve dependencies self.resolve_fixtures(&fixture_names)
}
```


### Dependency Resolution


The existing `resolve_fixtures` method handles dependency resolution: -Breadth-first traversal: Collects all fixtures and their dependencies -Topological sort: Orders fixtures so dependencies come before dependents -Cycle detection: Detects circular dependencies and reports errors


## Future Work


- Task 16.3: Implement fixture scopes (function, class, module, session)
- Task 16.4: Implement fixture teardown (yield-based fixtures)
- Task 16.5: Implement autouse fixtures
- Task 16.6: Write property tests for fixture injection


## References


- Design Document: `.kiro/specs/dx-py-production-ready/design.md`
- Requirements: `.kiro/specs/dx-py-production-ready/requirements.md`
- pytest Fixture Documentation: //docs.pytest.org/en/stable/fixture.html
