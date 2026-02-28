// Schema parser for form validation, DB schemas, and state definitions

use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

// Pre-compiled regex patterns
static SCHEMA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"schema\s+(\w+)\s*\{([^}]+)\}").unwrap());
static FIELD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\w+)\s*:\s*(\w+)\s*((?:@\w+(?:\([^)]*\))?\s*)*)").unwrap());
static VALIDATOR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"@(\w+(?:\([^)]*\))?)").unwrap());
static QUERY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"query\s+(\w+)\s*\(([^)]*)\)\s*=>\s*(\w+)\s+([^\s]+)").unwrap());
static TABLE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"table\s+(\w+)\s*\{([^}]+)\}").unwrap());
static COL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\w+)\s*:\s*(\w+)\s*(nullable)?\s*(primaryKey)?").unwrap());
static STATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"state\s+(\w+)\s*\{([^}]+)\}").unwrap());
static STATE_FIELD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\w+)\s*:\s*(\w+)").unwrap());

/// Form field schema
#[derive(Debug, Clone)]
pub struct FieldSchema {
    pub name: String,
    pub field_type: String,
    pub validators: Vec<String>,
    pub required: bool,
}

/// Form schema
#[derive(Debug, Clone)]
pub struct FormSchema {
    pub name: String,
    pub fields: Vec<FieldSchema>,
}

/// Parse form schema from .dx file
pub fn parse_form_schema(source: &str) -> Vec<FormSchema> {
    let mut schemas = Vec::new();

    for cap in SCHEMA_RE.captures_iter(source) {
        let name = cap[1].to_string();
        let body = &cap[2];

        let mut fields = Vec::new();

        for field_cap in FIELD_RE.captures_iter(body) {
            let field_name = field_cap[1].to_string();
            let field_type = field_cap[2].to_string();
            let validators_str = field_cap.get(3).map_or("", |m| m.as_str());

            // Parse validators
            let validators: Vec<String> =
                VALIDATOR_RE.captures_iter(validators_str).map(|v| v[1].to_string()).collect();

            let required = validators.contains(&"required".to_string());

            fields.push(FieldSchema {
                name: field_name,
                field_type,
                validators,
                required,
            });
        }

        schemas.push(FormSchema { name, fields });
    }

    schemas
}

/// Query definition
#[derive(Debug, Clone)]
pub struct QueryDefinition {
    pub name: String,
    pub endpoint: String,
    pub method: String,
    pub params: Vec<String>,
}

/// Parse query definitions
pub fn parse_query_definitions(source: &str) -> Vec<QueryDefinition> {
    let mut queries = Vec::new();

    for cap in QUERY_RE.captures_iter(source) {
        queries.push(QueryDefinition {
            name: cap[1].to_string(),
            method: cap[3].to_string(),
            endpoint: cap[4].to_string(),
            params: cap[2]
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
        });
    }

    queries
}

/// Database table schema
#[derive(Debug, Clone)]
pub struct TableSchema {
    pub name: String,
    pub columns: Vec<ColumnSchema>,
}

#[derive(Debug, Clone)]
pub struct ColumnSchema {
    pub name: String,
    pub column_type: String,
    pub nullable: bool,
    pub primary_key: bool,
}

/// Parse database schema
pub fn parse_db_schema(source: &str) -> Vec<TableSchema> {
    let mut tables = Vec::new();

    for cap in TABLE_RE.captures_iter(source) {
        let name = cap[1].to_string();
        let body = &cap[2];

        let mut columns = Vec::new();

        for col_cap in COL_RE.captures_iter(body) {
            columns.push(ColumnSchema {
                name: col_cap[1].to_string(),
                column_type: col_cap[2].to_string(),
                nullable: col_cap.get(3).is_some(),
                primary_key: col_cap.get(4).is_some(),
            });
        }

        tables.push(TableSchema { name, columns });
    }

    tables
}

/// State definition
#[derive(Debug, Clone)]
pub struct StateDefinition {
    pub name: String,
    pub fields: HashMap<String, String>,
}

/// Parse state definitions
pub fn parse_state_definitions(source: &str) -> Vec<StateDefinition> {
    let mut states = Vec::new();

    for cap in STATE_RE.captures_iter(source) {
        let name = cap[1].to_string();
        let body = &cap[2];

        let mut fields = HashMap::new();

        for field_cap in STATE_FIELD_RE.captures_iter(body) {
            fields.insert(field_cap[1].to_string(), field_cap[2].to_string());
        }

        states.push(StateDefinition { name, fields });
    }

    states
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_form_schema() {
        let source = r#"
        schema LoginForm {
            email: string @email @required
            password: string @minLength(8) @required
        }
        "#;

        let schemas = parse_form_schema(source);
        assert_eq!(schemas.len(), 1);
        assert_eq!(schemas[0].name, "LoginForm");
        assert_eq!(schemas[0].fields.len(), 2);
        assert_eq!(schemas[0].fields[0].name, "email");
        assert!(schemas[0].fields[0].validators.contains(&"email".to_string()));
    }

    #[test]
    fn test_parse_query_definitions() {
        let source = r#"
        query getUser(id) => GET /api/users/:id
        query createPost(title, body) => POST /api/posts
        "#;

        let queries = parse_query_definitions(source);
        assert_eq!(queries.len(), 2);
        assert_eq!(queries[0].name, "getUser");
        assert_eq!(queries[0].method, "GET");
    }

    #[test]
    fn test_parse_db_schema() {
        let source = r#"
        table users {
            id: integer primaryKey
            email: string
            name: string nullable
        }
        "#;

        let tables = parse_db_schema(source);
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].name, "users");
        assert_eq!(tables[0].columns.len(), 3);
        assert!(tables[0].columns[0].primary_key);
    }

    #[test]
    fn test_parse_state_definitions() {
        let source = r#"
        state AppState {
            count: number
            user: string
        }
        "#;

        let states = parse_state_definitions(source);
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].name, "AppState");
        assert_eq!(states[0].fields.len(), 2);
    }
}
