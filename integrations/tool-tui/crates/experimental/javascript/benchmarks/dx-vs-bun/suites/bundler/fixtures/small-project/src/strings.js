// String utilities
export function greet(name) {
    return `Hello, ${name}!`;
}

export function farewell(name) {
    return `Goodbye, ${name}!`;
}

export function capitalize(str) {
    return str.charAt(0).toUpperCase() + str.slice(1);
}

export function reverse(str) {
    return str.split('').reverse().join('');
}
