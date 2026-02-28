# Test stack operations
lst = []
print("Empty list:", lst)

# Manually simulate what list comprehension does
for i in range(3):
    lst.append(i)

print("After loop:", lst)

# Now test if the list is preserved
x = lst
print("Assigned:", x)
