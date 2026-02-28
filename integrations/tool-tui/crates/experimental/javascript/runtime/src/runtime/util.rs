//! Util module (promisify, inspect, format)

use std::fmt;

pub struct Util;

impl Util {
    pub fn format(fmt: &str, args: &[&dyn fmt::Display]) -> String {
        let mut result = fmt.to_string();
        for (i, arg) in args.iter().enumerate() {
            let placeholder = format!("%{}", if i == 0 { "s" } else { "" });
            if let Some(pos) = result.find(&placeholder) {
                result.replace_range(pos..pos + placeholder.len(), &format!("{}", arg));
            }
        }
        result
    }

    pub fn inspect<T: fmt::Debug>(value: &T) -> String {
        format!("{:#?}", value)
    }

    pub fn is_array<T>(_value: &[T]) -> bool {
        true
    }
    pub fn is_boolean(_value: bool) -> bool {
        true
    }
    pub fn is_null<T>(value: Option<T>) -> bool {
        value.is_none()
    }
    pub fn is_number(_value: f64) -> bool {
        true
    }
    pub fn is_string(_value: &str) -> bool {
        true
    }
    pub fn is_undefined<T>(value: Option<T>) -> bool {
        value.is_none()
    }
}
