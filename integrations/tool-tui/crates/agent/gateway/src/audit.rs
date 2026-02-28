//! Structured audit logging for security-relevant events.
//!
//! Records authentication attempts, access control decisions,
//! configuration changes, and other security events to a persistent
//! append-only log backed by SQLite.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Categories of auditable events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditCategory {
    /// Authentication (login, logout, token refresh, failures)
    Auth,
    /// Access control decisions (allow/deny)
    Access,
    /// Configuration changes
    Config,
    /// Session lifecycle (create, destroy, timeout)
    Session,
    /// Channel events (connect, disconnect, message)
    Channel,
    /// Admin actions
    Admin,
    /// Rate limiting events
    RateLimit,
    /// Security alerts (brute-force, anomalies)
    Alert,
}

impl std::fmt::Display for AuditCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auth => write!(f, "auth"),
            Self::Access => write!(f, "access"),
            Self::Config => write!(f, "config"),
            Self::Session => write!(f, "session"),
            Self::Channel => write!(f, "channel"),
            Self::Admin => write!(f, "admin"),
            Self::RateLimit => write!(f, "rate_limit"),
            Self::Alert => write!(f, "alert"),
        }
    }
}

/// Severity level for audit events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditSeverity {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for AuditSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

/// A single audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Auto-incremented ID (set by DB)
    pub id: Option<i64>,
    /// Timestamp of the event
    pub timestamp: DateTime<Utc>,
    /// Event category
    pub category: AuditCategory,
    /// Severity level
    pub severity: AuditSeverity,
    /// Human-readable action description
    pub action: String,
    /// Actor (user ID, IP address, or system component)
    pub actor: String,
    /// Target resource (session ID, channel name, config key)
    pub target: Option<String>,
    /// Whether the action succeeded
    pub success: bool,
    /// Additional details as JSON
    pub details: Option<String>,
    /// Source IP address
    pub source_ip: Option<String>,
}

/// Audit logger backed by SQLite
pub struct AuditLogger {
    db: Arc<Mutex<Connection>>,
    path: PathBuf,
}

impl AuditLogger {
    /// Create a new audit logger, creating the database if needed
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;

