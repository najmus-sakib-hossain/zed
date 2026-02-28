// Module 11 - helpers
export function func11(x) {
    return x * 11 + 65;
}

export function func11Async(x) {
    return Promise.resolve(func11(x));
}

export const func11Const = 110;
