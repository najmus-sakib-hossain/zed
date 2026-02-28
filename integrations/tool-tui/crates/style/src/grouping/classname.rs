//! Classname generator for auto-grouped CSS patterns
//!
//! Generates short, deterministic, unique classnames for groups of CSS classes.
//! Uses hash-based naming with collision detection and resolution.

use ahash::AHashSet;
use seahash::hash;

/// Generates short, deterministic, unique classnames for groups.
///
/// Format: `dxg-XXXXX` where XXXXX is a 5-char base36 hash.
pub struct ClassnameGenerator {
    /// Existing classnames to avoid conflicts
    existing: AHashSet<String>,
    /// Counter for collision resolution
    counter: u32,
}

impl ClassnameGenerator {
    /// Create a new ClassnameGenerator with a set of existing classnames to avoid.
    pub fn new(existing: AHashSet<String>) -> Self {
        Self {
            existing,
            counter: 0,
        }
    }

    /// Generate a classname for a set of classes.
    ///
    /// The generated classname:
    /// - Starts with "dxg-"
    /// - Has a 5-character base36 suffix
    /// - Is deterministic (same input produces same output)
    /// - Is unique (no collisions with existing classnames)
    pub fn generate(&mut self, classes: &[String]) -> String {
        // Sort for determinism
        let mut sorted = classes.to_vec();
        sorted.sort();

        // Hash the sorted class list
        let input = sorted.join(" ");
        let hash_value = hash(input.as_bytes());

        // Convert to base36 and take 5 chars
        let base36 = to_base36(hash_value);
        let short = if base36.len() >= 5 {
            &base36[..5]
        } else {
            &base36[..]
        };

        let mut candidate = format!("dxg-{}", short);

        // Handle collisions
        let mut collision_counter = 0u32;
        while self.existing.contains(&candidate) {
            collision_counter += 1;
            self.counter += 1;
            let new_hash = hash_value.wrapping_add(collision_counter as u64);
            let new_base36 = to_base36(new_hash);
            let new_short = if new_base36.len() >= 5 {
                &new_base36[..5]
            } else {
                &new_base36[..]
            };
            candidate = format!("dxg-{}", new_short);

            // Safety limit to prevent infinite loops
            if collision_counter > 1000 {
                // Fallback to counter-based unique name
                candidate = format!("dxg-{:05x}", self.counter);
                break;
            }
        }

        self.existing.insert(candidate.clone());
        candidate
    }

    /// Check if a classname already exists.
    pub fn contains(&self, name: &str) -> bool {
        self.existing.contains(name)
    }

    /// Get the number of existing classnames.
    pub fn len(&self) -> usize {
        self.existing.len()
    }

    /// Check if there are no existing classnames.
    pub fn is_empty(&self) -> bool {
        self.existing.is_empty()
    }
}

/// Convert a u64 to base36 string (0-9, a-z).
fn to_base36(mut n: u64) -> String {
    const CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";

    if n == 0 {
        return "0".to_string();
    }

    let mut result = Vec::with_capacity(13); // max 13 chars for u64 in base36

    while n > 0 {
        result.push(CHARS[(n % 36) as usize]);
        n /= 36;
    }

    result.reverse();
    String::from_utf8(result).unwrap()
}

