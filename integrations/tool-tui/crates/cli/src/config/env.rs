//! Environment Variable Substitution
//!
//! Replaces `${VAR}` and `${VAR:-default}` patterns in configuration values.
//! Supports recursive substitution and nested defaults.

use std::collections::HashMap;
use std::env;

use regex::Regex;

/// Substitute environment variables in a string.
///
/// Supports patterns:
/// - `${VAR}` - replaced with env var value, error if not set
/// - `${VAR:-default}` - replaced with env var value, or default if not set
/// - `${VAR:?error message}` - replaced with env var value, error with message if not set
///
/// # Examples
/// ```ignore
/// let result = substitute_env_vars("host: ${HOST:-localhost}");
/// assert!(result.is_ok());
/// ```
pub fn substitute_env_vars(input: &str) -> Result<String, EnvSubstError> {
    substitute_env_vars_with_map(input, &env_to_map())
}

/// Substitute environment variables using a provided map (for testing)
pub fn substitute_env_vars_with_map(
    input: &str,
    env_map: &HashMap<String, String>,
) -> Result<String, EnvSubstError> {
    let re = Regex::new(r"\$\{([^}]+)\}").expect("Invalid regex");
    let mut result = input.to_string();
    let mut iterations = 0;
    let max_iterations = 10; // Prevent infinite recursion

    loop {
        let previous = result.clone();
        result = re
            .replace_all(&result, |caps: &regex::Captures| {
                let expr = &caps[1];
                resolve_var_expression(expr, env_map).unwrap_or_else(|_| caps[0].to_string())
            })
            .to_string();

        iterations += 1;
        if result == previous || iterations >= max_iterations {
            break;
        }
    }

    // Check for any remaining unresolved required variables
    for cap in re.captures_iter(&result) {
        let expr = &cap[1];
        // If it still contains ${...}, check if it was required
        if expr.contains(":?") {
            let parts: Vec<&str> = expr.splitn(2, ":?").collect();
            let var_name = parts[0];
            let error_msg = parts.get(1).unwrap_or(&"required but not set");
            return Err(EnvSubstError::RequiredVarMissing {
                var: var_name.to_string(),
                message: error_msg.to_string(),
            });
        } else if !expr.contains(":-") {
            return Err(EnvSubstError::VarNotFound(expr.to_string()));
        }
    }

    Ok(result)
}

/// Resolve a single variable expression
fn resolve_var_expression(
    expr: &str,
    env_map: &HashMap<String, String>,
) -> Result<String, EnvSubstError> {
    // Handle ${VAR:?error} - required with custom error
    if let Some(idx) = expr.find(":?") {
        let var_name = &expr[..idx];
        let error_msg = &expr[idx + 2..];
        return env_map.get(var_name).cloned().ok_or_else(|| EnvSubstError::RequiredVarMissing {
            var: var_name.to_string(),
            message: error_msg.to_string(),
        });
    }

    // Handle ${VAR:-default} - with default value
    if let Some(idx) = expr.find(":-") {
        let var_name = &expr[..idx];
        let default_val = &expr[idx + 2..];
        return Ok(env_map.get(var_name).cloned().unwrap_or_else(|| default_val.to_string()));
    }

    // Handle ${VAR} - simple substitution
    env_map
        .get(expr)
        .cloned()
        .ok_or_else(|| EnvSubstError::VarNotFound(expr.to_string()))
}

/// Substitute env vars in all string values of a YAML document
pub fn substitute_in_yaml_value(value: &mut serde_yaml::Value) -> Result<(), EnvSubstError> {
    substitute_in_yaml_value_with_map(value, &env_to_map())
}

