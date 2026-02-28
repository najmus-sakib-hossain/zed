//! Property-based tests for SQLite compatibility.
//!
//! Tests:
//! - Property 12: SQLite Query Correctness
//! - Property 13: SQLite Transaction Atomicity

use dx_compat_sqlite::{Database, Value};
use proptest::prelude::*;

/// Generate arbitrary SQLite-safe strings (no null bytes).
fn arb_sqlite_string() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_]{1,50}".prop_map(|s| s)
}

/// Generate arbitrary SQLite values.
#[allow(dead_code)]
fn arb_value() -> impl Strategy<Value = Value> {
    prop_oneof![
        Just(Value::Null),
        any::<i64>().prop_map(Value::Integer),
        any::<f64>()
            .prop_filter("finite floats only", |f| f.is_finite())
            .prop_map(Value::Real),
        arb_sqlite_string().prop_map(Value::Text),
        prop::collection::vec(any::<u8>(), 0..100).prop_map(Value::Blob),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 12: SQLite Query Correctness
    ///
    /// For any valid data inserted into SQLite:
    /// - SELECT should return exactly what was inserted
    /// - Column types should be preserved
    /// - NULL values should be handled correctly
    #[test]
    fn prop_sqlite_query_correctness(
        name in arb_sqlite_string(),
        age in any::<i64>(),
        score in any::<f64>().prop_filter("finite", |f| f.is_finite()),
    ) {
        let db = Database::memory().expect("Failed to create database");

        // Create table
        db.exec("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER, score REAL)")
            .expect("Failed to create table");

        // Insert data
        db.run(
            "INSERT INTO users (name, age, score) VALUES (?, ?, ?)",
            &[Value::Text(name.clone()), Value::Integer(age), Value::Real(score)],
        ).expect("Failed to insert");

        // Query back
        let rows = db.query("SELECT name, age, score FROM users WHERE id = 1", &[])
            .expect("Failed to query");

        prop_assert_eq!(rows.len(), 1);

        let row = &rows[0];
        prop_assert_eq!(row.get("name").and_then(|v| v.as_text()), Some(name.as_str()));
        prop_assert_eq!(row.get("age").and_then(|v| v.as_integer()), Some(age));

        // Float comparison with tolerance
        if let Some(Value::Real(returned_score)) = row.get("score") {
            prop_assert!((returned_score - score).abs() < 1e-10);
        } else {
            prop_assert!(false, "Expected Real value for score");
        }
    }

    /// Property 12b: NULL value handling
    #[test]
    fn prop_sqlite_null_handling(
        name in arb_sqlite_string(),
    ) {
        let db = Database::memory().expect("Failed to create database");

        db.exec("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT, optional TEXT)")
            .expect("Failed to create table");

        // Insert with NULL
        db.run(
            "INSERT INTO test (name, optional) VALUES (?, ?)",
            &[Value::Text(name.clone()), Value::Null],
        ).expect("Failed to insert");

        let rows = db.query("SELECT * FROM test WHERE id = 1", &[])
            .expect("Failed to query");

        prop_assert_eq!(rows.len(), 1);
        prop_assert!(rows[0].get("optional").map(|v| v.is_null()).unwrap_or(false));
    }

    /// Property 12c: Blob data round-trip
    #[test]
    fn prop_sqlite_blob_roundtrip(
        data in prop::collection::vec(any::<u8>(), 0..1000),
    ) {
        let db = Database::memory().expect("Failed to create database");

        db.exec("CREATE TABLE blobs (id INTEGER PRIMARY KEY, data BLOB)")
            .expect("Failed to create table");

        db.run(
            "INSERT INTO blobs (data) VALUES (?)",
            &[Value::Blob(data.clone())],
        ).expect("Failed to insert");

        let rows = db.query("SELECT data FROM blobs WHERE id = 1", &[])
            .expect("Failed to query");

        prop_assert_eq!(rows.len(), 1);
        prop_assert_eq!(rows[0].get("data").and_then(|v| v.as_blob()), Some(data.as_slice()));
    }

    /// Property 13: SQLite Transaction Atomicity
    ///
    /// For any sequence of operations in a transaction:
    /// - If transaction commits, all operations are visible
    /// - If transaction rolls back, no operations are visible
    #[test]
    fn prop_sqlite_transaction_atomicity(
        values in prop::collection::vec(any::<i64>(), 1..10),
        should_commit in any::<bool>(),
    ) {
        let db = Database::memory().expect("Failed to create database");

        db.exec("CREATE TABLE test (id INTEGER PRIMARY KEY, value INTEGER)")
            .expect("Failed to create table");

        let result = db.transaction(|tx| {
            for value in &values {
                tx.run("INSERT INTO test (value) VALUES (?)", &[Value::Integer(*value)])?;
            }

            if should_commit {
                Ok(())
            } else {
                Err(dx_compat_sqlite::SqliteError::Query("Intentional rollback".to_string()))
            }
        });

        let rows = db.query("SELECT * FROM test", &[]).expect("Failed to query");

        if should_commit {
            prop_assert!(result.is_ok());
            prop_assert_eq!(rows.len(), values.len());
        } else {
            prop_assert!(result.is_err());
            prop_assert_eq!(rows.len(), 0, "Transaction should have rolled back");
        }
    }

    /// Property 13b: Transaction isolation - partial failure rollback
    #[test]
    fn prop_sqlite_transaction_partial_failure(
        initial_values in prop::collection::vec(any::<i64>(), 1..5),
        fail_at_index in 0usize..5,
    ) {
        let db = Database::memory().expect("Failed to create database");

        db.exec("CREATE TABLE test (id INTEGER PRIMARY KEY, value INTEGER UNIQUE)")
            .expect("Failed to create table");

        // Insert initial values
        for value in &initial_values {
            let _ = db.run("INSERT INTO test (value) VALUES (?)", &[Value::Integer(*value)]);
        }

        let initial_count = db.query("SELECT * FROM test", &[])
            .expect("Failed to query").len();

        // Try transaction that will fail partway through
        let result = db.transaction(|tx| {
            for (i, value) in initial_values.iter().enumerate() {
                if i == fail_at_index % initial_values.len() {
                    return Err(dx_compat_sqlite::SqliteError::Query("Intentional failure".to_string()));
                }
                // Try to insert duplicate (may fail due to UNIQUE constraint)
                let _ = tx.run("INSERT INTO test (value) VALUES (?)", &[Value::Integer(*value + 1000)]);
            }
            Ok(())
        });

        // After rollback, count should be same as before
        let final_count = db.query("SELECT * FROM test", &[])
            .expect("Failed to query").len();

        prop_assert!(result.is_err());
        prop_assert_eq!(initial_count, final_count, "Transaction should have rolled back completely");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepared_statement_reuse() {
        let db = Database::memory().expect("Failed to create database");

        db.exec("CREATE TABLE test (id INTEGER PRIMARY KEY, value INTEGER)")
            .expect("Failed to create table");

        let stmt = db.prepare("INSERT INTO test (value) VALUES (?)").unwrap();

        for i in 0..100 {
            stmt.run(&[Value::Integer(i)]).unwrap();
        }

        let count = db.query("SELECT COUNT(*) as cnt FROM test", &[]).unwrap();
        assert_eq!(count[0].get("cnt").unwrap().as_integer(), Some(100));
    }

    #[test]
    fn test_concurrent_reads() {
        let db = Database::memory().expect("Failed to create database");

        db.exec("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)")
            .expect("Failed to create table");

        for i in 0..10 {
            db.run("INSERT INTO test (value) VALUES (?)", &[Value::Text(format!("value_{}", i))])
                .unwrap();
        }

        // Multiple queries should work
        let rows1 = db.query("SELECT * FROM test WHERE id < 5", &[]).unwrap();
        let rows2 = db.query("SELECT * FROM test WHERE id >= 5", &[]).unwrap();

        assert_eq!(rows1.len() + rows2.len(), 10);
    }
}
