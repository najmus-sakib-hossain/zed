<?php

declare(strict_types=1);

/**
 * Sample PHP file for dx-check testing.
 *
 * This file demonstrates PHP formatting and linting capabilities.
 */

namespace Sample;

/**
 * A generic stack implementation.
 *
 * @template T
 */
class Stack
{
    /** @var array<int, T> */
    private array $data = [];

    /**
     * Push a value onto the stack.
     *
     * @param T $value
     */
    public function push(mixed $value): void
    {
        $this->data[] = $value;
    }

    /**
     * Pop a value from the stack.
     *
     * @return T|null
     */
    public function pop(): mixed
    {
        if (empty($this->data)) {
            return null;
        }
        return array_pop($this->data);
    }

    /**
     * Peek at the top value without removing it.
     *
     * @return T|null
     */
    public function peek(): mixed
    {
        if (empty($this->data)) {
            return null;
        }
        return $this->data[count($this->data) - 1];
    }

    /**
     * Check if the stack is empty.
     */
    public function isEmpty(): bool
    {
        return empty($this->data);
    }

    /**
     * Get the size of the stack.
     */
    public function size(): int
    {
        return count($this->data);
    }
}

/**
 * A simple calculator with operation history.
 */
class Calculator
{
    private float $value;
    /** @var array<int, string> */
    private array $history = [];

    public function __construct(float $initialValue = 0.0)
    {
        $this->value = $initialValue;
    }

    public function add(float $x): self
    {
        $this->value += $x;
        $this->history[] = "add({$x})";
        return $this;
    }

    public function subtract(float $x): self
    {
        $this->value -= $x;
        $this->history[] = "subtract({$x})";
        return $this;
    }

    public function multiply(float $x): self
    {
        $this->value *= $x;
        $this->history[] = "multiply({$x})";
        return $this;
    }

    public function divide(float $x): self
    {
        if ($x === 0.0) {
            throw new \InvalidArgumentException('Cannot divide by zero');
        }
        $this->value /= $x;
        $this->history[] = "divide({$x})";
        return $this;
    }

    public function getValue(): float
    {
        return $this->value;
    }

    /**
     * @return array<int, string>
     */
    public function getHistory(): array
    {
        return $this->history;
    }

    public function reset(): self
    {
        $this->value = 0.0;
        $this->history = [];
        return $this;
    }
}

/**
 * Generate the first n Fibonacci numbers.
 *
 * @return array<int, int>
 */
function fibonacci(int $n): array
{
    if ($n <= 0) {
        return [];
    }
    if ($n === 1) {
        return [0];
    }

    $result = [0, 1];
    for ($i = 2; $i < $n; $i++) {
        $result[] = $result[$i - 1] + $result[$i - 2];
    }
    return $result;
}

/**
 * Check if a string is a palindrome.
 */
function isPalindrome(string $s): bool
{
    $cleaned = preg_replace('/[^a-zA-Z0-9]/', '', $s);
    $cleaned = strtolower($cleaned ?? '');
    return $cleaned === strrev($cleaned);
}

// Main execution
function main(): void
{
    // Test Stack
    $stack = new Stack();
    $stack->push(1);
    $stack->push(2);
    $stack->push(3);

    echo "Stack size: " . $stack->size() . "\n";
    echo "Top element: " . $stack->peek() . "\n";

    while (!$stack->isEmpty()) {
        echo "Popped: " . $stack->pop() . "\n";
    }

    // Test Fibonacci
    $fib = fibonacci(10);
    echo "Fibonacci: " . implode(', ', $fib) . "\n";

    // Test palindrome
    $test = "A man a plan a canal Panama";
    $isPalin = isPalindrome($test) ? '' : 'not ';
    echo "\"{$test}\" is {$isPalin}a palindrome\n";

    // Test Calculator
    $calc = new Calculator(10);
    $calc->add(5)->multiply(2)->subtract(3);
    echo "Calculator result: " . $calc->getValue() . "\n";
    echo "Calculator history: " . implode(', ', $calc->getHistory()) . "\n";
}

main();
