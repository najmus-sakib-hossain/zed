
# Deoptimization Implementation

## Overview

This document describes the deoptimization implementation for task 10.4 of the dx-py-production-ready spec.

## Requirements Validated

- Requirement 7.3: WHEN a type guard fails in JIT code, THE Runtime SHALL deoptimize back to the interpreter
- Requirement 7.4: WHEN JIT compilation fails, THE Runtime SHALL fall back to interpretation without crashing

## Architecture

### Type Guards

Type guards are runtime checks inserted into JIT-compiled code to verify that values have the expected types. The baseline compiler inserts type guards for speculative optimizations, particularly for arithmetic operations that assume integer types. Key Components: -TypeGuard (`deopt.rs`): Represents a type check at a specific location in native code -Tracks the kind of check (IsInt, IsFloat, IsString, etc.) -Records native code offset and corresponding bytecode offset -Maintains statistics on check/failure counts -Type Guard Runtime Helpers (`deopt.rs`): -`rt_type_guard_is_int`: Checks if a value is an integer -`rt_type_guard_is_float`: Checks if a value is a float -`rt_type_guard_is_string`: Checks if a value is a string -`rt_type_guard_is_not_none`: Checks if a value is not None

### Deoptimization Flow

When a type guard fails: -Guard Detection: The JIT code calls a type guard helper (e.g., `rt_type_guard_is_int`) -Guard Failure: If the check fails, the code branches to a deoptimization block -Trigger Deopt: The deopt block calls `rt_trigger_deopt` with:-Function ID -Bytecode offset to resume at -Deoptimization reason (TypeGuardFailed, IntegerOverflow, etc.) -Return Sentinel: The JIT code returns a sentinel value (0/None) to signal deoptimization -Interpreter Resumption: The runtime detects the deoptimization and resumes execution in the interpreter

### Frame State Reconstruction

DeoptFrameState captures the state needed to resume in the interpreter: -Bytecode offset: Where to resume execution -Stack values: Operand stack contents with their locations (registers, stack slots, constants) -Local variables: Local variable values with their locations -Deoptimization reason: Why the deopt occurred DeoptMetadata tracks all deoptimization points for a function: -Maps native code offsets to frame states -Counts total deoptimizations -Implements a "give up" threshold (default: 10 deopts)

### Baseline Compiler Integration

The baseline compiler (`baseline.rs`) implements speculative optimizations with type guards: Guarded Operations: -`emit_guarded_int_add`: Integer addition with type guards -`emit_guarded_int_sub`: Integer subtraction with type guards -`emit_guarded_int_mul`: Integer multiplication with type guards Implementation Pattern:
```rust
fn emit_guarded_int_add(builder, state, a, b, func_id) -> Value { // Guard that 'a' is an integer emit_int_type_guard(builder, state, a, func_id);
// Guard that 'b' is an integer emit_int_type_guard(builder, state, b, func_id);
// Both guards passed, perform integer addition builder.ins().iadd(a, b)
}
```
Type Guard Emission:
```rust
fn emit_int_type_guard(builder, state, value, func_id) -> (Block, Block) { // Call rt_type_guard_is_int let is_int = call_guard_function(value);
// Branch on result if is_int { continue_block // Guard passed, continue execution } else { deopt_block // Guard failed, trigger deoptimization }
}
```

### Deoptimization Manager

DeoptManager (`deopt.rs`) provides centralized deoptimization tracking: -Registers deopt metadata for each compiled function -Handles deoptimization events -Tracks statistics (total deopts, deopts by reason, functions given up) -Determines when to give up on optimization DeoptResult indicates what action to take after a deopt: -`should_recompile`: Whether to recompile with updated type feedback -`should_give_up`: Whether to stop trying to optimize this function

## Usage Example

```rust
use dx_py_jit::{BaselineCompiler, FunctionId};
let mut compiler = BaselineCompiler::new()?;
let func_id = FunctionId(1);
// Compile with type guards let code_ptr = compiler.compile(func_id, &code_object)?;
// Get compiled code with deopt metadata let compiled = compiler.cache.get(&func_id).unwrap();
// Check type guards assert!(!compiled.type_guards.is_empty());
// Check deopt points assert!(!compiled.deopt_metadata.deopt_points.is_empty());
```

## Testing

The implementation includes comprehensive tests: -test_deoptimization_metadata_generation: Verifies that type guards and deopt points are generated -test_multiple_guarded_operations: Tests multiple guarded operations in sequence -test_deopt_metadata_func_id: Verifies correct function ID tracking -test_deopt_manager: Tests deoptimization event handling -test_type_guard: Tests type guard creation and failure tracking

## Performance Considerations

- Type Guard Overhead: Each guard adds a runtime check and branch
- Deoptimization Cost: Deoptimizing is expensive (frame reconstruction, interpreter switch)
- Give Up Threshold: After too many deopts (default: 10), the function stays in interpreter mode
- Recompilation: Functions can be recompiled with better type feedback after deoptimization

## Future Enhancements

- Inline Caching: Cache type checks for polymorphic sites
- Adaptive Guards: Adjust guard placement based on failure patterns
- OSR (On-Stack Replacement): Deoptimize mid-function execution
- Profile-Guided Optimization: Use deopt statistics to guide recompilation
