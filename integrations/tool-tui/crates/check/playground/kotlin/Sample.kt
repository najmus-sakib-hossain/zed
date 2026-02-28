/**
 * Sample Kotlin file for dx-check testing.
 *
 * This file demonstrates Kotlin formatting and linting capabilities.
 */

package sample

/**
 * A generic stack implementation.
 */
class Stack<T> {
    private val data = mutableListOf<T>()

    fun push(value: T) {
        data.add(value)
    }

    fun pop(): T? {
        if (data.isEmpty()) return null
        return data.removeAt(data.size - 1)
    }

    fun peek(): T? {
        if (data.isEmpty()) return null
        return data.last()
    }

    fun isEmpty(): Boolean = data.isEmpty()

    fun size(): Int = data.size
}

/**
 * A simple calculator with operation history.
 */
class Calculator(initialValue: Double = 0.0) {
    var value: Double = initialValue
        private set

    private val history = mutableListOf<String>()

    fun add(x: Double): Calculator {
        value += x
        history.add("add($x)")
        return this
    }

    fun subtract(x: Double): Calculator {
        value -= x
        history.add("subtract($x)")
        return this
    }

    fun multiply(x: Double): Calculator {
        value *= x
        history.add("multiply($x)")
        return this
    }

    fun divide(x: Double): Calculator {
        require(x != 0.0) { "Cannot divide by zero" }
        value /= x
        history.add("divide($x)")
        return this
    }

    fun getHistory(): List<String> = history.toList()

    fun reset(): Calculator {
        value = 0.0
        history.clear()
        return this
    }
}

/**
 * Generate the first n Fibonacci numbers.
 */
fun fibonacci(n: Int): List<Long> {
    if (n <= 0) return emptyList()
    if (n == 1) return listOf(0L)

    val result = mutableListOf(0L, 1L)
    for (i in 2 until n) {
        result.add(result[i - 1] + result[i - 2])
    }
    return result
}

/**
 * Check if a string is a palindrome.
 */
fun isPalindrome(s: String): Boolean {
    val cleaned = s.filter { it.isLetterOrDigit() }.lowercase()
    return cleaned == cleaned.reversed()
}

/**
 * Extension function to format a list nicely.
 */
fun <T> List<T>.formatList(): String = joinToString(", ", "[", "]")

fun main() {
    // Test Stack
    val stack = Stack<Int>()
    stack.push(1)
    stack.push(2)
    stack.push(3)

    println("Stack size: ${stack.size()}")
    println("Top element: ${stack.peek()}")

    while (!stack.isEmpty()) {
        println("Popped: ${stack.pop()}")
    }

    // Test Fibonacci
    val fib = fibonacci(10)
    println("Fibonacci: ${fib.formatList()}")

    // Test palindrome
    val test = "A man a plan a canal Panama"
    val isPalin = if (isPalindrome(test)) "" else "not "
    println("\"$test\" is ${isPalin}a palindrome")

    // Test Calculator
    val calc = Calculator(10.0)
    calc.add(5.0).multiply(2.0).subtract(3.0)
    println("Calculator result: ${calc.value}")
    println("Calculator history: ${calc.getHistory().formatList()}")
}
