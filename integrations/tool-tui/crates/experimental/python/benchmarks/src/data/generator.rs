//! Test data generator implementation

use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

/// Pattern for generated test files
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestPattern {
    SimpleFunctions,
    Classes,
    Fixtures,
    Async,
    Parametrized,
    Mixed,
}

/// Size category for generated data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataSize {
    Small,  // ~1KB
    Medium, // ~100KB
    Large,  // ~10MB
}

impl DataSize {
    /// Get the target size in bytes for this data size category
    pub fn target_bytes(&self) -> usize {
        match self {
            DataSize::Small => 1024,             // 1KB
            DataSize::Medium => 100 * 1024,      // 100KB
            DataSize::Large => 10 * 1024 * 1024, // 10MB
        }
    }

    /// Get the acceptable range (±50%) for this data size
    pub fn acceptable_range(&self) -> (usize, usize) {
        let target = self.target_bytes();
        let min = target / 2;
        let max = target + target / 2;
        (min, max)
    }
}

/// Generated test file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFile {
    pub name: String,
    pub content: String,
}

/// Generated test project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestProject {
    pub name: String,
    pub pyproject_toml: String,
    pub dependency_count: usize,
}

/// Generator for deterministic test data using seeded RNG
pub struct TestDataGenerator {
    seed: u64,
    rng: ChaCha8Rng,
}

