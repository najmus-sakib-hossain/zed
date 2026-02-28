#!/usr/bin/env python3
"""Test that all stdlib modules can be imported"""

# Test os.path
import os.path
print("✓ os.path imported")
print(f"  os.path.join('a', 'b') = {os.path.join('a', 'b')}")

# Test pathlib
import pathlib
print("✓ pathlib imported")

# Test re
import re
print("✓ re imported")

# Test datetime
import datetime
print("✓ datetime imported")

# Test collections
import collections
print("✓ collections imported")

# Test itertools
import itertools
print("✓ itertools imported")

# Test functools
import functools
print("✓ functools imported")

print("\n✅ All stdlib modules imported successfully!")
