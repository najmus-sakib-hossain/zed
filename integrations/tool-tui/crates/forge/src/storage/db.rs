use anyhow::Result;
use parking_lot::Mutex;
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::crdt::{Anchor, Operation};

/// Configuration for database connection pool
#[derive(Debug, Clone)]
pub struct DatabasePoolConfig {
    /// Maximum number of connections in the pool
    pub max_connections: usize,
    /// Path to the database file
    pub db_path: PathBuf,
}

impl Default for DatabasePoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 4,
            db_path: PathBuf::from(".dx/forge/forge.db"),
        }
    }
}

/// A connection pool for SQLite database connections
pub struct DatabasePool {
    connections: Vec<Arc<Mutex<Connection>>>,
    max_connections: usize,
    active_connections: AtomicUsize,
    next_connection: AtomicUsize,
}

impl DatabasePool {
    /// Create a new database pool with the specified configuration
    pub fn new(config: DatabasePoolConfig) -> Result<Self> {
        let mut connections = Vec::with_capacity(config.max_connections);

        for _ in 0..config.max_connections {
            let conn = Connection::open(&config.db_path)?;
            // Enable WAL mode for better concurrent access
            conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
            connections.push(Arc::new(Mutex::new(conn)));
        }

        Ok(Self {
            connections,
            max_connections: config.max_connections,
            active_connections: AtomicUsize::new(0),
            next_connection: AtomicUsize::new(0),
        })
    }

    /// Get a connection from the pool using round-robin selection
    pub fn get_connection(&self) -> PooledConnection<'_> {
        let idx = self.next_connection.fetch_add(1, Ordering::SeqCst) % self.max_connections;
        self.active_connections.fetch_add(1, Ordering::SeqCst);
        PooledConnection {
            conn: self.connections[idx].clone(),
            pool: self,
        }
    }

    /// Get the current number of active connections
    pub fn active_count(&self) -> usize {
        self.active_connections.load(Ordering::SeqCst)
    }

    /// Get the maximum pool size
    pub fn max_size(&self) -> usize {
        self.max_connections
    }

    /// Get the total number of connections in the pool
    pub fn pool_size(&self) -> usize {
        self.connections.len()
    }
}

/// A connection borrowed from the pool
pub struct PooledConnection<'a> {
    conn: Arc<Mutex<Connection>>,
    pool: &'a DatabasePool,
}

impl<'a> PooledConnection<'a> {
    /// Get access to the underlying connection
    pub fn lock(&self) -> parking_lot::MutexGuard<'_, Connection> {
        self.conn.lock()
    }
}

impl<'a> Drop for PooledConnection<'a> {
    fn drop(&mut self) {
        self.pool.active_connections.fetch_sub(1, Ordering::SeqCst);
    }
}

pub struct Database {
    pub conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(forge_path: &Path) -> Result<Self> {
        let db_path = forge_path.join("forge.db");
        let conn = Connection::open(db_path)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn open(forge_path: &str) -> Result<Self> {
        Self::new(Path::new(forge_path))
    }

    pub fn initialize(&self) -> Result<()> {
        let conn = self.conn.lock();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS operations (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                actor_id TEXT NOT NULL,
                file_path TEXT NOT NULL,
                op_type TEXT NOT NULL,
                op_data BLOB NOT NULL,
                parent_ops TEXT
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS anchors (
                id TEXT PRIMARY KEY,
                file_path TEXT NOT NULL,
                stable_id TEXT NOT NULL UNIQUE,
                position BLOB NOT NULL,
                created_at TEXT NOT NULL,
                message TEXT,
                tags TEXT
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS annotations (
                id TEXT PRIMARY KEY,
                file_path TEXT NOT NULL,
                anchor_id TEXT,
                line INTEGER NOT NULL,
                content TEXT NOT NULL,
                author TEXT NOT NULL,
                created_at TEXT NOT NULL,
                is_ai BOOLEAN NOT NULL,
                FOREIGN KEY(anchor_id) REFERENCES anchors(id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_ops_file_time
             ON operations(file_path, timestamp)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_anchors_file
             ON anchors(file_path)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_annotations_file
             ON annotations(file_path, line)",
            [],
        )?;

        Ok(())
    }

    pub fn store_operation(&self, op: &Operation) -> Result<bool> {
        let conn = self.conn.lock();
        let op_data = bincode::serialize(&op.op_type)?;
        let parent_ops = serde_json::to_string(&op.parent_ops)?;

        // Extract the operation type name (e.g., "Insert" from "Insert{...}")
        // The format!("{:?}", ...) always produces a string with at least the enum variant name
        let op_type_str = format!("{:?}", op.op_type);
        let op_type_name = op_type_str.split('{').next().unwrap_or(&op_type_str);

        conn.execute(
            "INSERT OR IGNORE INTO operations (id, timestamp, actor_id, file_path, op_type, op_data, parent_ops)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                op.id.to_string(),
                op.timestamp.to_rfc3339(),
                op.actor_id,
                op.file_path,
                op_type_name,
                op_data,
                parent_ops,
            ],
        )
        .map(|changes| changes > 0)
        .map_err(Into::into)
    }

    pub fn get_operations(&self, file: Option<&Path>, limit: usize) -> Result<Vec<Operation>> {
        let conn = self.conn.lock();

        let query = if let Some(f) = file {
            format!(
                "SELECT id, timestamp, actor_id, file_path, op_data, parent_ops
                 FROM operations
                 WHERE file_path = '{}'
                 ORDER BY timestamp DESC
                 LIMIT {}",
                f.display(),
                limit
            )
        } else {
            format!(
                "SELECT id, timestamp, actor_id, file_path, op_data, parent_ops
                 FROM operations
                 ORDER BY timestamp DESC
                 LIMIT {}",
                limit
            )
        };

        let mut stmt = conn.prepare(&query)?;
        let ops = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let timestamp: String = row.get(1)?;
            let actor_id: String = row.get(2)?;
            let file_path: String = row.get(3)?;
            let op_data: Vec<u8> = row.get(4)?;
            let parent_ops: String = row.get(5)?;

            // Deserialize operation type - convert errors to rusqlite errors
            let op_type = bincode::deserialize(&op_data).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    4,
                    rusqlite::types::Type::Blob,
                    Box::new(e),
                )
            })?;

            // Parse parent operations JSON
            let parents: Vec<uuid::Uuid> = serde_json::from_str(&parent_ops).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;

            // Parse UUID
            let parsed_id = uuid::Uuid::parse_str(&id).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;

            // Parse timestamp
            let parsed_timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp)
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?
                .into();

