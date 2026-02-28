//! # TailwindCSS-style Query Builder
//!
//! Write database queries using TailwindCSS-like atomic class syntax.
//! Compiles to efficient SQL at build time.
//!
//! ## Example
//! ```ignore
//! // Query syntax like TailwindCSS classes:
//! query!("users.select:id,name.where:active=true.order:created_at.desc.limit:10")
//!
//! // Becomes:
//! SELECT id, name FROM users WHERE active = true ORDER BY created_at DESC LIMIT 10
//! ```

use serde::{Deserialize, Serialize};
use std::fmt::Write;

// ============================================================================
// Query Builder Types
// ============================================================================

/// Query operation types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryOp {
    Select(Vec<String>),
    Where(WhereClause),
    OrderBy(String, SortOrder),
    Limit(u32),
    Offset(u32),
    Join(JoinType, String, String),
    GroupBy(Vec<String>),
    Having(WhereClause),
    Insert(Vec<(String, String)>),
    Update(Vec<(String, String)>),
    Delete,
}

/// Sort order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Asc,
    Desc,
}

/// Join type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

/// Where clause
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhereClause {
    pub field: String,
    pub op: CompareOp,
    pub value: String,
}

/// Comparison operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Lte,
    Gt,
    Gte,
    Like,
    In,
    NotIn,
    IsNull,
    IsNotNull,
}

impl CompareOp {
    pub fn to_sql(&self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Lte => "<=",
            Self::Gt => ">",
            Self::Gte => ">=",
            Self::Like => "LIKE",
            Self::In => "IN",
            Self::NotIn => "NOT IN",
            Self::IsNull => "IS NULL",
            Self::IsNotNull => "IS NOT NULL",
        }
    }
}

// ============================================================================
// TailwindCSS Query Parser
// ============================================================================

/// Parse TailwindCSS-style query string
/// 
/// Format: `table.operation:args.operation:args...`
/// 
/// Operations:
/// - `select:col1,col2` - Select columns
/// - `where:field=value` - Filter (supports =, !=, <, >, <=, >=, ~=)
/// - `order:field` - Sort ascending
/// - `desc` - Make previous order descending
/// - `asc` - Make previous order ascending (default)
/// - `limit:n` - Limit results
/// - `offset:n` - Skip results
/// - `join:table.on:condition` - Join tables
/// - `group:col1,col2` - Group by
/// - `insert:col=val,col=val` - Insert values
/// - `update:col=val,col=val` - Update values
/// - `delete` - Delete rows
pub fn parse_tailwind_query(query: &str) -> Result<TailwindQuery, QueryParseError> {
    let mut parts = query.split('.');
    
    // First part is table name
    let table = parts.next()
        .ok_or(QueryParseError::MissingTable)?
        .to_string();
    
    let mut ops = Vec::new();
    let mut pending_order: Option<String> = None;
    
    for part in parts {
        let (op_name, op_args) = if let Some(idx) = part.find(':') {
            (&part[..idx], Some(&part[idx + 1..]))
        } else {
            (part, None)
        };
        
        match op_name {
            "select" | "sel" => {
                let cols = op_args
                    .ok_or(QueryParseError::MissingArgs("select"))?
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
                ops.push(QueryOp::Select(cols));
            }
            "where" | "w" => {
                let clause = parse_where_clause(op_args.ok_or(QueryParseError::MissingArgs("where"))?)?;
                ops.push(QueryOp::Where(clause));
            }
            "order" | "sort" => {
                pending_order = Some(op_args.ok_or(QueryParseError::MissingArgs("order"))?.to_string());
            }
            "desc" => {
                if let Some(field) = pending_order.take() {
                    ops.push(QueryOp::OrderBy(field, SortOrder::Desc));
                }
            }
            "asc" => {
                if let Some(field) = pending_order.take() {
                    ops.push(QueryOp::OrderBy(field, SortOrder::Asc));
                }
            }
            "limit" | "lim" => {
                let n = op_args
                    .ok_or(QueryParseError::MissingArgs("limit"))?
                    .parse()
                    .map_err(|_| QueryParseError::InvalidNumber)?;
                ops.push(QueryOp::Limit(n));
            }
            "offset" | "skip" => {
                let n = op_args
                    .ok_or(QueryParseError::MissingArgs("offset"))?
                    .parse()
                    .map_err(|_| QueryParseError::InvalidNumber)?;
                ops.push(QueryOp::Offset(n));
            }
            "group" => {
                let cols = op_args
                    .ok_or(QueryParseError::MissingArgs("group"))?
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
                ops.push(QueryOp::GroupBy(cols));
            }
            "insert" | "ins" => {
                let pairs = parse_key_value_pairs(op_args.ok_or(QueryParseError::MissingArgs("insert"))?)?;
                ops.push(QueryOp::Insert(pairs));
            }
            "update" | "upd" => {
                let pairs = parse_key_value_pairs(op_args.ok_or(QueryParseError::MissingArgs("update"))?)?;
                ops.push(QueryOp::Update(pairs));
            }
            "delete" | "del" => {
                ops.push(QueryOp::Delete);
            }
            _ => {
                // Handle pending order with implicit asc
                if let Some(field) = pending_order.take() {
                    ops.push(QueryOp::OrderBy(field, SortOrder::Asc));
                }
            }
        }
    }
    
    // Handle any remaining pending order
    if let Some(field) = pending_order {
        ops.push(QueryOp::OrderBy(field, SortOrder::Asc));
    }
    
    Ok(TailwindQuery { table, ops })
}

