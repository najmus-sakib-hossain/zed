//! Database adapter compatibility for Django
//!
//! Provides compatibility layers for:
//! - SQLite3 C extension support
//! - psycopg2 (PostgreSQL) compatibility layer

use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during database operations
#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Query error: {0}")]
    QueryError(String),

    #[error("Type conversion error: {0}")]
    TypeConversionError(String),

    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),
}

/// Database value types compatible with Python's DB-API 2.0
#[derive(Debug, Clone, PartialEq)]
pub enum DbValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
    Boolean(bool),
    Timestamp(i64),
    Date(i32, u8, u8),     // year, month, day
    Time(u8, u8, u8, u32), // hour, minute, second, microsecond
}

impl DbValue {
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            DbValue::Integer(i) => Some(*i),
            DbValue::Real(f) => Some(*f as i64),
            DbValue::Text(s) => s.parse().ok(),
            DbValue::Boolean(b) => Some(if *b { 1 } else { 0 }),
            DbValue::Timestamp(ts) => Some(*ts),
            _ => None,
        }
    }

    pub fn as_real(&self) -> Option<f64> {
        match self {
            DbValue::Integer(i) => Some(*i as f64),
            DbValue::Real(f) => Some(*f),
            DbValue::Text(s) => s.parse().ok(),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<String> {
        match self {
            DbValue::Text(s) => Some(s.clone()),
            DbValue::Integer(i) => Some(i.to_string()),
            DbValue::Real(f) => Some(f.to_string()),
            DbValue::Boolean(b) => Some(b.to_string()),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, DbValue::Null)
    }

    /// Convert to Python-compatible type string
    pub fn type_name(&self) -> &'static str {
        match self {
            DbValue::Null => "NoneType",
            DbValue::Integer(_) => "int",
            DbValue::Real(_) => "float",
            DbValue::Text(_) => "str",
            DbValue::Blob(_) => "bytes",
            DbValue::Boolean(_) => "bool",
            DbValue::Timestamp(_) => "datetime",
            DbValue::Date(..) => "date",
            DbValue::Time(..) => "time",
        }
    }
}

/// A row of database results
#[derive(Debug, Clone)]
pub struct DbRow {
    columns: Vec<String>,
    values: Vec<DbValue>,
}

impl DbRow {
    pub fn new(columns: Vec<String>, values: Vec<DbValue>) -> Self {
        Self { columns, values }
    }

    pub fn get(&self, column: &str) -> Option<&DbValue> {
        self.columns
            .iter()
            .position(|c| c.eq_ignore_ascii_case(column))
            .and_then(|i| self.values.get(i))
    }

