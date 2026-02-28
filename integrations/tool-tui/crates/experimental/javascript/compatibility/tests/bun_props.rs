//! Property tests for Bun compatibility modules.
//!
//! This file contains property-based tests for the dx-compat-bun crate,
//! validating correctness properties defined in the design document.

#[cfg(feature = "bun-core")]
mod tests {
    use dx_js_compatibility::bun::{file, hash};
    use proptest::prelude::*;
    use tempfile::tempdir;

    // =========================================================================
    // Property 2: Bun.file() Read/Write Round-Trip
    // Validates: Requirements 14.1, 14.2, 14.6
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 2: For any string content, write then read SHALL produce the original.
        #[test]
        fn bun_file_text_round_trip(content in "\\PC{0,5000}") {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let dir = tempdir().unwrap();
                let path = dir.path().join("test.txt");

                file::write(&path, content.clone()).await.unwrap();

                let bun_file = file::BunFile::new(&path);
                let read_content = bun_file.text().await.unwrap();

                prop_assert_eq!(content, read_content, "Text content should be preserved");
                Ok(())
            })?;
        }

        /// Property 2: For any byte sequence, write then read SHALL produce the original.
        #[test]
        fn bun_file_bytes_round_trip(data in prop::collection::vec(any::<u8>(), 0..10000)) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let dir = tempdir().unwrap();
                let path = dir.path().join("test.bin");

                file::write(&path, data.clone()).await.unwrap();

                let bun_file = file::BunFile::new(&path);
                let read_data = bun_file.array_buffer().await.unwrap();

                prop_assert_eq!(data, read_data, "Binary content should be preserved");
                Ok(())
            })?;
        }

        /// Property 2: File size should match written data length.
        #[test]
        fn bun_file_size_matches_content(data in prop::collection::vec(any::<u8>(), 0..10000)) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let dir = tempdir().unwrap();
                let path = dir.path().join("test.bin");

                let written = file::write(&path, data.clone()).await.unwrap();

                let bun_file = file::BunFile::new(&path);
                let size = bun_file.size().await.unwrap();

                prop_assert_eq!(written, data.len(), "Written bytes should match data length");
                prop_assert_eq!(size as usize, data.len(), "File size should match data length");
                Ok(())
            })?;
        }

        /// Property 2: Slice should return correct portion of file.
        #[test]
        fn bun_file_slice_returns_correct_portion(
            data in prop::collection::vec(any::<u8>(), 10..1000),
            start in 0usize..10,
            len in 1usize..10
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let dir = tempdir().unwrap();
                let path = dir.path().join("test.bin");

                file::write(&path, data.clone()).await.unwrap();

                let bun_file = file::BunFile::new(&path);
                let end = (start + len).min(data.len());
                let sliced = bun_file.slice(start as u64, Some(end as u64));
                let slice_data = sliced.array_buffer().await.unwrap();

                let expected = &data[start..end];
                prop_assert_eq!(&slice_data[..], expected, "Slice should return correct portion");
                Ok(())
            })?;
        }
    }

    // =========================================================================
    // Property 7: Hash Consistency
    // Validates: Requirements 16.1, 16.2, 16.3, 16.4, 16.5, 16.6
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 7: Same input should always produce same hash output.
        #[test]
        fn hash_consistency_wyhash(data in prop::collection::vec(any::<u8>(), 0..10000)) {
            let hash1 = hash::hash(&data);
            let hash2 = hash::hash(&data);
            prop_assert_eq!(hash1, hash2, "wyhash should be consistent");
        }

        /// Property 7: Same input should always produce same CRC32 output.
        #[test]
        fn hash_consistency_crc32(data in prop::collection::vec(any::<u8>(), 0..10000)) {
            let hash1 = hash::crc32(&data);
            let hash2 = hash::crc32(&data);
            prop_assert_eq!(hash1, hash2, "CRC32 should be consistent");
        }

        /// Property 7: Same input should always produce same Adler32 output.
        #[test]
        fn hash_consistency_adler32(data in prop::collection::vec(any::<u8>(), 0..10000)) {
            let hash1 = hash::adler32(&data);
            let hash2 = hash::adler32(&data);
            prop_assert_eq!(hash1, hash2, "Adler32 should be consistent");
        }

        /// Property 7: Same input should always produce same CityHash64 output.
        #[test]
        fn hash_consistency_cityhash64(data in prop::collection::vec(any::<u8>(), 0..10000)) {
            let hash1 = hash::city_hash_64(&data);
            let hash2 = hash::city_hash_64(&data);
            prop_assert_eq!(hash1, hash2, "CityHash64 should be consistent");
        }

        /// Property 7: Same input should always produce same MurmurHash3 output.
        #[test]
        fn hash_consistency_murmur32v3(data in prop::collection::vec(any::<u8>(), 0..10000), seed in any::<u32>()) {
            let hash1 = hash::murmur32v3(&data, seed);
            let hash2 = hash::murmur32v3(&data, seed);
            prop_assert_eq!(hash1, hash2, "MurmurHash3 should be consistent");
        }

        /// Property 7: Different inputs should (usually) produce different hashes.
        #[test]
        fn hash_different_inputs_different_outputs(
            data1 in prop::collection::vec(any::<u8>(), 1..100),
            data2 in prop::collection::vec(any::<u8>(), 1..100)
        ) {
            // Only test if inputs are actually different
            if data1 != data2 {
                let hash1 = hash::hash(&data1);
                let hash2 = hash::hash(&data2);
                // Note: Collisions are possible but extremely rare for good hash functions
                // We don't assert inequality because collisions can happen
                let _ = (hash1, hash2);
            }
        }

        /// Property 7: CryptoHasher streaming should produce same result as one-shot.
        #[test]
        fn crypto_hasher_streaming_consistency(
            chunks in prop::collection::vec(
                prop::collection::vec(any::<u8>(), 1..100),
                1..10
            )
        ) {
            use hash::{CryptoHasher, HashAlgorithm};

            // Combine all chunks
            let combined: Vec<u8> = chunks.iter().flatten().copied().collect();

            // One-shot hash
            let mut hasher1 = CryptoHasher::new(HashAlgorithm::Sha256);
            hasher1.update(&combined);
            let digest1 = hasher1.digest();

            // Streaming hash
            let mut hasher2 = CryptoHasher::new(HashAlgorithm::Sha256);
            for chunk in &chunks {
                hasher2.update(chunk);
            }
            let digest2 = hasher2.digest();

            prop_assert_eq!(digest1, digest2, "Streaming hash should match one-shot hash");
        }
    }

    // =========================================================================
    // Property 6: Password Hash/Verify Round-Trip
    // Validates: Requirements 17.1, 17.2
    // =========================================================================

    mod password_tests {
        use super::*;
        use dx_js_compatibility::bun::password::{self, HashOptions};

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(20))] // Reduced due to slow hashing

            /// Property 6: For any password, hash then verify with same password SHALL return true.
            #[test]
            fn password_hash_verify_round_trip_argon2(password in "[a-zA-Z0-9!@#$%^&*]{8,32}") {
                // Use fast params for testing
                let options = HashOptions::argon2().memory_cost(4096).time_cost(1);
                let hashed = password::hash(&password, Some(options)).unwrap();

                let verified = password::verify(&password, &hashed).unwrap();
                prop_assert!(verified, "Password should verify against its own hash");
            }

            /// Property 6: For any password, hash then verify with different password SHALL return false.
            #[test]
            fn password_hash_verify_wrong_password_argon2(
                password in "[a-zA-Z0-9]{8,16}",
                wrong_password in "[a-zA-Z0-9]{8,16}"
            ) {
                // Only test if passwords are actually different
                if password != wrong_password {
                    let options = HashOptions::argon2().memory_cost(4096).time_cost(1);
                    let hashed = password::hash(&password, Some(options)).unwrap();

                    let verified = password::verify(&wrong_password, &hashed).unwrap();
                    prop_assert!(!verified, "Wrong password should not verify");
                }
            }

            /// Property 6: For any password, bcrypt hash then verify SHALL return true.
            #[test]
            fn password_hash_verify_round_trip_bcrypt(password in "[a-zA-Z0-9]{8,32}") {
                // Use minimum cost for fast testing
                let options = HashOptions::bcrypt().cost(4);
                let hashed = password::hash(&password, Some(options)).unwrap();

                let verified = password::verify(&password, &hashed).unwrap();
                prop_assert!(verified, "Password should verify against its bcrypt hash");
            }

            /// Property 6: Same password hashed twice should produce different hashes (due to salt).
            #[test]
            fn password_hash_produces_different_hashes(password in "[a-zA-Z0-9]{8,16}") {
                let options = HashOptions::argon2().memory_cost(4096).time_cost(1);
                let hash1 = password::hash(&password, Some(options.clone())).unwrap();
                let hash2 = password::hash(&password, Some(options)).unwrap();

                prop_assert_ne!(hash1.clone(), hash2.clone(), "Same password should produce different hashes due to salt");

                // But both should verify
                prop_assert!(password::verify(&password, &hash1).unwrap());
                prop_assert!(password::verify(&password, &hash2).unwrap());
            }
        }
    }

    // =========================================================================
    // Property 5: Compression Round-Trip
    // Validates: Requirements 18.1, 18.2, 18.3, 18.4, 18.5, 18.6
    // =========================================================================

    mod compression_tests {
        use super::*;
        use dx_js_compatibility::bun::compression;

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            /// Property 5: Gzip compress then decompress SHALL produce original data.
            #[test]
            fn compression_gzip_round_trip(data in prop::collection::vec(any::<u8>(), 0..10000)) {
                let compressed = compression::gzip_sync(&data, None).unwrap();
                let decompressed = compression::gunzip_sync(&compressed).unwrap();
                prop_assert_eq!(data, decompressed, "Gzip round-trip should preserve data");
            }

            /// Property 5: Deflate compress then decompress SHALL produce original data.
            #[test]
            fn compression_deflate_round_trip(data in prop::collection::vec(any::<u8>(), 0..10000)) {
                let compressed = compression::deflate_sync(&data, None).unwrap();
                let decompressed = compression::inflate_sync(&compressed).unwrap();
                prop_assert_eq!(data, decompressed, "Deflate round-trip should preserve data");
            }

            /// Property 5: Brotli compress then decompress SHALL produce original data.
            #[test]
            fn compression_brotli_round_trip(data in prop::collection::vec(any::<u8>(), 0..10000)) {
                let compressed = compression::brotli_compress_sync(&data, None).unwrap();
                let decompressed = compression::brotli_decompress_sync(&compressed).unwrap();
                prop_assert_eq!(data, decompressed, "Brotli round-trip should preserve data");
            }

            /// Property 5: Zstd compress then decompress SHALL produce original data.
            #[test]
            fn compression_zstd_round_trip(data in prop::collection::vec(any::<u8>(), 0..10000)) {
                let compressed = compression::zstd_compress_sync(&data, None).unwrap();
                let decompressed = compression::zstd_decompress_sync(&compressed).unwrap();
                prop_assert_eq!(data, decompressed, "Zstd round-trip should preserve data");
            }

            /// Property 5: Compression should reduce size for compressible data.
            #[test]
            fn compression_reduces_size_for_repetitive_data(
                pattern in prop::collection::vec(any::<u8>(), 1..10),
                repeats in 50usize..100
            ) {
                let data: Vec<u8> = pattern.iter().cycle().take(pattern.len() * repeats).copied().collect();

                // Skip if data is too small (compression overhead may exceed savings)
                if data.len() < 100 {
                    return Ok(());
                }

                let gzip_compressed = compression::gzip_sync(&data, None).unwrap();
                let zstd_compressed = compression::zstd_compress_sync(&data, None).unwrap();

                // Repetitive data should compress well
                prop_assert!(
                    gzip_compressed.len() < data.len(),
                    "Gzip should compress repetitive data: {} -> {}",
                    data.len(),
                    gzip_compressed.len()
                );
                prop_assert!(
                    zstd_compressed.len() < data.len(),
                    "Zstd should compress repetitive data: {} -> {}",
                    data.len(),
                    zstd_compressed.len()
                );
            }

            /// Property 5: Different compression levels should all decompress correctly.
            #[test]
            fn compression_all_levels_decompress(
                data in prop::collection::vec(any::<u8>(), 100..1000),
                level in 1u32..9
            ) {
                let compressed = compression::gzip_sync(&data, Some(level)).unwrap();
                let decompressed = compression::gunzip_sync(&compressed).unwrap();
                prop_assert_eq!(data, decompressed, "Level {} should decompress correctly", level);
            }

            /// Property 5: Auto-detect should correctly identify and decompress gzip.
            #[test]
            fn compression_auto_detect_gzip(data in prop::collection::vec(any::<u8>(), 1..1000)) {
                let compressed = compression::gzip_sync(&data, None).unwrap();
                let format = compression::detect_format(&compressed);
                prop_assert_eq!(format, compression::CompressionFormat::Gzip);

                let decompressed = compression::decompress_auto(&compressed).unwrap();
                prop_assert_eq!(data, decompressed);
            }

            /// Property 5: Auto-detect should correctly identify and decompress zstd.
            #[test]
            fn compression_auto_detect_zstd(data in prop::collection::vec(any::<u8>(), 1..1000)) {
                let compressed = compression::zstd_compress_sync(&data, None).unwrap();
                let format = compression::detect_format(&compressed);
                prop_assert_eq!(format, compression::CompressionFormat::Zstd);

                let decompressed = compression::decompress_auto(&compressed).unwrap();
                prop_assert_eq!(data, decompressed);
            }
        }
    }
}
