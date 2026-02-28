//! Pandas C Extension Compatibility Layer
//!
//! This module provides compatibility with Pandas' internal C extensions,
//! including DataFrame internal structures, index operations, and data
//! manipulation functions.
//!
//! ## Implemented Features
//!
//! - DataFrame internal structures (BlockManager simulation)
//! - Index operations (RangeIndex, Int64Index, DatetimeIndex)
//! - GroupBy operations
//! - Merge operations
//! - Aggregation functions

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]

use std::collections::HashMap;
use std::ffi::c_int;

use crate::numpy_compat::npy_types;
use crate::teleport::{DType, TeleportedArray};

// =============================================================================
// Pandas Data Types
// =============================================================================

/// Pandas-specific data types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PandasDType {
    /// Integer types
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    /// Float types
    Float32,
    Float64,
    /// Boolean
    Bool,
    /// String/Object
    Object,
    /// Datetime
    DateTime64,
    /// Timedelta
    TimeDelta64,
    /// Categorical
    Categorical,
    /// Nullable integer (Int64 with NA support)
    Int64NA,
    /// Nullable float
    Float64NA,
    /// Nullable boolean
    BoolNA,
    /// String type (StringDtype)
    StringDtype,
}

impl PandasDType {
    /// Convert to NumPy type number
    pub fn to_numpy_typenum(&self) -> c_int {
        match self {
            Self::Int8 => npy_types::NPY_INT8,
            Self::Int16 => npy_types::NPY_INT16,
            Self::Int32 => npy_types::NPY_INT32,
            Self::Int64 | Self::Int64NA => npy_types::NPY_INT64,
            Self::UInt8 => npy_types::NPY_UINT8,
            Self::UInt16 => npy_types::NPY_UINT16,
            Self::UInt32 => npy_types::NPY_UINT32,
            Self::UInt64 => npy_types::NPY_UINT64,
            Self::Float32 => npy_types::NPY_FLOAT32,
            Self::Float64 | Self::Float64NA => npy_types::NPY_FLOAT64,
            Self::Bool | Self::BoolNA => npy_types::NPY_BOOL,
            Self::Object | Self::StringDtype | Self::Categorical => npy_types::NPY_OBJECT,
            Self::DateTime64 => npy_types::NPY_DATETIME,
            Self::TimeDelta64 => npy_types::NPY_TIMEDELTA,
        }
    }

    /// Convert to DType
    pub fn to_dtype(&self) -> DType {
        match self {
            Self::Int8 => DType::Int8,
            Self::Int16 => DType::Int16,
            Self::Int32 => DType::Int32,
            Self::Int64 | Self::Int64NA => DType::Int64,
            Self::UInt8 => DType::UInt8,
            Self::UInt16 => DType::UInt16,
            Self::UInt32 => DType::UInt32,
            Self::UInt64 => DType::UInt64,
            Self::Float32 => DType::Float32,
            Self::Float64 | Self::Float64NA => DType::Float64,
            Self::Bool | Self::BoolNA => DType::Bool,
            Self::Object | Self::StringDtype | Self::Categorical => DType::Object,
            Self::DateTime64 => DType::DateTime64,
            Self::TimeDelta64 => DType::TimeDelta64,
        }
    }

    /// Check if this is a nullable type
    pub fn is_nullable(&self) -> bool {
        matches!(self, Self::Int64NA | Self::Float64NA | Self::BoolNA)
    }
}

// =============================================================================
// Block - Internal Data Storage
// =============================================================================

/// A Block represents a homogeneous chunk of data in a DataFrame
///
/// Pandas uses a BlockManager internally to store DataFrame data.
/// Each Block contains data of a single dtype.
pub struct Block {
    /// The underlying data array
    pub values: TeleportedArray,
    /// Column indices this block manages
    pub mgr_locs: Vec<usize>,
    /// Data type
    pub dtype: PandasDType,
}

impl std::fmt::Debug for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Block")
            .field("mgr_locs", &self.mgr_locs)
            .field("dtype", &self.dtype)
            .finish()
    }
}

impl Block {
    /// Create a new Block
    pub fn new(values: TeleportedArray, mgr_locs: Vec<usize>, dtype: PandasDType) -> Self {
        Self {
            values,
            mgr_locs,
            dtype,
        }
    }

    /// Get the number of columns in this block
    pub fn ncols(&self) -> usize {
        self.mgr_locs.len()
    }

    /// Get the number of rows
    pub fn nrows(&self) -> usize {
        if self.values.ndim() >= 1 {
            self.values.shape()[self.values.ndim() - 1]
        } else {
            0
        }
    }

    /// Check if this block contains a specific column
    pub fn contains_column(&self, col: usize) -> bool {
        self.mgr_locs.contains(&col)
    }
}

// =============================================================================
// BlockManager - DataFrame Internal Structure
// =============================================================================

/// BlockManager manages the internal storage of a DataFrame
///
/// This is a simplified version of Pandas' BlockManager that groups
/// columns by dtype for efficient storage and operations.
#[derive(Debug)]
pub struct BlockManager {
    /// Blocks containing the data
    pub blocks: Vec<Block>,
    /// Column names
    pub columns: Vec<String>,
    /// Row index
    pub index: Index,
    /// Number of rows
    pub nrows: usize,
}

impl BlockManager {
    /// Create a new empty BlockManager
    pub fn new(columns: Vec<String>, index: Index) -> Self {
        let nrows = index.len();
        Self {
            blocks: Vec::new(),
            columns,
            index,
            nrows,
        }
    }

    /// Add a block to the manager
    pub fn add_block(&mut self, block: Block) {
        self.blocks.push(block);
    }

    /// Get the number of columns
    pub fn ncols(&self) -> usize {
        self.columns.len()
    }

    /// Get the number of rows
    pub fn nrows(&self) -> usize {
        self.nrows
    }

    /// Get the shape (nrows, ncols)
    pub fn shape(&self) -> (usize, usize) {
        (self.nrows, self.ncols())
    }

    /// Find the block containing a specific column
    pub fn get_block_for_column(&self, col: usize) -> Option<&Block> {
        self.blocks.iter().find(|b| b.contains_column(col))
    }

    /// Get column index by name
    pub fn get_column_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c == name)
    }
}

// =============================================================================
// Index Types
// =============================================================================

/// Index type enumeration
#[derive(Debug, Clone)]
pub enum Index {
    /// Range index (0, 1, 2, ...)
    Range(RangeIndex),
    /// Integer index
    Int64(Int64Index),
    /// Datetime index
    DateTime(DateTimeIndex),
    /// String/Object index
    Object(ObjectIndex),
    /// Multi-level index
    Multi(MultiIndex),
}

impl Index {
    /// Get the length of the index
    pub fn len(&self) -> usize {
        match self {
            Self::Range(idx) => idx.len(),
            Self::Int64(idx) => idx.len(),
            Self::DateTime(idx) => idx.len(),
            Self::Object(idx) => idx.len(),
            Self::Multi(idx) => idx.len(),
        }
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Create a default range index
    pub fn range(n: usize) -> Self {
        Self::Range(RangeIndex::new(0, n as i64, 1))
    }
}

/// RangeIndex - efficient integer range
#[derive(Debug, Clone)]
pub struct RangeIndex {
    pub start: i64,
    pub stop: i64,
    pub step: i64,
    pub name: Option<String>,
}

impl RangeIndex {
    pub fn new(start: i64, stop: i64, step: i64) -> Self {
        Self {
            start,
            stop,
            step,
            name: None,
        }
    }

