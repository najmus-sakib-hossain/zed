# Test list comprehension
print("Simple list:")
simple = [1, 2, 3]
print(simple)

print("\nList comprehension:")
squares = [x*x for x in range(5)]
print(squares)

print("\nManual equivalent:")
result = []
for x in range(5):
    result.append(x*x)
print(result)
