//! DX-Machine Batch Serialization Example
//!
//! Demonstrates how DX-Machine uses RKYV with batch optimization.
//!
//! Run with: cargo run --example dx_machine_batch -p dx-serializer

use rkyv::{Archive, Deserialize, Serialize};
use serializer::machine::{deserialize, deserialize_batch, serialize, serialize_batch};

#[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
#[rkyv(compare(PartialEq), derive(Debug))]
struct Person {
    id: u64,
    name: String,
    age: u32,
    salary: u32,
}

fn main() {
    println!("=== DX-Machine Batch Serialization Example ===\n");

    // Create test data
    let people = vec![
        Person {
            id: 1,
            name: "Alice".to_string(),
            age: 30,
            salary: 75000,
        },
        Person {
            id: 2,
            name: "Bob".to_string(),
            age: 25,
            salary: 65000,
        },
        Person {
            id: 3,
            name: "Charlie".to_string(),
            age: 35,
            salary: 85000,
        },
    ];

    println!("Original data:");
    for person in &people {
        println!("  {:?}", person);
    }

    // Single item serialization
    println!("\n--- Single Item Serialization ---");
    let single_bytes = serialize(&people[0]).unwrap();
    println!("Serialized size: {} bytes", single_bytes.len());

    // SAFETY: We just serialized this data, so it's valid
    let archived_single = unsafe { deserialize::<Person>(&single_bytes) };
    println!(
        "Deserialized: id={}, name={}, age={}, salary={}",
        archived_single.id, archived_single.name, archived_single.age, archived_single.salary
    );

    // Batch serialization (DX-Machine optimization)
    println!("\n--- Batch Serialization (DX-Machine) ---");
    let batch_bytes = serialize_batch(&people).unwrap();
    println!("Serialized {} items", batch_bytes.len());
    println!("Total size: {} bytes", batch_bytes.iter().map(|b| b.len()).sum::<usize>());

    // SAFETY: We just serialized this data, so it's valid
    let archived_batch = unsafe { deserialize_batch::<Person>(&batch_bytes) };
    println!("\nDeserialized batch:");
    for (i, archived) in archived_batch.iter().enumerate() {
        println!(
            "  [{}] id={}, name={}, age={}, salary={}",
            i, archived.id, archived.name, archived.age, archived.salary
        );
    }

    // Verify correctness
    println!("\n--- Verification ---");
    for (i, (original, archived)) in people.iter().zip(archived_batch.iter()).enumerate() {
        assert_eq!(original.id, archived.id);
        assert_eq!(original.name, archived.name.as_str());
        assert_eq!(original.age, archived.age);
        assert_eq!(original.salary, archived.salary);
        println!("  Item {} verified ✓", i);
    }

    println!("\n=== Key Advantages ===");
    println!("✓ Uses RKYV's zero-copy wire format");
    println!("✓ Pre-allocates Vec capacity (3-6× faster batch operations)");
    println!("✓ Compatible with standard RKYV deserialization");
    println!("✓ No custom format - pure RKYV with smart allocation");
}
