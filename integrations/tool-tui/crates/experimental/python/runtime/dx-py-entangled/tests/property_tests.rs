//! Property-based tests for Entangled Objects

use dx_py_entangled::object::TypeInfo;
use dx_py_entangled::{EntangledArray, EntangledHandle, EntangledObject, SharedMemoryRegion};
use proptest::prelude::*;
use std::sync::Arc;

fn create_test_region(suffix: &str) -> Arc<SharedMemoryRegion> {
    let name = format!("prop_test_{}_{}", std::process::id(), suffix);
    Arc::new(SharedMemoryRegion::create(&name, 4 * 1024 * 1024).unwrap())
}

/// Property 15: Entangled Object Cross-Process Consistency
mod consistency_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// Data written is exactly what is read back
        #[test]
        fn prop_read_write_consistency(
            data in prop::collection::vec(any::<u8>(), 1..1000)
        ) {
            let region = create_test_region("rw");

            let obj = EntangledObject::create(
                region,
                TypeInfo::Bytes,
                &data,
            ).unwrap();

            let read_back = obj.read();
            prop_assert_eq!(read_back, &data[..]);
        }

        /// Handle serialization preserves all fields
        #[test]
        fn prop_handle_roundtrip(
            data in prop::collection::vec(any::<u8>(), 1..500)
        ) {
            let region = create_test_region("handle");

            let obj = EntangledObject::create(
                region.clone(),
                TypeInfo::Bytes,
                &data,
            ).unwrap();

            let handle = EntangledHandle::from_object(&obj);
            let bytes = handle.to_bytes();
            let restored = EntangledHandle::from_bytes(&bytes).unwrap();

            prop_assert_eq!(handle.id, restored.id);
            prop_assert_eq!(handle.offset, restored.offset);
            prop_assert_eq!(handle.type_info, restored.type_info);
            prop_assert_eq!(handle.size, restored.size);
        }

        /// Multiple objects in same region are independent
        #[test]
        fn prop_object_independence(
            data1 in prop::collection::vec(any::<u8>(), 10..100),
            data2 in prop::collection::vec(any::<u8>(), 10..100)
        ) {
            let region = create_test_region("indep");

            let obj1 = EntangledObject::create(
                region.clone(),
                TypeInfo::Bytes,
                &data1,
            ).unwrap();

            let obj2 = EntangledObject::create(
                region.clone(),
                TypeInfo::Bytes,
                &data2,
            ).unwrap();

            // Objects have different IDs
            prop_assert_ne!(obj1.id(), obj2.id());

            // Data is independent
            prop_assert_eq!(obj1.read(), &data1[..]);
            prop_assert_eq!(obj2.read(), &data2[..]);
        }
    }
}

/// Property 16: Optimistic Concurrency Version Ordering
mod concurrency_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// Version increments monotonically
        #[test]
        fn prop_version_monotonic(
            writes in 1usize..20
        ) {
            let region = create_test_region("version");
            let data = vec![0u8; 100];

            let obj = EntangledObject::create(
                region,
                TypeInfo::Bytes,
                &data,
            ).unwrap();

            let mut last_version = obj.version();

            for i in 0..writes {
                let new_data = vec![i as u8; 100];
                let new_version = obj.write(&new_data, last_version).unwrap();

                prop_assert!(new_version > last_version);
                last_version = new_version;
            }
        }

        /// Stale version writes fail
        #[test]
        fn prop_stale_version_fails(
            initial in prop::collection::vec(any::<u8>(), 50..100)
        ) {
            let region = create_test_region("stale");

            let obj = EntangledObject::create(
                region,
                TypeInfo::Bytes,
                &initial,
            ).unwrap();

            let v1 = obj.version();

            // First write succeeds
            let new_data = vec![0xAA; initial.len()];
            obj.write(&new_data, v1).unwrap();

            // Second write with old version fails
            let result = obj.write(&vec![0xBB; initial.len()], v1);
            prop_assert!(result.is_err());
        }

        /// CAS succeeds only with matching data
        #[test]
        fn prop_cas_correctness(
            data in prop::collection::vec(any::<u8>(), 20..50)
        ) {
            let region = create_test_region("cas");

            let obj = EntangledObject::create(
                region,
                TypeInfo::Bytes,
                &data,
            ).unwrap();

            // CAS with correct expected value succeeds
            let new_data = vec![0xFF; data.len()];
            let success = obj.cas_write(&data, &new_data).unwrap();
            prop_assert!(success);

            // CAS with wrong expected value fails
            let success = obj.cas_write(&data, &vec![0x00; data.len()]).unwrap();
            prop_assert!(!success);
        }
    }
}

