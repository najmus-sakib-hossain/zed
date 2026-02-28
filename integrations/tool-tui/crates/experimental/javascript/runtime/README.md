
# DX JavaScript Runtime

⚠️ Early Development (v0.0.1) - Many features are experimental. APIs may change. JavaScript/TypeScript runtime with JIT compilation.

## Installation

```bash
cargo build --release -p dx-js-runtime ```


## Usage


```bash

# Run JavaScript

dx-js script.js

# Run TypeScript (types stripped)

dx-js app.ts

# Interactive REPL

dx-js

# Debug with Chrome DevTools

dx-js --inspect script.js dx-js --inspect=9230 script.js dx-js --inspect-brk script.js # Break on first line ```

## Language Features

+-----------+--------+
| Feature   | Status |
+===========+========+
| Variables | (let   |
+-----------+--------+



## Runtime Features

+---------+--------+
| Feature | Status |
+=========+========+
| console | API    |
+---------+--------+



## Node.js Compatibility

+--------+--------+
| Module | Status |
+========+========+
| fs     | ✅      |
+--------+--------+



## Examples

### Basic

```javascript
const add = (a, b) => a + b;
console.log(add(1, 2)); // 3 ```


### Async/Await


```javascript
async function fetchData() { const response = await fetch('https://api.example.com/data');
return response.json();
}
```


### Classes


```javascript
class Animal { constructor(name) { this.name = name;
}
}
class Dog extends Animal { speak() { console.log(`${this.name} barks`);
}
}
new Dog('Rex').speak();
```


### Modules


```javascript
// math.js export const add = (a, b) => a + b;
// main.js import { add } from './math.js';
console.log(add(1, 2));
```


## Environment Variables


+----------+-------------+
| Variable | Description |
+==========+=============+
| `DX      | DEBUG`      |
+----------+-------------+


## Performance


+-----------+-------+-----------+------------+
| Benchmark | DX-JS | Bun       | Difference |
+===========+=======+===========+============+
| Hello     | World | (startup) | 80ms       |
+-----------+-------+-----------+------------+


## Architecture


- Parser: OXC (fast JS/TS parsing)
- Compiler: Cranelift JIT (native code generation)
- Memory: Arena allocator with generational GC
- Cache: Persistent code cache


## License


MIT OR Apache-2.0
