"""
Example demonstrating yield-based fixture teardown

This example shows how pytest fixtures with yield work:
1. Code before yield runs during setup
2. The yielded value is injected into the test
3. Code after yield runs during teardown (cleanup)

Requirement 11.4: WHEN a fixture uses `yield`, THE Test_Runner SHALL execute teardown code after the test
"""

import pytest


@pytest.fixture
def temp_file(tmp_path):
    """Fixture that creates a temporary file and cleans it up after the test.
    
    Setup: Create and write to file
    Yield: Provide file path to test
    Teardown: Close and cleanup (handled by tmp_path)
    """
    file_path = tmp_path / "test_data.txt"
    file_path.write_text("initial content")
    print(f"Setup: Created file at {file_path}")
    
    yield file_path
    
    print(f"Teardown: Cleaning up file at {file_path}")
    # Cleanup happens here after the test completes


@pytest.fixture
def database_connection():
    """Fixture that simulates a database connection with proper cleanup.
    
    Setup: Open connection
    Yield: Provide connection to test
    Teardown: Close connection
    """
    class MockDB:
        def __init__(self):
            self.connected = True
            print("Setup: Database connection opened")
        
        def query(self, sql):
            if not self.connected:
                raise RuntimeError("Database not connected")
            return f"Result for: {sql}"
        
        def close(self):
            self.connected = False
            print("Teardown: Database connection closed")
    
    db = MockDB()
    yield db
    db.close()


@pytest.fixture
def api_client(database_connection):
    """Fixture that depends on database_connection.
    
    Demonstrates dependency chain teardown:
    - Setup order: database_connection, then api_client
    - Teardown order: api_client, then database_connection (reverse)
    """
    class MockAPI:
        def __init__(self, db):
            self.db = db
            print("Setup: API client initialized")
        
        def get_users(self):
            return self.db.query("SELECT * FROM users")
        
        def cleanup(self):
            print("Teardown: API client cleanup")
    
    api = MockAPI(database_connection)
    yield api
    api.cleanup()


# Tests using yield-based fixtures

def test_temp_file_usage(temp_file):
    """Test that uses a temporary file fixture.
    
    Expected execution order:
    1. Setup: temp_file fixture creates file
    2. Test: reads and modifies file
    3. Teardown: temp_file fixture cleans up
    """
    # Test can read the file
    content = temp_file.read_text()
    assert content == "initial content"
    
    # Test can modify the file
    temp_file.write_text("modified content")
    assert temp_file.read_text() == "modified content"
    
    # Teardown will run after this test completes


def test_database_connection(database_connection):
    """Test that uses a database connection fixture.
    
    Expected execution order:
    1. Setup: database_connection fixture opens connection
    2. Test: uses connection
    3. Teardown: database_connection fixture closes connection
    """
    result = database_connection.query("SELECT 1")
    assert "Result for:" in result
    
    # Connection will be closed in teardown


def test_api_client_with_dependency(api_client):
    """Test that uses api_client which depends on database_connection.
    
    Expected execution order:
    1. Setup: database_connection fixture opens connection
    2. Setup: api_client fixture initializes with connection
    3. Test: uses api client
    4. Teardown: api_client fixture cleanup (reverse order)
    5. Teardown: database_connection fixture closes connection
    """
    users = api_client.get_users()
    assert "SELECT * FROM users" in users
    
    # Teardown will run in reverse dependency order


def test_teardown_runs_even_on_failure(temp_file):
    """Test that demonstrates teardown runs even when test fails.
    
    Expected execution order:
    1. Setup: temp_file fixture creates file
    2. Test: fails with assertion error
    3. Teardown: temp_file fixture STILL cleans up (important!)
    """
    # This test will fail, but teardown should still run
    assert False, "This test intentionally fails"
    
    # Teardown will still execute despite the failure


@pytest.fixture
def multiple_resources():
    """Fixture demonstrating multiple resource cleanup.
    
    Shows that teardown code can handle multiple resources.
    """
    resources = []
    
    # Setup: acquire multiple resources
    for i in range(3):
        resource = f"resource_{i}"
        resources.append(resource)
        print(f"Setup: Acquired {resource}")
    
    yield resources
    
    # Teardown: release all resources in reverse order
    for resource in reversed(resources):
        print(f"Teardown: Released {resource}")


def test_multiple_resources(multiple_resources):
    """Test using fixture with multiple resources."""
    assert len(multiple_resources) == 3
    assert all(r.startswith("resource_") for r in multiple_resources)
    
    # All resources will be cleaned up in teardown


# Example of fixture without yield (no teardown)

@pytest.fixture
def simple_data():
    """Fixture without yield - no teardown code.
    
    This fixture just returns data, no cleanup needed.
    """
    return {"key": "value", "count": 42}


def test_simple_data(simple_data):
    """Test using a simple fixture without teardown."""
    assert simple_data["key"] == "value"
    assert simple_data["count"] == 42
    # No teardown for this fixture


if __name__ == "__main__":
    # Run with: pytest yield_fixture_example.py -v -s
    # The -s flag shows print statements to see setup/teardown order
    print("Run this file with pytest to see fixture setup and teardown in action")
    print("Example: pytest yield_fixture_example.py -v -s")
