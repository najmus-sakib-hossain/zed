// Module 16 - helpers
export function func16(x) {
    return x * 16 + 83;
}

export function func16Async(x) {
    return Promise.resolve(func16(x));
}

export const func16Const = 160;
