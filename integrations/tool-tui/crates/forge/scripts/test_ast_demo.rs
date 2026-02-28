// Test file for Forge LSP AST analysis
use std::collections::HashMap;

pub mod test_module;

/// A test struct for demonstration
pub struct Person {
    name: String,
    age: u32,
}

pub enum Status {
    Active,
    Inactive,
    Pending,
}

impl Person {
    pub fn new(name: String, age: u32) -> Self {
        Self { name, age }
    }

    pub fn greet(&self) {
        println!("Hello, I'm {}", self.name);
    }
}

pub async fn fetch_data() -> Result<String, String> {
    Ok("Data fetched".to_string())
}

pub fn calculate_sum(a: i32, b: i32) -> i32 {
    a + b
}

fn internal_helper() {
    println!("Internal function");
}
