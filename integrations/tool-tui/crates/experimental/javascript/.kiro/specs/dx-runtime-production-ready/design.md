
# Design Document: DX-JS Runtime Production Ready

## Overview

This design document describes the architecture and implementation approach for making the DX-JS runtime production-ready. The runtime is a JavaScript/TypeScript execution environment that uses Cranelift for JIT compilation. The current implementation has critical bugs in loop handling, incomplete features, memory safety issues, and missing error handling that must be addressed. The design follows a layered architecture: -Parser Layer - SWC-based JavaScript/TypeScript parsing -MIR Layer - Middle Intermediate Representation for typed analysis -Codegen Layer - Cranelift-based native code generation -Runtime Layer - Heap management, built-in functions, and execution

## Architecture

@flow:TD[]

## Components and Interfaces

### 1. MIR (Middle Intermediate Representation)

The MIR is the core intermediate representation between parsing and code generation.
```rust
pub struct TypedMIR { pub functions: Vec<TypedFunction>, pub globals: Vec<TypedGlobal>, pub entry_point: Option<FunctionId>, pub type_layouts: HashMap<TypeId, TypeLayout>, pub source_file: String, }
pub struct TypedFunction { pub id: FunctionId, pub name: String, pub params: Vec<TypedParam>, pub return_type: Type, pub blocks: Vec<TypedBlock>, pub locals: Vec<TypedLocal>, pub is_pure: bool, pub span: SourceSpan, }
```
Key MIR instructions for control flow: -`Terminator::Branch { condition, then_block, else_block }` - Conditional branching -`Terminator::Goto(BlockId)` - Unconditional jump -`TypedInstruction::Copy { dest, src }` - Variable assignment (critical for loops)

### 2. Codegen with Cranelift Variable System

The code generator uses Cranelift's `Variable` system for proper SSA handling:
```rust
// Variable mapping: LocalId -> Cranelift Variable let mut variables: HashMap<LocalId, Variable> = HashMap::new();
// For each local, create a Cranelift Variable for (idx, local) in func.locals.iter().enumerate() { let var = Variable::new(idx);
builder.declare_var(var, types::F64);
variables.insert(LocalId(local.index), var);
}
// Use def_var/use_var for proper phi node handling builder.def_var(var, value); // Define variable let value = builder.use_var(var); // Use variable (auto phi insertion)
```

### 3. RuntimeHeap

The runtime heap manages all JavaScript objects:
```rust
struct RuntimeHeap { closures: HashMap<u64, ClosureData>, arrays: HashMap<u64, Vec<f64>>, objects: HashMap<u64, HashMap<String, f64>>, generators: HashMap<u64, GeneratorData>, promises: HashMap<u64, PromiseData>, strings: HashMap<u64, String>, bigints: HashMap<u64, num_bigint::BigInt>, next_id: u64, }
```

### 4. Value Encoding (NaN-Boxing)

Values are encoded as f64 using NaN-boxing: -Regular numbers: Direct f64 values -Strings: `-(id + 1_000_000)` (negative tagged IDs) -BigInts: `-(id + 2_000_000)` (negative tagged IDs) -Objects/Arrays: Positive integer IDs -Undefined: `f64::NAN` -Null: Special sentinel value -Booleans: 0.0 (false) or 1.0 (true)

### 5. Exception Handling

```rust
// Thread-local exception state thread_local! { static CURRENT_EXCEPTION: RefCell<f64> = RefCell::new(f64::NAN);
static EXCEPTION_HANDLER_STACK: RefCell<Vec<ExceptionHandler>> = RefCell::new(Vec::new());
}
struct ExceptionHandler { catch_block: BlockId, finally_block: Option<BlockId>, }
```

## Data Models

### Generator State Machine

```rust
enum GeneratorState { Suspended, // Initial state or after yield Executing, // Currently running Completed, // Generator finished }
struct GeneratorData { function_id: u32, captured_vars: Vec<f64>, state: GeneratorState, current_value: f64, resume_point: BlockId, // Where to resume after yield }
```

### Promise State Machine

```rust
enum PromiseState { Pending, Fulfilled, Rejected, }
struct PromiseData { state: PromiseState, value: f64, callbacks: Vec<PromiseCallback>, }
struct PromiseCallback { on_fulfilled: Option<u64>, // Closure ID on_rejected: Option<u64>, // Closure ID result_promise: u64, // Promise ID for chaining }
```

### Class Representation

```rust
struct ClassData { constructor_id: Option<FunctionId>, prototype: u64, // Object ID for prototype super_class: Option<u64>, // Class ID of parent static_properties: HashMap<String, f64>, private_fields: HashMap<String, f64>, }
```

## Correctness Properties

A property is a characteristic or behavior that should hold true across all valid executions of a system—, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.

