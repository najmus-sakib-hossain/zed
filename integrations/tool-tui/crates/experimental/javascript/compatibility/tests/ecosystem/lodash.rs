//! Lodash Compatibility Tests
//!
//! This module tests compatibility with lodash, one of the most popular
//! JavaScript utility libraries.
//!
//! **Validates: Requirements 7.2**

use std::collections::HashMap;

/// Lodash function categories for testing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LodashCategory {
    Array,
    Collection,
    Date,
    Function,
    Lang,
    Math,
    Number,
    Object,
    String,
    Util,
}

/// A lodash function test case
#[derive(Debug, Clone)]
pub struct LodashTestCase {
    /// Function name (e.g., "map", "filter", "reduce")
    pub name: String,
    /// Category
    pub category: LodashCategory,
    /// JavaScript test code
    pub test_code: String,
    /// Expected result (as JSON string)
    pub expected: String,
}

/// Lodash compatibility test suite
pub struct LodashTestSuite {
    test_cases: Vec<LodashTestCase>,
}

impl LodashTestSuite {
    /// Create a new lodash test suite with standard test cases
    pub fn new() -> Self {
        let mut suite = Self {
            test_cases: Vec::new(),
        };
        
        // Add array function tests
        suite.add_array_tests();
        
        // Add collection function tests
        suite.add_collection_tests();
        
        // Add lang function tests
        suite.add_lang_tests();
        
        // Add object function tests
        suite.add_object_tests();
        
        // Add string function tests
        suite.add_string_tests();
        
        // Add math function tests
        suite.add_math_tests();
        
        suite
    }
    
    fn add_array_tests(&mut self) {
        // _.chunk
        self.test_cases.push(LodashTestCase {
            name: "chunk".to_string(),
            category: LodashCategory::Array,
            test_code: r#"
                const _ = require('lodash');
                JSON.stringify(_.chunk(['a', 'b', 'c', 'd'], 2));
            "#.to_string(),
            expected: r#"[["a","b"],["c","d"]]"#.to_string(),
        });
        
        // _.compact
        self.test_cases.push(LodashTestCase {
            name: "compact".to_string(),
            category: LodashCategory::Array,
            test_code: r#"
                const _ = require('lodash');
                JSON.stringify(_.compact([0, 1, false, 2, '', 3]));
            "#.to_string(),
            expected: r#"[1,2,3]"#.to_string(),
        });
        
        // _.concat
        self.test_cases.push(LodashTestCase {
            name: "concat".to_string(),
            category: LodashCategory::Array,
            test_code: r#"
                const _ = require('lodash');
                JSON.stringify(_.concat([1], 2, [3], [[4]]));
            "#.to_string(),
            expected: r#"[1,2,3,[4]]"#.to_string(),
        });
        
        // _.difference
        self.test_cases.push(LodashTestCase {
            name: "difference".to_string(),
            category: LodashCategory::Array,
            test_code: r#"
                const _ = require('lodash');
                JSON.stringify(_.difference([2, 1], [2, 3]));
            "#.to_string(),
            expected: r#"[1]"#.to_string(),
        });
        
        // _.drop
        self.test_cases.push(LodashTestCase {
            name: "drop".to_string(),
            category: LodashCategory::Array,
            test_code: r#"
                const _ = require('lodash');
                JSON.stringify(_.drop([1, 2, 3], 2));
            "#.to_string(),
            expected: r#"[3]"#.to_string(),
        });
        
        // _.flatten
        self.test_cases.push(LodashTestCase {
            name: "flatten".to_string(),
            category: LodashCategory::Array,
            test_code: r#"
                const _ = require('lodash');
                JSON.stringify(_.flatten([1, [2, [3, [4]], 5]]));
            "#.to_string(),
            expected: r#"[1,2,[3,[4]],5]"#.to_string(),
        });
        
        // _.flattenDeep
        self.test_cases.push(LodashTestCase {
            name: "flattenDeep".to_string(),
            category: LodashCategory::Array,
            test_code: r#"
                const _ = require('lodash');
                JSON.stringify(_.flattenDeep([1, [2, [3, [4]], 5]]));
            "#.to_string(),
            expected: r#"[1,2,3,4,5]"#.to_string(),
        });
        
        // _.uniq
        self.test_cases.push(LodashTestCase {
            name: "uniq".to_string(),
            category: LodashCategory::Array,
            test_code: r#"
                const _ = require('lodash');
                JSON.stringify(_.uniq([2, 1, 2]));
            "#.to_string(),
            expected: r#"[2,1]"#.to_string(),
        });
        
        // _.zip
        self.test_cases.push(LodashTestCase {
            name: "zip".to_string(),
            category: LodashCategory::Array,
            test_code: r#"
                const _ = require('lodash');
                JSON.stringify(_.zip(['a', 'b'], [1, 2], [true, false]));
            "#.to_string(),
            expected: r#"[["a",1,true],["b",2,false]]"#.to_string(),
        });
    }
    
