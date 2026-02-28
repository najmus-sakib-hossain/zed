"""Test file to verify failure reporting for parametrized tests"""
import pytest


@pytest.mark.parametrize("x,expected", [(1, 1), (2, 4), (3, 9)])
def test_square(x, expected):
    """Test that should fail on second parameter set"""
    assert x * x == expected


@pytest.mark.parametrize("value", [10, 20, 30])
@pytest.mark.parametrize("multiplier", [1, 2])
def test_cartesian(value, multiplier):
    """Test cartesian product - should fail on some combinations"""
    result = value * multiplier
    assert result < 50  # Will fail for (30, 2)


@pytest.mark.parametrize("name", ["alice", "bob", "charlie"])
def test_string_param(name):
    """Test with string parameters"""
    assert len(name) > 3  # Will fail for "bob"
