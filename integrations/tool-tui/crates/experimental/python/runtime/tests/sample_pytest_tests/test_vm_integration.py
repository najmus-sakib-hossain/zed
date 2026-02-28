"""Integration tests for DX-Py VM features.

This file tests the VM integration features that were implemented:
- User-defined functions (Requirement 1)
- Closures and nested functions (Requirement 2)
- Class definitions and instances (Requirement 3)
- Module imports (Requirement 4)
- List comprehensions (Requirement 5)
- Exception handling (Requirement 6)
- Context managers (Requirement 7)
- Decorators (Requirement 8)
- Builtin functions (Requirement 9)

Requirements: 10.1, 10.2, 10.3, 10.4
"""
import pytest


# =============================================================================
# Fixtures (Requirement 10.2)
# =============================================================================

@pytest.fixture
def counter_factory():
    """Factory fixture that creates counter closures."""
    def make_counter(start=0):
        count = start
        def counter():
            nonlocal count
            count += 1
            return count
        return counter
    return make_counter


@pytest.fixture
def sample_class():
    """Fixture providing a sample class for testing."""
    class Point:
        def __init__(self, x, y):
            self.x = x
            self.y = y
        
        def distance_from_origin(self):
            return (self.x ** 2 + self.y ** 2) ** 0.5
        
        def __repr__(self):
            return f"Point({self.x}, {self.y})"
    
    return Point


@pytest.fixture
def context_manager_tracker():
    """Fixture that tracks context manager calls."""
    class Tracker:
        def __init__(self):
            self.entered = False
            self.exited = False
            self.exit_args = None
        
        def __enter__(self):
            self.entered = True
            return self
        
        def __exit__(self, exc_type, exc_val, exc_tb):
            self.exited = True
            self.exit_args = (exc_type, exc_val, exc_tb)
            return False
    
    return Tracker


# =============================================================================
# User-Defined Functions Tests (Requirement 1)
# =============================================================================

def test_simple_function():
    """Test basic function definition and call."""
    def add(a, b):
        return a + b
    
    assert add(2, 3) == 5
    assert add(0, 0) == 0
    assert add(-1, 1) == 0


def test_function_with_defaults():
    """Test function with default argument values."""
    def greet(name, greeting="Hello"):
        return f"{greeting}, {name}!"
    
    assert greet("World") == "Hello, World!"
    assert greet("Python", "Hi") == "Hi, Python!"


def test_function_with_args():
    """Test function with *args."""
    def sum_all(*args):
        return sum(args)
    
    assert sum_all() == 0
    assert sum_all(1) == 1
    assert sum_all(1, 2, 3) == 6
    assert sum_all(1, 2, 3, 4, 5) == 15


def test_function_with_kwargs():
    """Test function with **kwargs."""
    def make_dict(**kwargs):
        return kwargs
    
    result = make_dict(a=1, b=2, c=3)
    assert result == {"a": 1, "b": 2, "c": 3}


def test_recursive_function():
    """Test recursive function calls."""
    def factorial(n):
        if n <= 1:
            return 1
        return n * factorial(n - 1)
    
    assert factorial(0) == 1
    assert factorial(1) == 1
    assert factorial(5) == 120
    assert factorial(10) == 3628800


# =============================================================================
# Closures and Nested Functions Tests (Requirement 2)
# =============================================================================

def test_simple_closure():
    """Test basic closure capturing outer variable."""
    def outer(x):
        def inner(y):
            return x + y
        return inner
    
    add_five = outer(5)
    assert add_five(3) == 8
    assert add_five(10) == 15


def test_closure_with_nonlocal(counter_factory):
    """Test closure with nonlocal variable modification."""
    counter = counter_factory(0)
    assert counter() == 1
    assert counter() == 2
    assert counter() == 3


def test_multiple_closures_independent(counter_factory):
    """Test that multiple closures have independent state."""
    counter1 = counter_factory(0)
    counter2 = counter_factory(100)
    
    assert counter1() == 1
    assert counter2() == 101
    assert counter1() == 2
    assert counter2() == 102


def test_nested_closures():
    """Test deeply nested closures."""
    def level1(a):
        def level2(b):
            def level3(c):
                return a + b + c
            return level3
        return level2
    
    f = level1(1)(2)
    assert f(3) == 6
    assert f(10) == 13


# =============================================================================
# Class Definitions and Instances Tests (Requirement 3)
# =============================================================================

def test_class_instantiation(sample_class):
    """Test class instantiation and attribute access."""
    p = sample_class(3, 4)
    assert p.x == 3
    assert p.y == 4


def test_method_call(sample_class):
    """Test method calls on instances."""
    p = sample_class(3, 4)
    assert p.distance_from_origin() == 5.0


def test_class_inheritance():
    """Test class inheritance and method override."""
    class Animal:
        def speak(self):
            return "..."
    
    class Dog(Animal):
        def speak(self):
            return "Woof!"
    
    class Cat(Animal):
        def speak(self):
            return "Meow!"
    
    dog = Dog()
    cat = Cat()
    
    assert dog.speak() == "Woof!"
    assert cat.speak() == "Meow!"


