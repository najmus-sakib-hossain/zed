//! Demonstration of fixture discovery functionality
//!
//! This example shows how to:
//! 1. Discover fixtures from Python files
//! 2. Build a fixture registry
//! 3. Query fixtures by name and scope
//! 4. Validate dependencies

use dx_py_discovery::FixtureDiscovery;
use dx_py_fixture::{FixtureRegistry, FixtureScope};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Fixture Discovery Demo ===\n");

    // Create discovery scanner and registry
    let mut discovery = FixtureDiscovery::new()?;
    let mut registry = FixtureRegistry::new();

    // Try to discover fixtures from sample conftest.py
    let conftest_path = PathBuf::from("../../../runtime/tests/sample_pytest_tests/conftest.py");
    
    if conftest_path.exists() {
        println!("📁 Discovering fixtures from conftest.py...");
        let fixtures = discovery.discover_file(&conftest_path)?;
        println!("   Found {} fixtures\n", fixtures.len());

        // Register all fixtures
        for fixture in &fixtures {
            println!("   ✓ {} (scope: {:?}, autouse: {}, generator: {})",
                fixture.name,
                fixture.scope,
                fixture.autouse,
                fixture.is_generator
            );
        }
        registry.register_all(fixtures);
        println!();
    } else {
        println!("⚠️  Sample conftest.py not found, using inline example\n");
        
        // Demonstrate with inline Python code
        let sample_code = r#"
import pytest

@pytest.fixture
def sample_data():
    return {"key": "value"}

@pytest.fixture(scope="module")
def module_config():
    return {"debug": True}

@pytest.fixture(autouse=True)
def auto_setup():
    print("Setup")

@pytest.fixture
def dependent_fixture(sample_data, module_config):
    return sample_data | module_config

@pytest.fixture
def temp_resource():
    resource = create_resource()
    yield resource
    cleanup(resource)
"#;

        println!("📝 Discovering fixtures from inline code...");
        let discovered = discovery.discover_source(sample_code)?;
        println!("   Found {} fixtures\n", discovered.len());

        // Convert discovered fixtures to FixtureDefinitions and register
        for disc in &discovered {
            println!("   ✓ {} (scope: {:?}, autouse: {}, generator: {}, deps: {:?})",
                disc.name,
                disc.scope,
                disc.autouse,
                disc.is_generator,
                disc.dependencies
            );
            
            // Create fixture definition from discovered fixture
            use dx_py_fixture::FixtureDefinition;
            let fixture = FixtureDefinition::new(&disc.name, "inline.py", disc.line)
                .with_scope(disc.scope)
                .with_autouse(disc.autouse)
                .with_dependencies(disc.dependencies.clone())
                .with_generator(disc.is_generator);
            registry.register(fixture);
        }
        println!();
    }

    // Query fixtures by scope
    println!("🔍 Querying fixtures by scope:");
    for scope in [FixtureScope::Function, FixtureScope::Module, FixtureScope::Session] {
        let fixtures = registry.get_fixtures_by_scope(scope);
        println!("   {:?}: {} fixtures", scope, fixtures.len());
    }
    println!();

    // Query autouse fixtures
    println!("🔍 Autouse fixtures:");
    let autouse = registry.get_autouse_fixtures(FixtureScope::Function);
    for fixture in autouse {
        println!("   ✓ {}", fixture.name);
    }
    println!();

    // Validate dependencies
    println!("✅ Validating fixture dependencies...");
    match registry.validate_dependencies() {
        Ok(_) => println!("   All dependencies are valid"),
        Err(e) => println!("   ❌ Dependency error: {}", e),
    }

    match registry.detect_circular_dependencies() {
        Ok(_) => println!("   No circular dependencies detected"),
        Err(e) => println!("   ❌ Circular dependency: {}", e),
    }
    println!();

    // Summary
    println!("📊 Summary:");
    println!("   Total fixtures: {}", registry.len());
    println!("   Fixture names: {:?}", registry.fixture_names());

    Ok(())
}
