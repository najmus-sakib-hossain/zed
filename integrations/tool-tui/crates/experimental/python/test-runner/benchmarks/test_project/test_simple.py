"""Simple test functions for benchmarking."""
import time


def test_addition():
    """Test basic addition."""
    assert 1 + 1 == 2


def test_subtraction():
    """Test basic subtraction."""
    assert 5 - 3 == 2


def test_multiplication():
    """Test basic multiplication."""
    assert 3 * 4 == 12


def test_division():
    """Test basic division."""
    assert 10 / 2 == 5


def test_string_concat():
    """Test string concatenation."""
    assert "hello" + " " + "world" == "hello world"


def test_list_append():
    """Test list operations."""
    lst = [1, 2, 3]
    lst.append(4)
    assert lst == [1, 2, 3, 4]


def test_dict_operations():
    """Test dictionary operations."""
    d = {"a": 1, "b": 2}
    d["c"] = 3
    assert d == {"a": 1, "b": 2, "c": 3}


def test_set_operations():
    """Test set operations."""
    s = {1, 2, 3}
    s.add(4)
    assert 4 in s


def test_tuple_unpacking():
    """Test tuple unpacking."""
    a, b, c = (1, 2, 3)
    assert a == 1 and b == 2 and c == 3


def test_list_comprehension():
    """Test list comprehension."""
    squares = [x**2 for x in range(5)]
    assert squares == [0, 1, 4, 9, 16]
