//! Type predictor with statistics

use dashmap::DashMap;
use dx_py_jit::compiler::FunctionId;
use dx_py_jit::profile::PyType;
use std::sync::atomic::{AtomicU64, Ordering};

/// Type statistics for a bytecode location
#[derive(Default)]
pub struct TypeStats {
    pub int_count: AtomicU64,
    pub float_count: AtomicU64,
    pub str_count: AtomicU64,
    pub list_count: AtomicU64,
    pub dict_count: AtomicU64,
    pub tuple_count: AtomicU64,
    pub none_count: AtomicU64,
    pub bool_count: AtomicU64,
    pub other_count: AtomicU64,
}

impl TypeStats {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a type observation
    pub fn record(&self, py_type: PyType) {
        match py_type {
            PyType::Int => self.int_count.fetch_add(1, Ordering::Relaxed),
            PyType::Float => self.float_count.fetch_add(1, Ordering::Relaxed),
            PyType::Str => self.str_count.fetch_add(1, Ordering::Relaxed),
            PyType::List => self.list_count.fetch_add(1, Ordering::Relaxed),
            PyType::Dict => self.dict_count.fetch_add(1, Ordering::Relaxed),
            PyType::Tuple => self.tuple_count.fetch_add(1, Ordering::Relaxed),
            PyType::None => self.none_count.fetch_add(1, Ordering::Relaxed),
            PyType::Bool => self.bool_count.fetch_add(1, Ordering::Relaxed),
            _ => self.other_count.fetch_add(1, Ordering::Relaxed),
        };
    }

    /// Get total observations
    pub fn total(&self) -> u64 {
        self.int_count.load(Ordering::Relaxed)
            + self.float_count.load(Ordering::Relaxed)
            + self.str_count.load(Ordering::Relaxed)
            + self.list_count.load(Ordering::Relaxed)
            + self.dict_count.load(Ordering::Relaxed)
            + self.tuple_count.load(Ordering::Relaxed)
            + self.none_count.load(Ordering::Relaxed)
            + self.bool_count.load(Ordering::Relaxed)
            + self.other_count.load(Ordering::Relaxed)
    }

    /// Get the most common type and its count
    pub fn most_common(&self) -> (PyType, u64) {
        let counts = [
            (PyType::Int, self.int_count.load(Ordering::Relaxed)),
            (PyType::Float, self.float_count.load(Ordering::Relaxed)),
            (PyType::Str, self.str_count.load(Ordering::Relaxed)),
            (PyType::List, self.list_count.load(Ordering::Relaxed)),
            (PyType::Dict, self.dict_count.load(Ordering::Relaxed)),
            (PyType::Tuple, self.tuple_count.load(Ordering::Relaxed)),
            (PyType::None, self.none_count.load(Ordering::Relaxed)),
            (PyType::Bool, self.bool_count.load(Ordering::Relaxed)),
        ];

        counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .unwrap_or((PyType::Unknown, 0))
    }
}

/// Speculative type predictor
pub struct TypePredictor {
    /// Global type statistics per (function, bytecode offset)
    type_stats: DashMap<(FunctionId, usize), TypeStats>,
    /// Minimum observations before making predictions
    min_observations: u64,
    /// Confidence threshold for predictions (0.0 to 1.0)
    confidence_threshold: f64,
}

impl TypePredictor {
    /// Create a new type predictor
    pub fn new() -> Self {
        Self {
            type_stats: DashMap::new(),
            min_observations: 100,
            confidence_threshold: 0.9,
        }
    }

    /// Create with custom settings
    pub fn with_settings(min_observations: u64, confidence_threshold: f64) -> Self {
        Self {
            type_stats: DashMap::new(),
            min_observations,
            confidence_threshold,
        }
    }

    /// Record a type observation
    pub fn record(&self, func_id: FunctionId, bc_offset: usize, py_type: PyType) {
        self.type_stats.entry((func_id, bc_offset)).or_default().record(py_type);
    }

    /// Predict the most likely type for a bytecode location
    ///
    /// Returns None if:
    /// - Not enough observations
    /// - No type has sufficient confidence
    pub fn predict(&self, func_id: FunctionId, bc_offset: usize) -> Option<PyType> {
        let stats = self.type_stats.get(&(func_id, bc_offset))?;

        let total = stats.total();
        if total < self.min_observations {
            return None; // Not enough data
        }

        let (best_type, best_count) = stats.most_common();
        let confidence = best_count as f64 / total as f64;

        if confidence >= self.confidence_threshold {
            Some(best_type)
        } else {
            None
        }
    }

