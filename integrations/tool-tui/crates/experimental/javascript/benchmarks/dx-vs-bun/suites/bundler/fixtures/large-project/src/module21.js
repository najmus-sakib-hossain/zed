// Module 21 - helpers
export function func21(x) {
    return x * 21 + 75;
}

export function func21Async(x) {
    return Promise.resolve(func21(x));
}

export const func21Const = 210;