    fn add_collection_tests(&mut self) {
        // _.map
        self.test_cases.push(LodashTestCase {
            name: "map".to_string(),
            category: LodashCategory::Collection,
            test_code: r#"
                const _ = require('lodash');
                JSON.stringify(_.map([4, 8], n => n * n));
            "#.to_string(),
            expected: r#"[16,64]"#.to_string(),
        });
        
        // _.filter
        self.test_cases.push(LodashTestCase {
            name: "filter".to_string(),
            category: LodashCategory::Collection,
            test_code: r#"
                const _ = require('lodash');
                JSON.stringify(_.filter([1, 2, 3, 4], n => n % 2 === 0));
            "#.to_string(),
            expected: r#"[2,4]"#.to_string(),
        });
        
        // _.reduce
        self.test_cases.push(LodashTestCase {
            name: "reduce".to_string(),
            category: LodashCategory::Collection,
            test_code: r#"
                const _ = require('lodash');
                _.reduce([1, 2, 3], (sum, n) => sum + n, 0);
            "#.to_string(),
            expected: "6".to_string(),
        });
        
        // _.find
        self.test_cases.push(LodashTestCase {
            name: "find".to_string(),
            category: LodashCategory::Collection,
            test_code: r#"
                const _ = require('lodash');
                JSON.stringify(_.find([1, 2, 3, 4], n => n > 2));
            "#.to_string(),
            expected: "3".to_string(),
        });
        
        // _.groupBy
        self.test_cases.push(LodashTestCase {
            name: "groupBy".to_string(),
            category: LodashCategory::Collection,
            test_code: r#"
                const _ = require('lodash');
                JSON.stringify(_.groupBy([6.1, 4.2, 6.3], Math.floor));
            "#.to_string(),
            expected: r#"{"4":[4.2],"6":[6.1,6.3]}"#.to_string(),
        });
        
        // _.sortBy
        self.test_cases.push(LodashTestCase {
            name: "sortBy".to_string(),
            category: LodashCategory::Collection,
            test_code: r#"
                const _ = require('lodash');
                JSON.stringify(_.sortBy([3, 1, 2]));
            "#.to_string(),
            expected: r#"[1,2,3]"#.to_string(),
        });
        
        // _.every
        self.test_cases.push(LodashTestCase {
            name: "every".to_string(),
            category: LodashCategory::Collection,
            test_code: r#"
                const _ = require('lodash');
                _.every([true, 1, null, 'yes'], Boolean);
            "#.to_string(),
            expected: "false".to_string(),
        });
        
        // _.some
        self.test_cases.push(LodashTestCase {
            name: "some".to_string(),
            category: LodashCategory::Collection,
            test_code: r#"
                const _ = require('lodash');
                _.some([null, 0, 'yes', false], Boolean);
            "#.to_string(),
            expected: "true".to_string(),
        });
    }
    
    fn add_lang_tests(&mut self) {
        // _.isArray
        self.test_cases.push(LodashTestCase {
            name: "isArray".to_string(),
            category: LodashCategory::Lang,
            test_code: r#"
                const _ = require('lodash');
                _.isArray([1, 2, 3]);
            "#.to_string(),
            expected: "true".to_string(),
        });
        
        // _.isObject
        self.test_cases.push(LodashTestCase {
            name: "isObject".to_string(),
            category: LodashCategory::Lang,
            test_code: r#"
                const _ = require('lodash');
                _.isObject({});
            "#.to_string(),
            expected: "true".to_string(),
        });
        
        // _.isString
        self.test_cases.push(LodashTestCase {
            name: "isString".to_string(),
            category: LodashCategory::Lang,
            test_code: r#"
                const _ = require('lodash');
                _.isString('abc');
            "#.to_string(),
            expected: "true".to_string(),
        });
        
        // _.isNumber
        self.test_cases.push(LodashTestCase {
            name: "isNumber".to_string(),
            category: LodashCategory::Lang,
            test_code: r#"
                const _ = require('lodash');
                _.isNumber(3);
            "#.to_string(),
            expected: "true".to_string(),
        });
        
        // _.isNil
        self.test_cases.push(LodashTestCase {
            name: "isNil".to_string(),
            category: LodashCategory::Lang,
            test_code: r#"
                const _ = require('lodash');
                [_.isNil(null), _.isNil(undefined), _.isNil(0)].join(',');
            "#.to_string(),
            expected: "true,true,false".to_string(),
        });
        
        // _.clone
        self.test_cases.push(LodashTestCase {
            name: "clone".to_string(),
            category: LodashCategory::Lang,
            test_code: r#"
                const _ = require('lodash');
                const obj = { a: 1 };
                const cloned = _.clone(obj);
                obj !== cloned && cloned.a === 1;
            "#.to_string(),
            expected: "true".to_string(),
        });
        
        // _.cloneDeep
        self.test_cases.push(LodashTestCase {
            name: "cloneDeep".to_string(),
            category: LodashCategory::Lang,
            test_code: r#"
                const _ = require('lodash');
                const obj = { a: { b: 1 } };
                const cloned = _.cloneDeep(obj);
                obj.a !== cloned.a && cloned.a.b === 1;
            "#.to_string(),
            expected: "true".to_string(),
        });
    }
    
