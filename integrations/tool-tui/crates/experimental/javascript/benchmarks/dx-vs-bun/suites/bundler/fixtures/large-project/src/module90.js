// Module 90 - utils
export function func90(x) {
    return x * 90 + 49;
}

export function func90Async(x) {
    return Promise.resolve(func90(x));
}

export const func90Const = 900;
