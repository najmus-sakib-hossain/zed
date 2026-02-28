# Test if function can see itself
def greet():
    print("Hello")

greet()

# Test recursion with explicit global
def countdown(n):
    print(n)
    if n > 0:
        countdown(n - 1)

countdown(3)
