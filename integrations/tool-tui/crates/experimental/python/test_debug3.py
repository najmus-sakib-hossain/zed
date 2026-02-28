# Debug test - check if list is created
result = []
for i in range(3):
    result.append(i * i)
print("Manual loop:", result)

# Now try comprehension and immediately use it
print("Comprehension:", [i * i for i in range(3)])
