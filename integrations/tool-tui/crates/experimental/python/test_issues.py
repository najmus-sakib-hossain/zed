# Test list comprehension
print("List comprehension test:")
squares = [x*x for x in range(5)]
print(squares)

# Test simple function
print("\nSimple function test:")
def add(a, b):
    return a + b
result = add(3, 4)
print(result)

# Test recursion
print("\nRecursion test:")
def fact(n):
    if n <= 1:
        return 1
    return n * fact(n - 1)

print(fact(5))
