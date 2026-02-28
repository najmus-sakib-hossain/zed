"""Parametrized tests for benchmarking."""
import pytest


@pytest.mark.parametrize("a,b,expected", [
    (1, 1, 2),
    (2, 3, 5),
    (10, 20, 30),
    (100, 200, 300),
    (-1, 1, 0),
])
def test_addition_parametrized(a, b, expected):
    """Test addition with multiple inputs."""
    assert a + b == expected


@pytest.mark.parametrize("value,expected", [
    ("hello", "HELLO"),
    ("world", "WORLD"),
    ("Python", "PYTHON"),
    ("TEST", "TEST"),
    ("", ""),
])
def test_upper_parametrized(value, expected):
    """Test string upper with multiple inputs."""
    assert value.upper() == expected


@pytest.mark.parametrize("lst,expected", [
    ([1, 2, 3], 6),
    ([10, 20, 30], 60),
    ([], 0),
    ([5], 5),
    ([-1, 1], 0),
])
def test_sum_parametrized(lst, expected):
    """Test sum with multiple inputs."""
    assert sum(lst) == expected


@pytest.mark.parametrize("n,expected", [
    (0, 1),
    (1, 1),
    (5, 120),
    (10, 3628800),
])
def test_factorial_parametrized(n, expected):
    """Test factorial calculation."""
    def factorial(x):
        if x <= 1:
            return 1
        return x * factorial(x - 1)
    assert factorial(n) == expected


@pytest.mark.parametrize("s,expected", [
    ("", True),
    ("a", True),
    ("aa", True),
    ("aba", True),
    ("abba", True),
    ("abc", False),
    ("racecar", True),
])
def test_palindrome_parametrized(s, expected):
    """Test palindrome detection."""
    assert (s == s[::-1]) == expected