    pub fn get_index(&self, index: usize) -> Option<&DbValue> {
        self.values.get(index)
    }

    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    pub fn values(&self) -> &[DbValue] {
        &self.values
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Cursor description (DB-API 2.0 compatible)
#[derive(Debug, Clone)]
pub struct ColumnDescription {
    pub name: String,
    pub type_code: i32,
    pub display_size: Option<i32>,
    pub internal_size: Option<i32>,
    pub precision: Option<i32>,
    pub scale: Option<i32>,
    pub null_ok: Option<bool>,
}

/// Database adapter trait (DB-API 2.0 compatible)
pub trait DatabaseAdapter: Send + Sync {
    fn connect(&mut self, connection_string: &str) -> Result<(), DatabaseError>;
    fn query(&self, sql: &str, params: &[DbValue]) -> Result<Vec<DbRow>, DatabaseError>;
    fn execute(&self, sql: &str, params: &[DbValue]) -> Result<usize, DatabaseError>;
    fn executemany(&self, sql: &str, params_list: &[Vec<DbValue>]) -> Result<usize, DatabaseError>;
    fn begin_transaction(&mut self) -> Result<(), DatabaseError>;
    fn commit(&mut self) -> Result<(), DatabaseError>;
    fn rollback(&mut self) -> Result<(), DatabaseError>;
    fn close(&mut self) -> Result<(), DatabaseError>;
    fn is_connected(&self) -> bool;
    fn last_insert_rowid(&self) -> Option<i64>;
    fn description(&self) -> Option<Vec<ColumnDescription>>;
}

/// SQLite3 adapter with C extension compatibility
pub struct SqliteAdapter {
    connection_string: Option<String>,
    connected: bool,
    in_transaction: bool,
    last_rowid: Option<i64>,
    last_description: Option<Vec<ColumnDescription>>,
    isolation_level: IsolationLevel,
    rows: Vec<DbRow>, // In-memory storage for stub
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    Deferred,
    Immediate,
    Exclusive,
    Autocommit,
}

impl Default for SqliteAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SqliteAdapter {
    pub fn new() -> Self {
        Self {
            connection_string: None,
            connected: false,
            in_transaction: false,
            last_rowid: None,
            last_description: None,
            isolation_level: IsolationLevel::Deferred,
            rows: Vec::new(),
        }
    }

    pub fn memory() -> Result<Self, DatabaseError> {
        let mut adapter = Self::new();
        adapter.connect(":memory:")?;
        Ok(adapter)
    }

    pub fn from_file(path: &Path) -> Result<Self, DatabaseError> {
        let mut adapter = Self::new();
        adapter.connect(path.to_str().unwrap_or(":memory:"))?;
        Ok(adapter)
    }

    pub fn set_isolation_level(&mut self, level: IsolationLevel) {
        self.isolation_level = level;
    }

    /// Create table (stub implementation)
    pub fn create_table(
        &mut self,
        name: &str,
        columns: &[(&str, &str)],
    ) -> Result<(), DatabaseError> {
        if !self.connected {
            return Err(DatabaseError::ConnectionError("Not connected".into()));
        }
        let cols: Vec<String> = columns.iter().map(|(n, t)| format!("{} {}", n, t)).collect();
        let sql = format!("CREATE TABLE {} ({})", name, cols.join(", "));
        self.execute(&sql, &[])?;
        Ok(())
    }

    /// Insert row (stub implementation)
    pub fn insert(&mut self, _table: &str, values: &[DbValue]) -> Result<i64, DatabaseError> {
        if !self.connected {
            return Err(DatabaseError::ConnectionError("Not connected".into()));
        }
        let rowid = self.rows.len() as i64 + 1;
        self.last_rowid = Some(rowid);
        self.rows.push(DbRow::new(vec![], values.to_vec()));
        Ok(rowid)
    }
}

impl DatabaseAdapter for SqliteAdapter {
    fn connect(&mut self, connection_string: &str) -> Result<(), DatabaseError> {
        self.connection_string = Some(connection_string.to_string());
        self.connected = true;
        Ok(())
    }

    fn query(&self, _sql: &str, _params: &[DbValue]) -> Result<Vec<DbRow>, DatabaseError> {
        if !self.connected {
            return Err(DatabaseError::ConnectionError("Not connected".into()));
        }
        Ok(self.rows.clone())
    }

    fn execute(&self, _sql: &str, _params: &[DbValue]) -> Result<usize, DatabaseError> {
        if !self.connected {
            return Err(DatabaseError::ConnectionError("Not connected".into()));
        }
        Ok(1)
    }

    fn executemany(&self, sql: &str, params_list: &[Vec<DbValue>]) -> Result<usize, DatabaseError> {
        let mut total = 0;
        for params in params_list {
            total += self.execute(sql, params)?;
        }
        Ok(total)
    }

    fn begin_transaction(&mut self) -> Result<(), DatabaseError> {
        if !self.connected {
            return Err(DatabaseError::ConnectionError("Not connected".into()));
        }
        if self.in_transaction {
            return Err(DatabaseError::TransactionError("Already in transaction".into()));
        }
        self.in_transaction = true;
        Ok(())
    }

    fn commit(&mut self) -> Result<(), DatabaseError> {
        if !self.in_transaction && self.isolation_level != IsolationLevel::Autocommit {
            return Err(DatabaseError::TransactionError("No active transaction".into()));
        }
        self.in_transaction = false;
        Ok(())
    }

    fn rollback(&mut self) -> Result<(), DatabaseError> {
        if !self.in_transaction {
            return Err(DatabaseError::TransactionError("No active transaction".into()));
        }
        self.in_transaction = false;
        Ok(())
    }

    fn close(&mut self) -> Result<(), DatabaseError> {
        self.connected = false;
        self.connection_string = None;
        self.rows.clear();
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn last_insert_rowid(&self) -> Option<i64> {
        self.last_rowid
    }

    fn description(&self) -> Option<Vec<ColumnDescription>> {
        self.last_description.clone()
    }
}

/// PostgreSQL adapter (psycopg2 compatibility)
pub struct PostgresAdapter {
    params: HashMap<String, String>,
    connected: bool,
    in_transaction: bool,
    autocommit: bool,
    last_rowid: Option<i64>,
    last_description: Option<Vec<ColumnDescription>>,
    server_version: Option<(i32, i32, i32)>,
}

impl Default for PostgresAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl PostgresAdapter {
    pub fn new() -> Self {
        Self {
            params: HashMap::new(),
            connected: false,
            in_transaction: false,
            autocommit: false,
            last_rowid: None,
            last_description: None,
            server_version: None,
        }
    }

    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }

    pub fn set_autocommit(&mut self, autocommit: bool) {
        self.autocommit = autocommit;
    }

    pub fn server_version(&self) -> Option<(i32, i32, i32)> {
        self.server_version
    }

    /// Parse connection string into parameters
    fn parse_connection_string(&mut self, conn_str: &str) {
        // Parse DSN format: host=localhost port=5432 dbname=test user=postgres
        for part in conn_str.split_whitespace() {
            if let Some((key, value)) = part.split_once('=') {
                self.params.insert(key.to_string(), value.to_string());
            }
        }
    }

    /// Get connection parameter
    pub fn get_param(&self, key: &str) -> Option<&String> {
        self.params.get(key)
    }
}

impl DatabaseAdapter for PostgresAdapter {
    fn connect(&mut self, connection_string: &str) -> Result<(), DatabaseError> {
        self.parse_connection_string(connection_string);
        self.connected = true;
        self.server_version = Some((15, 0, 0)); // Stub version
        Ok(())
    }

    fn query(&self, _sql: &str, _params: &[DbValue]) -> Result<Vec<DbRow>, DatabaseError> {
        if !self.connected {
            return Err(DatabaseError::ConnectionError("Not connected".into()));
        }
        Ok(Vec::new())
    }

    fn execute(&self, _sql: &str, _params: &[DbValue]) -> Result<usize, DatabaseError> {
        if !self.connected {
            return Err(DatabaseError::ConnectionError("Not connected".into()));
        }
        Ok(1)
    }

    fn executemany(&self, sql: &str, params_list: &[Vec<DbValue>]) -> Result<usize, DatabaseError> {
        let mut total = 0;
        for params in params_list {
            total += self.execute(sql, params)?;
        }
        Ok(total)
    }

    fn begin_transaction(&mut self) -> Result<(), DatabaseError> {
        if !self.connected {
            return Err(DatabaseError::ConnectionError("Not connected".into()));
        }
        self.in_transaction = true;
        Ok(())
    }

    fn commit(&mut self) -> Result<(), DatabaseError> {
        if !self.in_transaction && !self.autocommit {
            return Err(DatabaseError::TransactionError("No active transaction".into()));
        }
        self.in_transaction = false;
        Ok(())
    }

    fn rollback(&mut self) -> Result<(), DatabaseError> {
        if !self.in_transaction {
            return Err(DatabaseError::TransactionError("No active transaction".into()));
        }
        self.in_transaction = false;
        Ok(())
    }

    fn close(&mut self) -> Result<(), DatabaseError> {
        self.connected = false;
        self.params.clear();
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn last_insert_rowid(&self) -> Option<i64> {
        self.last_rowid
    }

    fn description(&self) -> Option<Vec<ColumnDescription>> {
        self.last_description.clone()
    }
}

/// Connection pool for database connections
pub struct ConnectionPool<A: DatabaseAdapter> {
    connections: Vec<A>,
    max_size: usize,
    connection_string: String,
}

impl<A: DatabaseAdapter + Default> ConnectionPool<A> {
    pub fn new(connection_string: &str, max_size: usize) -> Self {
        Self {
            connections: Vec::with_capacity(max_size),
            max_size,
            connection_string: connection_string.to_string(),
        }
    }

    pub fn get(&mut self) -> Result<&mut A, DatabaseError> {
        if self.connections.is_empty() && self.connections.len() < self.max_size {
            let mut conn = A::default();
            conn.connect(&self.connection_string)?;
            self.connections.push(conn);
        }
        self.connections
            .last_mut()
            .ok_or_else(|| DatabaseError::ConnectionError("Pool exhausted".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_value_conversions() {
        let int_val = DbValue::Integer(42);
        assert_eq!(int_val.as_integer(), Some(42));
        assert_eq!(int_val.as_real(), Some(42.0));
        assert_eq!(int_val.type_name(), "int");

        let real_val = DbValue::Real(3.125);
        assert_eq!(real_val.as_real(), Some(3.125));
        assert_eq!(real_val.as_integer(), Some(3));

        let text_val = DbValue::Text("hello".to_string());
        assert_eq!(text_val.as_text(), Some("hello".to_string()));

        let null_val = DbValue::Null;
        assert!(null_val.is_null());
        assert_eq!(null_val.type_name(), "NoneType");
    }

    #[test]
    fn test_db_row() {
        let row = DbRow::new(
            vec!["id".to_string(), "name".to_string()],
            vec![DbValue::Integer(1), DbValue::Text("test".to_string())],
        );

        assert_eq!(row.get("id"), Some(&DbValue::Integer(1)));
        assert_eq!(row.get("ID"), Some(&DbValue::Integer(1))); // Case insensitive
        assert_eq!(row.get("name"), Some(&DbValue::Text("test".to_string())));
        assert_eq!(row.get("unknown"), None);
        assert_eq!(row.len(), 2);
    }

    #[test]
    fn test_sqlite_adapter() {
        let mut adapter = SqliteAdapter::new();

        assert!(adapter.connect(":memory:").is_ok());
        assert!(adapter.is_connected());
        assert!(adapter.begin_transaction().is_ok());
        assert!(adapter.commit().is_ok());
        assert!(adapter.close().is_ok());
        assert!(!adapter.is_connected());
    }

    #[test]
    fn test_sqlite_memory() {
        let adapter = SqliteAdapter::memory().unwrap();
        assert!(adapter.is_connected());
    }

    #[test]
    fn test_sqlite_insert() {
        let mut adapter = SqliteAdapter::memory().unwrap();
        let rowid = adapter
            .insert("test", &[DbValue::Integer(1), DbValue::Text("hello".into())])
            .unwrap();
        assert_eq!(rowid, 1);
        assert_eq!(adapter.last_insert_rowid(), Some(1));
    }

    #[test]
    fn test_postgres_adapter() {
        let mut adapter = PostgresAdapter::new()
            .with_param("host", "localhost")
            .with_param("port", "5432");

        assert!(adapter.connect("dbname=test").is_ok());
        assert!(adapter.is_connected());
        assert_eq!(adapter.get_param("host"), Some(&"localhost".to_string()));
        assert!(adapter.server_version().is_some());
        assert!(adapter.close().is_ok());
    }

    #[test]
    fn test_postgres_autocommit() {
        let mut adapter = PostgresAdapter::new();
        adapter.connect("").unwrap();
        adapter.set_autocommit(true);
        assert!(adapter.commit().is_ok()); // Should work with autocommit
    }

    #[test]
    fn test_executemany() {
        let adapter = SqliteAdapter::memory().unwrap();
        let params = vec![
            vec![DbValue::Integer(1)],
            vec![DbValue::Integer(2)],
            vec![DbValue::Integer(3)],
        ];
        let count = adapter.executemany("INSERT INTO t VALUES (?)", &params).unwrap();
        assert_eq!(count, 3);
    }
}
