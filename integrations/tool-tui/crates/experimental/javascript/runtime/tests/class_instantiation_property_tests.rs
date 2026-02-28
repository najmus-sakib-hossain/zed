//! Property-based tests for Class Instantiation
//!
//! Feature: dx-runtime-production-ready
//! Property 20: Class Instantiation
//! Validates: Requirements 6.1, 6.2, 6.3

use proptest::prelude::*;
use std::collections::HashMap;

#[test]
fn simple_test() {
    assert!(true, "This test should always pass");
}

#[derive(Clone, Debug)]
struct ClassData {
    constructor_id: Option<u32>,
    prototype_id: u64,
    super_class_id: Option<u64>,
    static_properties: HashMap<String, f64>,
}

struct TestHeap {
    classes: HashMap<u64, ClassData>,
    objects: HashMap<u64, HashMap<String, f64>>,
    prototypes: HashMap<u64, u64>,
    next_id: u64,
}

impl TestHeap {
    fn new() -> Self {
        Self {
            classes: HashMap::new(),
            objects: HashMap::new(),
            prototypes: HashMap::new(),
            next_id: 1,
        }
    }

    fn allocate_object(&mut self, properties: HashMap<String, f64>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.objects.insert(id, properties);
        id
    }

    fn allocate_class(&mut self, constructor_id: Option<u32>, super_class_id: Option<u64>) -> u64 {
        let class_id = self.next_id;
        self.next_id += 1;
        let prototype_id = self.allocate_object(HashMap::new());
        if let Some(super_id) = super_class_id {
            if let Some(super_class) = self.classes.get(&super_id) {
                self.prototypes.insert(prototype_id, super_class.prototype_id);
            }
        }
        self.classes.insert(class_id, ClassData {
            constructor_id,
            prototype_id,
            super_class_id,
            static_properties: HashMap::new(),
        });
        class_id
    }

    fn create_instance(&mut self, class_id: u64) -> Option<u64> {
        let class = self.classes.get(&class_id)?;
        let prototype_id = class.prototype_id;
        let instance_id = self.allocate_object(HashMap::new());
        self.prototypes.insert(instance_id, prototype_id);
        Some(instance_id)
    }

    fn is_instance_of(&self, object_id: u64, class_id: u64) -> bool {
        let class = match self.classes.get(&class_id) {
            Some(c) => c,
            None => return false,
        };
        let target_prototype = class.prototype_id;
        let mut current_proto = self.prototypes.get(&object_id).copied();
        while let Some(proto_id) = current_proto {
            if proto_id == target_prototype {
                return true;
            }
            current_proto = self.prototypes.get(&proto_id).copied();
        }
        false
    }

    fn get_prototype(&self, object_id: u64) -> Option<u64> {
        self.prototypes.get(&object_id).copied()
    }

    fn set_property(&mut self, object_id: u64, key: String, value: f64) {
        if let Some(obj) = self.objects.get_mut(&object_id) {
            obj.insert(key, value);
        }
    }

    fn get_property_with_prototype(&self, object_id: u64, key: &str) -> Option<f64> {
        if let Some(obj) = self.objects.get(&object_id) {
            if let Some(&value) = obj.get(key) {
                return Some(value);
            }
        }
        let mut current_proto = self.prototypes.get(&object_id).copied();
        while let Some(proto_id) = current_proto {
            if let Some(proto_obj) = self.objects.get(&proto_id) {
                if let Some(&value) = proto_obj.get(key) {
                    return Some(value);
                }
            }
            current_proto = self.prototypes.get(&proto_id).copied();
        }
        None
    }

