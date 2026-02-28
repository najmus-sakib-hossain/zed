//! DX Test VM - Custom Bytecode VM
//!
//! Stack-based bytecode VM optimized for test execution.

use dx_test_core::*;
use std::time::Instant;

const STACK_SIZE: usize = 1024;
const LOCALS_SIZE: usize = 256;

/// Stack-only VM (no heap allocation during execution!)
pub struct TestVM {
    /// Value stack (fixed size, no allocations)
    stack: Vec<Value>,
    /// Stack pointer
    sp: usize,
    /// Local registers (faster than stack for locals)
    #[allow(dead_code)]
    locals: [Value; LOCALS_SIZE],
    /// Assertion results (pre-allocated)
    assertions: Vec<AssertionResult>,
    /// Current test status
    status: TestStatus,
}

impl TestVM {
    pub fn new() -> Self {
        Self {
            stack: vec![Value::undefined(); STACK_SIZE],
            sp: 0,
            locals: [Value::undefined(); LOCALS_SIZE],
            assertions: Vec::with_capacity(32),
            status: TestStatus::Passed,
        }
    }

    /// Execute bytecode with zero allocations
    pub fn execute(&mut self, bytecode: &[u8]) -> TestResult {
        let start = Instant::now();
        self.reset();

        let mut pc = 0;
        while pc < bytecode.len() {
            let opcode = bytecode[pc];
            pc += 1;

            match opcode {
                // Fast push for inline integers
                0x20 => {
                    // PushInt
                    if pc + 4 > bytecode.len() {
                        break;
                    }
                    let val = i32::from_le_bytes([
                        bytecode[pc],
                        bytecode[pc + 1],
                        bytecode[pc + 2],
                        bytecode[pc + 3],
                    ]);
                    pc += 4;
                    self.push(Value::int(val));
                }

                // Super fast boolean push
                0x22 => self.push(Value::bool(true)), // PushTrue
                0x23 => self.push(Value::bool(false)), // PushFalse
                0x24 => self.push(Value::null()),     // PushNull
                0x25 => self.push(Value::undefined()), // PushUndefined

                // Assertions
                0x50 => {
                    // AssertEq
                    if self.sp >= 2 {
                        let expected = self.pop();
                        let actual = self.pop();
                        let result = actual.0 == expected.0;

                        self.record_assertion(result, 0x50);
                        if !result {
                            self.status = TestStatus::Failed;
                        }
                    }
                }

                0x51 => {
                    // AssertDeepEq
                    if self.sp >= 2 {
                        let expected = self.pop();
                        let actual = self.pop();
                        let result = self.deep_eq(actual, expected);

                        self.record_assertion(result, 0x51);
                        if !result {
                            self.status = TestStatus::Failed;
                        }
                    }
                }

                0x52 => {
                    // AssertTruthy
                    if self.sp >= 1 {
                        let value = self.pop();
                        let result = value.is_truthy();

                        self.record_assertion(result, 0x52);
                        if !result {
                            self.status = TestStatus::Failed;
                        }
                    }
                }

                0x53 => {
                    // AssertFalsy
                    if self.sp >= 1 {
                        let value = self.pop();
                        let result = !value.is_truthy();

                        self.record_assertion(result, 0x53);
                        if !result {
                            self.status = TestStatus::Failed;
                        }
                    }
                }

                0x54 => {
                    // AssertNull
                    if self.sp >= 1 {
                        let value = self.pop();
                        let result = value.is_null();

                        self.record_assertion(result, 0x54);
                        if !result {
                            self.status = TestStatus::Failed;
                        }
                    }
                }

                // Arithmetic
                0x30 => {
                    // Add
                    if self.sp >= 2 {
                        let b = self.pop();
                        let a = self.pop();
                        self.push(self.add(a, b));
                    }
                }

                0x31 => {
                    // Sub
                    if self.sp >= 2 {
                        let b = self.pop();
                        let a = self.pop();
                        self.push(self.sub(a, b));
                    }
                }

                // Comparison
                0x40 => {
                    // Eq
                    if self.sp >= 2 {
                        let b = self.pop();
                        let a = self.pop();
                        self.push(Value::bool(a.0 == b.0));
                    }
                }

                0x46 => {
                    // StrictEq
                    if self.sp >= 2 {
                        let b = self.pop();
                        let a = self.pop();
                        self.push(Value::bool(a.0 == b.0));
                    }
                }

                // Test result
                0xF0 => {
                    // TestPass
                    self.status = TestStatus::Passed;
                }

                0xF1 => {
                    // TestFail
                    self.status = TestStatus::Failed;
                }

                0xF2 => {
                    // TestSkip
                    self.status = TestStatus::Skipped;
                }

                0xFF => break, // End

                _ => {
                    // Unknown opcode - ignore for now
                }
            }
        }

        let duration = start.elapsed();
        let first_failure = self.assertions.iter().position(|a| !a.passed).map(|i| i as u16);

        TestResult {
            status: self.status,
            duration,
            assertions: self.assertions.len() as u16,
            first_failure,
            error_message: first_failure.map(|_| "Assertion failed".to_string()),
        }
    }

    #[inline(always)]
    fn push(&mut self, value: Value) {
        if self.sp < self.stack.len() {
            self.stack[self.sp] = value;
            self.sp += 1;
        }
    }

    #[inline(always)]
    fn pop(&mut self) -> Value {
        if self.sp > 0 {
            self.sp -= 1;
            self.stack[self.sp]
        } else {
            Value::undefined()
        }
    }

    fn add(&self, a: Value, b: Value) -> Value {
        if let (Some(a_int), Some(b_int)) = (a.as_int(), b.as_int()) {
            Value::int(a_int.wrapping_add(b_int))
        } else if let (Some(a_float), Some(b_float)) = (a.as_float(), b.as_float()) {
            Value::float(a_float + b_float)
        } else {
            Value::undefined()
        }
    }

    fn sub(&self, a: Value, b: Value) -> Value {
        if let (Some(a_int), Some(b_int)) = (a.as_int(), b.as_int()) {
            Value::int(a_int.wrapping_sub(b_int))
        } else if let (Some(a_float), Some(b_float)) = (a.as_float(), b.as_float()) {
            Value::float(a_float - b_float)
        } else {
            Value::undefined()
        }
    }

    fn deep_eq(&self, a: Value, b: Value) -> bool {
        // Simplified deep equality
        a.0 == b.0
    }

    fn record_assertion(&mut self, passed: bool, opcode: u8) {
        self.assertions.push(AssertionResult {
            passed,
            opcode,
            index: self.assertions.len() as u16,
            message: None,
        });
    }

    pub fn reset(&mut self) {
        self.sp = 0;
        self.assertions.clear();
        self.status = TestStatus::Passed;
    }
}

impl Default for TestVM {
    fn default() -> Self {
        Self::new()
    }
}
