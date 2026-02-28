// Module 2 - services
export function func2(x) {
    return x * 2 + 80;
}

export function func2Async(x) {
    return Promise.resolve(func2(x));
}

export const func2Const = 20;