def test_super_call():
    """Test super() for calling parent methods."""
    class Base:
        def __init__(self, value):
            self.value = value
    
    class Derived(Base):
        def __init__(self, value, extra):
            super().__init__(value)
            self.extra = extra
    
    d = Derived(10, 20)
    assert d.value == 10
    assert d.extra == 20


def test_class_attributes():
    """Test class-level attributes."""
    class Counter:
        count = 0
        
        def __init__(self):
            Counter.count += 1
    
    assert Counter.count == 0
    c1 = Counter()
    assert Counter.count == 1
    c2 = Counter()
    assert Counter.count == 2


# =============================================================================
# List Comprehensions Tests (Requirement 5)
# =============================================================================

def test_simple_comprehension():
    """Test basic list comprehension."""
    result = [x * 2 for x in range(5)]
    assert result == [0, 2, 4, 6, 8]


def test_comprehension_with_condition():
    """Test list comprehension with if clause."""
    result = [x for x in range(10) if x % 2 == 0]
    assert result == [0, 2, 4, 6, 8]


def test_nested_comprehension():
    """Test nested list comprehension."""
    result = [[i * j for j in range(3)] for i in range(3)]
    assert result == [[0, 0, 0], [0, 1, 2], [0, 2, 4]]


def test_comprehension_with_function():
    """Test list comprehension calling a function."""
    def square(x):
        return x * x
    
    result = [square(x) for x in range(5)]
    assert result == [0, 1, 4, 9, 16]


# =============================================================================
# Exception Handling Tests (Requirement 6)
# =============================================================================

def test_try_except_basic():
    """Test basic try/except."""
    result = None
    try:
        result = 1 / 0
    except ZeroDivisionError:
        result = "caught"
    
    assert result == "caught"


def test_try_except_finally():
    """Test try/except/finally."""
    cleanup_called = False
    
    try:
        x = 1 / 0
    except ZeroDivisionError:
        pass
    finally:
        cleanup_called = True
    
    assert cleanup_called


def test_exception_binding():
    """Test binding exception to variable."""
    caught_message = None
    
    try:
        raise ValueError("test error")
    except ValueError as e:
        caught_message = str(e)
    
    assert caught_message == "test error"


def test_multiple_except_clauses():
    """Test multiple except clauses."""
    def get_error_type(value):
        try:
            if value == "key":
                d = {}
                return d["missing"]
            elif value == "index":
                lst = []
                return lst[10]
            elif value == "type":
                return "str" + 42
        except KeyError:
            return "KeyError"
        except IndexError:
            return "IndexError"
        except TypeError:
            return "TypeError"
    
    assert get_error_type("key") == "KeyError"
    assert get_error_type("index") == "IndexError"
    assert get_error_type("type") == "TypeError"


# =============================================================================
# Context Managers Tests (Requirement 7)
# =============================================================================

def test_context_manager_enter_exit(context_manager_tracker):
    """Test context manager __enter__ and __exit__ calls."""
    tracker = context_manager_tracker()
    
    assert not tracker.entered
    assert not tracker.exited
    
    with tracker:
        assert tracker.entered
        assert not tracker.exited
    
    assert tracker.exited
    assert tracker.exit_args == (None, None, None)


def test_context_manager_with_exception(context_manager_tracker):
    """Test context manager with exception in block."""
    tracker = context_manager_tracker()
    
    try:
        with tracker:
            raise ValueError("test")
    except ValueError:
        pass
    
    assert tracker.exited
    assert tracker.exit_args[0] is ValueError


def test_context_manager_as_binding(context_manager_tracker):
    """Test 'as' binding in with statement."""
    tracker_class = context_manager_tracker
    
    with tracker_class() as t:
        assert t.entered
        t.custom_value = 42
    
    assert t.custom_value == 42


# =============================================================================
# Decorators Tests (Requirement 8)
# =============================================================================

def test_simple_decorator():
    """Test basic decorator application."""
    def double_result(func):
        def wrapper(*args, **kwargs):
            return func(*args, **kwargs) * 2
        return wrapper
    
    @double_result
    def add(a, b):
        return a + b
    
    assert add(2, 3) == 10  # (2 + 3) * 2


def test_stacked_decorators():
    """Test multiple stacked decorators."""
    def add_one(func):
        def wrapper(*args, **kwargs):
            return func(*args, **kwargs) + 1
        return wrapper
    
    def double(func):
        def wrapper(*args, **kwargs):
            return func(*args, **kwargs) * 2
        return wrapper
    
    @add_one
    @double
    def get_five():
        return 5
    
    # Applied bottom-to-top: double(5) = 10, add_one(10) = 11
    assert get_five() == 11


def test_decorator_with_arguments():
    """Test decorator that takes arguments."""
    def multiply_by(factor):
        def decorator(func):
            def wrapper(*args, **kwargs):
                return func(*args, **kwargs) * factor
            return wrapper
        return decorator
    
    @multiply_by(3)
    def get_value():
        return 10
    
    assert get_value() == 30


