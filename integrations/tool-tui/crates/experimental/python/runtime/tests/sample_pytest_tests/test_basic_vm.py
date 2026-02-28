"""Basic VM integration tests without fixtures.

These tests verify basic Python functionality without pytest fixtures,
making them suitable for testing the basic test-runner execution.

Requirements: 10.1, 10.3
"""


# =============================================================================
# Basic Function Tests
# =============================================================================

def test_simple_addition():
    """Test basic addition."""
    assert 1 + 1 == 2


def test_simple_subtraction():
    """Test basic subtraction."""
    assert 5 - 3 == 2


def test_simple_multiplication():
    """Test basic multiplication."""
    assert 3 * 4 == 12


def test_simple_division():
    """Test basic division."""
    assert 10 / 2 == 5.0


def test_integer_division():
    """Test integer division."""
    assert 10 // 3 == 3


def test_modulo():
    """Test modulo operation."""
    assert 10 % 3 == 1


# =============================================================================
# String Tests
# =============================================================================

def test_string_concatenation():
    """Test string concatenation."""
    assert "Hello" + " " + "World" == "Hello World"


def test_string_upper():
    """Test string upper method."""
    assert "hello".upper() == "HELLO"


def test_string_lower():
    """Test string lower method."""
    assert "HELLO".lower() == "hello"


def test_string_strip():
    """Test string strip method."""
    assert "  hello  ".strip() == "hello"


def test_string_split():
    """Test string split method."""
    assert "a,b,c".split(",") == ["a", "b", "c"]


def test_string_join():
    """Test string join method."""
    assert ",".join(["a", "b", "c"]) == "a,b,c"


# =============================================================================
# List Tests
# =============================================================================

def test_list_append():
    """Test list append."""
    lst = [1, 2, 3]
    lst.append(4)
    assert lst == [1, 2, 3, 4]


def test_list_extend():
    """Test list extend."""
    lst = [1, 2]
    lst.extend([3, 4])
    assert lst == [1, 2, 3, 4]


def test_list_pop():
    """Test list pop."""
    lst = [1, 2, 3]
    val = lst.pop()
    assert val == 3
    assert lst == [1, 2]


def test_list_index():
    """Test list indexing."""
    lst = [10, 20, 30]
    assert lst[0] == 10
    assert lst[-1] == 30


def test_list_slice():
    """Test list slicing."""
    lst = [1, 2, 3, 4, 5]
    assert lst[1:4] == [2, 3, 4]
    assert lst[:3] == [1, 2, 3]
    assert lst[2:] == [3, 4, 5]


# =============================================================================
# Dictionary Tests
# =============================================================================

def test_dict_access():
    """Test dictionary access."""
    d = {"a": 1, "b": 2}
    assert d["a"] == 1
    assert d["b"] == 2


def test_dict_get():
    """Test dictionary get method."""
    d = {"a": 1}
    assert d.get("a") == 1
    assert d.get("b") is None
    assert d.get("b", 0) == 0


def test_dict_keys():
    """Test dictionary keys."""
    d = {"a": 1, "b": 2}
    assert set(d.keys()) == {"a", "b"}


def test_dict_values():
    """Test dictionary values."""
    d = {"a": 1, "b": 2}
    assert set(d.values()) == {1, 2}


def test_dict_items():
    """Test dictionary items."""
    d = {"a": 1, "b": 2}
    assert set(d.items()) == {("a", 1), ("b", 2)}


# =============================================================================
# Control Flow Tests
# =============================================================================

def test_if_else():
    """Test if/else statement."""
    x = 10
    if x > 5:
        result = "big"
    else:
        result = "small"
    assert result == "big"


def test_for_loop():
    """Test for loop."""
    total = 0
    for i in range(5):
        total += i
    assert total == 10


def test_while_loop():
    """Test while loop."""
    count = 0
    while count < 5:
        count += 1
    assert count == 5


