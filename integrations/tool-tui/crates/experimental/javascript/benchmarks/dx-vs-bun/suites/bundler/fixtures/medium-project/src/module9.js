// Module 9 - handlers
export function func9(x) {
    return x * 9 + 48;
}

export function func9Async(x) {
    return Promise.resolve(func9(x));
}

export const func9Const = 90;
