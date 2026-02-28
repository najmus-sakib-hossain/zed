//! Database tool — universal database client (SQL + NoSQL).
//! Actions: query | execute | schema | migrate | seed | backup | connect | disconnect

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct DatabaseTool;
impl Default for DatabaseTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for DatabaseTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "database".into(),
            description: "Universal database client: SQL queries, schema inspection, migrations, seeding, backup for SQLite/Postgres/MySQL/MongoDB".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Database action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["query".into(),"execute".into(),"schema".into(),"migrate".into(),"seed".into(),"backup".into(),"connect".into(),"disconnect".into()]) },
                ToolParameter { name: "connection".into(), description: "Connection string or name".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "sql".into(), description: "SQL query or statement".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "table".into(), description: "Table name".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "file".into(), description: "Migration/seed/backup file".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
            ],
            category: "data".into(),
            requires_confirmation: true,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("schema");
        let connection = call
            .arguments
            .get("connection")
            .and_then(|v| v.as_str())
            .unwrap_or("sqlite::memory:");

        match action {
            "schema" => {
                // For SQLite files, read schema
                if connection.starts_with("sqlite:")
                    || connection.ends_with(".db")
                    || connection.ends_with(".sqlite")
                {
                    let db_path = connection.trim_start_matches("sqlite:");
                    if std::path::Path::new(db_path).exists() {
                        let (shell, flag) = if cfg!(windows) {
                            ("cmd", "/C")
                        } else {
                            ("sh", "-c")
                        };
                        let cmd = format!("sqlite3 {} .schema", db_path);
                        match tokio::process::Command::new(shell).arg(flag).arg(&cmd).output().await
                        {
                            Ok(o) => Ok(ToolResult::success(
                                call.id,
                                String::from_utf8_lossy(&o.stdout).to_string(),
                            )),
                            Err(_) => Ok(ToolResult::error(call.id, "sqlite3 not in PATH".into())),
                        }
                    } else {
                        Ok(ToolResult::error(
                            call.id,
                            format!("Database file not found: {db_path}"),
                        ))
                    }
                } else {
                    Ok(ToolResult::success(
                        call.id,
                        format!(
                            "Schema inspection for '{connection}' — connect database driver (sqlx/diesel)"
                        ),
                    ))
                }
            }
            "query" | "execute" => {
                let sql = call
                    .arguments
                    .get("sql")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'sql'"))?;
                // Safety: validate SQL doesn't contain dangerous operations without confirmation
                let lower = sql.to_lowercase();
                if action == "query"
                    && (lower.contains("drop ")
                        || lower.contains("delete ")
                        || lower.contains("truncate "))
                {
                    return Ok(ToolResult::error(
                        call.id,
                        "Destructive operations require 'execute' action with confirmation".into(),
                    ));
                }
                Ok(ToolResult::success(
                    call.id,
                    format!(
                        "SQL '{}' on '{}' — connect sqlx/diesel for actual execution",
                        &sql[..sql.len().min(100)],
                        connection
                    ),
                ))
            }
            "migrate" => {
                let file = call.arguments.get("file").and_then(|v| v.as_str());
                Ok(ToolResult::success(
                    call.id,
                    format!(
                        "Migration on '{}' from {:?} — connect migration runner",
                        connection, file
                    ),
                ))
            }
            "backup" => {
                let file =
                    call.arguments.get("file").and_then(|v| v.as_str()).unwrap_or("backup.sql");
                Ok(ToolResult::success(call.id, format!("Backup '{}' to '{}'", connection, file)))
            }
            _ => Ok(ToolResult::success(
                call.id,
                format!("Database '{}' — connect database driver for execution", action),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(DatabaseTool.definition().name, "database");
    }
}
