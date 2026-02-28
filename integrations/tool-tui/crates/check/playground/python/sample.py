#!/usr/bin/env python3
"""Sample Python file for dx-check testing.

This file demonstrates Python formatting and linting capabilities.
"""

from typing import List, Optional
import json


class Calculator:
    """A simple calculator class."""

    def __init__(self, initial_value: float = 0.0) -> None:
        """Initialize the calculator with an optional starting value."""
        self.value = initial_value
        self.history: List[str] = []

    def add(self, x: float) -> "Calculator":
        """Add a value to the current result."""
        self.value += x
        self.history.append(f"add({x})")
        return self

    def subtract(self, x: float) -> "Calculator":
        """Subtract a value from the current result."""
        self.value -= x
        self.history.append(f"subtract({x})")
        return self

    def multiply(self, x: float) -> "Calculator":
        """Multiply the current result by a value."""
        self.value *= x
        self.history.append(f"multiply({x})")
        return self

    def divide(self, x: float) -> "Calculator":
        """Divide the current result by a value."""
        if x == 0:
            raise ValueError("Cannot divide by zero")
        self.value /= x
        self.history.append(f"divide({x})")
        return self

    def reset(self) -> "Calculator":
        """Reset the calculator to zero."""
        self.value = 0.0
        self.history.clear()
        return self

    def get_history(self) -> List[str]:
        """Get the operation history."""
        return self.history.copy()


def fibonacci(n: int) -> List[int]:
    """Generate the first n Fibonacci numbers."""
    if n <= 0:
        return []
    if n == 1:
        return [0]

    result = [0, 1]
    for _ in range(2, n):
        result.append(result[-1] + result[-2])
    return result


def main() -> None:
    """Main entry point."""
    calc = Calculator(10)
    calc.add(5).multiply(2).subtract(3)
    print(f"Result: {calc.value}")
    print(f"History: {calc.get_history()}")

    fib = fibonacci(10)
    print(f"Fibonacci: {fib}")


if __name__ == "__main__":
    main()