impl TestDataGenerator {
    /// Create a new generator with the specified seed
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            rng: ChaCha8Rng::seed_from_u64(seed),
        }
    }

    /// Get the seed used by this generator
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Reset the RNG to its initial state (same seed)
    pub fn reset(&mut self) {
        self.rng = ChaCha8Rng::seed_from_u64(self.seed);
    }

    /// Generate a random alphanumeric string of the given length
    fn random_string(&mut self, len: usize) -> String {
        const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        (0..len)
            .map(|_| {
                let idx = self.rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// Generate a random identifier (lowercase letters and underscores)
    fn random_identifier(&mut self, len: usize) -> String {
        const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz_";
        let first_char = (b'a' + self.rng.gen_range(0..26)) as char;
        let rest: String = (1..len)
            .map(|_| {
                let idx = self.rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();
        format!("{}{}", first_char, rest)
    }
}

impl TestDataGenerator {
    /// Generate JSON data of the specified size
    pub fn generate_json_data(&mut self, size: DataSize) -> String {
        let target_size = size.target_bytes();
        let mut json = String::with_capacity(target_size);

        json.push_str("{\n");

        let mut current_size = 2; // Opening brace and newline
        let mut first = true;

        while current_size < target_size {
            if !first {
                json.push_str(",\n");
                current_size += 2;
            }
            first = false;

            let key = self.random_identifier(8);
            let value_type = self.rng.gen_range(0..4);

            let entry = match value_type {
                0 => {
                    // String value
                    let val = self.random_string(20);
                    format!("  \"{}\": \"{}\"", key, val)
                }
                1 => {
                    // Number value
                    let val: f64 = self.rng.gen_range(-1000.0..1000.0);
                    format!("  \"{}\": {:.2}", key, val)
                }
                2 => {
                    // Boolean value
                    let val: bool = self.rng.gen();
                    format!("  \"{}\": {}", key, val)
                }
                _ => {
                    // Array of numbers
                    let count = self.rng.gen_range(3..10);
                    let nums: Vec<String> =
                        (0..count).map(|_| format!("{}", self.rng.gen_range(0..100))).collect();
                    format!("  \"{}\": [{}]", key, nums.join(", "))
                }
            };

            current_size += entry.len();
            json.push_str(&entry);
        }

        json.push_str("\n}");
        json
    }

    /// Generate string data of the specified size
    pub fn generate_string_data(&mut self, size: DataSize) -> String {
        let target_size = size.target_bytes();
        let mut result = String::with_capacity(target_size);

        while result.len() < target_size {
            // Generate words of varying lengths
            let word_len = self.rng.gen_range(3..12);
            let word = self.random_string(word_len);

            if !result.is_empty() {
                // Add space or newline
                if self.rng.gen_bool(0.1) {
                    result.push('\n');
                } else {
                    result.push(' ');
                }
            }

            result.push_str(&word);
        }

        result
    }

    /// Generate test files with the specified pattern
    pub fn generate_test_files(&mut self, count: usize, pattern: TestPattern) -> Vec<TestFile> {
        (0..count)
            .map(|i| {
                let name = format!("test_{}.py", self.random_identifier(8));
                let content = self.generate_test_file_content(pattern, i);
                TestFile { name, content }
            })
            .collect()
    }

    /// Generate content for a single test file
    fn generate_test_file_content(&mut self, pattern: TestPattern, index: usize) -> String {
        match pattern {
            TestPattern::SimpleFunctions => self.generate_simple_functions_test(index),
            TestPattern::Classes => self.generate_class_test(index),
            TestPattern::Fixtures => self.generate_fixtures_test(index),
            TestPattern::Async => self.generate_async_test(index),
            TestPattern::Parametrized => self.generate_parametrized_test(index),
            TestPattern::Mixed => self.generate_mixed_test(index),
        }
    }

    fn generate_simple_functions_test(&mut self, _index: usize) -> String {
        let mut content = String::new();
        let num_tests = self.rng.gen_range(3..8);

        for i in 0..num_tests {
            let func_name = self.random_identifier(10);
            content.push_str(&format!(
                "def test_{}():\n    assert {} + {} == {}\n\n",
                func_name,
                i,
                i + 1,
                i + i + 1
            ));
        }

        content
    }

    fn generate_class_test(&mut self, _index: usize) -> String {
        let class_name = format!("Test{}", self.random_identifier(8));
        let mut content = format!("class {}:\n", class_name);

        let num_methods = self.rng.gen_range(2..5);
        for i in 0..num_methods {
            let method_name = self.random_identifier(8);
            content.push_str(&format!(
                "    def test_{}(self):\n        assert {} * 2 == {}\n\n",
                method_name,
                i,
                i * 2
            ));
        }

        content
    }

    fn generate_fixtures_test(&mut self, _index: usize) -> String {
        let fixture_name = self.random_identifier(8);
        let mut content = String::new();

        content.push_str("import pytest\n\n");
        content.push_str(&format!(
            "@pytest.fixture\ndef {}():\n    return {}\n\n",
            fixture_name,
            self.rng.gen_range(1..100)
        ));

        let num_tests = self.rng.gen_range(2..4);
        for _ in 0..num_tests {
            let test_name = self.random_identifier(10);
            content.push_str(&format!(
                "def test_{}({}):\n    assert {} > 0\n\n",
                test_name, fixture_name, fixture_name
            ));
        }

        content
    }

    fn generate_async_test(&mut self, _index: usize) -> String {
        let mut content = String::new();
        content.push_str("import pytest\nimport asyncio\n\n");

        let num_tests = self.rng.gen_range(2..5);
        for _ in 0..num_tests {
            let test_name = self.random_identifier(10);
            let delay = self.rng.gen_range(1..10) as f64 / 1000.0;
            content.push_str(&format!(
                "@pytest.mark.asyncio\nasync def test_{}():\n    await asyncio.sleep({})\n    assert True\n\n",
                test_name, delay
            ));
        }

        content
    }

    fn generate_parametrized_test(&mut self, _index: usize) -> String {
        let mut content = String::new();
        content.push_str("import pytest\n\n");

        let test_name = self.random_identifier(10);
        let num_params = self.rng.gen_range(3..8);
        let params: Vec<String> = (0..num_params).map(|i| format!("({}, {})", i, i * 2)).collect();

        content.push_str(&format!(
            "@pytest.mark.parametrize(\"input,expected\", [{}])\n",
            params.join(", ")
        ));
        content.push_str(&format!(
            "def test_{}(input, expected):\n    assert input * 2 == expected\n",
            test_name
        ));

        content
    }

    fn generate_mixed_test(&mut self, index: usize) -> String {
        // Mix different patterns
        let pattern = match index % 5 {
            0 => TestPattern::SimpleFunctions,
            1 => TestPattern::Classes,
            2 => TestPattern::Fixtures,
            3 => TestPattern::Async,
            _ => TestPattern::Parametrized,
        };
        self.generate_test_file_content(pattern, index)
    }

    /// Generate a test project with the specified number of dependencies
    pub fn generate_project(&mut self, deps: usize) -> TestProject {
        let name = format!("test_project_{}", self.random_identifier(6));

        let common_deps = [
            "requests",
            "flask",
            "django",
            "numpy",
            "pandas",
            "pytest",
            "click",
            "pydantic",
            "fastapi",
            "sqlalchemy",
            "aiohttp",
            "httpx",
            "rich",
            "typer",
            "black",
            "ruff",
            "mypy",
        ];

        let mut dependencies = Vec::new();
        for _ in 0..deps.min(common_deps.len()) {
            let idx = self.rng.gen_range(0..common_deps.len());
            let dep = common_deps[idx];
            if !dependencies.contains(&dep) {
                dependencies.push(dep);
            }
        }

        // Add more synthetic deps if needed
        while dependencies.len() < deps {
            let dep_name = format!("dep_{}", self.random_identifier(6));
            dependencies.push(Box::leak(dep_name.into_boxed_str()));
        }

        let deps_str: Vec<String> = dependencies.iter().map(|d| format!("\"{}\"", d)).collect();

        let pyproject_toml = format!(
            r#"[project]
name = "{}"
version = "0.1.0"
dependencies = [
    {}
]

[build-system]
requires = ["setuptools>=61.0"]
build-backend = "setuptools.build_meta"
"#,
            name,
            deps_str.join(",\n    ")
        );

        TestProject {
            name,
            pyproject_toml,
            dependency_count: dependencies.len(),
        }
    }
}
