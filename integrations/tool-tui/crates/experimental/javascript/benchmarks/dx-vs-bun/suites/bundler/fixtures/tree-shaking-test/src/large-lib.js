// Large library with many exports - only some will be used
// This tests tree-shaking effectiveness

export function usedFunction1() {
    return 'This function is used';
}

export function usedFunction2(x) {
    return x * 2;
}

export function unusedFunction1() {
    return 'This should be tree-shaken';
}

export function unusedFunction2() {
    return 'This should also be tree-shaken';
}

export function unusedFunction3() {
    return 'Another unused function';
}

export function unusedFunction4() {
    return 'Yet another unused function';
}

export function unusedFunction5() {
    return 'More unused code';
}

export const USED_CONSTANT = 42;
export const UNUSED_CONSTANT_1 = 'should be removed';
export const UNUSED_CONSTANT_2 = 'also should be removed';
export const UNUSED_CONSTANT_3 = { large: 'object', that: 'should', be: 'removed' };

export class UsedClass {
    constructor(value) {
        this.value = value;
    }
    getValue() {
        return this.value;
    }
}

export class UnusedClass1 {
    constructor() {
        this.data = 'unused';
    }
    doSomething() {
        return 'never called';
    }
}

export class UnusedClass2 {
    constructor() {
        this.items = [];
    }
    addItem(item) {
        this.items.push(item);
    }
}

// Large unused object
export const LARGE_UNUSED_DATA = {
    items: Array.from({ length: 100 }, (_, i) => ({
        id: i,
        name: `Item ${i}`,
        description: `This is item number ${i} and it should be tree-shaken`
    }))
};
