
# dx-db-teleport: Reactive Database Caching

(LICENSE) "Sub-millisecond database responses through reactive caching" dx-db-teleport provides reactive database caching with automatic invalidation via Postgres NOTIFY, enabling sub-0.1ms cache access for dx-www applications.

## Performance Targets

+--------+--------+-------------+
| Metric | Target | Description |
+========+========+=============+
| Cache  | Access | <           |
+--------+--------+-------------+



## Key Features

### 1. Reactive Cache Invalidation

Automatic cache invalidation via Postgres NOTIFY:
```rust
use dx_db_teleport::{DbTeleport, CacheConfig};
let config = CacheConfig { max_entries: 10000, ttl_seconds: 300, enable_notify: true, };
let teleport = DbTeleport::new(config);
// Cache is automatically invalidated when Postgres sends NOTIFY // for tables that the cached query depends on ```


### 2. Zero-Copy Binary Results


Query results are stored as pre-serialized binary for instant retrieval:
```rust
use dx_db_teleport::{Query, QueryResult};
// Execute and cache let result = teleport.execute_and_cache( "user_by_id", "SELECT * FROM users WHERE id = $1", &[&user_id], ).await?;
// Subsequent calls return cached binary (< 0.1ms)
let cached = teleport.get_cached("user_by_id", params_hash)?;
```


### 3. Query Dependency Tracking


Automatic tracking of which tables a query depends on:
```rust
// Query depends on "users" table let query = Query::new("SELECT * FROM users WHERE id = $1")
.depends_on(&["users"]);
// When "users" table changes, cache is invalidated teleport.register_query("user_by_id", query);
```


## Architecture


@tree[]


## Modules


+---------+-------------+
| Module  | Description |
+=========+=============+
| `cache` | LRU         |
+---------+-------------+


## Testing


The crate includes comprehensive property-based tests:
```bash

# Run all tests

cargo test --package dx-db-teleport

# Run property tests

cargo test --package dx-db-teleport --test property_tests ```
Test Coverage: -10 property-based tests (proptest) -Cache consistency validation -Invalidation correctness -Access latency verification

## Correctness Properties

- Cache Consistency
- Cached data matches original query result
- Invalidation Correctness
- NOTIFY triggers remove affected entries
- Access Latency
- get_cached() returns within 0.1ms target

## Integration with dx-reactor

dx-db-teleport integrates seamlessly with dx-reactor for high-performance web applications:
```rust
use dx_reactor::DxReactor;
use dx_db_teleport::DbTeleport;
let reactor = DxReactor::build()
.workers(WorkerStrategy::ThreadPerCore)
.build();
let teleport = DbTeleport::new(CacheConfig::default());
// Use teleport in request handlers for sub-millisecond DB access ```


## License


MIT OR Apache-2.0