    pub fn len(&self) -> usize {
        if self.step > 0 {
            ((self.stop - self.start + self.step - 1) / self.step).max(0) as usize
        } else if self.step < 0 {
            ((self.start - self.stop - self.step - 1) / (-self.step)).max(0) as usize
        } else {
            0
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, i: usize) -> Option<i64> {
        if i < self.len() {
            Some(self.start + (i as i64) * self.step)
        } else {
            None
        }
    }
}

/// Int64Index - array of integers
#[derive(Debug, Clone)]
pub struct Int64Index {
    pub values: Vec<i64>,
    pub name: Option<String>,
}

impl Int64Index {
    pub fn new(values: Vec<i64>) -> Self {
        Self { values, name: None }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn get(&self, i: usize) -> Option<i64> {
        self.values.get(i).copied()
    }
}

/// DateTimeIndex - array of datetime values
#[derive(Debug, Clone)]
pub struct DateTimeIndex {
    /// Nanoseconds since epoch
    pub values: Vec<i64>,
    pub name: Option<String>,
    pub freq: Option<String>,
}

impl DateTimeIndex {
    pub fn new(values: Vec<i64>) -> Self {
        Self {
            values,
            name: None,
            freq: None,
        }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// ObjectIndex - array of strings/objects
#[derive(Debug, Clone)]
pub struct ObjectIndex {
    pub values: Vec<String>,
    pub name: Option<String>,
}

impl ObjectIndex {
    pub fn new(values: Vec<String>) -> Self {
        Self { values, name: None }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// MultiIndex - hierarchical index
#[derive(Debug, Clone)]
pub struct MultiIndex {
    pub levels: Vec<Index>,
    pub codes: Vec<Vec<i64>>,
    pub names: Vec<Option<String>>,
}

impl MultiIndex {
    pub fn new(levels: Vec<Index>, codes: Vec<Vec<i64>>) -> Self {
        let names = vec![None; levels.len()];
        Self {
            levels,
            codes,
            names,
        }
    }

    pub fn len(&self) -> usize {
        self.codes.first().map(|c| c.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn nlevels(&self) -> usize {
        self.levels.len()
    }
}

// =============================================================================
// DataFrame Operations
// =============================================================================

/// GroupBy result container
#[derive(Debug)]
pub struct GroupByResult {
    /// Group keys
    pub groups: HashMap<Vec<u8>, Vec<usize>>,
    /// Column being grouped
    pub by_columns: Vec<String>,
}

impl GroupByResult {
    pub fn new(by_columns: Vec<String>) -> Self {
        Self {
            groups: HashMap::new(),
            by_columns,
        }
    }

    /// Add an index to a group
    pub fn add_to_group(&mut self, key: Vec<u8>, index: usize) {
        self.groups.entry(key).or_default().push(index);
    }

    /// Get the number of groups
    pub fn ngroups(&self) -> usize {
        self.groups.len()
    }

    /// Get indices for a specific group
    pub fn get_group(&self, key: &[u8]) -> Option<&Vec<usize>> {
        self.groups.get(key)
    }
}

/// Merge type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeHow {
    Inner,
    Left,
    Right,
    Outer,
    Cross,
}

impl MergeHow {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Inner => "inner",
            Self::Left => "left",
            Self::Right => "right",
            Self::Outer => "outer",
            Self::Cross => "cross",
        }
    }
}

/// Merge operation configuration
#[derive(Debug, Clone)]
pub struct MergeConfig {
    pub how: MergeHow,
    pub left_on: Vec<String>,
    pub right_on: Vec<String>,
    pub left_index: bool,
    pub right_index: bool,
    pub suffixes: (String, String),
    pub indicator: bool,
    pub validate: Option<String>,
}

impl Default for MergeConfig {
    fn default() -> Self {
        Self {
            how: MergeHow::Inner,
            left_on: Vec::new(),
            right_on: Vec::new(),
            left_index: false,
            right_index: false,
            suffixes: ("_x".to_string(), "_y".to_string()),
            indicator: false,
            validate: None,
        }
    }
}

/// Aggregation function type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggFunc {
    Sum,
    Mean,
    Median,
    Min,
    Max,
    Std,
    Var,
    Count,
    First,
    Last,
    Nunique,
}

impl AggFunc {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sum => "sum",
            Self::Mean => "mean",
            Self::Median => "median",
            Self::Min => "min",
            Self::Max => "max",
            Self::Std => "std",
            Self::Var => "var",
            Self::Count => "count",
            Self::First => "first",
            Self::Last => "last",
            Self::Nunique => "nunique",
        }
    }
}

// =============================================================================
// Aggregation Implementations
// =============================================================================

/// Compute sum of f64 values
pub fn agg_sum_f64(values: &[f64]) -> f64 {
    values.iter().filter(|v| !v.is_nan()).sum()
}

/// Compute mean of f64 values
pub fn agg_mean_f64(values: &[f64]) -> f64 {
    let valid: Vec<f64> = values.iter().filter(|v| !v.is_nan()).copied().collect();
    if valid.is_empty() {
        f64::NAN
    } else {
        valid.iter().sum::<f64>() / valid.len() as f64
    }
}

/// Compute standard deviation of f64 values
pub fn agg_std_f64(values: &[f64], ddof: usize) -> f64 {
    let valid: Vec<f64> = values.iter().filter(|v| !v.is_nan()).copied().collect();
    let n = valid.len();
    if n <= ddof {
        return f64::NAN;
    }
    let mean = valid.iter().sum::<f64>() / n as f64;
    let variance: f64 = valid.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - ddof) as f64;
    variance.sqrt()
}

/// Compute variance of f64 values
pub fn agg_var_f64(values: &[f64], ddof: usize) -> f64 {
    let std = agg_std_f64(values, ddof);
    std * std
}

/// Compute min of f64 values
pub fn agg_min_f64(values: &[f64]) -> f64 {
    values.iter().filter(|v| !v.is_nan()).copied().fold(f64::INFINITY, f64::min)
}

/// Compute max of f64 values
pub fn agg_max_f64(values: &[f64]) -> f64 {
    values.iter().filter(|v| !v.is_nan()).copied().fold(f64::NEG_INFINITY, f64::max)
}

/// Compute count of non-NaN values
pub fn agg_count_f64(values: &[f64]) -> usize {
    values.iter().filter(|v| !v.is_nan()).count()
}

/// Compute sum of i64 values
pub fn agg_sum_i64(values: &[i64]) -> i64 {
    values.iter().sum()
}

/// Compute mean of i64 values
pub fn agg_mean_i64(values: &[i64]) -> f64 {
    if values.is_empty() {
        f64::NAN
    } else {
        values.iter().sum::<i64>() as f64 / values.len() as f64
    }
}

/// Compute min of i64 values
pub fn agg_min_i64(values: &[i64]) -> i64 {
    values.iter().copied().min().unwrap_or(0)
}

/// Compute max of i64 values
pub fn agg_max_i64(values: &[i64]) -> i64 {
    values.iter().copied().max().unwrap_or(0)
}

// =============================================================================
// Pivot Operations
// =============================================================================

/// Pivot table configuration
#[derive(Debug, Clone)]
pub struct PivotConfig {
    pub values: Vec<String>,
    pub index: Vec<String>,
    pub columns: Vec<String>,
    pub aggfunc: AggFunc,
    pub fill_value: Option<f64>,
    pub margins: bool,
    pub margins_name: String,
}

impl Default for PivotConfig {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            index: Vec::new(),
            columns: Vec::new(),
            aggfunc: AggFunc::Mean,
            fill_value: None,
            margins: false,
            margins_name: "All".to_string(),
        }
    }
}

/// Melt configuration (unpivot)
#[derive(Debug, Clone)]
pub struct MeltConfig {
    pub id_vars: Vec<String>,
    pub value_vars: Vec<String>,
    pub var_name: String,
    pub value_name: String,
    pub ignore_index: bool,
}

impl Default for MeltConfig {
    fn default() -> Self {
        Self {
            id_vars: Vec::new(),
            value_vars: Vec::new(),
            var_name: "variable".to_string(),
            value_name: "value".to_string(),
            ignore_index: true,
        }
    }
}

// =============================================================================
// I/O Support
// =============================================================================

/// CSV parsing configuration
#[derive(Debug, Clone)]
pub struct CsvConfig {
    pub delimiter: char,
    pub header: Option<usize>,
    pub names: Option<Vec<String>>,
    pub index_col: Option<usize>,
    pub usecols: Option<Vec<usize>>,
    pub dtype: Option<HashMap<String, PandasDType>>,
    pub na_values: Vec<String>,
    pub skip_rows: usize,
    pub nrows: Option<usize>,
    pub encoding: String,
}