mod array_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(30))]

        /// Float array data is preserved
        #[test]
        fn prop_f64_array_roundtrip(
            data in prop::collection::vec(
                any::<f64>().prop_filter("finite", |f| f.is_finite()),
                1..100
            )
        ) {
            let region = create_test_region("f64arr");
            let shape = vec![data.len() as u64];

            let arr = EntangledArray::from_f64(region, &data, &shape).unwrap();

            let slice = arr.as_f64_slice().unwrap();
            prop_assert_eq!(slice.len(), data.len());

            for (a, b) in slice.iter().zip(data.iter()) {
                prop_assert!((a - b).abs() < 1e-10);
            }
        }

        /// Int array data is preserved
        #[test]
        fn prop_i64_array_roundtrip(
            data in prop::collection::vec(any::<i64>(), 1..100)
        ) {
            let region = create_test_region("i64arr");
            let shape = vec![data.len() as u64];

            let arr = EntangledArray::from_i64(region, &data, &shape).unwrap();

            let slice = arr.as_i64_slice().unwrap();
            prop_assert_eq!(slice, &data[..]);
        }

        /// Add scalar preserves array structure
        #[test]
        fn prop_add_scalar_structure(
            data in prop::collection::vec(
                any::<f64>().prop_filter("finite", |f| f.is_finite() && f.abs() < 1e10),
                1..50
            ),
            scalar in any::<f64>().prop_filter("finite", |f| f.is_finite() && f.abs() < 1e10)
        ) {
            let region = create_test_region("addscalar");
            let shape = vec![data.len() as u64];

            let arr = EntangledArray::from_f64(region, &data, &shape).unwrap();
            arr.add_scalar_f64(scalar).unwrap();

            let slice = arr.as_f64_slice().unwrap();
            prop_assert_eq!(slice.len(), data.len());

            for (result, original) in slice.iter().zip(data.iter()) {
                let expected = original + scalar;
                prop_assert!((result - expected).abs() < 1e-10);
            }
        }

        /// Shape is preserved
        #[test]
        fn prop_shape_preserved(
            dim1 in 1u64..10,
            dim2 in 1u64..10
        ) {
            let region = create_test_region("shape");
            let len = (dim1 * dim2) as usize;
            let data: Vec<f64> = (0..len).map(|i| i as f64).collect();
            let shape = vec![dim1, dim2];

            let arr = EntangledArray::from_f64(region.clone(), &data, &shape).unwrap();

            prop_assert_eq!(arr.shape(), &shape[..]);
            prop_assert_eq!(arr.ndim(), 2);
            prop_assert_eq!(arr.len(), len);

            // Reopen and verify
            let arr2 = EntangledArray::open(region, arr.get_handle().offset).unwrap();
            prop_assert_eq!(arr2.shape(), &shape[..]);
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_type_info_values() {
        assert_eq!(TypeInfo::Bytes as u8, 0);
        assert_eq!(TypeInfo::Int as u8, 1);
        assert_eq!(TypeInfo::Float as u8, 2);
        assert_eq!(TypeInfo::String as u8, 3);
        assert_eq!(TypeInfo::FloatArray as u8, 4);
        assert_eq!(TypeInfo::IntArray as u8, 5);
        assert_eq!(TypeInfo::Object as u8, 6);
    }

    #[test]
    fn test_region_allocation() {
        let region = create_test_region("alloc");

        let off1 = region.allocate(100, 8).unwrap();
        let off2 = region.allocate(200, 16).unwrap();

        assert!(off2 > off1);
        assert_eq!(off2 % 16, 0);
    }
}
