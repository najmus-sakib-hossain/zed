//! Built-in Instance Methods
//!
//! Native implementations of JavaScript built-in prototype methods for
//! Array, String, Object, Number, and other core types.

use crate::error::{DxError, DxResult};
use crate::value::Value;
use std::collections::HashMap;

/// Type alias for array map callback
pub type ArrayMapCallback = Box<dyn Fn(Value, usize) -> Value>;
/// Type alias for array filter callback
pub type ArrayFilterCallback = Box<dyn Fn(&Value, usize) -> bool>;
/// Type alias for array reduce callback
pub type ArrayReduceCallback = Box<dyn Fn(Value, Value, usize) -> Value>;
/// Type alias for array forEach callback
pub type ArrayForEachCallback = Box<dyn Fn(&Value, usize)>;
/// Type alias for array sort compare callback
pub type ArraySortCompareCallback = Box<dyn Fn(&Value, &Value) -> i32>;

/// Registry for all instance methods
pub struct InstanceMethodRegistry {
    /// Array.prototype methods
    pub array_methods: ArrayPrototype,
    /// String.prototype methods
    pub string_methods: StringPrototype,
    /// Object.prototype methods
    pub object_methods: ObjectPrototype,
    /// Number.prototype methods
    pub number_methods: NumberPrototype,
}

impl InstanceMethodRegistry {
    pub fn new() -> Self {
        Self {
            array_methods: ArrayPrototype,
            string_methods: StringPrototype,
            object_methods: ObjectPrototype,
            number_methods: NumberPrototype,
        }
    }
}

impl Default for InstanceMethodRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Array.prototype methods
pub struct ArrayPrototype;

impl ArrayPrototype {
    /// Array.prototype.map(callback)
    pub fn map(&self, array: Vec<Value>, callback: ArrayMapCallback) -> Vec<Value> {
        array.into_iter().enumerate().map(|(i, val)| callback(val, i)).collect()
    }

    /// Array.prototype.filter(callback)
    pub fn filter(&self, array: Vec<Value>, callback: ArrayFilterCallback) -> Vec<Value> {
        array
            .into_iter()
            .enumerate()
            .filter(|(i, val)| callback(val, *i))
            .map(|(_, v)| v)
            .collect()
    }

    /// Array.prototype.reduce(callback, initial)
    pub fn reduce(
        &self,
        array: Vec<Value>,
        callback: ArrayReduceCallback,
        initial: Value,
    ) -> Value {
        array
            .into_iter()
            .enumerate()
            .fold(initial, |acc, (i, val)| callback(acc, val, i))
    }

    /// Array.prototype.forEach(callback)
    pub fn for_each(&self, array: &[Value], callback: ArrayForEachCallback) {
        array.iter().enumerate().for_each(|(i, val)| callback(val, i));
    }

    /// Array.prototype.find(callback)
    pub fn find(&self, array: &[Value], callback: ArrayFilterCallback) -> Option<Value> {
        array.iter().enumerate().find_map(|(i, val)| {
            if callback(val, i) {
                Some(val.clone())
            } else {
                None
            }
        })
    }

    /// Array.prototype.findIndex(callback)
    pub fn find_index(&self, array: &[Value], callback: ArrayFilterCallback) -> i32 {
        array
            .iter()
            .enumerate()
            .find_map(|(i, val)| {
                if callback(val, i) {
                    Some(i as i32)
                } else {
                    None
                }
            })
            .unwrap_or(-1)
    }

    /// Array.prototype.every(callback)
    pub fn every(&self, array: &[Value], callback: ArrayFilterCallback) -> bool {
        array.iter().enumerate().all(|(i, val)| callback(val, i))
    }

    /// Array.prototype.some(callback)
    pub fn some(&self, array: &[Value], callback: ArrayFilterCallback) -> bool {
        array.iter().enumerate().any(|(i, val)| callback(val, i))
    }

    /// Array.prototype.includes(value)
    pub fn includes(&self, array: &[Value], search: &Value) -> bool {
        array.contains(search)
    }

    /// Array.prototype.indexOf(value)
    pub fn index_of(&self, array: &[Value], search: &Value) -> i32 {
        array.iter().position(|v| v == search).map(|i| i as i32).unwrap_or(-1)
    }

    /// Array.prototype.lastIndexOf(value)
    pub fn last_index_of(&self, array: &[Value], search: &Value) -> i32 {
        array.iter().rposition(|v| v == search).map(|i| i as i32).unwrap_or(-1)
    }

    /// Array.prototype.join(separator)
    pub fn join(&self, array: &[Value], separator: &str) -> String {
        array.iter().map(|v| format!("{:?}", v)).collect::<Vec<_>>().join(separator)
    }

    /// Array.prototype.slice(start, end)
    pub fn slice(&self, array: &[Value], start: i32, end: Option<i32>) -> Vec<Value> {
        let len = array.len() as i32;
        let start = if start < 0 {
            (len + start).max(0)
        } else {
            start.min(len)
        } as usize;
        let end = end
            .map(|e| if e < 0 { (len + e).max(0) } else { e.min(len) } as usize)
            .unwrap_or(array.len());
        array[start..end].to_vec()
    }

