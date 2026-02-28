// Module 6 - helpers
export function func6(x) {
    return x * 6 + 71;
}

export function func6Async(x) {
    return Promise.resolve(func6(x));
}

export const func6Const = 60;