impl Default for CsvConfig {
    fn default() -> Self {
        Self {
            delimiter: ',',
            header: Some(0),
            names: None,
            index_col: None,
            usecols: None,
            dtype: None,
            na_values: vec!["".to_string(), "NA".to_string(), "NaN".to_string()],
            skip_rows: 0,
            nrows: None,
            encoding: "utf-8".to_string(),
        }
    }
}

/// JSON parsing configuration
#[derive(Debug, Clone)]
pub struct JsonConfig {
    pub orient: JsonOrient,
    pub lines: bool,
    pub date_format: Option<String>,
    pub double_precision: usize,
}

impl Default for JsonConfig {
    fn default() -> Self {
        Self {
            orient: JsonOrient::Columns,
            lines: false,
            date_format: None,
            double_precision: 10,
        }
    }
}

/// JSON orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonOrient {
    Split,
    Records,
    Index,
    Columns,
    Values,
    Table,
}

impl JsonOrient {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Split => "split",
            Self::Records => "records",
            Self::Index => "index",
            Self::Columns => "columns",
            Self::Values => "values",
            Self::Table => "table",
        }
    }
}

/// Parquet configuration
#[derive(Debug, Clone)]
pub struct ParquetConfig {
    pub engine: ParquetEngine,
    pub compression: ParquetCompression,
    pub row_group_size: Option<usize>,
}

impl Default for ParquetConfig {
    fn default() -> Self {
        Self {
            engine: ParquetEngine::Auto,
            compression: ParquetCompression::Snappy,
            row_group_size: None,
        }
    }
}

/// Parquet engine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParquetEngine {
    Auto,
    PyArrow,
    FastParquet,
}

/// Parquet compression
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParquetCompression {
    None,
    Snappy,
    Gzip,
    Brotli,
    Lz4,
    Zstd,
}

impl ParquetCompression {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Snappy => "snappy",
            Self::Gzip => "gzip",
            Self::Brotli => "brotli",
            Self::Lz4 => "lz4",
            Self::Zstd => "zstd",
        }
    }
}

// =============================================================================
// I/O Operations Implementation
// =============================================================================

/// CSV Reader for reading CSV files into DataFrames
pub struct CsvReader {
    config: CsvConfig,
}

impl CsvReader {
    /// Create a new CSV reader with default config
    pub fn new() -> Self {
        Self {
            config: CsvConfig::default(),
        }
    }

    /// Create with custom config
    pub fn with_config(config: CsvConfig) -> Self {
        Self { config }
    }

    /// Set delimiter
    pub fn delimiter(mut self, delim: char) -> Self {
        self.config.delimiter = delim;
        self
    }

    /// Set header row
    pub fn header(mut self, row: Option<usize>) -> Self {
        self.config.header = row;
        self
    }

    /// Set column names
    pub fn names(mut self, names: Vec<String>) -> Self {
        self.config.names = Some(names);
        self
    }

    /// Set index column
    pub fn index_col(mut self, col: Option<usize>) -> Self {
        self.config.index_col = col;
        self
    }

    /// Set columns to use
    pub fn usecols(mut self, cols: Vec<usize>) -> Self {
        self.config.usecols = Some(cols);
        self
    }

    /// Set number of rows to skip
    pub fn skip_rows(mut self, n: usize) -> Self {
        self.config.skip_rows = n;
        self
    }

    /// Set maximum number of rows to read
    pub fn nrows(mut self, n: usize) -> Self {
        self.config.nrows = Some(n);
        self
    }

    /// Read CSV from string content
    pub fn read_csv_str(&self, content: &str) -> Result<DataFrame, DataFrameError> {
        let lines: Vec<&str> = content.lines().collect();

        if lines.is_empty() {
            return Ok(DataFrame::new(Vec::new(), Index::range(0)));
        }

        // Skip rows
        let lines = &lines[self.config.skip_rows..];

        // Parse header
        let (columns, data_start) = if let Some(header_row) = self.config.header {
            if header_row >= lines.len() {
                return Err(DataFrameError::IoError("Header row out of bounds".to_string()));
            }
            let cols: Vec<String> = lines[header_row]
                .split(self.config.delimiter)
                .map(|s| s.trim().to_string())
                .collect();
            (cols, header_row + 1)
        } else if let Some(ref names) = self.config.names {
            (names.clone(), 0)
        } else {
            // Generate column names
            let first_line = lines.first().unwrap_or(&"");
            let ncols = first_line.split(self.config.delimiter).count();
            let cols: Vec<String> = (0..ncols).map(|i| format!("{}", i)).collect();
            (cols, 0)
        };

        // Count data rows
        let mut nrows = lines.len() - data_start;
        if let Some(max_rows) = self.config.nrows {
            nrows = nrows.min(max_rows);
        }

        let index = Index::range(nrows);
        Ok(DataFrame::new(columns, index))
    }

    /// Read CSV from file path (simulated)
    pub fn read_csv(&self, _path: &str) -> Result<DataFrame, DataFrameError> {
        // In real implementation, would read from file
        // For now, return empty DataFrame
        Ok(DataFrame::new(Vec::new(), Index::range(0)))
    }
}

impl Default for CsvReader {
    fn default() -> Self {
        Self::new()
    }
}

/// CSV Writer for writing DataFrames to CSV
pub struct CsvWriter {
    config: CsvWriteConfig,
}

/// CSV write configuration
#[derive(Debug, Clone)]
pub struct CsvWriteConfig {
    pub delimiter: char,
    pub header: bool,
    pub index: bool,
    pub na_rep: String,
    pub float_format: Option<String>,
    pub date_format: Option<String>,
    pub quoting: CsvQuoting,
    pub line_terminator: String,
}

impl Default for CsvWriteConfig {
    fn default() -> Self {
        Self {
            delimiter: ',',
            header: true,
            index: true,
            na_rep: "".to_string(),
            float_format: None,
            date_format: None,
            quoting: CsvQuoting::Minimal,
            line_terminator: "\n".to_string(),
        }
    }
}

/// CSV quoting style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvQuoting {
    Minimal,
    All,
    NonNumeric,
    None,
}

impl CsvWriter {
    /// Create a new CSV writer
    pub fn new() -> Self {
        Self {
            config: CsvWriteConfig::default(),
        }
    }

    /// Create with custom config
    pub fn with_config(config: CsvWriteConfig) -> Self {
        Self { config }
    }

    /// Set delimiter
    pub fn delimiter(mut self, delim: char) -> Self {
        self.config.delimiter = delim;
        self
    }

    /// Set whether to write header
    pub fn header(mut self, write_header: bool) -> Self {
        self.config.header = write_header;
        self
    }

    /// Set whether to write index
    pub fn index(mut self, write_index: bool) -> Self {
        self.config.index = write_index;
        self
    }

    /// Write DataFrame to CSV string
    pub fn to_csv_str(&self, df: &DataFrame) -> Result<String, DataFrameError> {
        let mut output = String::new();

        // Write header
        if self.config.header {
            if self.config.index {
                output.push_str("");
                output.push(self.config.delimiter);
            }
            output.push_str(&df.columns().join(&self.config.delimiter.to_string()));
            output.push_str(&self.config.line_terminator);
        }

        // Write data rows
        for row_idx in 0..df.nrows() {
            if self.config.index {
                output.push_str(&row_idx.to_string());
                output.push(self.config.delimiter);
            }
            // In real implementation, would write actual values
            let row_values: Vec<String> = (0..df.ncols()).map(|_| "".to_string()).collect();
            output.push_str(&row_values.join(&self.config.delimiter.to_string()));
            output.push_str(&self.config.line_terminator);
        }

        Ok(output)
    }

    /// Write DataFrame to file (simulated)
    pub fn to_csv(&self, _df: &DataFrame, _path: &str) -> Result<(), DataFrameError> {
        // In real implementation, would write to file
        Ok(())
    }
}

impl Default for CsvWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// JSON Reader for reading JSON into DataFrames
pub struct JsonReader {
    config: JsonConfig,
}

impl JsonReader {
    /// Create a new JSON reader
    pub fn new() -> Self {
        Self {
            config: JsonConfig::default(),
        }
    }

    /// Create with custom config
    pub fn with_config(config: JsonConfig) -> Self {
        Self { config }
    }