def test_list_comprehension():
    """Test list comprehension."""
    squares = [x * x for x in range(5)]
    assert squares == [0, 1, 4, 9, 16]


def test_dict_comprehension():
    """Test dictionary comprehension."""
    d = {x: x * x for x in range(3)}
    assert d == {0: 0, 1: 1, 2: 4}


# =============================================================================
# Function Tests
# =============================================================================

def test_function_definition():
    """Test function definition and call."""
    def add(a, b):
        return a + b
    
    assert add(2, 3) == 5


def test_function_default_args():
    """Test function with default arguments."""
    def greet(name, greeting="Hello"):
        return f"{greeting}, {name}!"
    
    assert greet("World") == "Hello, World!"
    assert greet("Python", "Hi") == "Hi, Python!"


def test_lambda():
    """Test lambda function."""
    double = lambda x: x * 2
    assert double(5) == 10


def test_recursive_function():
    """Test recursive function."""
    def factorial(n):
        if n <= 1:
            return 1
        return n * factorial(n - 1)
    
    assert factorial(5) == 120


# =============================================================================
# Class Tests
# =============================================================================

def test_class_definition():
    """Test class definition and instantiation."""
    class Point:
        def __init__(self, x, y):
            self.x = x
            self.y = y
    
    p = Point(3, 4)
    assert p.x == 3
    assert p.y == 4


def test_class_method():
    """Test class method."""
    class Calculator:
        def add(self, a, b):
            return a + b
    
    calc = Calculator()
    assert calc.add(2, 3) == 5


def test_class_inheritance():
    """Test class inheritance."""
    class Animal:
        def speak(self):
            return "..."
    
    class Dog(Animal):
        def speak(self):
            return "Woof!"
    
    dog = Dog()
    assert dog.speak() == "Woof!"


# =============================================================================
# Builtin Function Tests
# =============================================================================

def test_len():
    """Test len builtin."""
    assert len([1, 2, 3]) == 3
    assert len("hello") == 5
    assert len({"a": 1, "b": 2}) == 2


def test_range():
    """Test range builtin."""
    assert list(range(5)) == [0, 1, 2, 3, 4]
    assert list(range(2, 5)) == [2, 3, 4]
    assert list(range(0, 10, 2)) == [0, 2, 4, 6, 8]


def test_sum():
    """Test sum builtin."""
    assert sum([1, 2, 3, 4, 5]) == 15
    assert sum([]) == 0


def test_min_max():
    """Test min and max builtins."""
    assert min([3, 1, 4, 1, 5]) == 1
    assert max([3, 1, 4, 1, 5]) == 5


def test_sorted():
    """Test sorted builtin."""
    assert sorted([3, 1, 4, 1, 5]) == [1, 1, 3, 4, 5]
    assert sorted([3, 1, 4], reverse=True) == [4, 3, 1]


def test_enumerate():
    """Test enumerate builtin."""
    result = list(enumerate(["a", "b", "c"]))
    assert result == [(0, "a"), (1, "b"), (2, "c")]


def test_zip():
    """Test zip builtin."""
    result = list(zip([1, 2, 3], ["a", "b", "c"]))
    assert result == [(1, "a"), (2, "b"), (3, "c")]


def test_map():
    """Test map builtin."""
    result = list(map(lambda x: x * 2, [1, 2, 3]))
    assert result == [2, 4, 6]


def test_filter():
    """Test filter builtin."""
    result = list(filter(lambda x: x % 2 == 0, [1, 2, 3, 4, 5]))
    assert result == [2, 4]


def test_isinstance():
    """Test isinstance builtin."""
    assert isinstance(42, int)
    assert isinstance("hello", str)
    assert isinstance([1, 2], list)
    assert not isinstance(42, str)


def test_type():
    """Test type builtin."""
    assert type(42) == int
    assert type("hello") == str
    assert type([1, 2]) == list
