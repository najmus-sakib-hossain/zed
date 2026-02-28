//! Brutal Reality Check: Does DX-Machine Actually Work?
//!
//! This test verifies that DX-Machine can serialize and deserialize
//! complex real-world data structures correctly.

use rkyv::{Archive, Deserialize, Serialize};
use serializer::machine::{deserialize, deserialize_batch, serialize, serialize_batch};

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[rkyv(compare(PartialEq), derive(Debug))]
struct User {
    id: u64,
    username: String,
    email: String,
    age: u32,
    is_active: bool,
    balance: f64,
    tags: Vec<String>,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[rkyv(compare(PartialEq), derive(Debug))]
struct Product {
    id: u64,
    name: String,
    description: String,
    price: f64,
    stock: u32,
    categories: Vec<String>,
    metadata: Vec<(String, String)>,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[rkyv(compare(PartialEq), derive(Debug))]
struct Order {
    id: u64,
    user_id: u64,
    items: Vec<OrderItem>,
    total: f64,
    status: String,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[rkyv(compare(PartialEq), derive(Debug))]
struct OrderItem {
    product_id: u64,
    quantity: u32,
    price: f64,
}

#[test]
fn test_single_user_roundtrip() {
    let user = User {
        id: 1,
        username: "alice".to_string(),
        email: "alice@example.com".to_string(),
        age: 30,
        is_active: true,
        balance: 1234.56,
        tags: vec!["premium".to_string(), "verified".to_string()],
    };

    // Serialize
    let bytes = serialize(&user).expect("Failed to serialize");
    assert!(!bytes.is_empty(), "Serialized bytes should not be empty");

    // Deserialize
    let archived = unsafe { deserialize::<User>(&bytes) };

    // Verify
    assert_eq!(archived.id, user.id);
    assert_eq!(archived.username.as_str(), user.username);
    assert_eq!(archived.email.as_str(), user.email);
    assert_eq!(archived.age, user.age);
    assert_eq!(archived.is_active, user.is_active);
    assert_eq!(archived.balance, user.balance);
    assert_eq!(archived.tags.len(), user.tags.len());
    for (i, tag) in user.tags.iter().enumerate() {
        assert_eq!(archived.tags[i].as_str(), tag);
    }
}

#[test]
fn test_batch_users_roundtrip() {
    let users = vec![
        User {
            id: 1,
            username: "alice".to_string(),
            email: "alice@example.com".to_string(),
            age: 30,
            is_active: true,
            balance: 1234.56,
            tags: vec!["premium".to_string()],
        },
        User {
            id: 2,
            username: "bob".to_string(),
            email: "bob@example.com".to_string(),
            age: 25,
            is_active: false,
            balance: 567.89,
            tags: vec!["basic".to_string(), "new".to_string()],
        },
        User {
            id: 3,
            username: "charlie".to_string(),
            email: "charlie@example.com".to_string(),
            age: 35,
            is_active: true,
            balance: 9999.99,
            tags: vec![],
        },
    ];

    // Batch serialize
    let batches = serialize_batch(&users).expect("Failed to serialize batch");
    assert_eq!(batches.len(), users.len());

    // Batch deserialize
    let archived = unsafe { deserialize_batch::<User>(&batches) };
    assert_eq!(archived.len(), users.len());

    // Verify each user
    for (i, (original, arch)) in users.iter().zip(archived.iter()).enumerate() {
        assert_eq!(arch.id, original.id, "User {} id mismatch", i);
        assert_eq!(arch.username.as_str(), original.username, "User {} username mismatch", i);
        assert_eq!(arch.email.as_str(), original.email, "User {} email mismatch", i);
        assert_eq!(arch.age, original.age, "User {} age mismatch", i);
        assert_eq!(arch.is_active, original.is_active, "User {} is_active mismatch", i);
        assert_eq!(arch.balance, original.balance, "User {} balance mismatch", i);
        assert_eq!(arch.tags.len(), original.tags.len(), "User {} tags length mismatch", i);
    }
}

#[test]
fn test_complex_product_roundtrip() {
    let product = Product {
        id: 100,
        name: "Laptop".to_string(),
        description: "High-performance laptop with 32GB RAM".to_string(),
        price: 1299.99,
        stock: 50,
        categories: vec!["Electronics".to_string(), "Computers".to_string()],
        metadata: vec![
            ("brand".to_string(), "TechCorp".to_string()),
            ("warranty".to_string(), "2 years".to_string()),
        ],
    };

    let bytes = serialize(&product).expect("Failed to serialize product");
    let archived = unsafe { deserialize::<Product>(&bytes) };

    assert_eq!(archived.id, product.id);
    assert_eq!(archived.name.as_str(), product.name);
    assert_eq!(archived.description.as_str(), product.description);
    assert_eq!(archived.price, product.price);
    assert_eq!(archived.stock, product.stock);
    assert_eq!(archived.categories.len(), product.categories.len());
    assert_eq!(archived.metadata.len(), product.metadata.len());
}

#[test]
fn test_nested_order_roundtrip() {
    let order = Order {
        id: 1000,
        user_id: 1,
        items: vec![
            OrderItem {
                product_id: 100,
                quantity: 2,
                price: 1299.99,
            },
            OrderItem {
                product_id: 101,
                quantity: 1,
                price: 49.99,
            },
        ],
        total: 2649.97,
        status: "pending".to_string(),
    };

    let bytes = serialize(&order).expect("Failed to serialize order");
    let archived = unsafe { deserialize::<Order>(&bytes) };

    assert_eq!(archived.id, order.id);
    assert_eq!(archived.user_id, order.user_id);
    assert_eq!(archived.items.len(), order.items.len());
    assert_eq!(archived.total, order.total);
    assert_eq!(archived.status.as_str(), order.status);

    for (i, (original_item, arch_item)) in order.items.iter().zip(archived.items.iter()).enumerate()
    {
        assert_eq!(
            arch_item.product_id, original_item.product_id,
            "Item {} product_id mismatch",
            i
        );
        assert_eq!(arch_item.quantity, original_item.quantity, "Item {} quantity mismatch", i);
        assert_eq!(arch_item.price, original_item.price, "Item {} price mismatch", i);
    }
}

#[test]
fn test_large_batch_1000_items() {
    let users: Vec<User> = (0..1000)
        .map(|i| User {
            id: i,
            username: format!("user{}", i),
            email: format!("user{}@example.com", i),
            age: (20 + (i % 50)) as u32,
            is_active: i % 2 == 0,
            balance: (i as f64) * 10.5,
            tags: vec![format!("tag{}", i % 10)],
        })
        .collect();

    // Serialize batch
    let batches = serialize_batch(&users).expect("Failed to serialize 1000 users");
    assert_eq!(batches.len(), 1000);

    // Deserialize batch
    let archived = unsafe { deserialize_batch::<User>(&batches) };
    assert_eq!(archived.len(), 1000);

    // Spot check some items
    for i in [0, 100, 500, 999] {
        assert_eq!(archived[i].id, users[i].id);
        assert_eq!(archived[i].username.as_str(), users[i].username);
        assert_eq!(archived[i].age, users[i].age);
    }
}

#[test]
fn test_empty_collections() {
    let user = User {
        id: 1,
        username: "test".to_string(),
        email: "test@example.com".to_string(),
        age: 25,
        is_active: true,
        balance: 0.0,
        tags: vec![], // Empty vector
    };

    let bytes = serialize(&user).expect("Failed to serialize user with empty tags");
    let archived = unsafe { deserialize::<User>(&bytes) };

    assert_eq!(archived.tags.len(), 0);
}

#[test]
fn test_unicode_strings() {
    let user = User {
        id: 1,
        username: "用户名".to_string(),        // Chinese
        email: "тест@example.com".to_string(), // Cyrillic
        age: 30,
        is_active: true,
        balance: 100.0,
        tags: vec!["🚀".to_string(), "émoji".to_string()], // Emoji and accents
    };

    let bytes = serialize(&user).expect("Failed to serialize unicode user");
    let archived = unsafe { deserialize::<User>(&bytes) };

    assert_eq!(archived.username.as_str(), "用户名");
    assert_eq!(archived.email.as_str(), "тест@example.com");
    assert_eq!(archived.tags[0].as_str(), "🚀");
    assert_eq!(archived.tags[1].as_str(), "émoji");
}

#[test]
fn test_extreme_values() {
    let user = User {
        id: u64::MAX,
        username: "a".repeat(1000), // Long string
        email: "test@example.com".to_string(),
        age: u32::MAX,
        is_active: true,
        balance: f64::MAX,
        tags: (0..100).map(|i| format!("tag{}", i)).collect(), // Many tags
    };

    let bytes = serialize(&user).expect("Failed to serialize extreme user");
    let archived = unsafe { deserialize::<User>(&bytes) };

    assert_eq!(archived.id, u64::MAX);
    assert_eq!(archived.username.len(), 1000);
    assert_eq!(archived.age, u32::MAX);
    assert_eq!(archived.balance, f64::MAX);
    assert_eq!(archived.tags.len(), 100);
}