/// Substitute env vars in YAML value using provided map
pub fn substitute_in_yaml_value_with_map(
    value: &mut serde_yaml::Value,
    env_map: &HashMap<String, String>,
) -> Result<(), EnvSubstError> {
    match value {
        serde_yaml::Value::String(s) => {
            *s = substitute_env_vars_with_map(s, env_map)?;
        }
        serde_yaml::Value::Mapping(map) => {
            let keys: Vec<serde_yaml::Value> = map.keys().cloned().collect();
            for key in keys {
                if let Some(v) = map.get_mut(&key) {
                    substitute_in_yaml_value_with_map(v, env_map)?;
                }
            }
        }
        serde_yaml::Value::Sequence(seq) => {
            for item in seq.iter_mut() {
                substitute_in_yaml_value_with_map(item, env_map)?;
            }
        }
        _ => {} // Numbers, bools, null - no substitution needed
    }
    Ok(())
}

/// Convert current environment to a HashMap
fn env_to_map() -> HashMap<String, String> {
    env::vars().collect()
}

/// Environment substitution errors
#[derive(Debug, thiserror::Error)]
pub enum EnvSubstError {
    #[error("Environment variable not found: {0}")]
    VarNotFound(String),

    #[error("Required environment variable '{var}' not set: {message}")]
    RequiredVarMissing { var: String, message: String },

    #[error("Recursive substitution limit exceeded")]
    RecursionLimit,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_env() -> HashMap<String, String> {
        let mut env = HashMap::new();
        env.insert("HOME".to_string(), "/home/user".to_string());
        env.insert("PORT".to_string(), "8080".to_string());
        env.insert("API_KEY".to_string(), "sk-test-123".to_string());
        env.insert("NESTED".to_string(), "${HOME}/config".to_string());
        env
    }

    #[test]
    fn test_simple_substitution() {
        let env = test_env();
        let result = substitute_env_vars_with_map("port: ${PORT}", &env).unwrap();
        assert_eq!(result, "port: 8080");
    }

    #[test]
    fn test_default_value() {
        let env = test_env();
        let result = substitute_env_vars_with_map("host: ${HOST:-localhost}", &env).unwrap();
        assert_eq!(result, "host: localhost");
    }

    #[test]
    fn test_default_value_not_used() {
        let env = test_env();
        let result = substitute_env_vars_with_map("port: ${PORT:-3000}", &env).unwrap();
        assert_eq!(result, "port: 8080");
    }

    #[test]
    fn test_multiple_substitutions() {
        let env = test_env();
        let result = substitute_env_vars_with_map("${HOME}:${PORT}", &env).unwrap();
        assert_eq!(result, "/home/user:8080");
    }

    #[test]
    fn test_required_var_missing() {
        let env = test_env();
        let result =
            substitute_env_vars_with_map("key: ${MISSING_VAR:?Please set MISSING_VAR}", &env);
        assert!(result.is_err());
        match result.unwrap_err() {
            EnvSubstError::RequiredVarMissing { var, message } => {
                assert_eq!(var, "MISSING_VAR");
                assert_eq!(message, "Please set MISSING_VAR");
            }
            _ => panic!("Expected RequiredVarMissing"),
        }
    }

    #[test]
    fn test_no_substitution_needed() {
        let env = test_env();
        let result = substitute_env_vars_with_map("plain text without vars", &env).unwrap();
        assert_eq!(result, "plain text without vars");
    }

    #[test]
    fn test_yaml_value_substitution() {
        let env = test_env();
        let mut value = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        if let serde_yaml::Value::Mapping(ref mut map) = value {
            map.insert(
                serde_yaml::Value::String("port".to_string()),
                serde_yaml::Value::String("${PORT}".to_string()),
            );
            map.insert(
                serde_yaml::Value::String("host".to_string()),
                serde_yaml::Value::String("${HOST:-localhost}".to_string()),
            );
        }
        substitute_in_yaml_value_with_map(&mut value, &env).unwrap();
        if let serde_yaml::Value::Mapping(ref map) = value {
            assert_eq!(
                map.get(&serde_yaml::Value::String("port".to_string())),
                Some(&serde_yaml::Value::String("8080".to_string()))
            );
            assert_eq!(
                map.get(&serde_yaml::Value::String("host".to_string())),
                Some(&serde_yaml::Value::String("localhost".to_string()))
            );
        }
    }
}
