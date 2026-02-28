/**
 * Sample Kotlin script file for dx-check testing.
 *
 * This file demonstrates Kotlin script formatting and linting capabilities.
 */

// Simple data class
data class Person(val name: String, val age: Int)

// Extension function
fun Person.greet(): String = "Hello, my name is $name and I'm $age years old."

// Main script logic
val people = listOf(
    Person("Alice", 30),
    Person("Bob", 25),
    Person("Charlie", 35),
)

println("People:")
people.forEach { person ->
    println("  - ${person.greet()}")
}

// Calculate average age
val averageAge = people.map { it.age }.average()
println("\nAverage age: $averageAge")

// Filter and sort
val adults = people
    .filter { it.age >= 30 }
    .sortedBy { it.name }

println("\nAdults (30+):")
adults.forEach { println("  - ${it.name}: ${it.age}") }

// Quick Fibonacci
fun fib(n: Int): Long = when {
    n <= 0 -> 0
    n == 1 -> 1
    else -> fib(n - 1) + fib(n - 2)
}

println("\nFirst 10 Fibonacci numbers:")
(0 until 10).map { fib(it) }.also { println(it) }
