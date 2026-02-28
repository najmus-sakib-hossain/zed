//! Property-based tests for TestDataGenerator
//!
//! **Feature: comparative-benchmarks**

use dx_py_benchmarks::data::{DataSize, TestDataGenerator, TestPattern};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 16: Deterministic Generation Round-Trip**
    /// *For any* seed value S and data generation parameters P, calling the generator
    /// twice with (S, P) SHALL produce identical output.
    /// **Validates: Requirements 7.2, 8.3**
    #[test]
    fn property_deterministic_generation_round_trip(seed in any::<u64>()) {
        // Create two generators with the same seed
        let mut gen1 = TestDataGenerator::new(seed);
        let mut gen2 = TestDataGenerator::new(seed);

        // Generate JSON data with both
        let json1 = gen1.generate_json_data(DataSize::Small);
        let json2 = gen2.generate_json_data(DataSize::Small);

        prop_assert_eq!(&json1, &json2,
            "Same seed should produce identical JSON data");
    }

    /// Test deterministic string generation
    #[test]
    fn property_deterministic_string_generation(seed in any::<u64>()) {
        let mut gen1 = TestDataGenerator::new(seed);
        let mut gen2 = TestDataGenerator::new(seed);

        let str1 = gen1.generate_string_data(DataSize::Small);
        let str2 = gen2.generate_string_data(DataSize::Small);

        prop_assert_eq!(&str1, &str2,
            "Same seed should produce identical string data");
    }

    /// Test deterministic test file generation
    #[test]
    fn property_deterministic_test_file_generation(
        seed in any::<u64>(),
        count in 1usize..10
    ) {
        let mut gen1 = TestDataGenerator::new(seed);
        let mut gen2 = TestDataGenerator::new(seed);

        let files1 = gen1.generate_test_files(count, TestPattern::SimpleFunctions);
        let files2 = gen2.generate_test_files(count, TestPattern::SimpleFunctions);

        prop_assert_eq!(files1.len(), files2.len(),
            "Same seed should produce same number of files");

        for (f1, f2) in files1.iter().zip(files2.iter()) {
            prop_assert_eq!(&f1.name, &f2.name,
                "Same seed should produce identical file names");
            prop_assert_eq!(&f1.content, &f2.content,
                "Same seed should produce identical file content");
        }
    }

    /// Test that reset produces same sequence
    #[test]
    fn property_reset_produces_same_sequence(seed in any::<u64>()) {
        let mut gen = TestDataGenerator::new(seed);

        // Generate some data
        let json1 = gen.generate_json_data(DataSize::Small);
        let str1 = gen.generate_string_data(DataSize::Small);

        // Reset and generate again
        gen.reset();
        let json2 = gen.generate_json_data(DataSize::Small);
        let str2 = gen.generate_string_data(DataSize::Small);

        prop_assert_eq!(&json1, &json2,
            "Reset should produce same JSON data");
        prop_assert_eq!(&str1, &str2,
            "Reset should produce same string data");
    }

    /// Test that different seeds produce different output
    #[test]
    fn property_different_seeds_different_output(
        seed1 in any::<u64>(),
        seed2 in any::<u64>()
    ) {
        prop_assume!(seed1 != seed2);

        let mut gen1 = TestDataGenerator::new(seed1);
        let mut gen2 = TestDataGenerator::new(seed2);

        let json1 = gen1.generate_json_data(DataSize::Small);
        let json2 = gen2.generate_json_data(DataSize::Small);

        // Different seeds should (almost always) produce different output
        // There's a tiny chance of collision, but it's negligible
        prop_assert_ne!(&json1, &json2,
            "Different seeds should produce different JSON data");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 17: Data Size Configurability**
    /// *For any* requested data size (Small, Medium, Large), the TestDataGenerator SHALL
    /// produce data within the expected size range (Small: ~1KB, Medium: ~100KB, Large: ~10MB)
    /// with tolerance of ±50%.
    /// **Validates: Requirements 8.1**
    #[test]
    fn property_data_size_small(seed in any::<u64>()) {
        let mut gen = TestDataGenerator::new(seed);
        let data = gen.generate_json_data(DataSize::Small);
        let (min, max) = DataSize::Small.acceptable_range();

        prop_assert!(data.len() >= min && data.len() <= max,
            "Small data size {} should be within [{}, {}]", data.len(), min, max);
    }

    #[test]
    fn property_data_size_medium(seed in any::<u64>()) {
        let mut gen = TestDataGenerator::new(seed);
        let data = gen.generate_json_data(DataSize::Medium);
        let (min, max) = DataSize::Medium.acceptable_range();

        prop_assert!(data.len() >= min && data.len() <= max,
            "Medium data size {} should be within [{}, {}]", data.len(), min, max);
    }

    /// Test string data size bounds
    #[test]
    fn property_string_data_size_bounds(seed in any::<u64>()) {
        let mut gen = TestDataGenerator::new(seed);

        for size in [DataSize::Small, DataSize::Medium] {
            let data = gen.generate_string_data(size);
            let (min, max) = size.acceptable_range();

            prop_assert!(data.len() >= min && data.len() <= max,
                "{:?} string data size {} should be within [{}, {}]", size, data.len(), min, max);
        }
    }

    /// Test that larger sizes produce larger data
    #[test]
    fn property_size_ordering(seed in any::<u64>()) {
        let mut gen = TestDataGenerator::new(seed);

        let small = gen.generate_json_data(DataSize::Small);
        gen.reset();
        let medium = gen.generate_json_data(DataSize::Medium);

        prop_assert!(small.len() < medium.len(),
            "Small ({}) should be less than Medium ({})", small.len(), medium.len());
    }
}

// Separate test with fewer iterations for large data (expensive to generate)
proptest! {
    #![proptest_config(ProptestConfig::with_cases(5))]

    #[test]
    fn property_data_size_large(seed in any::<u64>()) {
        let mut gen = TestDataGenerator::new(seed);
        let data = gen.generate_json_data(DataSize::Large);
        let (min, max) = DataSize::Large.acceptable_range();

        prop_assert!(data.len() >= min && data.len() <= max,
            "Large data size {} should be within [{}, {}]", data.len(), min, max);
    }
}