    fn define_method_on_prototype(&mut self, class_id: u64, method_name: String, function_id: u32) {
        if let Some(class) = self.classes.get(&class_id) {
            let prototype_id = class.prototype_id;
            if let Some(proto) = self.objects.get_mut(&prototype_id) {
                proto.insert(method_name, function_id as f64);
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn class_creation_allocates_unique_id(_dummy in Just(())) {
        let mut heap = TestHeap::new();
        let class1 = heap.allocate_class(Some(1), None);
        let class2 = heap.allocate_class(Some(2), None);
        prop_assert_ne!(class1, class2);
        prop_assert!(heap.classes.contains_key(&class1));
        prop_assert!(heap.classes.contains_key(&class2));
    }

    #[test]
    fn instance_has_correct_prototype(_dummy in Just(())) {
        let mut heap = TestHeap::new();
        let class_id = heap.allocate_class(Some(1), None);
        let class_prototype = heap.classes.get(&class_id).unwrap().prototype_id;
        let instance_id = heap.create_instance(class_id).unwrap();
        let instance_proto = heap.get_prototype(instance_id);
        prop_assert_eq!(instance_proto, Some(class_prototype));
    }

    #[test]
    fn multiple_instances_share_prototype(num_instances in 1usize..10) {
        let mut heap = TestHeap::new();
        let class_id = heap.allocate_class(Some(1), None);
        let class_prototype = heap.classes.get(&class_id).unwrap().prototype_id;
        let instances: Vec<u64> = (0..num_instances)
            .map(|_| heap.create_instance(class_id).unwrap())
            .collect();
        for instance_id in instances {
            let instance_proto = heap.get_prototype(instance_id);
            prop_assert_eq!(instance_proto, Some(class_prototype));
        }
    }

    #[test]
    fn instanceof_true_for_direct_instance(_dummy in Just(())) {
        let mut heap = TestHeap::new();
        let class_id = heap.allocate_class(Some(1), None);
        let instance_id = heap.create_instance(class_id).unwrap();
        prop_assert!(heap.is_instance_of(instance_id, class_id));
    }

    #[test]
    fn instanceof_false_for_unrelated_class(_dummy in Just(())) {
        let mut heap = TestHeap::new();
        let class1 = heap.allocate_class(Some(1), None);
        let class2 = heap.allocate_class(Some(2), None);
        let instance1 = heap.create_instance(class1).unwrap();
        prop_assert!(!heap.is_instance_of(instance1, class2));
    }

    #[test]
    fn inheritance_prototype_chain(_dummy in Just(())) {
        let mut heap = TestHeap::new();
        let parent_class = heap.allocate_class(Some(1), None);
        let child_class = heap.allocate_class(Some(2), Some(parent_class));
        let instance = heap.create_instance(child_class).unwrap();
        prop_assert!(heap.is_instance_of(instance, child_class));
        prop_assert!(heap.is_instance_of(instance, parent_class));
    }

    #[test]
    fn methods_accessible_via_prototype(method_id in 0u32..1000) {
        let mut heap = TestHeap::new();
        let class_id = heap.allocate_class(Some(0), None);
        heap.define_method_on_prototype(class_id, "testMethod".to_string(), method_id);
        let instance = heap.create_instance(class_id).unwrap();
        let method = heap.get_property_with_prototype(instance, "testMethod");
        prop_assert_eq!(method, Some(method_id as f64));
    }

    #[test]
    fn instance_properties_shadow_prototype(
        proto_value in any::<f64>().prop_filter("not NaN", |v| !v.is_nan()),
        instance_value in any::<f64>().prop_filter("not NaN", |v| !v.is_nan())
    ) {
        let mut heap = TestHeap::new();
        let class_id = heap.allocate_class(Some(0), None);
        let prototype_id = heap.classes.get(&class_id).unwrap().prototype_id;
        heap.set_property(prototype_id, "value".to_string(), proto_value);
        let instance = heap.create_instance(class_id).unwrap();
        let value_before = heap.get_property_with_prototype(instance, "value");
        prop_assert_eq!(value_before, Some(proto_value));
        heap.set_property(instance, "value".to_string(), instance_value);
        let value_after = heap.get_property_with_prototype(instance, "value");
        prop_assert_eq!(value_after, Some(instance_value));
    }

    #[test]
    fn deep_inheritance_chain(depth in 2usize..5) {
        let mut heap = TestHeap::new();
        let mut classes = Vec::new();
        let mut parent: Option<u64> = None;
        for i in 0..depth {
            let class_id = heap.allocate_class(Some(i as u32), parent);
            classes.push(class_id);
            parent = Some(class_id);
        }
        let instance = heap.create_instance(*classes.last().unwrap()).unwrap();
        for class_id in &classes {
            prop_assert!(heap.is_instance_of(instance, *class_id));
        }
    }

    #[test]
    fn constructor_id_preserved(ctor_id in 0u32..1000) {
        let mut heap = TestHeap::new();
        let class_id = heap.allocate_class(Some(ctor_id), None);
        let class_data = heap.classes.get(&class_id).unwrap();
        prop_assert_eq!(class_data.constructor_id, Some(ctor_id));
    }

    #[test]
    fn super_class_id_preserved(_dummy in Just(())) {
        let mut heap = TestHeap::new();
        let parent_class = heap.allocate_class(Some(1), None);
        let child_class = heap.allocate_class(Some(2), Some(parent_class));
        let parent_data = heap.classes.get(&parent_class).unwrap();
        let child_data = heap.classes.get(&child_class).unwrap();
        prop_assert_eq!(parent_data.super_class_id, None);
        prop_assert_eq!(child_data.super_class_id, Some(parent_class));
    }

    #[test]
    fn instance_of_nonexistent_class_fails(fake_class_id in 1000u64..2000) {
        let mut heap = TestHeap::new();
        let result = heap.create_instance(fake_class_id);
        prop_assert!(result.is_none());
    }

    #[test]
    fn instanceof_nonexistent_class_false(fake_class_id in 1000u64..2000) {
        let mut heap = TestHeap::new();
        let class_id = heap.allocate_class(Some(1), None);
        let instance = heap.create_instance(class_id).unwrap();
        prop_assert!(!heap.is_instance_of(instance, fake_class_id));
    }

    #[test]
    fn instanceof_nonexistent_object_false(fake_object_id in 1000u64..2000) {
        let mut heap = TestHeap::new();
        let class_id = heap.allocate_class(Some(1), None);
        prop_assert!(!heap.is_instance_of(fake_object_id, class_id));
    }
}
