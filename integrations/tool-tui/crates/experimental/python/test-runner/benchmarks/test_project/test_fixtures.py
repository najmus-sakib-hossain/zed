"""Tests with fixtures for benchmarking."""
import pytest


@pytest.fixture
def sample_list():
    """Provide a sample list."""
    return [1, 2, 3, 4, 5]


@pytest.fixture
def sample_dict():
    """Provide a sample dictionary."""
    return {"name": "test", "value": 42, "active": True}


@pytest.fixture
def sample_string():
    """Provide a sample string."""
    return "Hello, World!"


@pytest.fixture
def empty_list():
    """Provide an empty list."""
    return []


@pytest.fixture
def nested_dict():
    """Provide a nested dictionary."""
    return {
        "level1": {
            "level2": {
                "value": 100
            }
        }
    }


def test_list_length(sample_list):
    """Test list length with fixture."""
    assert len(sample_list) == 5


def test_list_sum(sample_list):
    """Test list sum with fixture."""
    assert sum(sample_list) == 15


def test_list_max(sample_list):
    """Test list max with fixture."""
    assert max(sample_list) == 5


def test_list_min(sample_list):
    """Test list min with fixture."""
    assert min(sample_list) == 1


def test_dict_keys(sample_dict):
    """Test dict keys with fixture."""
    assert set(sample_dict.keys()) == {"name", "value", "active"}


def test_dict_values(sample_dict):
    """Test dict values with fixture."""
    assert sample_dict["value"] == 42


def test_dict_get(sample_dict):
    """Test dict get with fixture."""
    assert sample_dict.get("missing", "default") == "default"


def test_string_length(sample_string):
    """Test string length with fixture."""
    assert len(sample_string) == 13


def test_string_split(sample_string):
    """Test string split with fixture."""
    assert sample_string.split(", ") == ["Hello", "World!"]


def test_empty_list_append(empty_list):
    """Test appending to empty list."""
    empty_list.append(1)
    assert empty_list == [1]


def test_nested_dict_access(nested_dict):
    """Test nested dict access."""
    assert nested_dict["level1"]["level2"]["value"] == 100


def test_combined_fixtures(sample_list, sample_dict):
    """Test with multiple fixtures."""
    assert len(sample_list) == 5
    assert len(sample_dict) == 3
