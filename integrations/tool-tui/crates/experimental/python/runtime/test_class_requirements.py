# Comprehensive test for Requirements 1.2 and 1.3
# Requirement 1.2: WHEN a class is instantiated with arguments, 
#                  THE Runtime SHALL call `__init__` with the instance as `self` and pass the arguments
# Requirement 1.3: WHEN a method accesses `self.attribute`, 
#                  THE Runtime SHALL correctly resolve instance attributes

print("=== Test 1: Class instantiation with arguments ===")
class Person:
    def __init__(self, name, age):
        self.name = name
        self.age = age
    
    def get_info(self):
        return self.name + " is " + str(self.age) + " years old"

p1 = Person("Alice", 30)
print(p1.get_info())

print("\n=== Test 2: Instance attribute access via self ===")
class Counter:
    def __init__(self, start):
        self.count = start
    
    def increment(self):
        self.count = self.count + 1
    
    def get_count(self):
        return self.count

c = Counter(10)
print("Initial count:", c.get_count())
c.increment()
print("After increment:", c.get_count())
c.increment()
print("After second increment:", c.get_count())

print("\n=== Test 3: Multiple instances with independent state ===")
c1 = Counter(0)
c2 = Counter(100)
c1.increment()
c1.increment()
c2.increment()
print("c1 count:", c1.get_count())
print("c2 count:", c2.get_count())

print("\n=== Test 4: Direct attribute access ===")
class Box:
    def __init__(self, width, height):
        self.width = width
        self.height = height

b = Box(10, 20)
print("Width:", b.width)
print("Height:", b.height)
b.width = 15
print("Updated width:", b.width)

print("\n=== Test 5: Class with no __init__ ===")
class Empty:
    pass

e = Empty()
e.value = 42
print("Empty instance value:", e.value)

print("\n=== Test 6: __init__ with no additional arguments ===")
class DefaultInit:
    def __init__(self):
        self.initialized = True

d = DefaultInit()
print("Initialized:", d.initialized)

print("\n=== All tests passed! ===")
