// Package sample provides sample Go code for dx-check testing.
//
// This file demonstrates Go formatting and linting capabilities.
package sample

import (
	"errors"
	"fmt"
	"strings"
)

// Stack is a generic stack implementation.
type Stack[T any] struct {
	data []T
}

// NewStack creates a new empty stack.
func NewStack[T any]() *Stack[T] {
	return &Stack[T]{
		data: make([]T, 0),
	}
}

// Push adds an element to the top of the stack.
func (s *Stack[T]) Push(value T) {
	s.data = append(s.data, value)
}

// Pop removes and returns the top element from the stack.
func (s *Stack[T]) Pop() (T, error) {
	var zero T
	if len(s.data) == 0 {
		return zero, errors.New("stack is empty")
	}
	value := s.data[len(s.data)-1]
	s.data = s.data[:len(s.data)-1]
	return value, nil
}

// Peek returns the top element without removing it.
func (s *Stack[T]) Peek() (T, error) {
	var zero T
	if len(s.data) == 0 {
		return zero, errors.New("stack is empty")
	}
	return s.data[len(s.data)-1], nil
}

// IsEmpty returns true if the stack is empty.
func (s *Stack[T]) IsEmpty() bool {
	return len(s.data) == 0
}

// Size returns the number of elements in the stack.
func (s *Stack[T]) Size() int {
	return len(s.data)
}

// Fibonacci generates the first n Fibonacci numbers.
func Fibonacci(n int) []int {
	if n <= 0 {
		return []int{}
	}
	if n == 1 {
		return []int{0}
	}

	result := make([]int, n)
	result[0] = 0
	result[1] = 1
	for i := 2; i < n; i++ {
		result[i] = result[i-1] + result[i-2]
	}
	return result
}

// IsPalindrome checks if a string is a palindrome.
func IsPalindrome(s string) bool {
	// Remove non-alphanumeric characters and convert to lowercase
	var cleaned strings.Builder
	for _, r := range s {
		if (r >= 'a' && r <= 'z') || (r >= 'A' && r <= 'Z') || (r >= '0' && r <= '9') {
			cleaned.WriteRune(r)
		}
	}
	str := strings.ToLower(cleaned.String())

	// Check if palindrome
	for i := 0; i < len(str)/2; i++ {
		if str[i] != str[len(str)-1-i] {
			return false
		}
	}
	return true
}

func main() {
	// Test Stack
	stack := NewStack[int]()
	stack.Push(1)
	stack.Push(2)
	stack.Push(3)

	fmt.Printf("Stack size: %d\n", stack.Size())
	if top, err := stack.Peek(); err == nil {
		fmt.Printf("Top element: %d\n", top)
	}

	for !stack.IsEmpty() {
		if val, err := stack.Pop(); err == nil {
			fmt.Printf("Popped: %d\n", val)
		}
	}

	// Test Fibonacci
	fib := Fibonacci(10)
	fmt.Printf("Fibonacci: %v\n", fib)

	// Test palindrome
	test := "A man a plan a canal Panama"
	fmt.Printf("\"%s\" is ", test)
	if !IsPalindrome(test) {
		fmt.Print("not ")
	}
	fmt.Println("a palindrome")
}