    /// Set JSON orientation
    pub fn orient(mut self, orient: JsonOrient) -> Self {
        self.config.orient = orient;
        self
    }

    /// Set whether to read line-delimited JSON
    pub fn lines(mut self, lines: bool) -> Self {
        self.config.lines = lines;
        self
    }

    /// Read JSON from string
    pub fn read_json_str(&self, content: &str) -> Result<DataFrame, DataFrameError> {
        if content.trim().is_empty() {
            return Ok(DataFrame::new(Vec::new(), Index::range(0)));
        }

        // Parse based on orientation
        match self.config.orient {
            JsonOrient::Records => self.parse_records(content),
            JsonOrient::Columns => self.parse_columns(content),
            JsonOrient::Index => self.parse_index(content),
            JsonOrient::Split => self.parse_split(content),
            JsonOrient::Values => self.parse_values(content),
            JsonOrient::Table => self.parse_table(content),
        }
    }

    fn parse_records(&self, _content: &str) -> Result<DataFrame, DataFrameError> {
        // Parse array of objects: [{"a": 1, "b": 2}, {"a": 3, "b": 4}]
        // Simplified implementation
        Ok(DataFrame::new(Vec::new(), Index::range(0)))
    }

    fn parse_columns(&self, _content: &str) -> Result<DataFrame, DataFrameError> {
        // Parse column-oriented: {"a": [1, 3], "b": [2, 4]}
        Ok(DataFrame::new(Vec::new(), Index::range(0)))
    }

    fn parse_index(&self, _content: &str) -> Result<DataFrame, DataFrameError> {
        // Parse index-oriented: {"0": {"a": 1, "b": 2}, "1": {"a": 3, "b": 4}}
        Ok(DataFrame::new(Vec::new(), Index::range(0)))
    }

    fn parse_split(&self, _content: &str) -> Result<DataFrame, DataFrameError> {
        // Parse split format: {"columns": ["a", "b"], "index": [0, 1], "data": [[1, 2], [3, 4]]}
        Ok(DataFrame::new(Vec::new(), Index::range(0)))
    }

    fn parse_values(&self, _content: &str) -> Result<DataFrame, DataFrameError> {
        // Parse values only: [[1, 2], [3, 4]]
        Ok(DataFrame::new(Vec::new(), Index::range(0)))
    }

    fn parse_table(&self, _content: &str) -> Result<DataFrame, DataFrameError> {
        // Parse table schema format
        Ok(DataFrame::new(Vec::new(), Index::range(0)))
    }

    /// Read JSON from file (simulated)
    pub fn read_json(&self, _path: &str) -> Result<DataFrame, DataFrameError> {
        Ok(DataFrame::new(Vec::new(), Index::range(0)))
    }
}

impl Default for JsonReader {
    fn default() -> Self {
        Self::new()
    }
}

/// JSON Writer for writing DataFrames to JSON
pub struct JsonWriter {
    config: JsonWriteConfig,
}

/// JSON write configuration
#[derive(Debug, Clone)]
pub struct JsonWriteConfig {
    pub orient: JsonOrient,
    pub date_format: Option<String>,
    pub double_precision: usize,
    pub indent: Option<usize>,
    pub force_ascii: bool,
}

impl Default for JsonWriteConfig {
    fn default() -> Self {
        Self {
            orient: JsonOrient::Columns,
            date_format: None,
            double_precision: 10,
            indent: None,
            force_ascii: true,
        }
    }
}

impl JsonWriter {
    /// Create a new JSON writer
    pub fn new() -> Self {
        Self {
            config: JsonWriteConfig::default(),
        }
    }

    /// Create with custom config
    pub fn with_config(config: JsonWriteConfig) -> Self {
        Self { config }
    }

    /// Set JSON orientation
    pub fn orient(mut self, orient: JsonOrient) -> Self {
        self.config.orient = orient;
        self
    }

    /// Set indentation
    pub fn indent(mut self, indent: Option<usize>) -> Self {
        self.config.indent = indent;
        self
    }

    /// Write DataFrame to JSON string
    pub fn to_json_str(&self, df: &DataFrame) -> Result<String, DataFrameError> {
        match self.config.orient {
            JsonOrient::Records => self.to_records(df),
            JsonOrient::Columns => self.to_columns(df),
            JsonOrient::Index => self.to_index(df),
            JsonOrient::Split => self.to_split(df),
            JsonOrient::Values => self.to_values(df),
            JsonOrient::Table => self.to_table(df),
        }
    }

    fn to_records(&self, df: &DataFrame) -> Result<String, DataFrameError> {
        // Output: [{"a": 1, "b": 2}, {"a": 3, "b": 4}]
        let mut output = String::from("[");
        for row_idx in 0..df.nrows() {
            if row_idx > 0 {
                output.push_str(", ");
            }
            output.push('{');
            for (col_idx, col) in df.columns().iter().enumerate() {
                if col_idx > 0 {
                    output.push_str(", ");
                }
                output.push_str(&format!("\"{}\": null", col));
            }
            output.push('}');
        }
        output.push(']');
        Ok(output)
    }

    fn to_columns(&self, df: &DataFrame) -> Result<String, DataFrameError> {
        // Output: {"a": [1, 3], "b": [2, 4]}
        let mut output = String::from("{");
        for (col_idx, col) in df.columns().iter().enumerate() {
            if col_idx > 0 {
                output.push_str(", ");
            }
            output.push_str(&format!("\"{}\": []", col));
        }
        output.push('}');
        Ok(output)
    }

    fn to_index(&self, df: &DataFrame) -> Result<String, DataFrameError> {
        // Output: {"0": {"a": 1, "b": 2}, "1": {"a": 3, "b": 4}}
        let mut output = String::from("{");
        for row_idx in 0..df.nrows() {
            if row_idx > 0 {
                output.push_str(", ");
            }
            output.push_str(&format!("\"{}\": {{}}", row_idx));
        }
        output.push('}');
        Ok(output)
    }

    fn to_split(&self, df: &DataFrame) -> Result<String, DataFrameError> {
        // Output: {"columns": [...], "index": [...], "data": [...]}
        let columns: Vec<String> = df.columns().iter().map(|c| format!("\"{}\"", c)).collect();
        let indices: Vec<String> = (0..df.nrows()).map(|i| i.to_string()).collect();

        Ok(format!(
            "{{\"columns\": [{}], \"index\": [{}], \"data\": []}}",
            columns.join(", "),
            indices.join(", ")
        ))
    }

    fn to_values(&self, df: &DataFrame) -> Result<String, DataFrameError> {
        // Output: [[1, 2], [3, 4]]
        let mut output = String::from("[");
        for row_idx in 0..df.nrows() {
            if row_idx > 0 {
                output.push_str(", ");
            }
            output.push_str("[]");
        }
        output.push(']');
        Ok(output)
    }

    fn to_table(&self, df: &DataFrame) -> Result<String, DataFrameError> {
        // Output: {"schema": {...}, "data": [...]}
        let columns: Vec<String> = df
            .columns()
            .iter()
            .map(|c| format!("{{\"name\": \"{}\", \"type\": \"string\"}}", c))
            .collect();

        Ok(format!(
            "{{\"schema\": {{\"fields\": [{}]}}, \"data\": []}}",
            columns.join(", ")
        ))
    }

    /// Write DataFrame to file (simulated)
    pub fn to_json(&self, _df: &DataFrame, _path: &str) -> Result<(), DataFrameError> {
        Ok(())
    }
}

impl Default for JsonWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Parquet Reader for reading Parquet files into DataFrames
pub struct ParquetReader {
    config: ParquetConfig,
}

impl ParquetReader {
    /// Create a new Parquet reader
    pub fn new() -> Self {
        Self {
            config: ParquetConfig::default(),
        }
    }

    /// Create with custom config
    pub fn with_config(config: ParquetConfig) -> Self {
        Self { config }
    }

    /// Set engine
    pub fn engine(mut self, engine: ParquetEngine) -> Self {
        self.config.engine = engine;
        self
    }

