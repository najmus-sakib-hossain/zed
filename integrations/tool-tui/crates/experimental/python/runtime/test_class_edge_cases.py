# Test edge cases for class instantiation

# Test 1: Class without __init__
class NoInit:
    pass

obj1 = NoInit()
print("NoInit instance created:", obj1)

# Test 2: Class with __init__ but no arguments
class NoArgs:
    def __init__(self):
        self.value = 42

obj2 = NoArgs()
print("NoArgs value:", obj2.value)

# Test 3: Class with default arguments (if supported)
class WithDefaults:
    def __init__(self, x, y=10):
        self.x = x
        self.y = y

obj3 = WithDefaults(5)
print("WithDefaults x:", obj3.x, "y:", obj3.y)

obj4 = WithDefaults(5, 20)
print("WithDefaults x:", obj4.x, "y:", obj4.y)

# Test 4: Nested attribute access
class Container:
    def __init__(self):
        self.inner = Inner()

class Inner:
    def __init__(self):
        self.value = 100

c = Container()
print("Nested value:", c.inner.value)