    fn add_object_tests(&mut self) {
        // _.keys
        self.test_cases.push(LodashTestCase {
            name: "keys".to_string(),
            category: LodashCategory::Object,
            test_code: r#"
                const _ = require('lodash');
                JSON.stringify(_.keys({ a: 1, b: 2 }).sort());
            "#.to_string(),
            expected: r#"["a","b"]"#.to_string(),
        });
        
        // _.values
        self.test_cases.push(LodashTestCase {
            name: "values".to_string(),
            category: LodashCategory::Object,
            test_code: r#"
                const _ = require('lodash');
                JSON.stringify(_.values({ a: 1, b: 2 }).sort());
            "#.to_string(),
            expected: r#"[1,2]"#.to_string(),
        });
        
        // _.pick
        self.test_cases.push(LodashTestCase {
            name: "pick".to_string(),
            category: LodashCategory::Object,
            test_code: r#"
                const _ = require('lodash');
                JSON.stringify(_.pick({ a: 1, b: 2, c: 3 }, ['a', 'c']));
            "#.to_string(),
            expected: r#"{"a":1,"c":3}"#.to_string(),
        });
        
        // _.omit
        self.test_cases.push(LodashTestCase {
            name: "omit".to_string(),
            category: LodashCategory::Object,
            test_code: r#"
                const _ = require('lodash');
                JSON.stringify(_.omit({ a: 1, b: 2, c: 3 }, ['a', 'c']));
            "#.to_string(),
            expected: r#"{"b":2}"#.to_string(),
        });
        
        // _.merge
        self.test_cases.push(LodashTestCase {
            name: "merge".to_string(),
            category: LodashCategory::Object,
            test_code: r#"
                const _ = require('lodash');
                JSON.stringify(_.merge({ a: 1 }, { b: 2 }, { c: 3 }));
            "#.to_string(),
            expected: r#"{"a":1,"b":2,"c":3}"#.to_string(),
        });
        
        // _.get
        self.test_cases.push(LodashTestCase {
            name: "get".to_string(),
            category: LodashCategory::Object,
            test_code: r#"
                const _ = require('lodash');
                const obj = { a: { b: { c: 3 } } };
                _.get(obj, 'a.b.c');
            "#.to_string(),
            expected: "3".to_string(),
        });
        
        // _.set
        self.test_cases.push(LodashTestCase {
            name: "set".to_string(),
            category: LodashCategory::Object,
            test_code: r#"
                const _ = require('lodash');
                const obj = { a: { b: { c: 3 } } };
                _.set(obj, 'a.b.c', 4);
                obj.a.b.c;
            "#.to_string(),
            expected: "4".to_string(),
        });
    }
    
    fn add_string_tests(&mut self) {
        // _.camelCase
        self.test_cases.push(LodashTestCase {
            name: "camelCase".to_string(),
            category: LodashCategory::String,
            test_code: r#"
                const _ = require('lodash');
                _.camelCase('Foo Bar');
            "#.to_string(),
            expected: "fooBar".to_string(),
        });
        
        // _.capitalize
        self.test_cases.push(LodashTestCase {
            name: "capitalize".to_string(),
            category: LodashCategory::String,
            test_code: r#"
                const _ = require('lodash');
                _.capitalize('FRED');
            "#.to_string(),
            expected: "Fred".to_string(),
        });
        
        // _.kebabCase
        self.test_cases.push(LodashTestCase {
            name: "kebabCase".to_string(),
            category: LodashCategory::String,
            test_code: r#"
                const _ = require('lodash');
                _.kebabCase('Foo Bar');
            "#.to_string(),
            expected: "foo-bar".to_string(),
        });
        
        // _.snakeCase
        self.test_cases.push(LodashTestCase {
            name: "snakeCase".to_string(),
            category: LodashCategory::String,
            test_code: r#"
                const _ = require('lodash');
                _.snakeCase('Foo Bar');
            "#.to_string(),
            expected: "foo_bar".to_string(),
        });
        
        // _.trim
        self.test_cases.push(LodashTestCase {
            name: "trim".to_string(),
            category: LodashCategory::String,
            test_code: r#"
                const _ = require('lodash');
                _.trim('  abc  ');
            "#.to_string(),
            expected: "abc".to_string(),
        });
        
        // _.pad
        self.test_cases.push(LodashTestCase {
            name: "pad".to_string(),
            category: LodashCategory::String,
            test_code: r#"
                const _ = require('lodash');
                _.pad('abc', 8);
            "#.to_string(),
            expected: "  abc   ".to_string(),
        });
    }
    