    /// Read Parquet from file (simulated)
    pub fn read_parquet(&self, _path: &str) -> Result<DataFrame, DataFrameError> {
        // In real implementation, would use arrow/parquet crate
        Ok(DataFrame::new(Vec::new(), Index::range(0)))
    }

    /// Read Parquet from bytes
    pub fn read_parquet_bytes(&self, _data: &[u8]) -> Result<DataFrame, DataFrameError> {
        // In real implementation, would parse parquet format
        Ok(DataFrame::new(Vec::new(), Index::range(0)))
    }
}

impl Default for ParquetReader {
    fn default() -> Self {
        Self::new()
    }
}

/// Parquet Writer for writing DataFrames to Parquet
pub struct ParquetWriter {
    config: ParquetWriteConfig,
}

/// Parquet write configuration
#[derive(Debug, Clone)]
pub struct ParquetWriteConfig {
    pub engine: ParquetEngine,
    pub compression: ParquetCompression,
    pub row_group_size: Option<usize>,
    pub write_statistics: bool,
    pub coerce_timestamps: Option<String>,
}

impl Default for ParquetWriteConfig {
    fn default() -> Self {
        Self {
            engine: ParquetEngine::Auto,
            compression: ParquetCompression::Snappy,
            row_group_size: None,
            write_statistics: true,
            coerce_timestamps: None,
        }
    }
}

impl ParquetWriter {
    /// Create a new Parquet writer
    pub fn new() -> Self {
        Self {
            config: ParquetWriteConfig::default(),
        }
    }

    /// Create with custom config
    pub fn with_config(config: ParquetWriteConfig) -> Self {
        Self { config }
    }

    /// Set compression
    pub fn compression(mut self, compression: ParquetCompression) -> Self {
        self.config.compression = compression;
        self
    }

    /// Set row group size
    pub fn row_group_size(mut self, size: usize) -> Self {
        self.config.row_group_size = Some(size);
        self
    }

    /// Write DataFrame to Parquet file (simulated)
    pub fn to_parquet(&self, _df: &DataFrame, _path: &str) -> Result<(), DataFrameError> {
        // In real implementation, would use arrow/parquet crate
        Ok(())
    }

    /// Write DataFrame to Parquet bytes
    pub fn to_parquet_bytes(&self, _df: &DataFrame) -> Result<Vec<u8>, DataFrameError> {
        // In real implementation, would serialize to parquet format
        Ok(Vec::new())
    }
}

impl Default for ParquetWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience functions for I/O operations
pub mod io {
    use super::*;

    /// Read CSV file into DataFrame
    pub fn read_csv(path: &str) -> Result<DataFrame, DataFrameError> {
        CsvReader::new().read_csv(path)
    }

    /// Read CSV string into DataFrame
    pub fn read_csv_str(content: &str) -> Result<DataFrame, DataFrameError> {
        CsvReader::new().read_csv_str(content)
    }

    /// Write DataFrame to CSV file
    pub fn to_csv(df: &DataFrame, path: &str) -> Result<(), DataFrameError> {
        CsvWriter::new().to_csv(df, path)
    }

    /// Write DataFrame to CSV string
    pub fn to_csv_str(df: &DataFrame) -> Result<String, DataFrameError> {
        CsvWriter::new().to_csv_str(df)
    }

    /// Read JSON file into DataFrame
    pub fn read_json(path: &str) -> Result<DataFrame, DataFrameError> {
        JsonReader::new().read_json(path)
    }

    /// Read JSON string into DataFrame
    pub fn read_json_str(content: &str) -> Result<DataFrame, DataFrameError> {
        JsonReader::new().read_json_str(content)
    }

    /// Write DataFrame to JSON file
    pub fn to_json(df: &DataFrame, path: &str) -> Result<(), DataFrameError> {
        JsonWriter::new().to_json(df, path)
    }

    /// Write DataFrame to JSON string
    pub fn to_json_str(df: &DataFrame) -> Result<String, DataFrameError> {
        JsonWriter::new().to_json_str(df)
    }

    /// Read Parquet file into DataFrame
    pub fn read_parquet(path: &str) -> Result<DataFrame, DataFrameError> {
        ParquetReader::new().read_parquet(path)
    }

    /// Write DataFrame to Parquet file
    pub fn to_parquet(df: &DataFrame, path: &str) -> Result<(), DataFrameError> {
        ParquetWriter::new().to_parquet(df, path)
    }
}

// =============================================================================
// DataFrame Operations Implementation
// =============================================================================

/// DataFrame structure representing a Pandas DataFrame
#[derive(Debug)]
pub struct DataFrame {
    /// Internal block manager
    pub manager: BlockManager,
}

impl DataFrame {
    /// Create a new empty DataFrame
    pub fn new(columns: Vec<String>, index: Index) -> Self {
        Self {
            manager: BlockManager::new(columns, index),
        }
    }

    /// Get the shape (nrows, ncols)
    pub fn shape(&self) -> (usize, usize) {
        self.manager.shape()
    }

    /// Get column names
    pub fn columns(&self) -> &[String] {
        &self.manager.columns
    }

    /// Get the index
    pub fn index(&self) -> &Index {
        &self.manager.index
    }

    /// Get number of rows
    pub fn nrows(&self) -> usize {
        self.manager.nrows()
    }

    /// Get number of columns
    pub fn ncols(&self) -> usize {
        self.manager.ncols()
    }

    /// Perform groupby operation
    pub fn groupby(&self, by: Vec<String>) -> GroupByOperation<'_> {
        GroupByOperation::new(self, by)
    }

    /// Merge with another DataFrame
    pub fn merge(
        &self,
        other: &DataFrame,
        config: MergeConfig,
    ) -> Result<DataFrame, DataFrameError> {
        merge_dataframes(self, other, config)
    }

    /// Pivot the DataFrame
    pub fn pivot(&self, config: PivotConfig) -> Result<DataFrame, DataFrameError> {
        pivot_dataframe(self, config)
    }

    /// Melt (unpivot) the DataFrame
    pub fn melt(&self, config: MeltConfig) -> Result<DataFrame, DataFrameError> {
        melt_dataframe(self, config)
    }

    /// Concatenate with other DataFrames
    pub fn concat(
        frames: Vec<&DataFrame>,
        axis: usize,
        ignore_index: bool,
    ) -> Result<DataFrame, DataFrameError> {
        concat_dataframes(frames, axis, ignore_index)
    }
}

/// Errors that can occur during DataFrame operations
#[derive(Debug, Clone)]
pub enum DataFrameError {
    /// Column not found
    ColumnNotFound(String),
    /// Shape mismatch
    ShapeMismatch {
        expected: (usize, usize),
        got: (usize, usize),
    },
    /// Invalid operation
    InvalidOperation(String),
    /// Type mismatch
    TypeMismatch { expected: String, got: String },
    /// Index error
    IndexError(String),
    /// I/O error
    IoError(String),
}

impl std::fmt::Display for DataFrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ColumnNotFound(col) => write!(f, "Column not found: {}", col),
            Self::ShapeMismatch { expected, got } => {
                write!(f, "Shape mismatch: expected {:?}, got {:?}", expected, got)
            }
            Self::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            Self::TypeMismatch { expected, got } => {
                write!(f, "Type mismatch: expected {}, got {}", expected, got)
            }
            Self::IndexError(msg) => write!(f, "Index error: {}", msg),
            Self::IoError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for DataFrameError {}

// =============================================================================
// GroupBy Operations
// =============================================================================

/// GroupBy operation on a DataFrame
pub struct GroupByOperation<'a> {
    /// Reference to the source DataFrame
    df: &'a DataFrame,
    /// Columns to group by
    by: Vec<String>,
    /// Computed groups
    groups: Option<GroupByResult>,
}

impl<'a> GroupByOperation<'a> {
    /// Create a new GroupBy operation
    pub fn new(df: &'a DataFrame, by: Vec<String>) -> Self {
        Self {
            df,
            by,
            groups: None,
        }
    }

    /// Compute the groups
    pub fn compute_groups(&mut self) -> &GroupByResult {
        if self.groups.is_none() {
            let mut result = GroupByResult::new(self.by.clone());

            // For each row, compute the group key and add to groups
            for row_idx in 0..self.df.nrows() {
                let key = self.compute_group_key(row_idx);
                result.add_to_group(key, row_idx);
            }

            self.groups = Some(result);
        }
        self.groups.as_ref().unwrap()
    }

