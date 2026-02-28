
# DX Compatibility Layer

⚠️ Early Development (v0.0.1) - Many APIs are stubs or have limited functionality. Node.js and Web API compatibility for DX runtime.

## Installation

```bash
cargo build --release -p dx-js-compatibility ```


## Node.js APIs


+-----------+-----------+
| Module    | Functions |
+===========+===========+
| `console` | log       |
+-----------+-----------+


## Web APIs


+---------+--------+
| API     | Status |
+=========+========+
| `fetch` | ✅      |
+---------+--------+


## Usage



### Rust API


```rust
use dx_js_compatibility::*;
// Path operations let joined = path::join(&["src", "index.ts"]);
let dir = path::dirname("/path/to/file.js");
// Crypto let hash = crypto::sha256(data);
let uuid = crypto::random_uuid();
// URL parsing let url = url::parse("https://example.com/path?query=value");
```


### JavaScript


```javascript
// File system import { readFile, writeFile } from 'fs';
import { readFileSync } from 'fs';
const content = await readFile('file.txt', 'utf8');
await writeFile('output.txt', content);
// Path import { join, dirname } from 'path';
const fullPath = join('src', 'index.js');
// HTTP import { createServer } from 'http';
const server = createServer((req, res) => { res.end('Hello');
});
server.listen(3000);
// Crypto import { createHash, randomUUID } from 'crypto';
const hash = createHash('sha256').update('data').digest('hex');
const id = randomUUID();
// Events import { EventEmitter } from 'events';
const emitter = new EventEmitter();
emitter.on('event', (data) => console.log(data));
emitter.emit('event', 'hello');
```


## License


MIT OR Apache-2.0