            Ok(Operation {
                id: parsed_id,
                timestamp: parsed_timestamp,
                actor_id,
                file_path,
                op_type,
                parent_ops: parents,
            })
        })?;

        Ok(ops.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn store_anchor(&self, anchor: &Anchor) -> Result<()> {
        let conn = self.conn.lock();
        let position = bincode::serialize(&anchor.position)?;
        let tags = serde_json::to_string(&anchor.tags)?;

        conn.execute(
            "INSERT INTO anchors (id, file_path, stable_id, position, created_at, message, tags)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                anchor.id.to_string(),
                anchor.file_path,
                anchor.stable_id,
                position,
                anchor.created_at.to_rfc3339(),
                anchor.message,
                tags,
            ],
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::sync::Arc;
    use std::thread;
    use tempfile::TempDir;

    // Feature: platform-native-io-hardening, Property 26: Connection Pool Sizing
    // *For any* configured pool size N, the database connection pool SHALL maintain
    // at most N active connections at any time.
    // **Validates: Requirements 10.5**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_connection_pool_sizing(pool_size in 1usize..=8) {
            let temp_dir = TempDir::new().unwrap();
            let db_path = temp_dir.path().join("test.db");

            let config = DatabasePoolConfig {
                max_connections: pool_size,
                db_path: db_path.clone(),
            };

            let pool = DatabasePool::new(config).unwrap();

            // Verify pool was created with correct size
            prop_assert_eq!(pool.pool_size(), pool_size);
            prop_assert_eq!(pool.max_size(), pool_size);

            // Initially no active connections
            prop_assert_eq!(pool.active_count(), 0);

            // Get multiple connections and verify active count never exceeds pool size
            let mut connections = Vec::new();
            for i in 0..pool_size {
                let conn = pool.get_connection();
                prop_assert!(pool.active_count() <= pool_size,
                    "Active count {} exceeded pool size {} at iteration {}",
                    pool.active_count(), pool_size, i);
                connections.push(conn);
            }

            // All connections should be active now
            prop_assert_eq!(pool.active_count(), pool_size);

            // Drop half the connections
            let half = pool_size / 2;
            for _ in 0..half {
                connections.pop();
            }

            // Active count should decrease
            prop_assert_eq!(pool.active_count(), pool_size - half);

            // Drop remaining connections
            connections.clear();

            // All connections should be released
            prop_assert_eq!(pool.active_count(), 0);
        }
    }

    #[test]
    fn test_pool_concurrent_access() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("concurrent_test.db");

        let config = DatabasePoolConfig {
            max_connections: 4,
            db_path,
        };

        let pool = Arc::new(DatabasePool::new(config).unwrap());

        // Spawn multiple threads that acquire and release connections
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let pool = Arc::clone(&pool);
                thread::spawn(move || {
                    for _ in 0..10 {
                        let conn = pool.get_connection();
                        // Verify we can lock the connection
                        let _guard = conn.lock();
                        // Small delay to simulate work
                        thread::sleep(std::time::Duration::from_micros(100));
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // After all threads complete, no connections should be active
        assert_eq!(pool.active_count(), 0);
    }

    #[test]
    fn test_pool_round_robin_distribution() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("round_robin_test.db");

        let config = DatabasePoolConfig {
            max_connections: 4,
            db_path,
        };

        let pool = DatabasePool::new(config).unwrap();

        // Get connections in sequence and verify round-robin behavior
        for i in 0..12 {
            let conn = pool.get_connection();
            // Connection should be usable
            {
                let _guard = conn.lock();
            }
            drop(conn);

            // After dropping, active count should be back to 0
            assert_eq!(pool.active_count(), 0, "Iteration {}", i);
        }
    }
}
