// Module 36 - helpers
export function func36(x) {
    return x * 36 + 2;
}

export function func36Async(x) {
    return Promise.resolve(func36(x));
}

export const func36Const = 360;
