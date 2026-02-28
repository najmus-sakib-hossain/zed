//! SQLite database implementation.
//!
//! High-performance SQLite wrapper targeting 200k+ operations/second.

use crate::error::SqliteResult;
use crate::statement::PreparedStatement;
use lru::LruCache;
use parking_lot::Mutex;
use rusqlite::{params_from_iter, Connection, OpenFlags, Row};
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Arc;

/// SQLite database connection.
///
/// Provides a high-performance interface to SQLite with:
/// - WAL mode enabled by default
/// - Statement caching for repeated queries
/// - Transaction support
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    statement_cache: Arc<Mutex<LruCache<String, ()>>>,
}

impl Database {
    /// Open or create a database at the given path.
    ///
    /// # Arguments
    /// * `path` - Path to the database file
    ///
    /// # Example
    /// ```ignore
    /// let db = Database::new("my_database.db")?;
    /// ```
    pub fn new(path: impl AsRef<Path>) -> SqliteResult<Self> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;

        Self::configure_connection(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            statement_cache: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap()))),
        })
    }

    /// Open an in-memory database.
    ///
    /// Useful for testing or temporary data.
    pub fn memory() -> SqliteResult<Self> {
        let conn = Connection::open_in_memory()?;
        Self::configure_connection(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            statement_cache: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap()))),
        })
    }

    fn configure_connection(conn: &Connection) -> SqliteResult<()> {
        // Enable WAL mode for better concurrent read performance
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA cache_size=-64000;
             PRAGMA temp_store=MEMORY;
             PRAGMA mmap_size=268435456;",
        )?;
        Ok(())
    }

    /// Execute SQL without returning results.
    ///
    /// # Arguments
    /// * `sql` - SQL statement(s) to execute
    pub fn exec(&self, sql: &str) -> SqliteResult<()> {
        let conn = self.conn.lock();
        conn.execute_batch(sql)?;
        Ok(())
    }

    /// Execute a single SQL statement with parameters.
    ///
    /// # Arguments
    /// * `sql` - SQL statement
    /// * `params` - Parameter values
    ///
    /// # Returns
    /// Number of rows affected.
    pub fn run(&self, sql: &str, params: &[Value]) -> SqliteResult<usize> {
        let conn = self.conn.lock();
        let rusqlite_params: Vec<_> = params.iter().map(|v| v.to_rusqlite()).collect();
        let count = conn.execute(sql, params_from_iter(rusqlite_params))?;
        Ok(count)
    }

    /// Query and return all matching rows.
    ///
    /// # Arguments
    /// * `sql` - SQL query
    /// * `params` - Parameter values
    ///
    /// # Returns
    /// Vector of rows as HashMaps.
    pub fn query(&self, sql: &str, params: &[Value]) -> SqliteResult<Vec<QueryRow>> {
        let conn = self.conn.lock();
        let rusqlite_params: Vec<_> = params.iter().map(|v| v.to_rusqlite()).collect();
        let mut stmt = conn.prepare_cached(sql)?;

        let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

        let rows = stmt.query_map(params_from_iter(rusqlite_params), |row| {
            let mut query_row = QueryRow::new();
            for (i, name) in column_names.iter().enumerate() {
                let value = row_to_value(row, i)?;
                query_row.insert(name.clone(), value);
            }
            Ok(query_row)
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Query and return the first matching row.
    ///
    /// # Arguments
    /// * `sql` - SQL query
    /// * `params` - Parameter values
    ///
    /// # Returns
    /// Optional row as HashMap.
    pub fn get(&self, sql: &str, params: &[Value]) -> SqliteResult<Option<QueryRow>> {
        let mut rows = self.query(sql, params)?;
        Ok(rows.pop())
    }

    /// Prepare a statement for repeated execution.
    ///
    /// # Arguments
    /// * `sql` - SQL statement
    pub fn prepare(&self, sql: &str) -> SqliteResult<PreparedStatement> {
        // Track in cache
        self.statement_cache.lock().put(sql.to_string(), ());
        PreparedStatement::new(Arc::clone(&self.conn), sql.to_string())
    }

    /// Execute a function within a transaction.
    ///
    /// If the function returns Ok, the transaction is committed.
    /// If it returns Err, the transaction is rolled back.
    pub fn transaction<F, T>(&self, f: F) -> SqliteResult<T>
    where
        F: FnOnce(&Transaction) -> SqliteResult<T>,
    {
        let conn = self.conn.lock();
        conn.execute("BEGIN TRANSACTION", [])?;

        let tx = Transaction { conn: &conn };
        match f(&tx) {
            Ok(result) => {
                conn.execute("COMMIT", [])?;
                Ok(result)
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", []);
                Err(e)
            }
        }
    }

    /// Close the database connection.
    pub fn close(self) -> SqliteResult<()> {
        // Connection is closed when dropped
        Ok(())
    }
}

/// Transaction handle for atomic operations.
pub struct Transaction<'a> {
    conn: &'a Connection,
}

impl Transaction<'_> {
    /// Execute SQL within the transaction.
    pub fn exec(&self, sql: &str) -> SqliteResult<()> {
        self.conn.execute_batch(sql)?;
        Ok(())
    }

    /// Run a statement within the transaction.
    pub fn run(&self, sql: &str, params: &[Value]) -> SqliteResult<usize> {
        let rusqlite_params: Vec<_> = params.iter().map(|v| v.to_rusqlite()).collect();
        let count = self.conn.execute(sql, params_from_iter(rusqlite_params))?;
        Ok(count)
    }

    /// Query within the transaction.
    pub fn query(&self, sql: &str, params: &[Value]) -> SqliteResult<Vec<QueryRow>> {
        let rusqlite_params: Vec<_> = params.iter().map(|v| v.to_rusqlite()).collect();
        let mut stmt = self.conn.prepare(sql)?;

        let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

        let rows = stmt.query_map(params_from_iter(rusqlite_params), |row| {
            let mut query_row = QueryRow::new();
            for (i, name) in column_names.iter().enumerate() {
                let value = row_to_value(row, i)?;
                query_row.insert(name.clone(), value);
            }
            Ok(query_row)
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }
}

/// Query result row.
pub type QueryRow = std::collections::HashMap<String, Value>;

/// SQLite value type.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// NULL value
    Null,
    /// Integer value
    Integer(i64),
    /// Real (float) value
    Real(f64),
    /// Text value
    Text(String),
    /// Blob (binary) value
    Blob(Vec<u8>),
}

impl Value {
    /// Convert to rusqlite value reference.
    pub(crate) fn to_rusqlite(&self) -> rusqlite::types::Value {
        match self {
            Value::Null => rusqlite::types::Value::Null,
            Value::Integer(i) => rusqlite::types::Value::Integer(*i),
            Value::Real(f) => rusqlite::types::Value::Real(*f),
            Value::Text(s) => rusqlite::types::Value::Text(s.clone()),
            Value::Blob(b) => rusqlite::types::Value::Blob(b.clone()),
        }
    }

    /// Check if value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Get as integer.
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Value::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Get as real.
    pub fn as_real(&self) -> Option<f64> {
        match self {
            Value::Real(f) => Some(*f),
            Value::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Get as text.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(s) => Some(s),
            _ => None,
        }
    }

    /// Get as blob.
    pub fn as_blob(&self) -> Option<&[u8]> {
        match self {
            Value::Blob(b) => Some(b),
            _ => None,
        }
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Integer(v as i64)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Integer(v)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Real(v)
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::Text(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::Text(v.to_string())
    }
}

impl From<Vec<u8>> for Value {
    fn from(v: Vec<u8>) -> Self {
        Value::Blob(v)
    }
}

impl<T> From<Option<T>> for Value
where
    T: Into<Value>,
{
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => v.into(),
            None => Value::Null,
        }
    }
}

