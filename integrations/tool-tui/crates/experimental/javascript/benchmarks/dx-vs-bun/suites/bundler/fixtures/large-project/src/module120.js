// Module 120 - utils
export function func120(x) {
    return x * 120 + 96;
}

export function func120Async(x) {
    return Promise.resolve(func120(x));
}

export const func120Const = 1200;
