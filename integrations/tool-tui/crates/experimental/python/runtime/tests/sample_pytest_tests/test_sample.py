"""Sample pytest test file with fixtures for DX-Py integration testing.

This file demonstrates pytest features that the DX-Py runtime should support:
- Fixtures (function, class, module, session scoped)
- Assertions with detailed failure messages
- Parametrized tests
- Exception testing with pytest.raises

Requirements: 10.1, 10.2, 10.3, 10.4
"""
import pytest


# =============================================================================
# Fixtures (Requirement 10.2)
# =============================================================================

@pytest.fixture
def sample_list():
    """Provide a sample list for testing."""
    return [1, 2, 3, 4, 5]


@pytest.fixture
def sample_dict():
    """Provide a sample dictionary for testing."""
    return {"name": "test", "value": 42, "active": True}


@pytest.fixture
def calculator():
    """Provide a simple calculator class instance."""
    class Calculator:
        def add(self, a, b):
            return a + b
        
        def subtract(self, a, b):
            return a - b
        
        def multiply(self, a, b):
            return a * b
        
        def divide(self, a, b):
            if b == 0:
                raise ValueError("Cannot divide by zero")
            return a / b
    
    return Calculator()


@pytest.fixture
def temp_data():
    """Fixture with setup and implicit teardown."""
    data = {"setup": True, "items": []}
    data["items"].append("initialized")
    return data


# =============================================================================
# Basic Assertion Tests (Requirement 10.3)
# =============================================================================

def test_basic_assertion():
    """Test basic assert statement."""
    assert True
    assert 1 + 1 == 2
    assert "hello" == "hello"


def test_assertion_with_message():
    """Test assert with custom message."""
    value = 42
    assert value == 42, f"Expected 42, got {value}"


def test_list_assertions(sample_list):
    """Test assertions with list fixture."""
    assert len(sample_list) == 5
    assert sample_list[0] == 1
    assert sample_list[-1] == 5
    assert 3 in sample_list
    assert 10 not in sample_list


def test_dict_assertions(sample_dict):
    """Test assertions with dict fixture."""
    assert "name" in sample_dict
    assert sample_dict["value"] == 42
    assert sample_dict.get("missing") is None


def test_calculator_add(calculator):
    """Test calculator addition."""
    assert calculator.add(2, 3) == 5
    assert calculator.add(-1, 1) == 0
    assert calculator.add(0, 0) == 0


def test_calculator_subtract(calculator):
    """Test calculator subtraction."""
    assert calculator.subtract(5, 3) == 2
    assert calculator.subtract(1, 1) == 0


def test_calculator_multiply(calculator):
    """Test calculator multiplication."""
    assert calculator.multiply(3, 4) == 12
    assert calculator.multiply(0, 100) == 0


# =============================================================================
# Parametrized Tests (Requirement 10.1)
# =============================================================================

@pytest.mark.parametrize("a,b,expected", [
    (1, 1, 2),
    (2, 3, 5),
    (10, 20, 30),
    (100, 200, 300),
    (-1, 1, 0),
    (0, 0, 0),
])
def test_addition_parametrized(a, b, expected):
    """Test addition with multiple parameter sets."""
    assert a + b == expected


@pytest.mark.parametrize("value,expected", [
    ("hello", "HELLO"),
    ("world", "WORLD"),
    ("Python", "PYTHON"),
    ("", ""),
    ("MiXeD", "MIXED"),
])
def test_upper_parametrized(value, expected):
    """Test string upper with multiple inputs."""
    assert value.upper() == expected


@pytest.mark.parametrize("lst,expected_sum", [
    ([1, 2, 3], 6),
    ([10, 20, 30], 60),
    ([], 0),
    ([5], 5),
    ([-1, 1], 0),
])
def test_sum_parametrized(lst, expected_sum):
    """Test sum with multiple list inputs."""
    assert sum(lst) == expected_sum


@pytest.mark.parametrize("n,expected", [
    (0, 1),
    (1, 1),
    (2, 2),
    (3, 6),
    (4, 24),
    (5, 120),
])
def test_factorial_parametrized(n, expected):
    """Test factorial calculation with parametrize."""
    def factorial(x):
        if x <= 1:
            return 1
        return x * factorial(x - 1)
    
    assert factorial(n) == expected


# =============================================================================
# Exception Testing with pytest.raises (Requirement 10.4)
# =============================================================================

def test_raises_value_error(calculator):
    """Test that division by zero raises ValueError."""
    with pytest.raises(ValueError):
        calculator.divide(10, 0)


def test_raises_with_match(calculator):
    """Test exception with message matching."""
    with pytest.raises(ValueError, match="Cannot divide by zero"):
        calculator.divide(5, 0)


def test_raises_key_error():
    """Test KeyError is raised for missing dict key."""
    d = {"a": 1}
    with pytest.raises(KeyError):
        _ = d["missing"]


def test_raises_index_error():
    """Test IndexError for out of bounds access."""
    lst = [1, 2, 3]
    with pytest.raises(IndexError):
        _ = lst[10]


def test_raises_type_error():
    """Test TypeError for invalid operations."""
    with pytest.raises(TypeError):
        _ = "string" + 42


# =============================================================================
# Combined Fixture Tests
# =============================================================================

def test_multiple_fixtures(sample_list, sample_dict, calculator):
    """Test using multiple fixtures together."""
    # Use sample_list
    total = sum(sample_list)
    assert total == 15
    
    # Use sample_dict
    assert sample_dict["name"] == "test"
    
    # Use calculator
    result = calculator.add(total, sample_dict["value"])
    assert result == 57  # 15 + 42


def test_fixture_modification(temp_data):
    """Test that fixture data can be modified within test."""
    assert temp_data["setup"] is True
    assert "initialized" in temp_data["items"]
    
    # Modify the fixture data
    temp_data["items"].append("modified")
    assert len(temp_data["items"]) == 2


# =============================================================================
# Edge Cases and Special Assertions
# =============================================================================

def test_none_assertions():
    """Test assertions involving None."""
    value = None
    assert value is None
    assert value != 0
    assert value != ""
    assert value != []


def test_boolean_assertions():
    """Test boolean assertions."""
    assert True
    assert not False
    assert bool(1)
    assert not bool(0)
    assert bool("non-empty")
    assert not bool("")


def test_comparison_assertions():
    """Test comparison assertions."""
    assert 5 > 3
    assert 3 < 5
    assert 5 >= 5
    assert 5 <= 5
    assert 5 != 3


def test_collection_membership():
    """Test collection membership assertions."""
    lst = [1, 2, 3]
    assert 2 in lst
    assert 4 not in lst
    
    s = {1, 2, 3}
    assert 1 in s
    
    d = {"a": 1}
    assert "a" in d
    assert "b" not in d


def test_string_assertions():
    """Test string-specific assertions."""
    s = "Hello, World!"
    assert s.startswith("Hello")
    assert s.endswith("!")
    assert "World" in s
    assert s.lower() == "hello, world!"
