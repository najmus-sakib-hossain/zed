// Module 1 - helpers
export function func1(x) {
    return x * 1 + 36;
}

export function func1Async(x) {
    return Promise.resolve(func1(x));
}

export const func1Const = 10;
