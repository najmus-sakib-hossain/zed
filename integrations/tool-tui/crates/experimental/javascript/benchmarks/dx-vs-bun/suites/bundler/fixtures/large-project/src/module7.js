// Module 7 - services
export function func7(x) {
    return x * 7 + 72;
}

export function func7Async(x) {
    return Promise.resolve(func7(x));
}

export const func7Const = 70;
