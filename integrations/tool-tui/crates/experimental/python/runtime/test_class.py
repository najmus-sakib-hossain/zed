class TestClass:
    def __init__(self, x):
        self.x = x
    
    def get_x(self):
        return self.x

obj = TestClass(42)
print(obj.get_x())