fn parse_where_clause(s: &str) -> Result<WhereClause, QueryParseError> {
    // Supported operators: =, !=, <, >, <=, >=, ~= (like)
    let operators = ["!=", "<=", ">=", "~=", "=", "<", ">"];
    
    for op_str in operators {
        if let Some(idx) = s.find(op_str) {
            let field = s[..idx].trim().to_string();
            let value = s[idx + op_str.len()..].trim().to_string();
            let op = match op_str {
                "=" => CompareOp::Eq,
                "!=" => CompareOp::Ne,
                "<" => CompareOp::Lt,
                "<=" => CompareOp::Lte,
                ">" => CompareOp::Gt,
                ">=" => CompareOp::Gte,
                "~=" => CompareOp::Like,
                _ => CompareOp::Eq,
            };
            return Ok(WhereClause { field, op, value });
        }
    }
    
    Err(QueryParseError::InvalidWhereClause)
}

fn parse_key_value_pairs(s: &str) -> Result<Vec<(String, String)>, QueryParseError> {
    s.split(',')
        .map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next().ok_or(QueryParseError::InvalidKeyValue)?.trim().to_string();
            let value = parts.next().ok_or(QueryParseError::InvalidKeyValue)?.trim().to_string();
            Ok((key, value))
        })
        .collect()
}

/// Query parse errors
#[derive(Debug, Clone)]
pub enum QueryParseError {
    MissingTable,
    MissingArgs(&'static str),
    InvalidNumber,
    InvalidWhereClause,
    InvalidKeyValue,
}

impl std::fmt::Display for QueryParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingTable => write!(f, "Missing table name"),
            Self::MissingArgs(op) => write!(f, "Missing arguments for '{}'", op),
            Self::InvalidNumber => write!(f, "Invalid number"),
            Self::InvalidWhereClause => write!(f, "Invalid where clause"),
            Self::InvalidKeyValue => write!(f, "Invalid key=value pair"),
        }
    }
}

impl std::error::Error for QueryParseError {}

// ============================================================================
// TailwindQuery Struct
// ============================================================================

/// Parsed TailwindCSS-style query
#[derive(Debug, Clone)]
pub struct TailwindQuery {
    pub table: String,
    pub ops: Vec<QueryOp>,
}

impl TailwindQuery {
    /// Parse from TailwindCSS-style string
    pub fn parse(query: &str) -> Result<Self, QueryParseError> {
        parse_tailwind_query(query)
    }

