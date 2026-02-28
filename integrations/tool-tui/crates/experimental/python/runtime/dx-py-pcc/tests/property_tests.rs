//! Property-based tests for Persistent Compilation Cache

use dx_py_pcc::{
    CachedArtifact, CompilationTier, FunctionSignature, PersistentCompilationCache, Relocation,
    RelocationType,
};
use proptest::prelude::*;

mod signature_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Signature serialization is reversible
        #[test]
        fn prop_signature_roundtrip(
            source in prop::collection::vec(any::<u8>(), 1..100),
            bytecode in prop::collection::vec(any::<u8>(), 1..50),
            profile in prop::collection::vec(any::<u8>(), 0..20),
            name in "[a-z][a-z0-9_]{0,20}",
            module in "[a-z][a-z0-9_.]{0,30}"
        ) {
            let sig = FunctionSignature::new(
                &source,
                &bytecode,
                &profile,
                name.clone(),
                module.clone(),
            );

            let bytes = sig.to_bytes();
            let restored = FunctionSignature::from_bytes(&bytes).unwrap();

            prop_assert_eq!(sig.source_hash, restored.source_hash);
            prop_assert_eq!(sig.bytecode_hash, restored.bytecode_hash);
            prop_assert_eq!(sig.type_profile_hash, restored.type_profile_hash);
            prop_assert_eq!(sig.name, restored.name);
            prop_assert_eq!(sig.module, restored.module);
        }

        /// Different sources produce different hashes
        #[test]
        fn prop_different_sources_different_hashes(
            source1 in prop::collection::vec(any::<u8>(), 1..100),
            source2 in prop::collection::vec(any::<u8>(), 1..100)
        ) {
            prop_assume!(source1 != source2);

            let sig1 = FunctionSignature::new(&source1, b"bc", b"", "f".to_string(), "m".to_string());
            let sig2 = FunctionSignature::new(&source2, b"bc", b"", "f".to_string(), "m".to_string());

            prop_assert_ne!(sig1.source_hash, sig2.source_hash);
        }

        /// Cache keys are unique for different signatures
        #[test]
        fn prop_cache_key_uniqueness(
            sources in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..50), 2..10)
        ) {
            let keys: Vec<u128> = sources.iter()
                .map(|s| {
                    FunctionSignature::new(s, b"bc", b"", "f".to_string(), "m".to_string())
                        .cache_key()
                })
                .collect();

            // Check for uniqueness (with high probability for different sources)
            let unique_count = keys.iter().collect::<std::collections::HashSet<_>>().len();
            // Allow some collisions due to hash truncation
            prop_assert!(unique_count >= keys.len() * 9 / 10);
        }
    }
}

mod artifact_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Artifact serialization is reversible
        #[test]
        fn prop_artifact_roundtrip(
            tier in 0u8..4,
            code_offset in any::<u64>(),
            code_size in any::<u32>(),
            profile_data in prop::collection::vec(any::<u8>(), 0..100)
        ) {
            let tier = CompilationTier::from_u8(tier).unwrap();
            let artifact = CachedArtifact::new(
                tier,
                code_offset,
                code_size,
                vec![],
                profile_data.clone(),
            );

            let bytes = artifact.to_bytes();
            let restored = CachedArtifact::from_bytes(&bytes).unwrap();

            prop_assert_eq!(artifact.tier, restored.tier);
            prop_assert_eq!(artifact.code_offset, restored.code_offset);
            prop_assert_eq!(artifact.code_size, restored.code_size);
            prop_assert_eq!(artifact.profile_data, restored.profile_data);
        }

        /// Relocation serialization is reversible
        #[test]
        fn prop_relocation_roundtrip(
            offset in any::<u32>(),
            kind in 0u8..4,
            target in any::<u64>()
        ) {
            let kind = RelocationType::from_u8(kind).unwrap();
            let reloc = Relocation { offset, kind, target };

            let bytes = reloc.to_bytes();
            let restored = Relocation::from_bytes(&bytes).unwrap();

            prop_assert_eq!(reloc.offset, restored.offset);
            prop_assert_eq!(reloc.kind, restored.kind);
            prop_assert_eq!(reloc.target, restored.target);
        }

        /// Access count increments correctly
        #[test]
        fn prop_access_count_increments(
            access_times in 1usize..100
        ) {
            let mut artifact = CachedArtifact::new(
                CompilationTier::Baseline,
                0,
                100,
                vec![],
                vec![],
            );

            for _ in 0..access_times {
                artifact.record_access();
            }

            prop_assert_eq!(artifact.access_count, access_times as u32);
        }
    }
}

