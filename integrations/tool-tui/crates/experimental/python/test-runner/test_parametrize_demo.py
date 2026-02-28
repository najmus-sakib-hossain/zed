"""Demo file to test parametrize parsing"""
import pytest


@pytest.mark.parametrize("x", [1, 2, 3])
def test_single_param(x):
    """Test with single parameter"""
    assert x > 0


@pytest.mark.parametrize("x,y", [(1, 2), (3, 4), (5, 6)])
def test_multiple_params(x, y):
    """Test with multiple parameters"""
    assert x < y


@pytest.mark.parametrize("x", [1, 2])
@pytest.mark.parametrize("y", ["a", "b"])
def test_cartesian_product(x, y):
    """Test with cartesian product of parameters"""
    assert x > 0
    assert len(y) == 1


@pytest.mark.parametrize("value", [
    pytest.param(1, id="one"),
    pytest.param(2, id="two"),
    pytest.param(3, id="three"),
])
def test_custom_ids(value):
    """Test with custom IDs"""
    assert value > 0


@pytest.mark.parametrize("value", [
    pytest.param(1, marks=pytest.mark.xfail),
    2,
    3,
])
def test_with_xfail(value):
    """Test with xfail marker on one parameter"""
    assert value != 1
