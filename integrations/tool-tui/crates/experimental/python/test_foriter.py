# Test ForIter behavior
lst = []
it = iter(range(3))
print("Before loop")
for i in it:
    print("In loop:", i)
    lst.append(i)
print("After loop")
print("List:", lst)
