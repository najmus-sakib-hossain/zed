# Test 1: List comprehension
result = [x*2 for x in range(5)]
print("List comp:", result)

# Test 2: String methods
s = "hello"
print("Upper:", s.upper())
print("Lower:", s.lower())
print("Replace:", s.replace("l", "x"))

# Test 3: List methods
lst = [1, 2, 3]
lst.append(4)
print("After append:", lst)
lst.reverse()
print("After reverse:", lst)

# Test 4: Dict operations
d = {"a": 1, "b": 2}
d["c"] = 3
print("Dict:", d)

# Test 5: Exception handling
try:
    x = 1 / 0
except ZeroDivisionError:
    print("Caught division by zero")

# Test 6: Class
class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y
    
    def __str__(self):
        return f"Point({self.x}, {self.y})"

p = Point(3, 4)
print("Point:", p)

# Test 7: JSON
import json
data = {"name": "test", "value": 42}
print("JSON:", json.dumps(data))
