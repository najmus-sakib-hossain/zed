export function add(a, b) {
    return a + b;
}

export function subtract(a, b) {
    return a - b;
}

import { add } from './math.js';

const result = add(2, 3);
console.log('Result:', result);

