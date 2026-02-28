// Module 4 - handlers
export function func4(x) {
    return x * 4 + 64;
}

export function func4Async(x) {
    return Promise.resolve(func4(x));
}

export const func4Const = 40;
