/**
 * Utility functions for fresh project
 */

function add(a, b) {
    return a + b;
}

function multiply(a, b) {
    return a * b;
}

function formatDate(date) {
    return date.toISOString();
}

module.exports = { add, multiply, formatDate };
