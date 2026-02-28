/**
 * Sample C++ file for dx-check testing.
 *
 * This file demonstrates C++ formatting and linting capabilities.
 */

#include <algorithm>
#include <iostream>
#include <memory>
#include <string>
#include <vector>

namespace sample {

/**
 * A simple stack implementation using templates.
 */
template <typename T>
class Stack {
public:
    Stack() = default;
    ~Stack() = default;

    // Disable copy
    Stack(const Stack&) = delete;
    Stack& operator=(const Stack&) = delete;

    // Enable move
    Stack(Stack&&) noexcept = default;
    Stack& operator=(Stack&&) noexcept = default;

    void push(T value) { data_.push_back(std::move(value)); }

    T pop() {
        if (data_.empty()) {
            throw std::runtime_error("Stack is empty");
        }
        T value = std::move(data_.back());
        data_.pop_back();
        return value;
    }

    [[nodiscard]] const T& top() const {
        if (data_.empty()) {
            throw std::runtime_error("Stack is empty");
        }
        return data_.back();
    }

    [[nodiscard]] bool empty() const noexcept { return data_.empty(); }

    [[nodiscard]] size_t size() const noexcept { return data_.size(); }

private:
    std::vector<T> data_;
};

/**
 * Calculate factorial using recursion.
 */
constexpr uint64_t factorial(uint64_t n) {
    return n <= 1 ? 1 : n * factorial(n - 1);
}

/**
 * Check if a string is a palindrome.
 */
bool is_palindrome(const std::string& str) {
    std::string cleaned;
    std::copy_if(str.begin(), str.end(), std::back_inserter(cleaned),
                 [](char c) { return std::isalnum(c); });

    std::string reversed = cleaned;
    std::reverse(reversed.begin(), reversed.end());

    return std::equal(cleaned.begin(), cleaned.end(), reversed.begin(),
                      [](char a, char b) {
                          return std::tolower(a) == std::tolower(b);
                      });
}

}  // namespace sample

int main() {
    // Test Stack
    sample::Stack<int> stack;
    stack.push(1);
    stack.push(2);
    stack.push(3);

    std::cout << "Stack size: " << stack.size() << std::endl;
    std::cout << "Top element: " << stack.top() << std::endl;

    while (!stack.empty()) {
        std::cout << "Popped: " << stack.pop() << std::endl;
    }

    // Test factorial
    std::cout << "5! = " << sample::factorial(5) << std::endl;

    // Test palindrome
    std::string test = "A man a plan a canal Panama";
    std::cout << "\"" << test << "\" is "
              << (sample::is_palindrome(test) ? "" : "not ") << "a palindrome"
              << std::endl;

    return 0;
}