        // Create the audit_log table (append-only)
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS audit_log (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp   TEXT NOT NULL,
                category    TEXT NOT NULL,
                severity    TEXT NOT NULL,
                action      TEXT NOT NULL,
                actor       TEXT NOT NULL,
                target      TEXT,
                success     INTEGER NOT NULL DEFAULT 1,
                details     TEXT,
                source_ip   TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp);
            CREATE INDEX IF NOT EXISTS idx_audit_category ON audit_log(category);
            CREATE INDEX IF NOT EXISTS idx_audit_actor ON audit_log(actor);
            CREATE INDEX IF NOT EXISTS idx_audit_severity ON audit_log(severity);",
        )?;

        // Enable WAL mode for better concurrent read performance
        conn.pragma_update(None, "journal_mode", "WAL")?;

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            path: path.to_path_buf(),
        })
    }

    /// Log an audit event
    pub async fn log(&self, entry: &AuditEntry) -> anyhow::Result<()> {
        let db = self.db.lock().await;
        db.execute(
            "INSERT INTO audit_log (timestamp, category, severity, action, actor, target, success, details, source_ip)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                entry.timestamp.to_rfc3339(),
                entry.category.to_string(),
                entry.severity.to_string(),
                entry.action,
                entry.actor,
                entry.target,
                entry.success as i32,
                entry.details,
                entry.source_ip,
            ],
        )?;

        // Also emit as tracing event for real-time monitoring
        tracing::info!(
            category = %entry.category,
            severity = %entry.severity,
            action = %entry.action,
            actor = %entry.actor,
            success = entry.success,
            "AUDIT: {}",
            entry.action
        );

        Ok(())
    }

    /// Query audit entries with filters
    pub async fn query(&self, filter: &AuditFilter) -> anyhow::Result<Vec<AuditEntry>> {
        let db = self.db.lock().await;

        let mut sql = String::from(
            "SELECT id, timestamp, category, severity, action, actor, target, success, details, source_ip FROM audit_log WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref cat) = filter.category {
            sql.push_str(&format!(" AND category = ?{}", param_values.len() + 1));
            param_values.push(Box::new(cat.to_string()));
        }
        if let Some(ref sev) = filter.severity {
            sql.push_str(&format!(" AND severity = ?{}", param_values.len() + 1));
            param_values.push(Box::new(sev.to_string()));
        }
        if let Some(ref actor) = filter.actor {
            sql.push_str(&format!(" AND actor = ?{}", param_values.len() + 1));
            param_values.push(Box::new(actor.clone()));
        }
        if let Some(ref since) = filter.since {
            sql.push_str(&format!(" AND timestamp >= ?{}", param_values.len() + 1));
            param_values.push(Box::new(since.to_rfc3339()));
        }
        if let Some(ref until) = filter.until {
            sql.push_str(&format!(" AND timestamp <= ?{}", param_values.len() + 1));
            param_values.push(Box::new(until.to_rfc3339()));
        }
        if filter.failures_only {
            sql.push_str(" AND success = 0");
        }

        sql.push_str(" ORDER BY timestamp DESC");

        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = db.prepare(&sql)?;
        let entries = stmt
            .query_map(params_refs.as_slice(), |row| {
                let category_str: String = row.get(2)?;
                let severity_str: String = row.get(3)?;
                let success_int: i32 = row.get(7)?;

                Ok(AuditEntry {
                    id: Some(row.get(0)?),
                    timestamp: row
                        .get::<_, String>(1)
                        .map(|s| {
                            DateTime::parse_from_rfc3339(&s)
                                .map(|dt| dt.with_timezone(&Utc))
                                .unwrap_or_else(|_| Utc::now())
                        })
                        .unwrap_or_else(|_| Utc::now()),
                    category: match category_str.as_str() {
                        "auth" => AuditCategory::Auth,
                        "access" => AuditCategory::Access,
                        "config" => AuditCategory::Config,
                        "session" => AuditCategory::Session,
                        "channel" => AuditCategory::Channel,
                        "admin" => AuditCategory::Admin,
                        "rate_limit" => AuditCategory::RateLimit,
                        "alert" => AuditCategory::Alert,
                        _ => AuditCategory::Admin,
                    },
                    severity: match severity_str.as_str() {
                        "info" => AuditSeverity::Info,
                        "warning" => AuditSeverity::Warning,
                        "critical" => AuditSeverity::Critical,
                        _ => AuditSeverity::Info,
                    },
                    action: row.get(4)?,
                    actor: row.get(5)?,
                    target: row.get(6)?,
                    success: success_int != 0,
                    details: row.get(8)?,
                    source_ip: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// Count entries matching a filter (useful for rate-limit checks)
    pub async fn count(
        &self,
        category: AuditCategory,
        actor: &str,
        since: DateTime<Utc>,
    ) -> anyhow::Result<usize> {
        let db = self.db.lock().await;
        let count: i64 = db.query_row(
            "SELECT COUNT(*) FROM audit_log WHERE category = ?1 AND actor = ?2 AND timestamp >= ?3",
            params![category.to_string(), actor, since.to_rfc3339()],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Prune old entries (retention policy)
    pub async fn prune(&self, older_than: DateTime<Utc>) -> anyhow::Result<usize> {
        let db = self.db.lock().await;
        let deleted = db.execute(
            "DELETE FROM audit_log WHERE timestamp < ?1",
            params![older_than.to_rfc3339()],
        )?;
        Ok(deleted)
    }

    /// Get the database path
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Filter for querying audit log entries
#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    pub category: Option<AuditCategory>,
    pub severity: Option<AuditSeverity>,
    pub actor: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub failures_only: bool,
    pub limit: Option<usize>,
}

/// Helper to quickly build audit entries
pub fn audit_entry(
    category: AuditCategory,
    severity: AuditSeverity,
    action: impl Into<String>,
    actor: impl Into<String>,
) -> AuditEntry {
    AuditEntry {
        id: None,
        timestamp: Utc::now(),
        category,
        severity,
        action: action.into(),
        actor: actor.into(),
        target: None,
        success: true,
        details: None,
        source_ip: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn test_logger() -> AuditLogger {
        let tmp = NamedTempFile::new().unwrap();
        AuditLogger::new(tmp.path()).unwrap()
    }

    #[tokio::test]
    async fn test_log_and_query() {
        let logger = test_logger();

        let entry = AuditEntry {
            id: None,
            timestamp: Utc::now(),
            category: AuditCategory::Auth,
            severity: AuditSeverity::Info,
            action: "login_success".into(),
            actor: "user@example.com".into(),
            target: Some("session-123".into()),
            success: true,
            details: Some(r#"{"method":"token"}"#.into()),
            source_ip: Some("127.0.0.1".into()),
        };

        logger.log(&entry).await.unwrap();

        let results = logger
            .query(&AuditFilter {
                category: Some(AuditCategory::Auth),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action, "login_success");
        assert_eq!(results[0].actor, "user@example.com");
        assert!(results[0].success);
    }

    #[tokio::test]
    async fn test_failures_only_filter() {
        let logger = test_logger();

        // Success entry
        let mut entry = audit_entry(AuditCategory::Auth, AuditSeverity::Info, "login_ok", "user1");
        entry.success = true;
        logger.log(&entry).await.unwrap();

        // Failure entry
        let mut entry =
            audit_entry(AuditCategory::Auth, AuditSeverity::Warning, "login_failed", "user2");
        entry.success = false;
        logger.log(&entry).await.unwrap();

        let results = logger
            .query(&AuditFilter {
                failures_only: true,
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].actor, "user2");
        assert!(!results[0].success);
    }

    #[tokio::test]
    async fn test_count() {
        let logger = test_logger();

        for i in 0..5 {
            let entry = audit_entry(
                AuditCategory::Auth,
                AuditSeverity::Warning,
                format!("attempt_{}", i),
                "attacker",
            );
            logger.log(&entry).await.unwrap();
        }

        let count = logger
            .count(AuditCategory::Auth, "attacker", Utc::now() - chrono::Duration::minutes(1))
            .await
            .unwrap();

        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn test_prune() {
        let logger = test_logger();

        let entry = audit_entry(AuditCategory::Session, AuditSeverity::Info, "created", "user1");
        logger.log(&entry).await.unwrap();

        // Prune everything older than the future
        let pruned = logger.prune(Utc::now() + chrono::Duration::hours(1)).await.unwrap();
        assert_eq!(pruned, 1);

        let results = logger.query(&AuditFilter::default()).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_audit_entry_helper() {
        let entry =
            audit_entry(AuditCategory::Config, AuditSeverity::Critical, "key_rotated", "admin");
        assert_eq!(entry.category, AuditCategory::Config);
        assert_eq!(entry.severity, AuditSeverity::Critical);
        assert!(entry.success);
        assert!(entry.id.is_none());
    }
}
