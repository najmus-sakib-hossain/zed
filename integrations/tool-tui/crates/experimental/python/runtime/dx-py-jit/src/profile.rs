//! Function profiling for JIT compilation decisions
//!
//! This module provides type feedback collection for the optimizing JIT compiler.
//! Type feedback is collected during interpretation and used to make specialization
//! decisions during optimized compilation.
//!
//! ## Type Feedback States
//!
//! - **Uninitialized**: No types observed yet
//! - **Monomorphic**: Single type observed - can emit specialized code without guards
//! - **Polymorphic**: 2-4 types observed - emit inline caches with type guards
//! - **Megamorphic**: Too many types - use generic code path

use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering};

/// Python type tags for type feedback
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PyType {
    Unknown = 0,
    None = 1,
    Bool = 2,
    Int = 3,
    Float = 4,
    Str = 5,
    Bytes = 6,
    List = 7,
    Tuple = 8,
    Dict = 9,
    Set = 10,
    Function = 11,
    Class = 12,
    Object = 13,
    Module = 14,
    Iterator = 15,
    Generator = 16,
    Coroutine = 17,
    NativeFunction = 18,
    BoundMethod = 19,
}

impl PyType {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Unknown,
            1 => Self::None,
            2 => Self::Bool,
            3 => Self::Int,
            4 => Self::Float,
            5 => Self::Str,
            6 => Self::Bytes,
            7 => Self::List,
            8 => Self::Tuple,
            9 => Self::Dict,
            10 => Self::Set,
            11 => Self::Function,
            12 => Self::Class,
            13 => Self::Object,
            14 => Self::Module,
            15 => Self::Iterator,
            16 => Self::Generator,
            17 => Self::Coroutine,
            18 => Self::NativeFunction,
            19 => Self::BoundMethod,
            _ => Self::Unknown,
        }
    }

    /// Check if this type supports fast integer arithmetic
    pub fn supports_fast_int_arithmetic(&self) -> bool {
        matches!(self, Self::Int | Self::Bool)
    }

    /// Check if this type supports fast float arithmetic
    pub fn supports_fast_float_arithmetic(&self) -> bool {
        matches!(self, Self::Float)
    }

    /// Check if this type is a numeric type
    pub fn is_numeric(&self) -> bool {
        matches!(self, Self::Int | Self::Float | Self::Bool)
    }

    /// Check if this type is a sequence type
    pub fn is_sequence(&self) -> bool {
        matches!(self, Self::List | Self::Tuple | Self::Str | Self::Bytes)
    }

    /// Check if this type is callable
    pub fn is_callable(&self) -> bool {
        matches!(self, Self::Function | Self::Class | Self::NativeFunction | Self::BoundMethod)
    }
}

/// Type feedback state for a bytecode location
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeState {
    /// No types observed yet
    Uninitialized,
    /// Single type observed - can specialize without guards
    Monomorphic,
    /// 2-4 types observed - use inline caches
    Polymorphic,
    /// Too many types - use generic code
    Megamorphic,
}

/// Function profile collected during interpretation
#[derive(Default)]
pub struct FunctionProfile {
    /// Number of times this function has been called
    pub call_count: AtomicU64,
    /// Type feedback for each bytecode location
    pub type_feedback: Vec<TypeFeedback>,
    /// Branch counts (taken, not_taken) for each branch
    pub branch_counts: Vec<(AtomicU64, AtomicU64)>,
    /// Number of deoptimizations
    pub deopt_count: AtomicU32,
    /// Total execution time in nanoseconds (for hot function detection)
    pub total_time_ns: AtomicU64,
    /// Number of loop iterations (for loop optimization decisions)
    pub loop_iterations: AtomicU64,
}

impl FunctionProfile {
    /// Create a new profile for a function with the given number of bytecode locations
    pub fn new(bytecode_len: usize, branch_count: usize) -> Self {
        Self {
            call_count: AtomicU64::new(0),
            type_feedback: (0..bytecode_len).map(|_| TypeFeedback::new()).collect(),
            branch_counts: (0..branch_count)
                .map(|_| (AtomicU64::new(0), AtomicU64::new(0)))
                .collect(),
            deopt_count: AtomicU32::new(0),
            total_time_ns: AtomicU64::new(0),
            loop_iterations: AtomicU64::new(0),
        }
    }