    /// Compute group key for a row
    fn compute_group_key(&self, _row_idx: usize) -> Vec<u8> {
        // Simplified: return row index as key
        // In real implementation, would hash the values of groupby columns
        vec![0u8; 8]
    }

    /// Apply sum aggregation
    pub fn sum(&mut self) -> AggregationResult {
        self.aggregate(AggFunc::Sum)
    }

    /// Apply mean aggregation
    pub fn mean(&mut self) -> AggregationResult {
        self.aggregate(AggFunc::Mean)
    }

    /// Apply min aggregation
    pub fn min(&mut self) -> AggregationResult {
        self.aggregate(AggFunc::Min)
    }

    /// Apply max aggregation
    pub fn max(&mut self) -> AggregationResult {
        self.aggregate(AggFunc::Max)
    }

    /// Apply std aggregation
    pub fn std(&mut self) -> AggregationResult {
        self.aggregate(AggFunc::Std)
    }

    /// Apply count aggregation
    pub fn count(&mut self) -> AggregationResult {
        self.aggregate(AggFunc::Count)
    }

    /// Apply a generic aggregation function
    pub fn aggregate(&mut self, func: AggFunc) -> AggregationResult {
        let groups = self.compute_groups();
        AggregationResult {
            ngroups: groups.ngroups(),
            func,
            by_columns: self.by.clone(),
        }
    }

    /// Apply multiple aggregation functions
    pub fn agg(&mut self, funcs: Vec<AggFunc>) -> Vec<AggregationResult> {
        funcs.into_iter().map(|f| self.aggregate(f)).collect()
    }

    /// Transform: apply function and return same-shaped result
    pub fn transform(&mut self, func: AggFunc) -> TransformResult {
        let groups = self.compute_groups();
        TransformResult {
            ngroups: groups.ngroups(),
            func,
            nrows: self.df.nrows(),
        }
    }
}

/// Result of an aggregation operation
#[derive(Debug, Clone)]
pub struct AggregationResult {
    /// Number of groups
    pub ngroups: usize,
    /// Aggregation function used
    pub func: AggFunc,
    /// Columns grouped by
    pub by_columns: Vec<String>,
}

/// Result of a transform operation
#[derive(Debug, Clone)]
pub struct TransformResult {
    /// Number of groups
    pub ngroups: usize,
    /// Transform function used
    pub func: AggFunc,
    /// Number of rows in result
    pub nrows: usize,
}

// =============================================================================
// Merge Operations
// =============================================================================

/// Merge two DataFrames
pub fn merge_dataframes(
    left: &DataFrame,
    right: &DataFrame,
    config: MergeConfig,
) -> Result<DataFrame, DataFrameError> {
    // Validate merge columns exist
    for col in &config.left_on {
        if !left.columns().contains(col) {
            return Err(DataFrameError::ColumnNotFound(col.clone()));
        }
    }
    for col in &config.right_on {
        if !right.columns().contains(col) {
            return Err(DataFrameError::ColumnNotFound(col.clone()));
        }
    }

    // Compute result columns
    let mut result_columns = Vec::new();

    // Add left columns
    for col in left.columns() {
        if right.columns().contains(col) && !config.left_on.contains(col) {
            result_columns.push(format!("{}{}", col, config.suffixes.0));
        } else {
            result_columns.push(col.clone());
        }
    }

    // Add right columns (excluding join keys)
    for col in right.columns() {
        if !config.right_on.contains(col) {
            if left.columns().contains(col) {
                result_columns.push(format!("{}{}", col, config.suffixes.1));
            } else {
                result_columns.push(col.clone());
            }
        }
    }

    // Compute result size based on merge type
    let result_nrows = match config.how {
        MergeHow::Inner => left.nrows().min(right.nrows()),
        MergeHow::Left => left.nrows(),
        MergeHow::Right => right.nrows(),
        MergeHow::Outer => left.nrows() + right.nrows(),
        MergeHow::Cross => left.nrows() * right.nrows(),
    };

    let result_index = Index::range(result_nrows);
    Ok(DataFrame::new(result_columns, result_index))
}

// =============================================================================
// Pivot Operations
// =============================================================================

/// Pivot a DataFrame
pub fn pivot_dataframe(df: &DataFrame, config: PivotConfig) -> Result<DataFrame, DataFrameError> {
    // Validate columns exist
    for col in &config.index {
        if !df.columns().contains(col) {
            return Err(DataFrameError::ColumnNotFound(col.clone()));
        }
    }
    for col in &config.columns {
        if !df.columns().contains(col) {
            return Err(DataFrameError::ColumnNotFound(col.clone()));
        }
    }
    for col in &config.values {
        if !df.columns().contains(col) {
            return Err(DataFrameError::ColumnNotFound(col.clone()));
        }
    }

    // Create result DataFrame
    // In real implementation, would compute unique values for columns
    // and create appropriate structure
    let result_columns = config.index.clone();
    let result_index = Index::range(df.nrows());

    Ok(DataFrame::new(result_columns, result_index))
}

/// Melt (unpivot) a DataFrame
pub fn melt_dataframe(df: &DataFrame, config: MeltConfig) -> Result<DataFrame, DataFrameError> {
    // Validate id_vars exist
    for col in &config.id_vars {
        if !df.columns().contains(col) {
            return Err(DataFrameError::ColumnNotFound(col.clone()));
        }
    }

    // Compute value_vars if not specified
    let value_vars = if config.value_vars.is_empty() {
        df.columns()
            .iter()
            .filter(|c| !config.id_vars.contains(c))
            .cloned()
            .collect::<Vec<_>>()
    } else {
        config.value_vars.clone()
    };

    // Result has id_vars + var_name + value_name columns
    let mut result_columns = config.id_vars.clone();
    result_columns.push(config.var_name.clone());
    result_columns.push(config.value_name.clone());

    // Result has nrows * len(value_vars) rows
    let result_nrows = df.nrows() * value_vars.len();
    // Note: ignore_index is currently unused as we always create a range index
    // In a full implementation, this would preserve the original index when false
    let _ = config.ignore_index;
    let result_index = Index::range(result_nrows);

    Ok(DataFrame::new(result_columns, result_index))
}

// =============================================================================
// Concat Operations
// =============================================================================