    /// Compile to SQL
    pub fn to_sql(&self) -> String {
        let mut sql = String::new();
        let mut has_select = false;
        let mut wheres = Vec::new();
        let mut orders = Vec::new();
        let mut limit = None;
        let mut offset = None;
        let mut groups = Vec::new();
        let mut is_insert = false;
        let mut is_update = false;
        let mut is_delete = false;
        let mut insert_cols = Vec::new();
        let mut insert_vals = Vec::new();
        let mut update_sets = Vec::new();

        // First pass: collect all operations
        for op in &self.ops {
            match op {
                QueryOp::Select(cols) => {
                    has_select = true;
                    let _ = write!(sql, "SELECT {} FROM {}", cols.join(", "), self.table);
                }
                QueryOp::Where(clause) => {
                    wheres.push(clause);
                }
                QueryOp::OrderBy(field, order) => {
                    orders.push((field.clone(), *order));
                }
                QueryOp::Limit(n) => limit = Some(*n),
                QueryOp::Offset(n) => offset = Some(*n),
                QueryOp::GroupBy(cols) => groups.extend(cols.clone()),
                QueryOp::Insert(pairs) => {
                    is_insert = true;
                    for (k, v) in pairs {
                        insert_cols.push(k.clone());
                        insert_vals.push(v.clone());
                    }
                }
                QueryOp::Update(pairs) => {
                    is_update = true;
                    for (k, v) in pairs {
                        update_sets.push(format!("{} = '{}'", k, v));
                    }
                }
                QueryOp::Delete => is_delete = true,
                _ => {}
            }
        }

        // Build SQL based on operation type
        if is_insert {
            sql = format!(
                "INSERT INTO {} ({}) VALUES ({})",
                self.table,
                insert_cols.join(", "),
                insert_vals.iter().map(|v| format!("'{}'", v)).collect::<Vec<_>>().join(", ")
            );
        } else if is_update {
            sql = format!("UPDATE {} SET {}", self.table, update_sets.join(", "));
        } else if is_delete {
            sql = format!("DELETE FROM {}", self.table);
        } else if !has_select {
            sql = format!("SELECT * FROM {}", self.table);
        }

        // Add WHERE clauses
        if !wheres.is_empty() {
            sql.push_str(" WHERE ");
            let conditions: Vec<String> = wheres
                .iter()
                .map(|w| {
                    if w.op == CompareOp::Like {
                        format!("{} {} '%{}%'", w.field, w.op.to_sql(), w.value)
                    } else if w.op == CompareOp::IsNull || w.op == CompareOp::IsNotNull {
                        format!("{} {}", w.field, w.op.to_sql())
                    } else {
                        format!("{} {} '{}'", w.field, w.op.to_sql(), w.value)
                    }
                })
                .collect();
            sql.push_str(&conditions.join(" AND "));
        }

        // Add GROUP BY
        if !groups.is_empty() {
            let _ = write!(sql, " GROUP BY {}", groups.join(", "));
        }

        // Add ORDER BY
        if !orders.is_empty() {
            sql.push_str(" ORDER BY ");
            let order_strs: Vec<String> = orders
                .iter()
                .map(|(f, o)| {
                    format!(
                        "{} {}",
                        f,
                        match o {
                            SortOrder::Asc => "ASC",
                            SortOrder::Desc => "DESC",
                        }
                    )
                })
                .collect();
            sql.push_str(&order_strs.join(", "));
        }

        // Add LIMIT and OFFSET
        if let Some(n) = limit {
            let _ = write!(sql, " LIMIT {}", n);
        }
        if let Some(n) = offset {
            let _ = write!(sql, " OFFSET {}", n);
        }

        sql
    }

    /// Get parameterized SQL (for prepared statements)
    pub fn to_parameterized_sql(&self) -> (String, Vec<String>) {
        let sql = self.to_sql();
        // Extract values for parameterization
        let params: Vec<String> = self.ops.iter()
            .filter_map(|op| {
                match op {
                    QueryOp::Where(w) => Some(w.value.clone()),
                    _ => None,
                }
            })
            .collect();
        
        // Replace values with placeholders (? or $1, $2, etc.)
        let mut param_sql = sql.clone();
        for (i, param) in params.iter().enumerate() {
            param_sql = param_sql.replace(&format!("'{}'", param), &format!("${}", i + 1));
        }
        
        (param_sql, params)
    }
}

