"""Shared pytest fixtures for the test suite.

This conftest.py demonstrates different fixture scopes that the DX-Py
runtime should support when running pytest tests.

Requirements: 10.2
"""
import pytest


@pytest.fixture(scope="module")
def module_data():
    """Module-scoped fixture - created once per module."""
    return {"module": "test_module", "count": 0}


@pytest.fixture(scope="session")
def session_config():
    """Session-scoped fixture - created once per test session."""
    return {
        "debug": False,
        "timeout": 30,
        "retries": 3,
    }


@pytest.fixture
def counter():
    """Function-scoped counter fixture."""
    class Counter:
        def __init__(self):
            self.value = 0
        
        def increment(self):
            self.value += 1
            return self.value
        
        def decrement(self):
            self.value -= 1
            return self.value
        
        def reset(self):
            self.value = 0
    
    return Counter()


@pytest.fixture
def temp_file(tmp_path):
    """Fixture providing a temporary file path."""
    file_path = tmp_path / "test_file.txt"
    file_path.write_text("test content")
    yield file_path
    # Cleanup happens automatically with tmp_path


@pytest.fixture
def sample_data():
    """Fixture providing sample test data."""
    return {
        "numbers": [1, 2, 3, 4, 5],
        "strings": ["a", "b", "c"],
        "nested": {"inner": {"value": 42}},
    }
