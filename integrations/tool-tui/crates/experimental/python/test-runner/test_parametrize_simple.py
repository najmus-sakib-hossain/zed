"""Simple test file with one parametrize decorator"""
import pytest


@pytest.mark.parametrize("x", [1, 2, 3])
def test_simple(x):
    """Test with single parameter"""
    assert x > 0