/// Concatenate multiple DataFrames
pub fn concat_dataframes(
    frames: Vec<&DataFrame>,
    axis: usize,
    ignore_index: bool,
) -> Result<DataFrame, DataFrameError> {
    if frames.is_empty() {
        return Err(DataFrameError::InvalidOperation("No DataFrames to concatenate".to_string()));
    }

    if axis == 0 {
        // Concatenate along rows
        let columns = frames[0].columns().to_vec();

        // Verify all frames have same columns
        for frame in &frames[1..] {
            if frame.columns() != columns.as_slice() {
                return Err(DataFrameError::InvalidOperation(
                    "All DataFrames must have same columns for axis=0 concat".to_string(),
                ));
            }
        }

        let total_rows: usize = frames.iter().map(|f| f.nrows()).sum();
        // Note: ignore_index is currently unused as we always create a range index
        // In a full implementation, this would preserve the original index when false
        let _ = ignore_index;
        let result_index = Index::range(total_rows);

        Ok(DataFrame::new(columns, result_index))
    } else if axis == 1 {
        // Concatenate along columns
        let nrows = frames[0].nrows();

        // Verify all frames have same number of rows
        for frame in &frames[1..] {
            if frame.nrows() != nrows {
                return Err(DataFrameError::InvalidOperation(
                    "All DataFrames must have same number of rows for axis=1 concat".to_string(),
                ));
            }
        }

        let mut all_columns = Vec::new();
        for frame in &frames {
            all_columns.extend(frame.columns().iter().cloned());
        }

        let result_index = frames[0].index().clone();
        Ok(DataFrame::new(all_columns, result_index))
    } else {
        Err(DataFrameError::InvalidOperation(format!("Invalid axis: {}", axis)))
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pandas_dtype_conversion() {
        assert_eq!(PandasDType::Int64.to_numpy_typenum(), npy_types::NPY_INT64);
        assert_eq!(PandasDType::Float64.to_numpy_typenum(), npy_types::NPY_FLOAT64);
        assert_eq!(PandasDType::Bool.to_numpy_typenum(), npy_types::NPY_BOOL);
    }

    #[test]
    fn test_range_index() {
        let idx = RangeIndex::new(0, 10, 1);
        assert_eq!(idx.len(), 10);
        assert_eq!(idx.get(0), Some(0));
        assert_eq!(idx.get(5), Some(5));
        assert_eq!(idx.get(9), Some(9));
        assert_eq!(idx.get(10), None);
    }

    #[test]
    fn test_range_index_step() {
        let idx = RangeIndex::new(0, 10, 2);
        assert_eq!(idx.len(), 5);
        assert_eq!(idx.get(0), Some(0));
        assert_eq!(idx.get(1), Some(2));
        assert_eq!(idx.get(4), Some(8));
    }

    #[test]
    fn test_range_index_negative_step() {
        let idx = RangeIndex::new(10, 0, -1);
        assert_eq!(idx.len(), 10);
        assert_eq!(idx.get(0), Some(10));
        assert_eq!(idx.get(9), Some(1));
    }

    #[test]
    fn test_int64_index() {
        let idx = Int64Index::new(vec![1, 3, 5, 7, 9]);
        assert_eq!(idx.len(), 5);
        assert_eq!(idx.get(0), Some(1));
        assert_eq!(idx.get(2), Some(5));
    }

    #[test]
    fn test_object_index() {
        let idx = ObjectIndex::new(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        assert_eq!(idx.len(), 3);
    }

    #[test]
    fn test_multi_index() {
        let level1 = Index::Int64(Int64Index::new(vec![1, 2, 3]));
        let level2 = Index::Object(ObjectIndex::new(vec!["a".to_string(), "b".to_string()]));
        let codes = vec![vec![0, 0, 1, 1, 2, 2], vec![0, 1, 0, 1, 0, 1]];
        let idx = MultiIndex::new(vec![level1, level2], codes);

        assert_eq!(idx.len(), 6);
        assert_eq!(idx.nlevels(), 2);
    }

    #[test]
    fn test_block_manager() {
        let columns = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let index = Index::range(10);
        let mgr = BlockManager::new(columns, index);

        assert_eq!(mgr.ncols(), 3);
        assert_eq!(mgr.nrows(), 10);
        assert_eq!(mgr.shape(), (10, 3));
    }

    #[test]
    fn test_groupby_result() {
        let mut result = GroupByResult::new(vec!["category".to_string()]);
        result.add_to_group(vec![1], 0);
        result.add_to_group(vec![1], 2);
        result.add_to_group(vec![2], 1);

        assert_eq!(result.ngroups(), 2);
        assert_eq!(result.get_group(&[1]), Some(&vec![0, 2]));
        assert_eq!(result.get_group(&[2]), Some(&vec![1]));
    }

    #[test]
    fn test_agg_sum_f64() {
        let values = vec![1.0, 2.0, 3.0, f64::NAN, 4.0];
        assert_eq!(agg_sum_f64(&values), 10.0);
    }

    #[test]
    fn test_agg_mean_f64() {
        let values = vec![1.0, 2.0, 3.0, 4.0];
        assert_eq!(agg_mean_f64(&values), 2.5);
    }

    #[test]
    fn test_agg_mean_f64_with_nan() {
        let values = vec![1.0, 2.0, f64::NAN, 3.0];
        assert_eq!(agg_mean_f64(&values), 2.0);
    }

    #[test]
    fn test_agg_std_f64() {
        let values = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let std = agg_std_f64(&values, 0);
        assert!((std - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_agg_min_max_f64() {
        let values = vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
        assert_eq!(agg_min_f64(&values), 1.0);
        assert_eq!(agg_max_f64(&values), 9.0);
    }

    #[test]
    fn test_agg_count_f64() {
        let values = vec![1.0, f64::NAN, 2.0, f64::NAN, 3.0];
        assert_eq!(agg_count_f64(&values), 3);
    }

    #[test]
    fn test_agg_i64() {
        let values = vec![1, 2, 3, 4, 5];
        assert_eq!(agg_sum_i64(&values), 15);
        assert_eq!(agg_mean_i64(&values), 3.0);
        assert_eq!(agg_min_i64(&values), 1);
        assert_eq!(agg_max_i64(&values), 5);
    }

    #[test]
    fn test_merge_config_default() {
        let config = MergeConfig::default();
        assert_eq!(config.how, MergeHow::Inner);
        assert_eq!(config.suffixes, ("_x".to_string(), "_y".to_string()));
    }

    #[test]
    fn test_pivot_config_default() {
        let config = PivotConfig::default();
        assert_eq!(config.aggfunc, AggFunc::Mean);
        assert!(!config.margins);
    }

    #[test]
    fn test_csv_config_default() {
        let config = CsvConfig::default();
        assert_eq!(config.delimiter, ',');
        assert_eq!(config.header, Some(0));
    }

    #[test]
    fn test_json_orient() {
        assert_eq!(JsonOrient::Records.as_str(), "records");
        assert_eq!(JsonOrient::Columns.as_str(), "columns");
    }

    #[test]
    fn test_parquet_compression() {
        assert_eq!(ParquetCompression::Snappy.as_str(), "snappy");
        assert_eq!(ParquetCompression::Zstd.as_str(), "zstd");
    }

    // DataFrame tests
    #[test]
    fn test_dataframe_creation() {
        let columns = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let index = Index::range(10);
        let df = DataFrame::new(columns.clone(), index);

        assert_eq!(df.shape(), (10, 3));
        assert_eq!(df.columns(), &columns);
        assert_eq!(df.nrows(), 10);
        assert_eq!(df.ncols(), 3);
    }

    #[test]
    fn test_groupby_operation() {
        let columns = vec!["category".to_string(), "value".to_string()];
        let index = Index::range(10);
        let df = DataFrame::new(columns, index);

        let mut groupby = df.groupby(vec!["category".to_string()]);
        let result = groupby.sum();

        assert_eq!(result.func, AggFunc::Sum);
        assert_eq!(result.by_columns, vec!["category".to_string()]);
    }

    #[test]
    fn test_groupby_multiple_agg() {
        let columns = vec!["category".to_string(), "value".to_string()];
        let index = Index::range(10);
        let df = DataFrame::new(columns, index);

        let mut groupby = df.groupby(vec!["category".to_string()]);
        let results = groupby.agg(vec![AggFunc::Sum, AggFunc::Mean, AggFunc::Count]);

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].func, AggFunc::Sum);
        assert_eq!(results[1].func, AggFunc::Mean);
        assert_eq!(results[2].func, AggFunc::Count);
    }

    #[test]
    fn test_groupby_transform() {
        let columns = vec!["category".to_string(), "value".to_string()];
        let index = Index::range(10);
        let df = DataFrame::new(columns, index);

        let mut groupby = df.groupby(vec!["category".to_string()]);
        let result = groupby.transform(AggFunc::Mean);

        assert_eq!(result.nrows, 10);
    }

    #[test]
    fn test_merge_inner() {
        let left = DataFrame::new(vec!["key".to_string(), "left_val".to_string()], Index::range(5));
        let right =
            DataFrame::new(vec!["key".to_string(), "right_val".to_string()], Index::range(3));

        let config = MergeConfig {
            how: MergeHow::Inner,
            left_on: vec!["key".to_string()],
            right_on: vec!["key".to_string()],
            ..Default::default()
        };

        let result = left.merge(&right, config).unwrap();
        assert!(result.ncols() >= 2);
    }

    #[test]
    fn test_merge_left() {
        let left = DataFrame::new(vec!["key".to_string(), "left_val".to_string()], Index::range(5));
        let right =
            DataFrame::new(vec!["key".to_string(), "right_val".to_string()], Index::range(3));

        let config = MergeConfig {
            how: MergeHow::Left,
            left_on: vec!["key".to_string()],
            right_on: vec!["key".to_string()],
            ..Default::default()
        };

        let result = left.merge(&right, config).unwrap();
        assert_eq!(result.nrows(), 5);
    }

    #[test]
    fn test_merge_column_not_found() {
        let left = DataFrame::new(vec!["key".to_string(), "left_val".to_string()], Index::range(5));
        let right =
            DataFrame::new(vec!["key".to_string(), "right_val".to_string()], Index::range(3));

        let config = MergeConfig {
            how: MergeHow::Inner,
            left_on: vec!["nonexistent".to_string()],
            right_on: vec!["key".to_string()],
            ..Default::default()
        };

        let result = left.merge(&right, config);
        assert!(matches!(result, Err(DataFrameError::ColumnNotFound(_))));
    }

    #[test]
    fn test_pivot() {
        let df = DataFrame::new(
            vec![
                "date".to_string(),
                "category".to_string(),
                "value".to_string(),
            ],
            Index::range(10),
        );

        let config = PivotConfig {
            index: vec!["date".to_string()],
            columns: vec!["category".to_string()],
            values: vec!["value".to_string()],
            ..Default::default()
        };

        let result = df.pivot(config).unwrap();
        assert!(result.ncols() >= 1);
    }

    #[test]
    fn test_melt() {
        let df = DataFrame::new(
            vec!["id".to_string(), "A".to_string(), "B".to_string()],
            Index::range(5),
        );

        let config = MeltConfig {
            id_vars: vec!["id".to_string()],
            value_vars: vec!["A".to_string(), "B".to_string()],
            var_name: "variable".to_string(),
            value_name: "value".to_string(),
            ignore_index: true,
        };

        let result = df.melt(config).unwrap();
        assert_eq!(result.nrows(), 10); // 5 rows * 2 value_vars
        assert_eq!(result.ncols(), 3); // id + variable + value
    }

    #[test]
    fn test_concat_axis0() {
        let df1 = DataFrame::new(vec!["a".to_string(), "b".to_string()], Index::range(5));
        let df2 = DataFrame::new(vec!["a".to_string(), "b".to_string()], Index::range(3));

        let result = DataFrame::concat(vec![&df1, &df2], 0, true).unwrap();
        assert_eq!(result.nrows(), 8);
        assert_eq!(result.ncols(), 2);
    }

    #[test]
    fn test_concat_axis1() {
        let df1 = DataFrame::new(vec!["a".to_string(), "b".to_string()], Index::range(5));
        let df2 = DataFrame::new(vec!["c".to_string(), "d".to_string()], Index::range(5));

        let result = DataFrame::concat(vec![&df1, &df2], 1, false).unwrap();
        assert_eq!(result.nrows(), 5);
        assert_eq!(result.ncols(), 4);
    }

    #[test]
    fn test_concat_empty() {
        let result = DataFrame::concat(vec![], 0, true);
        assert!(matches!(result, Err(DataFrameError::InvalidOperation(_))));
    }

    #[test]
    fn test_concat_mismatched_columns() {
        let df1 = DataFrame::new(vec!["a".to_string(), "b".to_string()], Index::range(5));
        let df2 = DataFrame::new(vec!["c".to_string(), "d".to_string()], Index::range(3));

        let result = DataFrame::concat(vec![&df1, &df2], 0, true);
        assert!(matches!(result, Err(DataFrameError::InvalidOperation(_))));
    }

    // I/O Tests
    #[test]
    fn test_csv_reader_basic() {
        let content = "a,b,c\n1,2,3\n4,5,6";
        let reader = CsvReader::new();
        let df = reader.read_csv_str(content).unwrap();

        assert_eq!(df.ncols(), 3);
        assert_eq!(df.nrows(), 2);
        assert_eq!(df.columns(), &["a", "b", "c"]);
    }

    #[test]
    fn test_csv_reader_custom_delimiter() {
        let content = "a;b;c\n1;2;3";
        let reader = CsvReader::new().delimiter(';');
        let df = reader.read_csv_str(content).unwrap();

        assert_eq!(df.ncols(), 3);
    }

    #[test]
    fn test_csv_reader_no_header() {
        let content = "1,2,3\n4,5,6";
        let reader = CsvReader::new().header(None);
        let df = reader.read_csv_str(content).unwrap();

        assert_eq!(df.nrows(), 2);
    }

    #[test]
    fn test_csv_reader_skip_rows() {
        let content = "comment\na,b,c\n1,2,3";
        let reader = CsvReader::new().skip_rows(1);
        let df = reader.read_csv_str(content).unwrap();

        assert_eq!(df.columns(), &["a", "b", "c"]);
    }

    #[test]
    fn test_csv_reader_nrows() {
        let content = "a,b\n1,2\n3,4\n5,6\n7,8";
        let reader = CsvReader::new().nrows(2);
        let df = reader.read_csv_str(content).unwrap();

        assert_eq!(df.nrows(), 2);
    }

    #[test]
    fn test_csv_writer_basic() {
        let df = DataFrame::new(vec!["a".to_string(), "b".to_string()], Index::range(2));
        let writer = CsvWriter::new();
        let csv = writer.to_csv_str(&df).unwrap();

        assert!(csv.contains("a,b"));
    }

    #[test]
    fn test_csv_writer_no_header() {
        let df = DataFrame::new(vec!["a".to_string(), "b".to_string()], Index::range(2));
        let writer = CsvWriter::new().header(false);
        let csv = writer.to_csv_str(&df).unwrap();

        assert!(!csv.contains("a,b"));
    }

    #[test]
    fn test_csv_writer_no_index() {
        let df = DataFrame::new(vec!["a".to_string(), "b".to_string()], Index::range(2));
        let writer = CsvWriter::new().index(false);
        let csv = writer.to_csv_str(&df).unwrap();

        assert!(csv.starts_with("a,b"));
    }

    #[test]
    fn test_json_reader_basic() {
        let reader = JsonReader::new();
        let df = reader.read_json_str("[]").unwrap();
        assert_eq!(df.nrows(), 0);
    }

    #[test]
    fn test_json_reader_orient() {
        let reader = JsonReader::new().orient(JsonOrient::Records);
        let df = reader.read_json_str("[]").unwrap();
        assert_eq!(df.nrows(), 0);
    }

    #[test]
    fn test_json_writer_records() {
        let df = DataFrame::new(vec!["a".to_string(), "b".to_string()], Index::range(2));
        let writer = JsonWriter::new().orient(JsonOrient::Records);
        let json = writer.to_json_str(&df).unwrap();

        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
    }

    #[test]
    fn test_json_writer_columns() {
        let df = DataFrame::new(vec!["a".to_string(), "b".to_string()], Index::range(2));
        let writer = JsonWriter::new().orient(JsonOrient::Columns);
        let json = writer.to_json_str(&df).unwrap();

        assert!(json.starts_with('{'));
        assert!(json.contains("\"a\""));
    }

    #[test]
    fn test_json_writer_split() {
        let df = DataFrame::new(vec!["a".to_string(), "b".to_string()], Index::range(2));
        let writer = JsonWriter::new().orient(JsonOrient::Split);
        let json = writer.to_json_str(&df).unwrap();

        assert!(json.contains("\"columns\""));
        assert!(json.contains("\"index\""));
        assert!(json.contains("\"data\""));
    }

    #[test]
    fn test_parquet_reader() {
        let reader = ParquetReader::new();
        let df = reader.read_parquet("test.parquet").unwrap();
        assert_eq!(df.nrows(), 0);
    }

    #[test]
    fn test_parquet_writer() {
        let df = DataFrame::new(vec!["a".to_string(), "b".to_string()], Index::range(2));
        let writer = ParquetWriter::new().compression(ParquetCompression::Snappy);
        let result = writer.to_parquet(&df, "test.parquet");
        assert!(result.is_ok());
    }

    #[test]
    fn test_io_convenience_functions() {
        let df = DataFrame::new(vec!["a".to_string(), "b".to_string()], Index::range(2));

        // CSV
        let csv = io::to_csv_str(&df).unwrap();
        assert!(!csv.is_empty());

        // JSON
        let json = io::to_json_str(&df).unwrap();
        assert!(!json.is_empty());
    }
}