# =============================================================================
# Builtin Functions Tests (Requirement 9)
# =============================================================================

def test_isinstance():
    """Test isinstance builtin."""
    assert isinstance(42, int)
    assert isinstance("hello", str)
    assert isinstance([1, 2], list)
    assert not isinstance(42, str)


def test_issubclass():
    """Test issubclass builtin."""
    class Animal:
        pass
    
    class Dog(Animal):
        pass
    
    assert issubclass(Dog, Animal)
    assert issubclass(Dog, object)
    assert not issubclass(Animal, Dog)


def test_getattr_setattr_hasattr():
    """Test attribute access builtins."""
    class Obj:
        x = 10
    
    o = Obj()
    
    assert hasattr(o, "x")
    assert not hasattr(o, "y")
    assert getattr(o, "x") == 10
    assert getattr(o, "y", "default") == "default"
    
    setattr(o, "y", 20)
    assert hasattr(o, "y")
    assert o.y == 20


def test_enumerate():
    """Test enumerate builtin."""
    result = list(enumerate(["a", "b", "c"]))
    assert result == [(0, "a"), (1, "b"), (2, "c")]
    
    result = list(enumerate(["x", "y"], start=10))
    assert result == [(10, "x"), (11, "y")]


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


def test_sorted():
    """Test sorted builtin."""
    assert sorted([3, 1, 4, 1, 5]) == [1, 1, 3, 4, 5]
    assert sorted([3, 1, 4], reverse=True) == [4, 3, 1]


def test_min_max_sum():
    """Test min, max, sum builtins."""
    lst = [3, 1, 4, 1, 5, 9, 2, 6]
    assert min(lst) == 1
    assert max(lst) == 9
    assert sum(lst) == 31


# =============================================================================
# Parametrized Tests (Requirement 10.1)
# =============================================================================

@pytest.mark.parametrize("input_val,expected", [
    (0, 1),
    (1, 1),
    (2, 2),
    (3, 6),
    (4, 24),
    (5, 120),
    (6, 720),
])
def test_factorial_parametrized(input_val, expected):
    """Parametrized test for factorial function."""
    def factorial(n):
        if n <= 1:
            return 1
        return n * factorial(n - 1)
    
    assert factorial(input_val) == expected


@pytest.mark.parametrize("n,expected", [
    (0, 0),
    (1, 1),
    (2, 1),
    (3, 2),
    (4, 3),
    (5, 5),
    (6, 8),
    (7, 13),
])
def test_fibonacci_parametrized(n, expected):
    """Parametrized test for fibonacci function."""
    def fib(n):
        if n <= 1:
            return n
        a, b = 0, 1
        for _ in range(2, n + 1):
            a, b = b, a + b
        return b
    
    assert fib(n) == expected


@pytest.mark.parametrize("lst,expected", [
    ([], []),
    ([1], [1]),
    ([3, 1, 2], [1, 2, 3]),
    ([5, 4, 3, 2, 1], [1, 2, 3, 4, 5]),
    (["c", "a", "b"], ["a", "b", "c"]),
])
def test_sorting_parametrized(lst, expected):
    """Parametrized test for sorting."""
    assert sorted(lst) == expected


# =============================================================================
# Exception Testing with pytest.raises (Requirement 10.4)
# =============================================================================

def test_raises_zero_division():
    """Test ZeroDivisionError is raised."""
    with pytest.raises(ZeroDivisionError):
        _ = 1 / 0


def test_raises_value_error():
    """Test ValueError is raised."""
    with pytest.raises(ValueError):
        int("not a number")


def test_raises_attribute_error():
    """Test AttributeError is raised."""
    with pytest.raises(AttributeError):
        "string".nonexistent_method()


def test_raises_with_match():
    """Test exception with message matching."""
    with pytest.raises(ValueError, match="invalid literal"):
        int("abc")


# =============================================================================
# Combined Integration Tests
# =============================================================================

def test_closure_in_class():
    """Test closure used within a class method."""
    class Multiplier:
        def __init__(self, factor):
            self.factor = factor
        
        def get_multiplier(self):
            factor = self.factor
            def multiply(x):
                return x * factor
            return multiply
    
    m = Multiplier(3)
    mult_by_3 = m.get_multiplier()
    assert mult_by_3(5) == 15


def test_decorated_method():
    """Test decorator on class method."""
    def log_call(func):
        def wrapper(*args, **kwargs):
            wrapper.call_count += 1
            return func(*args, **kwargs)
        wrapper.call_count = 0
        return wrapper
    
    class Calculator:
        @log_call
        def add(self, a, b):
            return a + b
    
    calc = Calculator()
    assert calc.add(1, 2) == 3
    assert calc.add(3, 4) == 7
    assert calc.add.call_count == 2


def test_comprehension_with_closure():
    """Test list comprehension with closure."""
    def make_adders(n):
        return [lambda x, i=i: x + i for i in range(n)]
    
    adders = make_adders(3)
    assert adders[0](10) == 10
    assert adders[1](10) == 11
    assert adders[2](10) == 12
