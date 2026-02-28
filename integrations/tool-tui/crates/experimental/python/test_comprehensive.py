# Test 1: Basic arithmetic
print("Test 1: Basic arithmetic")
print(1 + 2 * 3)
print(10 // 3)
print(10 % 3)

# Test 2: String operations
print("\nTest 2: String operations")
s = "hello world"
print(s.upper())
print(s.lower())
print(s.split())
print("-".join(["a", "b", "c"]))

# Test 3: List operations
print("\nTest 3: List operations")
lst = [1, 2, 3]
lst.append(4)
print(lst)
lst.pop()
print(lst)

# Test 4: Dict operations
print("\nTest 4: Dict operations")
d = {"a": 1, "b": 2}
print(d.get("a"))
print(list(d.keys()))

# Test 5: Control flow
print("\nTest 5: Control flow")
for i in range(3):
    print(i)

# Test 6: List comprehension
print("\nTest 6: List comprehension")
squares = [x*x for x in range(5)]
print(squares)

# Test 7: Function definition
print("\nTest 7: Function definition")
def add(a, b):
    return a + b
print(add(3, 4))

# Test 8: Recursion
print("\nTest 8: Recursion")
def factorial(n):
    if n <= 1:
        return 1
    return n * factorial(n - 1)
print(factorial(5))

# Test 9: Class
print("\nTest 9: Class")
class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y
    def magnitude(self):
        return (self.x**2 + self.y**2)**0.5

p = Point(3, 4)
print(p.x, p.y)
print(p.magnitude())

# Test 10: Exception handling
print("\nTest 10: Exception handling")
try:
    x = 1 / 0
except ZeroDivisionError:
    print("Caught division by zero")
finally:
    print("Finally block executed")

print("\nAll tests completed!")
