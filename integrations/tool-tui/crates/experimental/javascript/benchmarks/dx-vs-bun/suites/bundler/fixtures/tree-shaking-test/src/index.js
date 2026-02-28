// Tree-shaking test entry point
// Only imports a small subset of the large library
import { usedFunction1, usedFunction2, USED_CONSTANT, UsedClass } from './large-lib.js';

const result1 = usedFunction1();
const result2 = usedFunction2(USED_CONSTANT);

const instance = new UsedClass(100);
const value = instance.getValue();

console.log(result1, result2, value);

export { result1, result2, value };
