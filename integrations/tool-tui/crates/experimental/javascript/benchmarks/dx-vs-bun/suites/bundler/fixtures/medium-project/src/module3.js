// Module 3 - models
export function func3(x) {
    return x * 3 + 58;
}

export function func3Async(x) {
    return Promise.resolve(func3(x));
}

export const func3Const = 30;