    /// Array.prototype.concat(...arrays)
    pub fn concat(&self, arrays: Vec<Vec<Value>>) -> Vec<Value> {
        arrays.into_iter().flatten().collect()
    }

    /// Array.prototype.reverse()
    pub fn reverse(&self, mut array: Vec<Value>) -> Vec<Value> {
        array.reverse();
        array
    }

    /// Array.prototype.sort(compareFn)
    pub fn sort(
        &self,
        mut array: Vec<Value>,
        compare: Option<ArraySortCompareCallback>,
    ) -> Vec<Value> {
        if let Some(cmp) = compare {
            array.sort_by(|a, b| match cmp(a, b) {
                x if x < 0 => std::cmp::Ordering::Less,
                x if x > 0 => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            });
        } else {
            array.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
        }
        array
    }

    /// Array.prototype.flat(depth)
    pub fn flat(&self, array: Vec<Value>, depth: usize) -> Vec<Value> {
        if depth == 0 {
            return array;
        }
        let mut result = Vec::new();
        for val in array {
            // Simplified - would need proper Value::Array handling
            result.push(val);
        }
        result
    }

    /// Array.prototype.flatMap(callback)
    pub fn flat_map(
        &self,
        array: Vec<Value>,
        callback: Box<dyn Fn(Value, usize) -> Vec<Value>>,
    ) -> Vec<Value> {
        array.into_iter().enumerate().flat_map(|(i, val)| callback(val, i)).collect()
    }
}

/// String.prototype methods
pub struct StringPrototype;

impl StringPrototype {
    /// String.prototype.charAt(index)
    pub fn char_at(&self, s: &str, index: usize) -> String {
        s.chars().nth(index).map(|c| c.to_string()).unwrap_or_default()
    }

    /// String.prototype.charCodeAt(index)
    pub fn char_code_at(&self, s: &str, index: usize) -> Option<u32> {
        s.chars().nth(index).map(|c| c as u32)
    }

    /// String.prototype.concat(...strings)
    pub fn concat(&self, strings: Vec<&str>) -> String {
        strings.concat()
    }

    /// String.prototype.includes(search)
    pub fn includes(&self, s: &str, search: &str) -> bool {
        s.contains(search)
    }

    /// String.prototype.indexOf(search)
    pub fn index_of(&self, s: &str, search: &str) -> i32 {
        s.find(search).map(|i| i as i32).unwrap_or(-1)
    }

    /// String.prototype.lastIndexOf(search)
    pub fn last_index_of(&self, s: &str, search: &str) -> i32 {
        s.rfind(search).map(|i| i as i32).unwrap_or(-1)
    }

    /// String.prototype.slice(start, end)
    pub fn slice(&self, s: &str, start: i32, end: Option<i32>) -> String {
        let len = s.len() as i32;
        let start = if start < 0 {
            (len + start).max(0)
        } else {
            start.min(len)
        } as usize;
        let end = end
            .map(|e| if e < 0 { (len + e).max(0) } else { e.min(len) } as usize)
            .unwrap_or(s.len());
        s.chars().skip(start).take(end - start).collect()
    }

    /// String.prototype.substring(start, end)
    pub fn substring(&self, s: &str, start: usize, end: Option<usize>) -> String {
        let end = end.unwrap_or(s.len());
        s.chars().skip(start).take(end - start).collect()
    }

    /// String.prototype.substr(start, length)
    pub fn substr(&self, s: &str, start: i32, length: Option<usize>) -> String {
        let len = s.len() as i32;
        let start = if start < 0 {
            (len + start).max(0)
        } else {
            start.min(len)
        } as usize;
        let length = length.unwrap_or(s.len() - start);
        s.chars().skip(start).take(length).collect()
    }

    /// String.prototype.split(separator, limit)
    pub fn split(&self, s: &str, separator: Option<&str>, limit: Option<usize>) -> Vec<String> {
        match separator {
            Some(sep) if !sep.is_empty() => {
                let parts: Vec<String> = s.split(sep).map(String::from).collect();
                if let Some(lim) = limit {
                    parts.into_iter().take(lim).collect()
                } else {
                    parts
                }
            }
            _ => s.chars().map(|c| c.to_string()).collect(),
        }
    }

    /// String.prototype.toLowerCase()
    pub fn to_lower_case(&self, s: &str) -> String {
        s.to_lowercase()
    }

    /// String.prototype.toUpperCase()
    pub fn to_upper_case(&self, s: &str) -> String {
        s.to_uppercase()
    }

    /// String.prototype.trim()
    pub fn trim(&self, s: &str) -> String {
        s.trim().to_string()
    }

    /// String.prototype.trimStart()
    pub fn trim_start(&self, s: &str) -> String {
        s.trim_start().to_string()
    }

    /// String.prototype.trimEnd()
    pub fn trim_end(&self, s: &str) -> String {
        s.trim_end().to_string()
    }