    /// Record a function call
    #[inline]
    pub fn record_call(&self) {
        self.call_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get the call count
    #[inline]
    pub fn get_call_count(&self) -> u64 {
        self.call_count.load(Ordering::Relaxed)
    }

    /// Record a type observation at a bytecode location
    #[inline]
    pub fn record_type(&self, bc_offset: usize, py_type: PyType) {
        if let Some(feedback) = self.type_feedback.get(bc_offset) {
            feedback.record(py_type);
        }
    }

    /// Record types for a binary operation (both operands)
    #[inline]
    pub fn record_binary_types(&self, bc_offset: usize, left: PyType, right: PyType) {
        if let Some(feedback) = self.type_feedback.get(bc_offset) {
            feedback.record_binary(left, right);
        }
    }

    /// Get the type state at a bytecode location
    pub fn get_type_state(&self, bc_offset: usize) -> TypeState {
        self.type_feedback
            .get(bc_offset)
            .map(|f| f.get_state())
            .unwrap_or(TypeState::Uninitialized)
    }

    /// Record a branch taken
    #[inline]
    pub fn record_branch_taken(&self, branch_idx: usize) {
        if let Some((taken, _)) = self.branch_counts.get(branch_idx) {
            taken.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record a branch not taken
    #[inline]
    pub fn record_branch_not_taken(&self, branch_idx: usize) {
        if let Some((_, not_taken)) = self.branch_counts.get(branch_idx) {
            not_taken.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record a deoptimization
    #[inline]
    pub fn record_deopt(&self) {
        self.deopt_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get the deoptimization count
    #[inline]
    pub fn get_deopt_count(&self) -> u32 {
        self.deopt_count.load(Ordering::Relaxed)
    }

    /// Record execution time
    #[inline]
    pub fn record_time(&self, ns: u64) {
        self.total_time_ns.fetch_add(ns, Ordering::Relaxed);
    }

    /// Get total execution time
    #[inline]
    pub fn get_total_time_ns(&self) -> u64 {
        self.total_time_ns.load(Ordering::Relaxed)
    }

    /// Record loop iterations
    #[inline]
    pub fn record_loop_iteration(&self) {
        self.loop_iterations.fetch_add(1, Ordering::Relaxed);
    }

    /// Get loop iteration count
    #[inline]
    pub fn get_loop_iterations(&self) -> u64 {
        self.loop_iterations.load(Ordering::Relaxed)
    }

    /// Get branch probability (taken / total)
    pub fn get_branch_probability(&self, branch_idx: usize) -> Option<f64> {
        self.branch_counts.get(branch_idx).map(|(taken, not_taken)| {
            let t = taken.load(Ordering::Relaxed) as f64;
            let n = not_taken.load(Ordering::Relaxed) as f64;
            let total = t + n;
            if total > 0.0 {
                t / total
            } else {
                0.5
            }
        })
    }

    /// Check if this function is hot enough for optimizing compilation
    pub fn is_hot_for_optimization(&self) -> bool {
        let calls = self.get_call_count();
        let deopts = self.get_deopt_count() as u64;
        // Hot if called 1000+ times and not deoptimizing too often
        calls >= 1000 && deopts < calls / 10
    }

    /// Get a summary of type feedback for optimization decisions
    pub fn get_type_summary(&self) -> TypeFeedbackSummary {
        let mut monomorphic_sites = 0;
        let mut polymorphic_sites = 0;
        let mut megamorphic_sites = 0;

        for feedback in &self.type_feedback {
            match feedback.get_state() {
                TypeState::Monomorphic => monomorphic_sites += 1,
                TypeState::Polymorphic => polymorphic_sites += 1,
                TypeState::Megamorphic => megamorphic_sites += 1,
                TypeState::Uninitialized => {}
            }
        }

        TypeFeedbackSummary {
            monomorphic_sites,
            polymorphic_sites,
            megamorphic_sites,
            total_sites: self.type_feedback.len(),
        }
    }
}

/// Summary of type feedback for a function
#[derive(Debug, Clone)]
pub struct TypeFeedbackSummary {
    pub monomorphic_sites: usize,
    pub polymorphic_sites: usize,
    pub megamorphic_sites: usize,
    pub total_sites: usize,
}

impl TypeFeedbackSummary {
    /// Calculate the specialization potential (0.0 to 1.0)
    /// Higher values indicate more opportunity for type specialization
    pub fn specialization_potential(&self) -> f64 {
        if self.total_sites == 0 {
            return 0.0;
        }
        let specialized = self.monomorphic_sites + self.polymorphic_sites;
        specialized as f64 / self.total_sites as f64
    }
}

/// Type feedback for a single bytecode location
pub struct TypeFeedback {
    /// Observed types (up to 4)
    observed_types: [AtomicU8; 4],
    /// Number of types observed
    type_count: AtomicU8,
    /// Observation counts for each type slot
    observation_counts: [AtomicU32; 4],
    /// Total observations
    total_observations: AtomicU64,
    /// For binary operations: observed right-hand types
    rhs_types: [AtomicU8; 4],
    /// Number of RHS types observed
    rhs_type_count: AtomicU8,
}

impl TypeFeedback {
    /// Create new type feedback
    pub fn new() -> Self {
        Self {
            observed_types: [
                AtomicU8::new(PyType::Unknown as u8),
                AtomicU8::new(PyType::Unknown as u8),
                AtomicU8::new(PyType::Unknown as u8),
                AtomicU8::new(PyType::Unknown as u8),
            ],
            type_count: AtomicU8::new(0),
            observation_counts: [
                AtomicU32::new(0),
                AtomicU32::new(0),
                AtomicU32::new(0),
                AtomicU32::new(0),
            ],
            total_observations: AtomicU64::new(0),
            rhs_types: [
                AtomicU8::new(PyType::Unknown as u8),
                AtomicU8::new(PyType::Unknown as u8),
                AtomicU8::new(PyType::Unknown as u8),
                AtomicU8::new(PyType::Unknown as u8),
            ],
            rhs_type_count: AtomicU8::new(0),
        }
    }

    /// Record an observed type
    pub fn record(&self, py_type: PyType) {
        let type_byte = py_type as u8;
        let count = self.type_count.load(Ordering::Relaxed) as usize;
        self.total_observations.fetch_add(1, Ordering::Relaxed);

        // Check if we already have this type
        for i in 0..count.min(4) {
            if self.observed_types[i].load(Ordering::Relaxed) == type_byte {
                self.observation_counts[i].fetch_add(1, Ordering::Relaxed);
                return; // Already recorded, just increment count
            }
        }

        // Add new type if we have room
        if count < 4 {
            self.observed_types[count].store(type_byte, Ordering::Relaxed);
            self.observation_counts[count].fetch_add(1, Ordering::Relaxed);
            self.type_count.fetch_add(1, Ordering::Relaxed);
        }
        // If count >= 4, we're megamorphic - don't add more types
    }

    /// Record types for a binary operation
    pub fn record_binary(&self, left: PyType, right: PyType) {
        // Record left type normally
        self.record(left);

        // Record right type separately
        let type_byte = right as u8;
        let count = self.rhs_type_count.load(Ordering::Relaxed) as usize;

        // Check if we already have this RHS type
        for i in 0..count.min(4) {
            if self.rhs_types[i].load(Ordering::Relaxed) == type_byte {
                return;
            }
        }

        // Add new RHS type if we have room
        if count < 4 {
            self.rhs_types[count].store(type_byte, Ordering::Relaxed);
            self.rhs_type_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get the current type state
    pub fn get_state(&self) -> TypeState {
        let count = self.type_count.load(Ordering::Relaxed);
        match count {
            0 => TypeState::Uninitialized,
            1 => TypeState::Monomorphic,
            2..=4 => TypeState::Polymorphic,
            _ => TypeState::Megamorphic,
        }
    }

    /// Check if this site is monomorphic (single type)
    pub fn is_monomorphic(&self) -> bool {
        self.type_count.load(Ordering::Relaxed) == 1
    }

    /// Check if this site is polymorphic (2-4 types)
    pub fn is_polymorphic(&self) -> bool {
        let count = self.type_count.load(Ordering::Relaxed);
        (2..=4).contains(&count)
    }

    /// Check if this site is megamorphic (too many types)
    pub fn is_megamorphic(&self) -> bool {
        self.type_count.load(Ordering::Relaxed) > 4
    }

    /// Get the observed types
    pub fn get_types(&self) -> Vec<PyType> {
        let count = self.type_count.load(Ordering::Relaxed) as usize;
        (0..count.min(4))
            .map(|i| PyType::from_u8(self.observed_types[i].load(Ordering::Relaxed)))
            .collect()
    }

    /// Get the observed RHS types (for binary operations)
    pub fn get_rhs_types(&self) -> Vec<PyType> {
        let count = self.rhs_type_count.load(Ordering::Relaxed) as usize;
        (0..count.min(4))
            .map(|i| PyType::from_u8(self.rhs_types[i].load(Ordering::Relaxed)))
            .collect()
    }

    /// Get the primary type (most likely to be observed)
    pub fn get_primary_type(&self) -> Option<PyType> {
        if self.type_count.load(Ordering::Relaxed) > 0 {
            Some(PyType::from_u8(self.observed_types[0].load(Ordering::Relaxed)))
        } else {
            None
        }
    }

    /// Get types sorted by observation frequency (most common first)
    pub fn get_types_by_frequency(&self) -> Vec<(PyType, u32)> {
        let count = self.type_count.load(Ordering::Relaxed) as usize;
        let mut types: Vec<(PyType, u32)> = (0..count.min(4))
            .map(|i| {
                (
                    PyType::from_u8(self.observed_types[i].load(Ordering::Relaxed)),
                    self.observation_counts[i].load(Ordering::Relaxed),
                )
            })
            .collect();
        types.sort_by(|a, b| b.1.cmp(&a.1));
        types
    }

    /// Get the dominant type if one type accounts for >90% of observations
    pub fn get_dominant_type(&self) -> Option<PyType> {
        let total = self.total_observations.load(Ordering::Relaxed);
        if total == 0 {
            return None;
        }

        let count = self.type_count.load(Ordering::Relaxed) as usize;
        for i in 0..count.min(4) {
            let obs = self.observation_counts[i].load(Ordering::Relaxed) as u64;
            if obs * 10 >= total * 9 {
                // >90%
                return Some(PyType::from_u8(self.observed_types[i].load(Ordering::Relaxed)));
            }
        }
        None
    }

    /// Check if this is a homogeneous binary operation (same types on both sides)
    pub fn is_homogeneous_binary(&self) -> bool {
        let lhs_count = self.type_count.load(Ordering::Relaxed);
        let rhs_count = self.rhs_type_count.load(Ordering::Relaxed);

        if lhs_count != 1 || rhs_count != 1 {
            return false;
        }

        self.observed_types[0].load(Ordering::Relaxed) == self.rhs_types[0].load(Ordering::Relaxed)
    }

    /// Check if this binary operation is suitable for fast integer path
    pub fn can_use_fast_int_path(&self) -> bool {
        if !self.is_homogeneous_binary() {
            return false;
        }
        let ty = PyType::from_u8(self.observed_types[0].load(Ordering::Relaxed));
        ty.supports_fast_int_arithmetic()
    }

    /// Check if this binary operation is suitable for fast float path
    pub fn can_use_fast_float_path(&self) -> bool {
        if !self.is_homogeneous_binary() {
            return false;
        }
        let ty = PyType::from_u8(self.observed_types[0].load(Ordering::Relaxed));
        ty.supports_fast_float_arithmetic()
    }
}

impl Default for TypeFeedback {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_profile() {
        let profile = FunctionProfile::new(10, 2);

        assert_eq!(profile.get_call_count(), 0);

        profile.record_call();
        profile.record_call();

        assert_eq!(profile.get_call_count(), 2);
    }

    #[test]
    fn test_type_feedback() {
        let feedback = TypeFeedback::new();

        assert!(!feedback.is_monomorphic());
        assert_eq!(feedback.get_state(), TypeState::Uninitialized);

        feedback.record(PyType::Int);
        assert!(feedback.is_monomorphic());
        assert_eq!(feedback.get_state(), TypeState::Monomorphic);
        assert_eq!(feedback.get_types(), vec![PyType::Int]);

        feedback.record(PyType::Float);
        assert!(feedback.is_polymorphic());
        assert_eq!(feedback.get_state(), TypeState::Polymorphic);
        assert_eq!(feedback.get_types(), vec![PyType::Int, PyType::Float]);

        // Recording same type again shouldn't add it
        feedback.record(PyType::Int);
        assert_eq!(feedback.get_types().len(), 2);
    }

    #[test]
    fn test_branch_probability() {
        let profile = FunctionProfile::new(10, 1);

        // 75% taken
        for _ in 0..75 {
            profile.record_branch_taken(0);
        }
        for _ in 0..25 {
            profile.record_branch_not_taken(0);
        }

        let prob = profile.get_branch_probability(0).unwrap();
        assert!((prob - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_type_feedback_binary() {
        let feedback = TypeFeedback::new();

        feedback.record_binary(PyType::Int, PyType::Int);
        assert!(feedback.is_monomorphic());
        assert!(feedback.is_homogeneous_binary());
        assert!(feedback.can_use_fast_int_path());
        assert!(!feedback.can_use_fast_float_path());

        let feedback2 = TypeFeedback::new();
        feedback2.record_binary(PyType::Float, PyType::Float);
        assert!(feedback2.can_use_fast_float_path());
        assert!(!feedback2.can_use_fast_int_path());
    }

    #[test]
    fn test_type_feedback_frequency() {
        let feedback = TypeFeedback::new();

        // Record Int 10 times, Float 2 times
        for _ in 0..10 {
            feedback.record(PyType::Int);
        }
        for _ in 0..2 {
            feedback.record(PyType::Float);
        }

        let types = feedback.get_types_by_frequency();
        assert_eq!(types.len(), 2);
        assert_eq!(types[0].0, PyType::Int);
        assert_eq!(types[0].1, 10);
        assert_eq!(types[1].0, PyType::Float);
        assert_eq!(types[1].1, 2);
    }

    #[test]
    fn test_dominant_type() {
        let feedback = TypeFeedback::new();

        // Record Int 95 times, Float 5 times
        for _ in 0..95 {
            feedback.record(PyType::Int);
        }
        for _ in 0..5 {
            feedback.record(PyType::Float);
        }

        assert_eq!(feedback.get_dominant_type(), Some(PyType::Int));

        // Now test without dominant type
        let feedback2 = TypeFeedback::new();
        for _ in 0..50 {
            feedback2.record(PyType::Int);
        }
        for _ in 0..50 {
            feedback2.record(PyType::Float);
        }
        assert_eq!(feedback2.get_dominant_type(), None);
    }

    #[test]
    fn test_type_summary() {
        let profile = FunctionProfile::new(5, 0);

        // Site 0: monomorphic
        profile.record_type(0, PyType::Int);

        // Site 1: polymorphic (2 types)
        profile.record_type(1, PyType::Int);
        profile.record_type(1, PyType::Float);

        // Site 2: polymorphic (4 types - max stored)
        profile.record_type(2, PyType::Int);
        profile.record_type(2, PyType::Float);
        profile.record_type(2, PyType::Str);
        profile.record_type(2, PyType::List);

        // Sites 3 and 4: uninitialized

        let summary = profile.get_type_summary();
        assert_eq!(summary.monomorphic_sites, 1);
        assert_eq!(summary.polymorphic_sites, 2); // Sites 1 and 2 are polymorphic
        assert_eq!(summary.megamorphic_sites, 0);
        assert_eq!(summary.total_sites, 5);
    }

    #[test]
    fn test_hot_for_optimization() {
        let profile = FunctionProfile::new(10, 0);

        // Not hot yet
        assert!(!profile.is_hot_for_optimization());

        // Call 1000 times
        for _ in 0..1000 {
            profile.record_call();
        }
        assert!(profile.is_hot_for_optimization());

        // Too many deopts
        for _ in 0..200 {
            profile.record_deopt();
        }
        assert!(!profile.is_hot_for_optimization());
    }

    #[test]
    fn test_py_type_properties() {
        assert!(PyType::Int.supports_fast_int_arithmetic());
        assert!(PyType::Bool.supports_fast_int_arithmetic());
        assert!(!PyType::Float.supports_fast_int_arithmetic());

        assert!(PyType::Float.supports_fast_float_arithmetic());
        assert!(!PyType::Int.supports_fast_float_arithmetic());

        assert!(PyType::Int.is_numeric());
        assert!(PyType::Float.is_numeric());
        assert!(!PyType::Str.is_numeric());

        assert!(PyType::List.is_sequence());
        assert!(PyType::Str.is_sequence());
        assert!(!PyType::Dict.is_sequence());

        assert!(PyType::Function.is_callable());
        assert!(PyType::Class.is_callable());
        assert!(!PyType::Int.is_callable());
    }
}
