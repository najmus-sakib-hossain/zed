//! Audit logging for AI changes

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

use super::AuditAction;

/// Audit entry
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub timestamp: u64,
    pub action: String,
    pub change_id: String,
    pub user: String,
    pub reason: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Get audit log path
fn audit_log_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("dx")
        .join("audit.log")
}

/// Log an audit entry
pub fn log(entry: AuditEntry) -> Result<()> {
    let path = audit_log_path();

    // Ensure directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Append to log file
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&path)?;

    // Write as simple TSV format
    writeln!(
        file,
        "{}\t{}\t{}\t{}\t{}",
        entry.timestamp,
        entry.action,
        entry.change_id,
        entry.user,
        entry.reason.unwrap_or_default()
    )?;

    Ok(())
}

/// Query audit log
pub fn query(
    from: Option<&str>,
    to: Option<&str>,
    action: Option<AuditAction>,
) -> Result<Vec<AuditEntry>> {
    let path = audit_log_path();

    if !path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&path)?;
    let mut entries = Vec::new();

    // Parse from/to timestamps
    let from_ts = from.and_then(|s| parse_date(s));
    let to_ts = to.and_then(|s| parse_date(s));

    for line in content.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 4 {
            continue;
        }

        let timestamp: u64 = parts[0].parse().unwrap_or(0);

        // Filter by time range
        if let Some(from) = from_ts {
            if timestamp < from {
                continue;
            }
        }
        if let Some(to) = to_ts {
            if timestamp > to {
                continue;
            }
        }

        // Filter by action
        if let Some(ref action_filter) = action {
            let action_str = match action_filter {
                AuditAction::Proposed => "proposed",
                AuditAction::Validated => "validated",
                AuditAction::Approved => "approved",
                AuditAction::Rejected => "rejected",
                AuditAction::Applied => "applied",
                AuditAction::RolledBack => "rolledback",
            };
            if parts[1] != action_str {
                continue;
            }
        }

        entries.push(AuditEntry {
            timestamp,
            action: parts[1].to_string(),
            change_id: parts[2].to_string(),
            user: parts[3].to_string(),
            reason: parts.get(4).filter(|s| !s.is_empty()).map(|s| s.to_string()),
            metadata: HashMap::new(),
        });
    }

    // Sort by timestamp descending
    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(entries)
}

/// Parse date string to timestamp
fn parse_date(s: &str) -> Option<u64> {
    // Expected format: YYYY-MM-DD
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }

    let year: u32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;

    // Simple calculation (not accounting for leap years, etc.)
    let days_since_epoch = (year - 1970) * 365 + (month - 1) * 30 + day;
    Some((days_since_epoch as u64) * 86400)
}

/// Get audit statistics
pub fn stats(from: Option<&str>, to: Option<&str>) -> Result<AuditStats> {
    let entries = query(from, to, None)?;

    let mut stats = AuditStats::default();

    for entry in &entries {
        stats.total += 1;

        match entry.action.as_str() {
            "proposed" => stats.proposed += 1,
            "validated" => stats.validated += 1,
            "approved" => stats.approved += 1,
            "rejected" => stats.rejected += 1,
            "applied" => stats.applied += 1,
            "rolledback" => stats.rolled_back += 1,
            _ => {}
        }
    }

    if stats.proposed > 0 {
        stats.approval_rate = (stats.approved as f32 / stats.proposed as f32) * 100.0;
    }

    Ok(stats)
}

/// Audit statistics
#[derive(Debug, Default)]
pub struct AuditStats {
    pub total: u32,
    pub proposed: u32,
    pub validated: u32,
    pub approved: u32,
    pub rejected: u32,
    pub applied: u32,
    pub rolled_back: u32,
    pub approval_rate: f32,
}

/// Clean up old audit entries
pub fn cleanup(retention_days: u32) -> Result<u32> {
    let path = audit_log_path();

    if !path.exists() {
        return Ok(0);
    }

    let content = std::fs::read_to_string(&path)?;
    let cutoff = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - (retention_days as u64 * 86400);

    let mut kept = Vec::new();
    let mut removed = 0u32;

    for line in content.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.is_empty() {
            continue;
        }

        let timestamp: u64 = parts[0].parse().unwrap_or(0);

        if timestamp >= cutoff {
            kept.push(line);
        } else {
            removed += 1;
        }
    }

    // Write back
    std::fs::write(&path, kept.join("\n"))?;

    Ok(removed)
}

/// Export audit log to different formats
pub fn export(format: &str, output: &PathBuf) -> Result<()> {
    let entries = query(None, None, None)?;

    let content = match format {
        "json" => export_json(&entries),
        "csv" => export_csv(&entries),
        _ => anyhow::bail!("Unknown format: {}", format),
    };

    std::fs::write(output, content)?;

    Ok(())
}

fn export_json(entries: &[AuditEntry]) -> String {
    let mut output = String::from("[\n");

    for (i, entry) in entries.iter().enumerate() {
        let comma = if i < entries.len() - 1 { "," } else { "" };
        output.push_str(&format!(
            r#"  {{"timestamp":{},"action":"{}","change_id":"{}","user":"{}"}}{}"#,
            entry.timestamp, entry.action, entry.change_id, entry.user, comma
        ));
        output.push('\n');
    }

    output.push_str("]\n");
    output
}

fn export_csv(entries: &[AuditEntry]) -> String {
    let mut output = String::from("timestamp,action,change_id,user,reason\n");

    for entry in entries {
        output.push_str(&format!(
            "{},{},{},{},{}\n",
            entry.timestamp,
            entry.action,
            entry.change_id,
            entry.user,
            entry.reason.as_deref().unwrap_or("")
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date() {
        let ts = parse_date("2024-01-15").unwrap();
        assert!(ts > 0);
    }
}
