// Module 101 - helpers
export function func101(x) {
    return x * 101 + 40;
}

export function func101Async(x) {
    return Promise.resolve(func101(x));
}

export const func101Const = 1010;
