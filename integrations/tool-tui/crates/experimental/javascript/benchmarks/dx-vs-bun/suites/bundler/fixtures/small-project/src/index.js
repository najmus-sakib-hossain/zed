// Entry point for small bundler benchmark
import { add, multiply } from './math.js';
import { greet, farewell } from './strings.js';
import { createUser, validateUser } from './user.js';

const result = add(10, multiply(5, 3));
console.log(greet('World'));
console.log(farewell('World'));

const user = createUser('John', 'john@example.com');
if (validateUser(user)) {
    console.log('User is valid:', user.name);
}

export { result, user };