    fn add_math_tests(&mut self) {
        // _.sum
        self.test_cases.push(LodashTestCase {
            name: "sum".to_string(),
            category: LodashCategory::Math,
            test_code: r#"
                const _ = require('lodash');
                _.sum([4, 2, 8, 6]);
            "#.to_string(),
            expected: "20".to_string(),
        });
        
        // _.mean
        self.test_cases.push(LodashTestCase {
            name: "mean".to_string(),
            category: LodashCategory::Math,
            test_code: r#"
                const _ = require('lodash');
                _.mean([4, 2, 8, 6]);
            "#.to_string(),
            expected: "5".to_string(),
        });
        
        // _.max
        self.test_cases.push(LodashTestCase {
            name: "max".to_string(),
            category: LodashCategory::Math,
            test_code: r#"
                const _ = require('lodash');
                _.max([4, 2, 8, 6]);
            "#.to_string(),
            expected: "8".to_string(),
        });
        
        // _.min
        self.test_cases.push(LodashTestCase {
            name: "min".to_string(),
            category: LodashCategory::Math,
            test_code: r#"
                const _ = require('lodash');
                _.min([4, 2, 8, 6]);
            "#.to_string(),
            expected: "2".to_string(),
        });
        
        // _.clamp
        self.test_cases.push(LodashTestCase {
            name: "clamp".to_string(),
            category: LodashCategory::Math,
            test_code: r#"
                const _ = require('lodash');
                _.clamp(-10, -5, 5);
            "#.to_string(),
            expected: "-5".to_string(),
        });
        
        // _.round
        self.test_cases.push(LodashTestCase {
            name: "round".to_string(),
            category: LodashCategory::Math,
            test_code: r#"
                const _ = require('lodash');
                _.round(4.006, 2);
            "#.to_string(),
            expected: "4.01".to_string(),
        });
    }
    
    /// Get all test cases
    pub fn test_cases(&self) -> &[LodashTestCase] {
        &self.test_cases
    }
    
    /// Get test cases by category
    pub fn test_cases_by_category(&self, category: LodashCategory) -> Vec<&LodashTestCase> {
        self.test_cases.iter()
            .filter(|tc| tc.category == category)
            .collect()
    }
    
    /// Get test count by category
    pub fn count_by_category(&self) -> HashMap<LodashCategory, usize> {
        let mut counts = HashMap::new();
        for tc in &self.test_cases {
            *counts.entry(tc.category).or_insert(0) += 1;
        }
        counts
    }
}

impl Default for LodashTestSuite {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_suite_has_tests() {
        let suite = LodashTestSuite::new();
        assert!(!suite.test_cases().is_empty());
    }
    
    #[test]
    fn test_suite_has_all_categories() {
        let suite = LodashTestSuite::new();
        let counts = suite.count_by_category();
        
        assert!(counts.contains_key(&LodashCategory::Array));
        assert!(counts.contains_key(&LodashCategory::Collection));
        assert!(counts.contains_key(&LodashCategory::Lang));
        assert!(counts.contains_key(&LodashCategory::Object));
        assert!(counts.contains_key(&LodashCategory::String));
        assert!(counts.contains_key(&LodashCategory::Math));
    }
    
    #[test]
    fn test_array_tests_exist() {
        let suite = LodashTestSuite::new();
        let array_tests = suite.test_cases_by_category(LodashCategory::Array);
        
        let names: Vec<_> = array_tests.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"chunk"));
        assert!(names.contains(&"compact"));
        assert!(names.contains(&"uniq"));
    }
    
    #[test]
    fn test_collection_tests_exist() {
        let suite = LodashTestSuite::new();
        let collection_tests = suite.test_cases_by_category(LodashCategory::Collection);
        
        let names: Vec<_> = collection_tests.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"map"));
        assert!(names.contains(&"filter"));
        assert!(names.contains(&"reduce"));
    }
}
