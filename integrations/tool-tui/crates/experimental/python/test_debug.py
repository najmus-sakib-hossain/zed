# Simple test to debug recursion
def simple():
    return 42

x = simple()
print(x)

# Now test if function can see itself
def self_ref():
    return self_ref

f = self_ref()
print("Got function reference")
