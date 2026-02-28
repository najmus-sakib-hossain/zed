// Module 25 - utils
export function func25(x) {
    return x * 25 + 37;
}

export function func25Async(x) {
    return Promise.resolve(func25(x));
}

export const func25Const = 250;