/// Check if a classname is valid (starts with "dxg-" and contains only alphanumeric and hyphens).
pub fn is_valid_classname(name: &str) -> bool {
    if !name.starts_with("dxg-") {
        return false;
    }

    name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_base36_zero() {
        assert_eq!(to_base36(0), "0");
    }

    #[test]
    fn test_to_base36_small() {
        assert_eq!(to_base36(35), "z");
        assert_eq!(to_base36(36), "10");
        assert_eq!(to_base36(10), "a");
    }

    #[test]
    fn test_to_base36_large() {
        // 36^5 = 60466176
        let result = to_base36(60466176);
        assert_eq!(result, "100000");
    }

    #[test]
    fn test_generate_basic() {
        let mut cg = ClassnameGenerator::new(AHashSet::new());
        let name = cg.generate(&["flex".to_string(), "items-center".to_string()]);

        assert!(name.starts_with("dxg-"));
        assert!(name.len() <= 9); // "dxg-" + 5 chars max
    }

    #[test]
    fn test_generate_deterministic() {
        let mut gen1 = ClassnameGenerator::new(AHashSet::new());
        let mut gen2 = ClassnameGenerator::new(AHashSet::new());

        let classes = vec![
            "flex".to_string(),
            "items-center".to_string(),
            "p-4".to_string(),
        ];

        let name1 = gen1.generate(&classes);
        let name2 = gen2.generate(&classes);

        assert_eq!(name1, name2);
    }

    #[test]
    fn test_generate_order_independent() {
        let mut gen1 = ClassnameGenerator::new(AHashSet::new());
        let mut gen2 = ClassnameGenerator::new(AHashSet::new());

        let classes1 = vec![
            "p-4".to_string(),
            "flex".to_string(),
            "items-center".to_string(),
        ];
        let classes2 = vec![
            "flex".to_string(),
            "items-center".to_string(),
            "p-4".to_string(),
        ];

        let name1 = gen1.generate(&classes1);
        let name2 = gen2.generate(&classes2);

        assert_eq!(name1, name2);
    }

    #[test]
    fn test_generate_avoids_existing() {
        let mut existing = AHashSet::new();
        existing.insert("dxg-test1".to_string());

        let mut cg = ClassnameGenerator::new(existing);
        let name = cg.generate(&["flex".to_string()]);

        assert_ne!(name, "dxg-test1");
        assert!(name.starts_with("dxg-"));
    }

    #[test]
    fn test_generate_collision_resolution() {
        // Pre-populate with the expected hash result
        let mut cg = ClassnameGenerator::new(AHashSet::new());
        let classes = vec!["flex".to_string(), "items-center".to_string()];

        // Generate first name
        let name1 = cg.generate(&classes);

        // Try to generate with same classes - should get same result since it's already in existing
        // But if we create a new generator with the first name as existing, it should resolve collision
        let mut existing = AHashSet::new();
        existing.insert(name1.clone());
        let mut cg2 = ClassnameGenerator::new(existing);

        let name2 = cg2.generate(&classes);
        assert_ne!(name1, name2);
        assert!(name2.starts_with("dxg-"));
    }

    #[test]
    fn test_is_valid_classname() {
        assert!(is_valid_classname("dxg-abc12"));
        assert!(is_valid_classname("dxg-00000"));
        assert!(is_valid_classname("dxg-zzzzz"));

        assert!(!is_valid_classname("abc-12345"));
        assert!(!is_valid_classname("dxg_abc12"));
        assert!(!is_valid_classname("dxg-abc!2"));
        assert!(!is_valid_classname(""));
    }

    #[test]
    fn test_classname_length() {
        let mut cg = ClassnameGenerator::new(AHashSet::new());

        // Generate many classnames and verify length constraint
        for i in 0..100 {
            let classes = vec![format!("class-{}", i)];
            let name = cg.generate(&classes);

            // "dxg-" is 4 chars, suffix should be <= 5 chars
            let suffix = &name[4..];
            assert!(
                suffix.len() <= 5,
                "Suffix '{}' is {} chars, expected <= 5",
                suffix,
                suffix.len()
            );
        }
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    // Generate a vector of class names
    fn arb_class_names() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec("[a-z][a-z0-9-]{0,15}", 1..10)
    }

    // Generate a set of existing classnames
    fn arb_existing_classnames() -> impl Strategy<Value = AHashSet<String>> {
        prop::collection::vec("dxg-[a-z0-9]{5}", 0..20).prop_map(|v| v.into_iter().collect())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-style-production-ready, Property 6: Classname Length Constraint
        /// *For any* auto-generated classname, the suffix (after "dxg-") SHALL be 5 characters or fewer.
        /// **Validates: Requirements 3.1**
        #[test]
        fn prop_classname_length_constraint(classes in arb_class_names()) {
            let mut cg = ClassnameGenerator::new(AHashSet::new());
            let name = cg.generate(&classes);

            prop_assert!(name.starts_with("dxg-"), "Name should start with 'dxg-'");

            let suffix = &name[4..];
            prop_assert!(
                suffix.len() <= 5,
                "Suffix '{}' is {} chars, expected <= 5",
                suffix,
                suffix.len()
            );
        }

        /// Feature: dx-style-production-ready, Property 7: Classname Uniqueness
        /// *For any* set of existing classnames and any number of generated classnames,
        /// there SHALL be no duplicates between existing and generated names.
        /// **Validates: Requirements 3.2**
        #[test]
        fn prop_classname_uniqueness(
            existing in arb_existing_classnames(),
            class_sets in prop::collection::vec(arb_class_names(), 1..20)
        ) {
            let mut cg = ClassnameGenerator::new(existing.clone());
            let mut generated: AHashSet<String> = AHashSet::new();

            for classes in class_sets {
                let name = cg.generate(&classes);

                // Should not collide with existing
                prop_assert!(
                    !existing.contains(&name),
                    "Generated name '{}' collides with existing",
                    name
                );

                // Should not collide with previously generated
                prop_assert!(
                    !generated.contains(&name),
                    "Generated name '{}' is a duplicate",
                    name
                );

                generated.insert(name);
            }
        }

        /// Feature: dx-style-production-ready, Property 8: Classname Determinism
        /// *For any* set of input classes, calling the Classname_Generator multiple times
        /// with the same input SHALL produce the same output classname.
        /// **Validates: Requirements 3.3**
        #[test]
        fn prop_classname_determinism(classes in arb_class_names()) {
            let existing = AHashSet::new();
            let mut cg1 = ClassnameGenerator::new(existing.clone());
            let mut cg2 = ClassnameGenerator::new(existing);

            let name1 = cg1.generate(&classes);
            let name2 = cg2.generate(&classes);

            prop_assert_eq!(
                name1, name2,
                "Same input should produce same output"
            );
        }

        /// Feature: dx-style-production-ready, Property 9: Classname Validity
        /// *For any* auto-generated classname, it SHALL start with "dxg-" and contain
        /// only alphanumeric characters and hyphens.
        /// **Validates: Requirements 3.4, 3.5**
        #[test]
        fn prop_classname_validity(classes in arb_class_names()) {
            let mut cg = ClassnameGenerator::new(AHashSet::new());
            let name = cg.generate(&classes);

            prop_assert!(
                is_valid_classname(&name),
                "Generated name '{}' is not valid",
                name
            );

            prop_assert!(
                name.starts_with("dxg-"),
                "Name '{}' should start with 'dxg-'",
                name
            );

            prop_assert!(
                name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
                "Name '{}' contains invalid characters",
                name
            );
        }

        /// Property: base36 encoding produces valid characters
        #[test]
        fn prop_base36_valid_chars(n: u64) {
            let result = to_base36(n);

            prop_assert!(
                result.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
                "base36 result '{}' contains invalid characters",
                result
            );
        }

        /// Property: base36 encoding is deterministic
        #[test]
        fn prop_base36_deterministic(n: u64) {
            let result1 = to_base36(n);
            let result2 = to_base36(n);

            prop_assert_eq!(result1, result2);
        }
    }
}
