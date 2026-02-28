//! Collections integration tests

use dx_py_collections::{SimdList, SimdStorage, SwissDict};

#[test]
fn test_simd_list_int_sum() {
    let list = SimdList::from_ints(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    
    let sum = list.sum().unwrap();
    assert_eq!(sum, 55.0);
}

#[test]
fn test_simd_list_float_sum() {
    let list = SimdList::from_floats(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    
    let sum = list.sum().unwrap();
    assert!((sum - 15.0).abs() < 0.001);
}

#[test]
fn test_simd_list_filter() {
    let list = SimdList::from_ints(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    
    let indices = list.filter_gt_int(5);
    assert_eq!(indices, vec![5, 6, 7, 8, 9]); // indices of 6, 7, 8, 9, 10
}

#[test]
fn test_simd_list_index() {
    let list = SimdList::from_ints(vec![10, 20, 30, 40, 50]);
    
    assert_eq!(list.index_int(30), Some(2));
    assert_eq!(list.index_int(100), None);
}

#[test]
fn test_simd_list_count() {
    let list = SimdList::from_ints(vec![1, 2, 2, 3, 2, 4, 2, 5]);
    
    assert_eq!(list.count_int(2), 4);
    assert_eq!(list.count_int(100), 0);
}

#[test]
fn test_simd_storage_types() {
    // Integer storage
    let int_storage = SimdStorage::Ints(vec![1, 2, 3]);
    assert!(matches!(int_storage, SimdStorage::Ints(_)));
    
    // Float storage
    let float_storage = SimdStorage::Floats(vec![1.0, 2.0, 3.0]);
    assert!(matches!(float_storage, SimdStorage::Floats(_)));
}

#[test]
fn test_swiss_dict_basic() {
    let mut dict: SwissDict<String, i64> = SwissDict::new();
    
    // Insert
    dict.insert("one".to_string(), 1);
    dict.insert("two".to_string(), 2);
    dict.insert("three".to_string(), 3);
    
    assert_eq!(dict.len(), 3);
    
    // Get
    assert_eq!(dict.get(&"one".to_string()), Some(&1));
    assert_eq!(dict.get(&"two".to_string()), Some(&2));
    assert_eq!(dict.get(&"three".to_string()), Some(&3));
    assert_eq!(dict.get(&"four".to_string()), None);
}

#[test]
fn test_swiss_dict_update() {
    let mut dict: SwissDict<String, i64> = SwissDict::new();
    
    dict.insert("key".to_string(), 1);
    assert_eq!(dict.get(&"key".to_string()), Some(&1));
    
    dict.insert("key".to_string(), 2);
    assert_eq!(dict.get(&"key".to_string()), Some(&2));
    assert_eq!(dict.len(), 1);
}

#[test]
fn test_swiss_dict_remove() {
    let mut dict: SwissDict<String, i64> = SwissDict::new();
    
    dict.insert("key".to_string(), 1);
    assert_eq!(dict.len(), 1);
    
    let removed = dict.remove(&"key".to_string());
    assert_eq!(removed, Some(1));
    assert_eq!(dict.len(), 0);
    assert_eq!(dict.get(&"key".to_string()), None);
}

#[test]
fn test_swiss_dict_contains() {
    let mut dict: SwissDict<String, i64> = SwissDict::new();
    
    dict.insert("key".to_string(), 1);
    
    assert!(dict.contains_key(&"key".to_string()));
    assert!(!dict.contains_key(&"other".to_string()));
}

#[test]
fn test_swiss_dict_iter() {
    let mut dict: SwissDict<String, i64> = SwissDict::new();
    
    dict.insert("a".to_string(), 1);
    dict.insert("b".to_string(), 2);
    dict.insert("c".to_string(), 3);
    
    let mut sum = 0;
    for (_, v) in dict.iter() {
        sum += v;
    }
    assert_eq!(sum, 6);
}

#[test]
fn test_swiss_dict_grow() {
    let mut dict: SwissDict<i64, i64> = SwissDict::new();
    
    // Insert many items to trigger growth
    for i in 0..1000 {
        dict.insert(i, i * 2);
    }
    
    assert_eq!(dict.len(), 1000);
    
    // Verify all items are still accessible
    for i in 0..1000 {
        assert_eq!(dict.get(&i), Some(&(i * 2)));
    }
}

#[test]
fn test_simd_list_large() {
    // Test with large lists to exercise SIMD paths
    let large_list = SimdList::from_ints((0..10000).collect());
    
    let sum = large_list.sum_int();
    let expected: i64 = (0..10000).sum();
    assert_eq!(sum, expected);
}

#[test]
fn test_simd_list_map() {
    let list = SimdList::from_ints(vec![1, 2, 3, 4, 5]);
    
    let doubled = list.map_mul2_int().unwrap();
    // Check the storage contains the doubled values
    if let SimdStorage::Ints(values) = doubled.storage() {
        assert_eq!(values, &[2, 4, 6, 8, 10]);
    } else {
        panic!("Expected Ints storage");
    }
}