fn row_to_value(row: &Row, idx: usize) -> rusqlite::Result<Value> {
    use rusqlite::types::ValueRef;

    match row.get_ref(idx)? {
        ValueRef::Null => Ok(Value::Null),
        ValueRef::Integer(i) => Ok(Value::Integer(i)),
        ValueRef::Real(f) => Ok(Value::Real(f)),
        ValueRef::Text(s) => {
            Ok(Value::Text(std::str::from_utf8(s).unwrap_or_default().to_string()))
        }
        ValueRef::Blob(b) => Ok(Value::Blob(b.to_vec())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SqliteError;

    #[test]
    fn test_memory_database() {
        let db = Database::memory().unwrap();
        db.exec("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)").unwrap();
    }

    #[test]
    fn test_insert_and_query() {
        let db = Database::memory().unwrap();
        db.exec("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)")
            .unwrap();

        db.run(
            "INSERT INTO users (name, age) VALUES (?, ?)",
            &[Value::from("Alice"), Value::from(30)],
        )
        .unwrap();

        let rows = db.query("SELECT * FROM users WHERE name = ?", &[Value::from("Alice")]).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get("name").unwrap().as_text(), Some("Alice"));
        assert_eq!(rows[0].get("age").unwrap().as_integer(), Some(30));
    }

    #[test]
    fn test_get_single_row() {
        let db = Database::memory().unwrap();
        db.exec("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)").unwrap();
        db.run("INSERT INTO test (value) VALUES (?)", &[Value::from("test")]).unwrap();

        let row = db.get("SELECT * FROM test WHERE id = ?", &[Value::from(1)]).unwrap();
        assert!(row.is_some());
        assert_eq!(row.unwrap().get("value").unwrap().as_text(), Some("test"));
    }

    #[test]
    fn test_transaction_commit() {
        let db = Database::memory().unwrap();
        db.exec("CREATE TABLE test (id INTEGER PRIMARY KEY, value INTEGER)").unwrap();

        db.transaction(|tx| {
            tx.run("INSERT INTO test (value) VALUES (?)", &[Value::from(1)])?;
            tx.run("INSERT INTO test (value) VALUES (?)", &[Value::from(2)])?;
            Ok(())
        })
        .unwrap();

        let rows = db.query("SELECT * FROM test", &[]).unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_transaction_rollback() {
        let db = Database::memory().unwrap();
        db.exec("CREATE TABLE test (id INTEGER PRIMARY KEY, value INTEGER)").unwrap();

        let result: SqliteResult<()> = db.transaction(|tx| {
            tx.run("INSERT INTO test (value) VALUES (?)", &[Value::from(1)])?;
            Err(SqliteError::Query("Intentional error".to_string()))
        });

        assert!(result.is_err());

        let rows = db.query("SELECT * FROM test", &[]).unwrap();
        assert_eq!(rows.len(), 0); // Rolled back
    }

    #[test]
    fn test_value_conversions() {
        assert_eq!(Value::from(42).as_integer(), Some(42));
        assert_eq!(Value::from(3.14).as_real(), Some(3.14));
        assert_eq!(Value::from("hello").as_text(), Some("hello"));
        assert!(Value::Null.is_null());
    }
}
