# Real pytest test file
import pytest

def test_simple():
    assert 1 + 1 == 2

def test_strings():
    assert "hello".upper() == "HELLO"

@pytest.mark.parametrize("x,y,expected", [
    (1, 2, 3),
    (2, 3, 5),
    (10, 20, 30),
])
def test_addition(x, y, expected):
    assert x + y == expected

class TestClass:
    def test_method(self):
        assert True