    /// String.prototype.repeat(count)
    pub fn repeat(&self, s: &str, count: usize) -> String {
        s.repeat(count)
    }

    /// String.prototype.replace(search, replace)
    pub fn replace(&self, s: &str, search: &str, replace: &str) -> String {
        s.replacen(search, replace, 1)
    }

    /// String.prototype.replaceAll(search, replace)
    pub fn replace_all(&self, s: &str, search: &str, replace: &str) -> String {
        s.replace(search, replace)
    }

    /// String.prototype.startsWith(search)
    pub fn starts_with(&self, s: &str, search: &str) -> bool {
        s.starts_with(search)
    }

    /// String.prototype.endsWith(search)
    pub fn ends_with(&self, s: &str, search: &str) -> bool {
        s.ends_with(search)
    }

    /// String.prototype.padStart(length, padString)
    pub fn pad_start(&self, s: &str, length: usize, pad: &str) -> String {
        if s.len() >= length {
            return s.to_string();
        }
        let pad_len = length - s.len();
        let pad_str = pad.repeat((pad_len / pad.len()) + 1);
        format!("{}{}", &pad_str[..pad_len], s)
    }

    /// String.prototype.padEnd(length, padString)
    pub fn pad_end(&self, s: &str, length: usize, pad: &str) -> String {
        if s.len() >= length {
            return s.to_string();
        }
        let pad_len = length - s.len();
        let pad_str = pad.repeat((pad_len / pad.len()) + 1);
        format!("{}{}", s, &pad_str[..pad_len])
    }

    /// String.prototype.match(regexp)
    pub fn match_regex(&self, s: &str, pattern: &str) -> Option<Vec<String>> {
        // Simplified regex - would use regex crate in production
        if s.contains(pattern) {
            Some(vec![pattern.to_string()])
        } else {
            None
        }
    }
}

/// Object.prototype methods
pub struct ObjectPrototype;

impl ObjectPrototype {
    /// Object.prototype.hasOwnProperty(key)
    pub fn has_own_property(&self, obj: &HashMap<String, Value>, key: &str) -> bool {
        obj.contains_key(key)
    }

    /// Object.prototype.toString()
    pub fn to_string(&self, _obj: &HashMap<String, Value>) -> String {
        "[object Object]".to_string()
    }

    /// Object.prototype.valueOf()
    pub fn value_of(&self, obj: &HashMap<String, Value>) -> HashMap<String, Value> {
        obj.clone()
    }

    /// Object.prototype.propertyIsEnumerable(key)
    pub fn property_is_enumerable(&self, obj: &HashMap<String, Value>, key: &str) -> bool {
        obj.contains_key(key)
    }
}

/// Number.prototype methods
pub struct NumberPrototype;

impl NumberPrototype {
    /// Number.prototype.toFixed(digits)
    pub fn to_fixed(&self, num: f64, digits: u32) -> String {
        format!("{:.prec$}", num, prec = digits as usize)
    }

    /// Number.prototype.toExponential(digits)
    pub fn to_exponential(&self, num: f64, digits: Option<u32>) -> String {
        if let Some(d) = digits {
            format!("{:.prec$e}", num, prec = d as usize)
        } else {
            format!("{:e}", num)
        }
    }

    /// Number.prototype.toPrecision(precision)
    pub fn to_precision(&self, num: f64, precision: u32) -> String {
        format!("{:.prec$}", num, prec = precision as usize)
    }

    /// Number.prototype.toString(radix)
    pub fn to_string(&self, num: f64, radix: Option<u32>) -> DxResult<String> {
        let radix = radix.unwrap_or(10);
        if !(2..=36).contains(&radix) {
            return Err(DxError::RuntimeError("Invalid radix".to_string()));
        }

        if radix == 10 {
            Ok(num.to_string())
        } else if radix == 2 {
            Ok(format!("{:b}", num as i64))
        } else if radix == 8 {
            Ok(format!("{:o}", num as i64))
        } else if radix == 16 {
            Ok(format!("{:x}", num as i64))
        } else {
            // Simplified for other radixes
            Ok(num.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_array_map() {
        let proto = ArrayPrototype;
        let arr = vec![Value::Number(1.0), Value::Number(2.0), Value::Number(3.0)];
        let result = proto.map(
            arr,
            Box::new(|v, _| {
                if let Value::Number(n) = v {
                    Value::Number(n * 2.0)
                } else {
                    v
                }
            }),
        );
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_string_split() {
        let proto = StringPrototype;
        let result = proto.split("a,b,c", Some(","), None);
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_string_slice() {
        let proto = StringPrototype;
        assert_eq!(proto.slice("hello", 1, Some(4)), "ell");
        assert_eq!(proto.slice("hello", -3, None), "llo");
    }

    #[test]
    fn test_number_to_fixed() {
        let proto = NumberPrototype;
        assert_eq!(proto.to_fixed(std::f64::consts::PI, 2), "3.14");
    }
}