### Property 1: Memory Zero-Initialization

For any memory allocation from the Arena_Allocator, all bytes in the allocated region should be zero before first use. Validates: Requirements 1.1

### Property 2: Thread-Safe String Interner

For any sequence of concurrent string intern operations from multiple threads, the interner should return consistent IDs for identical strings without data races. Validates: Requirements 1.2

### Property 3: Safe Lifetime Management

For any ZeroCopyReader memory-mapped content, references should remain valid for their declared lifetime without use-after-free. Validates: Requirements 1.3

### Property 4: GC Pointer Validity

For any GC scan operation, only properly initialized and tagged values should be interpreted as heap pointers. Validates: Requirements 1.4

### Property 5: UTF-8 String Validation

For any byte sequence passed to string allocation, invalid UTF-8 sequences should be rejected with an error, while valid UTF-8 should be accepted. Validates: Requirements 1.5

### Property 6: Exception Type Classification

For any JavaScript error condition, the runtime should throw the correct exception type (TypeError, ReferenceError, SyntaxError, RangeError) per ECMAScript specification. Validates: Requirements 2.1

### Property 7: Stack Trace Preservation

For any exception thrown through nested function calls, the stack trace should contain all intermediate frames with source locations. Validates: Requirements 2.2

### Property 8: Error Propagation (No Silent NaN)

For any runtime function that encounters an error condition, it should propagate an exception rather than returning NaN silently. Validates: Requirements 2.3

### Property 9: Null/Undefined Property Access

For any property access operation on null or undefined, the runtime should throw a TypeError with a descriptive message. Validates: Requirements 2.4, 14.1, 14.2

### Property 10: Loop Variable Correctness

For any for/while loop with variable modifications (including i++, i--, i+=n), the loop variable should be correctly updated on each iteration and the loop should terminate when the condition becomes false. Validates: Requirements 3.1, 3.2, 3.3

### Property 11: Break/Continue Targeting

For any break or continue statement (including labeled), control flow should transfer to the correct target loop. Validates: Requirements 3.4, 3.5, 3.6

### Property 12: Phi Node Value Merging

For any variable modified in different branches of an if/else, the value at the join point should be the value from the taken branch. Validates: Requirements 3.7

### Property 13: Equality Operator Semantics

For any pair of values compared with == or ===, the result should match ECMAScript specification (== with coercion, === without). Validates: Requirements 4.1, 4.2

### Property 14: Type Coercion Semantics

For any value converted via ToBoolean, ToNumber, or ToString, the result should match ECMAScript specification. Validates: Requirements 4.4, 4.5, 4.6

### Property 15: String Concatenation

For any + operation where at least one operand is a string, the result should be string concatenation. Validates: Requirements 4.3

### Property 16: BigInt/Number Interaction

For any arithmetic operation mixing BigInt and Number, a TypeError should be thrown. Comparison operations should be allowed. Validates: Requirements 4.7

### Property 17: Async Function Promise Return

For any async function call, the return value should be a Promise that resolves to the function's return value or rejects with thrown errors. Validates: Requirements 5.1, 5.4, 5.5

### Property 18: Await Suspension

For any await expression, execution should suspend until the Promise settles, then resume with the resolved value or throw the rejection reason. Validates: Requirements 5.2, 5.3

### Property 19: Promise.all/race Semantics

For any Promise.all call, the result should be an array of all resolved values. For Promise.race, the result should be the first settled value. Validates: Requirements 5.6, 5.7

### Property 20: Class Instantiation

For any class instantiated with new, the constructor should be called with correct this binding and the instance should have the correct prototype chain. Validates: Requirements 6.1, 6.2, 6.3

### Property 21: Super Call Semantics

For any super() or super.method() call, the parent class constructor/method should be invoked with correct this binding. Validates: Requirements 6.4, 6.5

### Property 22: Static Method Attachment

For any static method definition, the method should be accessible on the class constructor, not on instances. Validates: Requirements 6.6

### Property 23: Getter/Setter Invocation

For any property access/assignment on an object with getters/setters, the accessor function should be invoked. Validates: Requirements 6.7

### Property 24: Private Field Encapsulation

