# Test class instantiation with arguments
# This tests Requirements 1.2 and 1.3:
# - Class instantiation with arguments
# - __init__ receives instance as self and passes arguments

class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y
    
    def get_x(self):
        return self.x
    
    def get_y(self):
        return self.y

# Test 1: Create instance with arguments
p = Point(10, 20)
print("x:", p.get_x())
print("y:", p.get_y())

# Test 2: Access instance attributes directly
print("p.x:", p.x)
print("p.y:", p.y)

# Test 3: Multiple instances
p1 = Point(1, 2)
p2 = Point(3, 4)
print("p1.x:", p1.x, "p1.y:", p1.y)
print("p2.x:", p2.x, "p2.y:", p2.y)
