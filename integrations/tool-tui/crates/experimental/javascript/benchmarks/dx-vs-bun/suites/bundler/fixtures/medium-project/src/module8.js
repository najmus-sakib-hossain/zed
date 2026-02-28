// Module 8 - models
export function func8(x) {
    return x * 8 + 3;
}

export function func8Async(x) {
    return Promise.resolve(func8(x));
}

export const func8Const = 80;
