# Test real Python functionality
print("Hello from dx-py!")

# Test list comprehension
numbers = [1, 2, 3, 4, 5]
squares = [x * x for x in numbers]
print(f"Squares: {squares}")

# Test class
class Person:
    def __init__(self, name, age):
        self.name = name
        self.age = age
    
    def greet(self):
        return f"Hello, I'm {self.name} and I'm {self.age} years old"

person = Person("Alice", 30)
print(person.greet())

# Test exception handling
try:
    result = 10 / 2
    print(f"Division result: {result}")
except ZeroDivisionError:
    print("Cannot divide by zero!")
finally:
    print("Finally block executed")

# Test JSON
import json
data = {"name": "Bob", "age": 25, "city": "NYC"}
json_str = json.dumps(data)
print(f"JSON: {json_str}")
parsed = json.loads(json_str)
print(f"Parsed: {parsed}")
