//! DX-Py FFI - Memory Teleportation (Zero-Copy) FFI
//!
//! This crate implements zero-copy data sharing with C extensions like NumPy:
//! - Direct pointer sharing for array data
//! - SIMD operations on teleported arrays
//! - GIL-free execution for pure computation
//! - C-API compatibility layer
//! - Buffer protocol (PEP 3118)
//! - NumPy C API compatibility

pub mod api_registry;
pub mod capi;
pub mod cpython_compat;
pub mod fast_ffi;
pub mod numpy_compat;
pub mod pandas_compat;
pub mod teleport;

pub use capi::CApiCompat;
pub use cpython_compat::{
    buffer_flags,
    check_api_implemented,
    compare_ops,
    dx_values_equal,
    get_api_tracker,
    record_api_call,
    type_flags,
    AllowThreads,
    // API Tracking
    ApiCategory,
    ApiCoverageStats,
    ApiFunction,
    ApiPriority,
    ApiTracker,
    // Argument parsing
    ArgParseError,
    ArgParseResult,
    BridgeError,
    BridgeResult,
    // PyObject Bridge
    DxValue,
    GilGuard,
    GilState,
    MissingApiError,
    ParsedArg,
    PyArg_ParseTuple,
    PyArg_ParseTupleAndKeywords,
    PyBufferProcs,
    PyGILState_STATE,
    PyObject,
    PyObjectBridge,
    PyTypeObject,
    Py_BuildValue,
    Py_buffer,
};
pub use fast_ffi::FastFfi;
pub use numpy_compat::{
    array_flags_to_npy, dtype_to_typenum, npy_flags, npy_to_array_flags, npy_types,
    typenum_to_dtype, PyArrayObject, PyArray_DATA, PyArray_DIM, PyArray_DIMS, PyArray_EMPTY,
    PyArray_FLAGS, PyArray_Free, PyArray_ISCONTIGUOUS, PyArray_ISFORTRAN, PyArray_ISWRITEABLE,
    PyArray_ITEMSIZE, PyArray_NBYTES, PyArray_NDIM, PyArray_SIZE, PyArray_STRIDE, PyArray_STRIDES,
    PyArray_SimpleNew, PyArray_SimpleNewFromData, PyArray_ZEROS,
};
pub use teleport::{
    ArrayFlags, ArrayInterface, DType, StructField, StructuredDType, TeleportedArray,
};

// Pandas compatibility exports
pub use pandas_compat::{
    agg_count_f64, agg_max_f64, agg_max_i64, agg_mean_f64, agg_mean_i64, agg_min_f64, agg_min_i64,
    agg_std_f64, agg_sum_f64, agg_sum_i64, agg_var_f64, AggFunc, Block, BlockManager, CsvConfig,
    DateTimeIndex, GroupByResult, Index, Int64Index, JsonConfig, JsonOrient, MeltConfig,
    MergeConfig, MergeHow, MultiIndex, ObjectIndex, PandasDType, ParquetCompression, ParquetConfig,
    ParquetEngine, PivotConfig, RangeIndex,
};

// API Registry exports
pub use api_registry::{
    get_api_registry, ApiFunctionDef, ApiRegistry, CategoryStats, CoverageHistory,
    CoverageSnapshot, CoverageTrend, RegistryCoverageStats,
};
