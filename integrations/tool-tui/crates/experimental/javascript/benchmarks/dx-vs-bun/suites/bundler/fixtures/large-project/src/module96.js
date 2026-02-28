// Module 96 - helpers
export function func96(x) {
    return x * 96 + 38;
}

export function func96Async(x) {
    return Promise.resolve(func96(x));
}

export const func96Const = 960;