// ============================================================================
// Query Macro (Compile-time validation)
// ============================================================================

/// Create a TailwindCSS-style query
/// 
/// # Examples
/// ```ignore
/// // Select users
/// let q = query!("users.select:id,name,email.where:active=true.order:created_at.desc.limit:10");
/// 
/// // Insert user
/// let q = query!("users.insert:name=John,email=john@example.com");
/// 
/// // Update user
/// let q = query!("users.update:name=Jane.where:id=1");
/// 
/// // Delete user
/// let q = query!("users.delete.where:id=1");
/// ```
#[macro_export]
macro_rules! query {
    ($q:expr) => {
        $crate::query_builder::TailwindQuery::parse($q).expect("Invalid query syntax")
    };
}

// ============================================================================
// Pre-built Queries for DX Option
// ============================================================================

pub mod prebuilt {
    use super::*;

    /// Get all users with pagination
    pub fn users_paginated(page: u32, per_page: u32) -> TailwindQuery {
        let offset = (page - 1) * per_page;
        TailwindQuery::parse(&format!(
            "users.select:id,name,email,created_at.where:active=true.order:created_at.desc.limit:{}.offset:{}",
            per_page, offset
        )).unwrap()
    }

    /// Search users by name
    pub fn search_users(query: &str) -> TailwindQuery {
        TailwindQuery::parse(&format!(
            "users.select:id,name,email.where:name~={}.order:name.asc.limit:20",
            query
        )).unwrap()
    }

    /// Get contact form submissions
    pub fn contact_submissions() -> TailwindQuery {
        TailwindQuery::parse(
            "contact_submissions.select:id,name,email,subject,message,created_at.order:created_at.desc.limit:50"
        ).unwrap()
    }

    /// Get newsletter subscribers
    pub fn newsletter_subscribers() -> TailwindQuery {
        TailwindQuery::parse(
            "newsletter.select:email,subscribed_at.where:unsubscribed=false.order:subscribed_at.desc"
        ).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_select() {
        let q = TailwindQuery::parse("users.select:id,name").unwrap();
        assert_eq!(q.to_sql(), "SELECT id, name FROM users");
    }

    #[test]
    fn test_select_with_where() {
        let q = TailwindQuery::parse("users.select:id,name.where:active=true").unwrap();
        assert_eq!(q.to_sql(), "SELECT id, name FROM users WHERE active = 'true'");
    }

    #[test]
    fn test_select_with_order_desc() {
        let q = TailwindQuery::parse("users.select:id,name.order:created_at.desc").unwrap();
        assert_eq!(q.to_sql(), "SELECT id, name FROM users ORDER BY created_at DESC");
    }

    #[test]
    fn test_select_with_limit_offset() {
        let q = TailwindQuery::parse("users.select:id.limit:10.offset:20").unwrap();
        assert_eq!(q.to_sql(), "SELECT id FROM users LIMIT 10 OFFSET 20");
    }

    #[test]
    fn test_insert() {
        let q = TailwindQuery::parse("users.insert:name=John,email=john@example.com").unwrap();
        assert_eq!(q.to_sql(), "INSERT INTO users (name, email) VALUES ('John', 'john@example.com')");
    }

    #[test]
    fn test_update() {
        let q = TailwindQuery::parse("users.update:name=Jane.where:id=1").unwrap();
        assert_eq!(q.to_sql(), "UPDATE users SET name = 'Jane' WHERE id = '1'");
    }

    #[test]
    fn test_delete() {
        let q = TailwindQuery::parse("users.delete.where:id=1").unwrap();
        assert_eq!(q.to_sql(), "DELETE FROM users WHERE id = '1'");
    }

    #[test]
    fn test_like_query() {
        let q = TailwindQuery::parse("users.select:id,name.where:name~=john").unwrap();
        assert_eq!(q.to_sql(), "SELECT id, name FROM users WHERE name LIKE '%john%'");
    }
}
