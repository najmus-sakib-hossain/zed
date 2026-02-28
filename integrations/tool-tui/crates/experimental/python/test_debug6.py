# Test if list is being built
lst = []
for i in range(3):
    lst.append(i)
print("After manual loop:", lst)

# Now test comprehension
comp = [i for i in range(3)]
print("After comprehension:", comp)
