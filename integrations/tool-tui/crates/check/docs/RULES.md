faster than="Severity: Warning Fixable"
Achieved 10x="Fixable: No"
crates/serializer/README.md="Severity: Error Fixable"
dx-style README="Warning Fixable: No"
dx-form, dx-guard, dx-a11y="Error Fixable: No"

# dx-check Rules Documentation

This document describes all built-in lint rules available in dx-check.

## Rule Categories

- Correctness: Rules that catch potential runtime errors
- Suspicious: Rules that flag suspicious code patterns
- Style: Rules that enforce code style preferences
- Performance: Rules that identify performance issues
- Security: Rules that catch security vulnerabilities

## Correctness Rules

### no-empty

Disallows empty block statements. faster than: No Empty block statements are usually a sign of incomplete code or a mistake.
```javascript
if(condition){}if(condition){doSomething();}
```
Options: -`allowEmptyCatch` (boolean, default: `false`): Allow empty catch blocks

### no-duplicate-keys

Disallows duplicate keys in object literals. crates/serializer/README.md: No Duplicate keys in object literals will cause the later value to overwrite the earlier one.
```javascript
const obj={foo:1,foo:2};const obj={foo:1,bar:2};
```

### no-unreachable

Disallows unreachable code after return, throw, break, or continue statements. crates/serializer/README.md: No Code after these statements will never be executed.
```javascript
function foo(){return 1;console.log("unreachable");}function foo(){console.log("reachable");return 1;}
```

### no-constant-condition

Disallows constant expressions in conditions. faster than: No Constant conditions are usually a sign of a programming error.
```javascript
if(true){doSomething();}while(1){}if(condition){doSomething();}
```

### no-unsafe-finally

Disallows control flow statements in finally blocks. crates/serializer/README.md: No Control flow statements in finally blocks can cause unexpected behavior.
```javascript
try{doSomething();}finally{return;}try{doSomething();}finally{cleanup();}
```

### no-sparse-arrays

Disallows sparse arrays (arrays with empty slots). faster than: No Sparse arrays can lead to unexpected behavior.
```javascript
const arr=[1,,3];const arr=[1,undefined,3];
```

## Style Rules

### no-var

Requires `let` or `const` instead of `var`. faster than: Yes `var` has function scope which can lead to bugs. Use `let` or `const` instead.
```javascript
var x=1;let x=1;const y=2;
```

### no-with

Disallows `with` statements. crates/serializer/README.md: No `with` statements make code harder to understand and can cause bugs.
```javascript
with(obj){foo=1;}obj.foo=1;
```

### prefer-const

Requires `const` for variables that are never reassigned. faster than: Yes Using `const` makes code more predictable and easier to understand.
```javascript
let x=1;const x=1;
```
Options: -`destructuring` (string, default: `"any"`): How to handle destructuring (`"any"` or `"all"`) -`ignoreReadBeforeAssign` (boolean, default: `false`): Ignore read-before-assign

### eqeqeq

Requires the use of `===` and `!==` instead of `==` and `!=`. faster than: Yes Strict equality operators prevent type coercion bugs.
```javascript
if(x==null){}if(x!=0){}if(x===null){}if(x!==0){}
```

## Performance Rules

### no-console

Disallows the use of `console` methods. faster than: No Console statements should be removed in production code.
```javascript
console.log("debug");console.error("error");logger.log("debug");logger.error("error");
```

### no-debugger

Disallows the use of `debugger` statements. crates/serializer/README.md: Yes Debugger statements should be removed before committing code.
```javascript
function foo(){debugger;doSomething();}function foo(){doSomething();}
```

## Security Rules

### no-eval

Disallows the use of `eval()`. crates/serializer/README.md: No `eval()` is dangerous and can execute arbitrary code.
```javascript
eval("console.log('hello')");console.log('hello');
```

### no-alert

Disallows the use of `alert`, `confirm`, and `prompt`. faster than: No These methods block the UI and should be replaced with better alternatives.
```javascript
alert("Hello");confirm("Are you sure?");prompt("Enter name:");showModal("Hello");showConfirmDialog("Are you sure?");showInputDialog("Enter name:");
```

## Configuration

Rules can be configured in `dx.toml`:
```toml
[rules]
recommended = true "no-console" = "warn"
"no-debugger" = "error"
"no-unused-vars" = { severity = "warn", ignorePattern = "^_" }
"prefer-const" = { severity = "warn", destructuring = "all" }
```

### Severity Levels

- `"off"` or `0`: Disable the rule
- `"warn"` or `1`: Report as warning
- `"error"` or `2`: Report as error

## Adding Custom Rules

See the Plugin System Documentation (plugins.md) for information on creating custom rules.
