//! DxArena Batch Serialization - Raw Memory Performance
//!
//! **WARNING**: This module provides raw memory serialization for maximum speed
//! but ONLY works with fixed-size primitive types (no strings, Vec, or complex types).
//!
//! **Use Cases**:
//! - High-frequency trading data (timestamps, prices, volumes)
//! - Sensor data streams (temperature, pressure, coordinates)
//! - Game state snapshots (positions, velocities, health)
//!
//! **Limitations**:
//! - No strings, Vec, or dynamic types
//! - No nested structures with pointers
//! - Fixed-size types only (u8, u16, u32, u64, i8, i16, i32, i64, f32, f64)
//!
//! **For general serialization, use the main DX-Machine API with RKYV instead.**

use super::arena::DxArena;
use super::quantum::{QuantumReader, QuantumWriter};

/// Trait for types that can be serialized to DxArena
pub trait DxSerialize {
    /// Size of the serialized representation in bytes
    fn serialized_size(&self) -> usize;

    /// Serialize into the writer
    fn serialize_into(&self, writer: &mut QuantumWriter<'_>);
}

/// Trait for types that can be deserialized from DxArena
pub trait DxDeserialize: Sized {
    /// Size of the serialized representation in bytes
    const SIZE: usize;

    /// Deserialize from the reader
    fn deserialize_from(reader: &QuantumReader<'_>) -> Self;
}

/// Batch serializer using DxArena
pub struct DxArenaBatch {
    arena: DxArena,
    item_count: usize,
    #[allow(dead_code)]
    item_size: usize,
}

impl DxArenaBatch {
    /// Create a new batch serializer
    pub fn new(item_size: usize, expected_count: usize) -> Self {
        let capacity = 8 + (item_size * expected_count); // 8 bytes header
        Self {
            arena: DxArena::new(capacity),
            item_count: 0,
            item_size,
        }
    }

    /// Serialize a batch of items
    pub fn serialize<T: DxSerialize>(items: &[T]) -> Vec<u8> {
        if items.is_empty() {
            return Vec::new();
        }

        let item_size = items[0].serialized_size();
        let mut batch = Self::new(item_size, items.len());

        // Write header: item_count (u32) + item_size (u32)
        {
            let mut writer = batch.arena.writer();
            writer.write_u32::<0>(items.len() as u32);
            writer.write_u32::<4>(item_size as u32);
        }
        batch.arena.advance(8);

        // Write each item
        for item in items {
            let mut writer = batch.arena.writer();
            item.serialize_into(&mut writer);
            batch.arena.advance(item_size);
            batch.item_count += 1;
        }

        batch.arena.to_vec()
    }

    /// Deserialize a batch of items
    pub fn deserialize<T: DxDeserialize>(bytes: &[u8]) -> Vec<T> {
        if bytes.len() < 8 {
            return Vec::new();
        }

        // Read header
        let reader = QuantumReader::new(bytes);
        let item_count = reader.read_u32::<0>() as usize;
        let item_size = reader.read_u32::<4>() as usize;

        if item_size != T::SIZE {
            panic!("Size mismatch: expected {}, got {}", T::SIZE, item_size);
        }

        let mut results = Vec::with_capacity(item_count);
        let mut offset = 8;

        for _ in 0..item_count {
            let reader = QuantumReader::new(&bytes[offset..]);
            results.push(T::deserialize_from(&reader));
            offset += item_size;
        }

        results
    }
}

// Implement DxSerialize/DxDeserialize for common types

impl DxSerialize for u64 {
    fn serialized_size(&self) -> usize {
        8
    }
    fn serialize_into(&self, writer: &mut QuantumWriter<'_>) {
        writer.write_u64::<0>(*self);
    }
}

impl DxDeserialize for u64 {
    const SIZE: usize = 8;
    fn deserialize_from(reader: &QuantumReader<'_>) -> Self {
        reader.read_u64::<0>()
    }
}

impl DxSerialize for u32 {
    fn serialized_size(&self) -> usize {
        4
    }
    fn serialize_into(&self, writer: &mut QuantumWriter<'_>) {
        writer.write_u32::<0>(*self);
    }
}

impl DxDeserialize for u32 {
    const SIZE: usize = 4;
    fn deserialize_from(reader: &QuantumReader<'_>) -> Self {
        reader.read_u32::<0>()
    }
}

// Macro to implement for fixed-size structs
#[macro_export]
macro_rules! impl_dx_serialize {
    ($type:ty, $size:expr, |$self:ident, $writer:ident| $serialize:block, |$reader:ident| $deserialize:block) => {
        impl DxSerialize for $type {
            fn serialized_size(&$self) -> usize { $size }
            fn serialize_into(&$self, $writer: &mut QuantumWriter<'_>) $serialize
        }

        impl DxDeserialize for $type {
            const SIZE: usize = $size;
            fn deserialize_from($reader: &QuantumReader<'_>) -> Self $deserialize
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct TestPerson {
        id: u64,
        age: u32,
        salary: u32,
    }

    impl DxSerialize for TestPerson {
        fn serialized_size(&self) -> usize {
            16
        }
        fn serialize_into(&self, writer: &mut QuantumWriter<'_>) {
            writer.write_u64::<0>(self.id);
            writer.write_u32::<8>(self.age);
            writer.write_u32::<12>(self.salary);
        }
    }

    impl DxDeserialize for TestPerson {
        const SIZE: usize = 16;
        fn deserialize_from(reader: &QuantumReader<'_>) -> Self {
            Self {
                id: reader.read_u64::<0>(),
                age: reader.read_u32::<8>(),
                salary: reader.read_u32::<12>(),
            }
        }
    }

    #[test]
    fn test_roundtrip_single() {
        let person = TestPerson {
            id: 1,
            age: 30,
            salary: 75000,
        };
        let bytes = DxArenaBatch::serialize(&[person.clone()]);
        let deserialized = DxArenaBatch::deserialize::<TestPerson>(&bytes);

        assert_eq!(deserialized.len(), 1);
        assert_eq!(deserialized[0], person);
    }

    #[test]
    fn test_roundtrip_batch() {
        let people = vec![
            TestPerson {
                id: 1,
                age: 30,
                salary: 75000,
            },
            TestPerson {
                id: 2,
                age: 25,
                salary: 65000,
            },
            TestPerson {
                id: 3,
                age: 35,
                salary: 85000,
            },
        ];

        let bytes = DxArenaBatch::serialize(&people);
        let deserialized = DxArenaBatch::deserialize::<TestPerson>(&bytes);

        assert_eq!(deserialized.len(), people.len());
        for (i, person) in people.iter().enumerate() {
            assert_eq!(&deserialized[i], person);
        }
    }

    #[test]
    fn test_roundtrip_large_batch() {
        let people: Vec<TestPerson> = (0..1000)
            .map(|i| TestPerson {
                id: i,
                age: (25 + (i % 40)) as u32,
                salary: (50000 + (i * 1000)) as u32,
            })
            .collect();

        let bytes = DxArenaBatch::serialize(&people);
        let deserialized = DxArenaBatch::deserialize::<TestPerson>(&bytes);

        assert_eq!(deserialized.len(), 1000);
        for (i, person) in people.iter().enumerate() {
            assert_eq!(&deserialized[i], person);
        }
    }
}
