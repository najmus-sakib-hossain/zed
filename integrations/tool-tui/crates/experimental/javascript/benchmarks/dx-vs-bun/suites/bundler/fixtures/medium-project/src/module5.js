// Module 5 - utils
export function func5(x) {
    return x * 5 + 24;
}

export function func5Async(x) {
    return Promise.resolve(func5(x));
}

export const func5Const = 50;
