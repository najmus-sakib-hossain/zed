//! Prepared statement implementation.

use crate::database::{QueryRow, Value};
use crate::error::SqliteResult;
use parking_lot::Mutex;
use rusqlite::{params_from_iter, Connection, Row};
use std::sync::Arc;

/// Prepared SQL statement for repeated execution.
pub struct PreparedStatement {
    conn: Arc<Mutex<Connection>>,
    sql: String,
}

impl PreparedStatement {
    /// Create a new prepared statement.
    pub(crate) fn new(conn: Arc<Mutex<Connection>>, sql: String) -> SqliteResult<Self> {
        // Validate the SQL by preparing it once
        {
            let conn = conn.lock();
            let _ = conn.prepare(&sql)?;
        }
        Ok(Self { conn, sql })
    }

    /// Execute and return all matching rows.
    pub fn all(&self, params: &[Value]) -> SqliteResult<Vec<QueryRow>> {
        let conn = self.conn.lock();
        let rusqlite_params: Vec<_> = params.iter().map(|v| v.to_rusqlite()).collect();
        let mut stmt = conn.prepare_cached(&self.sql)?;

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

    /// Execute and return the first matching row.
    pub fn get(&self, params: &[Value]) -> SqliteResult<Option<QueryRow>> {
        let mut rows = self.all(params)?;
        Ok(rows.pop())
    }

    /// Execute and return the number of affected rows.
    pub fn run(&self, params: &[Value]) -> SqliteResult<usize> {
        let conn = self.conn.lock();
        let rusqlite_params: Vec<_> = params.iter().map(|v| v.to_rusqlite()).collect();
        let count = conn.execute(&self.sql, params_from_iter(rusqlite_params))?;
        Ok(count)
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
    use crate::database::Database;

    #[test]
    fn test_prepared_statement_all() {
        let db = Database::memory().unwrap();
        db.exec("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)").unwrap();
        db.run("INSERT INTO test (name) VALUES (?)", &[Value::from("Alice")]).unwrap();
        db.run("INSERT INTO test (name) VALUES (?)", &[Value::from("Bob")]).unwrap();

        let stmt = db.prepare("SELECT * FROM test WHERE name = ?").unwrap();
        let rows = stmt.all(&[Value::from("Alice")]).unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get("name").unwrap().as_text(), Some("Alice"));
    }

    #[test]
    fn test_prepared_statement_get() {
        let db = Database::memory().unwrap();
        db.exec("CREATE TABLE test (id INTEGER PRIMARY KEY, value INTEGER)").unwrap();
        db.run("INSERT INTO test (value) VALUES (?)", &[Value::from(42)]).unwrap();

        let stmt = db.prepare("SELECT * FROM test WHERE id = ?").unwrap();
        let row = stmt.get(&[Value::from(1)]).unwrap();

        assert!(row.is_some());
        assert_eq!(row.unwrap().get("value").unwrap().as_integer(), Some(42));
    }

    #[test]
    fn test_prepared_statement_run() {
        let db = Database::memory().unwrap();
        db.exec("CREATE TABLE test (id INTEGER PRIMARY KEY, value INTEGER)").unwrap();

        let stmt = db.prepare("INSERT INTO test (value) VALUES (?)").unwrap();
        let count = stmt.run(&[Value::from(1)]).unwrap();

        assert_eq!(count, 1);
    }
}