For any private field (#field) access from outside the class, a TypeError should be thrown. Validates: Requirements 6.8

### Property 25: Destructuring Extraction

For any destructuring pattern (array or object), values should be extracted by index/name with defaults applied for undefined values. Validates: Requirements 7.1, 7.2, 7.3

### Property 26: Rest Pattern Collection

For any rest pattern (...rest) in destructuring, remaining elements/properties should be collected into an array/object. Validates: Requirements 7.4, 7.5, 7.6

### Property 27: Destructuring Null/Undefined Error

For any destructuring of null or undefined, a TypeError should be thrown. Validates: Requirements 7.7

### Property 28: Template Literal Interpolation

For any template literal with expressions, expressions should be evaluated and interpolated with line breaks preserved. Validates: Requirements 8.1, 8.2

### Property 29: Tagged Template Invocation

For any tagged template, the tag function should receive the strings array and interpolated values as arguments. Validates: Requirements 8.3

### Property 30: Generator State Machine

For any generator function, calling it should return a generator object. next() should execute until yield, returning {value, done}. The generator should complete with {value: undefined, done: true}. Validates: Requirements 9.1, 9.2, 9.3, 9.4

### Property 31: Generator Send/Throw/Return

For any generator, next(value) should pass value as yield result, throw(error) should throw inside generator, return(value) should complete with value. Validates: Requirements 9.5, 9.6, 9.7

### Property 32: Spread Operator Expansion

For any spread in array literal, function call, or object literal, elements/properties should be expanded correctly. Validates: Requirements 10.1, 10.2, 10.3

### Property 33: Spread Error Handling

For any spread of non-iterable in array context, TypeError should be thrown. Spreading null/undefined in object context should be skipped. Validates: Requirements 10.4, 10.5

### Property 34: Module Loading and Caching

For any module import, the module should be loaded, executed once, and cached for subsequent imports. Validates: Requirements 11.1, 11.2, 11.5

### Property 35: Circular Dependency Handling

For any circular module dependency, the runtime should handle it per ESM semantics (partial exports visible). Validates: Requirements 11.3

### Property 36: Dynamic Import Promise

For any dynamic import() expression, a Promise should be returned that resolves to the module namespace. Validates: Requirements 11.4

### Property 37: Constant Folding

For any expression with constant operands, the result should be computed at compile time. Validates: Requirements 12.1

### Property 38: Dead Code Elimination

For any unreachable code (after return, in false branch of constant condition), the code should not be emitted. Validates: Requirements 12.2

### Property 39: JSON Round-Trip

For any JSON-serializable value, JSON.parse(JSON.stringify(value)) should produce an equivalent value. Validates: Requirements 13.5, 13.6

### Property 40: Array Method Correctness

For any Array method (map, filter, reduce, etc.), the result should match ECMAScript specification. Validates: Requirements 13.1

### Property 41: String Method Correctness

For any String method (split, slice, substring, etc.), the result should match ECMAScript specification. Validates: Requirements 13.2

### Property 42: Non-Callable Error

For any call expression on a non-function value, a TypeError should be thrown. Validates: Requirements 14.3

### Property 43: Non-Constructor Error

For any new expression on a non-constructor, a TypeError should be thrown. Validates: Requirements 14.4

### Property 44: IEEE 754 Special Values

For any arithmetic operation producing NaN or Infinity, the value should propagate correctly per IEEE 754. Validates: Requirements 14.5, 14.6

## Error Handling

### Exception Types

+-----------+----------+------------+
| Error     | Type     | Conditions |
+===========+==========+============+
| TypeError | Property | access     |
+-----------+----------+------------+



### Stack Trace Format

```
TypeError: Cannot read property 'x' of undefined at functionName (file.js:10:5)
at callerFunction (file.js:20:10)
at <anonymous> (file.js:30:1)
```

## Testing Strategy

### Property-Based Testing

We use `proptest` for Rust property-based testing with minimum 100 iterations per property.
```rust
proptest! {


#![proptest_config(ProptestConfig::with_cases(100))]


// Feature: dx-runtime-production-ready, Property 10: Loop Variable Correctness


#[test]


fn prop_loop_variable_correctness( initial: i32, limit: i32, step: i32 ) { // Test that loop executes correct number of iterations prop_assume!(step != 0);
prop_assume!((limit - initial).abs() < 1000);
// ... test implementation }
}
```

### Unit Tests

Unit tests cover specific examples and edge cases: -Empty arrays/objects -Boundary values (MAX_SAFE_INTEGER, MIN_SAFE_INTEGER) -Unicode strings -Deeply nested structures

### Integration Tests

Integration tests verify end-to-end behavior: -Running JavaScript files through the full pipeline -Module loading and execution -Async/await with real timers

### MIRI Testing

For unsafe code blocks, run under MIRI to detect: -Use-after-free -Data races -Uninitialized memory access -Invalid pointer dereferences ```bash cargo +nightly miri test ```

## Notes

- The Cranelift Variable system is essential for correct SSA phi node handling in loops
- All values are encoded as f64 using NaN-boxing for efficient primitive representation
- The RuntimeHeap uses HashMap for simplicity; production may need more efficient structures
- Thread safety requires careful synchronization around the global heap
- Property-based tests should cover all acceptance criteria marked as testable