    /// Get the confidence for a specific type at a location
    pub fn get_confidence(&self, func_id: FunctionId, bc_offset: usize, py_type: PyType) -> f64 {
        let stats = match self.type_stats.get(&(func_id, bc_offset)) {
            Some(s) => s,
            None => return 0.0,
        };

        let total = stats.total();
        if total == 0 {
            return 0.0;
        }

        let count = match py_type {
            PyType::Int => stats.int_count.load(Ordering::Relaxed),
            PyType::Float => stats.float_count.load(Ordering::Relaxed),
            PyType::Str => stats.str_count.load(Ordering::Relaxed),
            PyType::List => stats.list_count.load(Ordering::Relaxed),
            PyType::Dict => stats.dict_count.load(Ordering::Relaxed),
            PyType::Tuple => stats.tuple_count.load(Ordering::Relaxed),
            PyType::None => stats.none_count.load(Ordering::Relaxed),
            PyType::Bool => stats.bool_count.load(Ordering::Relaxed),
            _ => stats.other_count.load(Ordering::Relaxed),
        };

        count as f64 / total as f64
    }

    /// Get all statistics for a location
    pub fn get_stats(&self, func_id: FunctionId, bc_offset: usize) -> Option<TypeStatsSnapshot> {
        let stats = self.type_stats.get(&(func_id, bc_offset))?;

        Some(TypeStatsSnapshot {
            int_count: stats.int_count.load(Ordering::Relaxed),
            float_count: stats.float_count.load(Ordering::Relaxed),
            str_count: stats.str_count.load(Ordering::Relaxed),
            list_count: stats.list_count.load(Ordering::Relaxed),
            dict_count: stats.dict_count.load(Ordering::Relaxed),
            tuple_count: stats.tuple_count.load(Ordering::Relaxed),
            none_count: stats.none_count.load(Ordering::Relaxed),
            bool_count: stats.bool_count.load(Ordering::Relaxed),
            other_count: stats.other_count.load(Ordering::Relaxed),
        })
    }

    /// Clear all statistics
    pub fn clear(&self) {
        self.type_stats.clear();
    }

    /// Clear statistics for a specific function
    pub fn clear_function(&self, func_id: FunctionId) {
        self.type_stats.retain(|(fid, _), _| *fid != func_id);
    }
}

impl Default for TypePredictor {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of type statistics (non-atomic)
#[derive(Debug, Clone)]
pub struct TypeStatsSnapshot {
    pub int_count: u64,
    pub float_count: u64,
    pub str_count: u64,
    pub list_count: u64,
    pub dict_count: u64,
    pub tuple_count: u64,
    pub none_count: u64,
    pub bool_count: u64,
    pub other_count: u64,
}

impl TypeStatsSnapshot {
    pub fn total(&self) -> u64 {
        self.int_count
            + self.float_count
            + self.str_count
            + self.list_count
            + self.dict_count
            + self.tuple_count
            + self.none_count
            + self.bool_count
            + self.other_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_stats() {
        let stats = TypeStats::new();

        stats.record(PyType::Int);
        stats.record(PyType::Int);
        stats.record(PyType::Float);

        assert_eq!(stats.total(), 3);
        assert_eq!(stats.most_common(), (PyType::Int, 2));
    }

    #[test]
    fn test_predictor_basic() {
        let predictor = TypePredictor::with_settings(10, 0.9);
        let func_id = FunctionId(1);

        // Record 95 ints and 5 floats
        for _ in 0..95 {
            predictor.record(func_id, 0, PyType::Int);
        }
        for _ in 0..5 {
            predictor.record(func_id, 0, PyType::Float);
        }

        // Should predict Int with 95% confidence
        assert_eq!(predictor.predict(func_id, 0), Some(PyType::Int));
    }

    #[test]
    fn test_predictor_insufficient_data() {
        let predictor = TypePredictor::with_settings(100, 0.9);
        let func_id = FunctionId(1);

        // Only 50 observations
        for _ in 0..50 {
            predictor.record(func_id, 0, PyType::Int);
        }

        // Should not predict (not enough data)
        assert_eq!(predictor.predict(func_id, 0), None);
    }

    #[test]
    fn test_predictor_low_confidence() {
        let predictor = TypePredictor::with_settings(10, 0.9);
        let func_id = FunctionId(1);

        // 60% int, 40% float - below 90% threshold
        for _ in 0..60 {
            predictor.record(func_id, 0, PyType::Int);
        }
        for _ in 0..40 {
            predictor.record(func_id, 0, PyType::Float);
        }

        // Should not predict (low confidence)
        assert_eq!(predictor.predict(func_id, 0), None);
    }

    #[test]
    fn test_get_confidence() {
        let predictor = TypePredictor::with_settings(10, 0.9);
        let func_id = FunctionId(1);

        for _ in 0..75 {
            predictor.record(func_id, 0, PyType::Int);
        }
        for _ in 0..25 {
            predictor.record(func_id, 0, PyType::Float);
        }

        let int_conf = predictor.get_confidence(func_id, 0, PyType::Int);
        let float_conf = predictor.get_confidence(func_id, 0, PyType::Float);

        assert!((int_conf - 0.75).abs() < 0.01);
        assert!((float_conf - 0.25).abs() < 0.01);
    }
}
