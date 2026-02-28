//! Sample Rust file for dx-check testing.
//!
//! This file demonstrates Rust formatting and linting capabilities.

use std::collections::VecDeque;

/// A generic stack implementation.
#[derive(Debug, Default)]
pub struct Stack<T> {
    data: Vec<T>,
}

impl<T> Stack<T> {
    /// Creates a new empty stack.
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Pushes a value onto the stack.
    pub fn push(&mut self, value: T) {
        self.data.push(value);
    }

    /// Pops a value from the stack.
    pub fn pop(&mut self) -> Option<T> {
        self.data.pop()
    }

    /// Returns a reference to the top element.
    pub fn peek(&self) -> Option<&T> {
        self.data.last()
    }

    /// Returns true if the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the number of elements in the stack.
    pub fn len(&self) -> usize {
        self.data.len()
    }
}

/// Generates the first n Fibonacci numbers.
pub fn fibonacci(n: usize) -> Vec<u64> {
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![0];
    }

    let mut result = vec![0, 1];
    for i in 2..n {
        let next = result[i - 1] + result[i - 2];
        result.push(next);
    }
    result
}

/// Checks if a string is a palindrome.
pub fn is_palindrome(s: &str) -> bool {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();

    let reversed: String = cleaned.chars().rev().collect();
    cleaned == reversed
}

/// A simple calculator with operation history.
#[derive(Debug)]
pub struct Calculator {
    value: f64,
    history: VecDeque<String>,
}

impl Calculator {
    /// Creates a new calculator with an initial value.
    pub fn new(initial_value: f64) -> Self {
        Self {
            value: initial_value,
            history: VecDeque::new(),
        }
    }

    /// Adds a value.
    pub fn add(&mut self, x: f64) -> &mut Self {
        self.value += x;
        self.history.push_back(format!("add({})", x));
        self
    }

    /// Subtracts a value.
    pub fn subtract(&mut self, x: f64) -> &mut Self {
        self.value -= x;
        self.history.push_back(format!("subtract({})", x));
        self
    }

    /// Multiplies by a value.
    pub fn multiply(&mut self, x: f64) -> &mut Self {
        self.value *= x;
        self.history.push_back(format!("multiply({})", x));
        self
    }

    /// Divides by a value.
    pub fn divide(&mut self, x: f64) -> Result<&mut Self, &'static str> {
        if x == 0.0 {
            return Err("Cannot divide by zero");
        }
        self.value /= x;
        self.history.push_back(format!("divide({})", x));
        Ok(self)
    }

    /// Returns the current value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns the operation history.
    pub fn history(&self) -> Vec<String> {
        self.history.iter().cloned().collect()
    }
}

fn main() {
    // Test Stack
    let mut stack = Stack::new();
    stack.push(1);
    stack.push(2);
    stack.push(3);

    println!("Stack size: {}", stack.len());
    if let Some(top) = stack.peek() {
        println!("Top element: {}", top);
    }

    while let Some(val) = stack.pop() {
        println!("Popped: {}", val);
    }

    // Test Fibonacci
    let fib = fibonacci(10);
    println!("Fibonacci: {:?}", fib);

    // Test palindrome
    let test = "A man a plan a canal Panama";
    println!(
        "\"{}\" is {}a palindrome",
        test,
        if is_palindrome(test) { "" } else { "not " }
    );

    // Test Calculator
    let mut calc = Calculator::new(10.0);
    calc.add(5.0).multiply(2.0).subtract(3.0);
    println!("Calculator result: {}", calc.value());
    println!("Calculator history: {:?}", calc.history());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack() {
        let mut stack = Stack::new();
        assert!(stack.is_empty());

        stack.push(1);
        stack.push(2);
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.peek(), Some(&2));

        assert_eq!(stack.pop(), Some(2));
        assert_eq!(stack.pop(), Some(1));
        assert!(stack.is_empty());
    }

    #[test]
    fn test_fibonacci() {
        assert_eq!(fibonacci(0), vec![]);
        assert_eq!(fibonacci(1), vec![0]);
        assert_eq!(fibonacci(5), vec![0, 1, 1, 2, 3]);
    }

    #[test]
    fn test_palindrome() {
        assert!(is_palindrome("A man a plan a canal Panama"));
        assert!(is_palindrome("racecar"));
        assert!(!is_palindrome("hello"));
    }
}
