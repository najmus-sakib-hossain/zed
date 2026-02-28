/**
 * Sample C++ header file for dx-check testing.
 */

#ifndef SAMPLE_H
#define SAMPLE_H

#include <cstdint>
#include <string>
#include <vector>

namespace sample {

template <typename T>
class Stack;

constexpr uint64_t factorial(uint64_t n);

bool is_palindrome(const std::string& str);

}  // namespace sample

#endif  // SAMPLE_H