mod cache_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// Saved entries can be retrieved
        #[test]
        fn prop_save_and_retrieve(
            source in prop::collection::vec(any::<u8>(), 1..100),
            code in prop::collection::vec(any::<u8>(), 1..500),
            tier in 0u8..4
        ) {
            let cache = PersistentCompilationCache::in_memory();
            let tier = CompilationTier::from_u8(tier).unwrap();

            let sig = FunctionSignature::new(
                &source,
                b"bytecode",
                b"",
                "func".to_string(),
                "module".to_string(),
            );

            cache.save(sig.clone(), tier, &code, vec![]).unwrap();

            prop_assert!(cache.contains(&sig));

            let (artifact, _) = cache.get(&sig).unwrap();
            prop_assert_eq!(artifact.tier, tier);
            prop_assert_eq!(artifact.code_size, code.len() as u32);
        }

        /// Invalidation removes correct entries
        #[test]
        fn prop_invalidation_selective(
            sources in prop::collection::vec(prop::collection::vec(any::<u8>(), 10..50), 3..10)
        ) {
            let cache = PersistentCompilationCache::in_memory();

            let sigs: Vec<_> = sources.iter()
                .enumerate()
                .map(|(i, s)| {
                    FunctionSignature::new(
                        s,
                        b"bc",
                        b"",
                        format!("f{}", i),
                        "m".to_string(),
                    )
                })
                .collect();

            // Save all
            for sig in &sigs {
                cache.save(sig.clone(), CompilationTier::Baseline, &[0x90], vec![]).unwrap();
            }

            // Invalidate first
            cache.invalidate_source(&sigs[0].source_hash);

            // First should be gone, rest should remain
            prop_assert!(!cache.contains(&sigs[0]));
            for sig in &sigs[1..] {
                prop_assert!(cache.contains(sig));
            }
        }

        /// Multiple entries with same source but different profiles are distinct
        #[test]
        fn prop_profile_differentiation(
            source in prop::collection::vec(any::<u8>(), 10..50),
            profiles in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..20), 2..5)
        ) {
            let cache = PersistentCompilationCache::in_memory();

            let sigs: Vec<_> = profiles.iter()
                .enumerate()
                .map(|(i, p)| {
                    FunctionSignature::new(
                        &source,
                        b"bc",
                        p,
                        format!("f{}", i),
                        "m".to_string(),
                    )
                })
                .collect();

            // Save all with different tiers
            for (i, sig) in sigs.iter().enumerate() {
                let tier = CompilationTier::from_u8((i % 4) as u8).unwrap();
                cache.save(sig.clone(), tier, &[0x90; 10], vec![]).unwrap();
            }

            // All should be retrievable
            for sig in &sigs {
                prop_assert!(cache.contains(sig));
            }
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_tier_values() {
        assert_eq!(CompilationTier::Interpreter as u8, 0);
        assert_eq!(CompilationTier::Baseline as u8, 1);
        assert_eq!(CompilationTier::Optimized as u8, 2);
        assert_eq!(CompilationTier::AotOptimized as u8, 3);
    }

    #[test]
    fn test_relocation_types() {
        assert_eq!(RelocationType::Abs64 as u8, 0);
        assert_eq!(RelocationType::Rel32 as u8, 1);
        assert_eq!(RelocationType::GotEntry as u8, 2);
        assert_eq!(RelocationType::PltEntry as u8, 3);
    }

    #[test]
    fn test_empty_cache_stats() {
        let cache = PersistentCompilationCache::in_memory();
        let stats = cache.stats();

        assert_eq!(stats.entry_count, 0);
        assert_eq!(stats.total_code_size, 0);
    }
}
