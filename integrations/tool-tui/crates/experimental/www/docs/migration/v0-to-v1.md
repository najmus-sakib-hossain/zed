
# Migration Guide: v0.x to v1.0

This guide helps you migrate from dx-www v0.x to v1.0.

## Overview

dx-www v1.0 introduces several breaking changes to improve API consistency, performance, and production readiness. This guide covers all breaking changes and provides migration examples.

## Breaking Changes

### 1. Edition Update

The workspace now uses Rust edition 2021 (previously incorrectly specified as 2024). Action Required: Update your `rust-toolchain.toml` if you have one:
```toml
[toolchain]
channel = "1.85"
```

### 2. Observability Configuration

The observability stack has been restructured for OpenTelemetry compatibility. Before (v0.x):
```rust
// No structured observability println!("Request received");
```
After (v1.0):
```rust
use dx_www_observability::{init_observability, ObservabilityConfig};
let config = ObservabilityConfig { service_name: "my-app".to_string(), otlp_endpoint: Some("http://localhost:4317".to_string()), metrics_port: 9090, sampling_rate: 0.1, };
init_observability(&config)?;
```

### 3. Health Endpoints

Health probe endpoints are now built-in and follow Kubernetes conventions. New Endpoints: -`GET /health/live` - Liveness probe (is the process alive?) -`GET /health/ready` - Readiness probe (is the service ready for traffic?) Integration:
```rust
use dx_www_server::ops::health::{HealthChecker, DatabaseHealthCheck};
let checker = HealthChecker::new()
.add_check(DatabaseHealthCheck::new(pool.clone()));
// Health endpoints are automatically wired in build_router()
```

### 4. Graceful Shutdown

Graceful shutdown is now built-in with configurable timeout. Before (v0.x):
```rust
// Manual signal handling required ```
After (v1.0):
```rust
use dx_www_server::ops::shutdown::GracefulShutdown;
use std::time::Duration;
let shutdown = GracefulShutdown::new(Duration::from_secs(30));
// Automatically handles SIGTERM/SIGINT ```

### 5. Circuit Breaker

External service calls should now use the circuit breaker pattern. Usage:
```rust
use dx_www_server::ops::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
let config = CircuitBreakerConfig { failure_threshold: 5, reset_timeout: Duration::from_secs(30), half_open_max_calls: 3, };
let breaker = CircuitBreaker::new(config);
// Wrap external calls let result = breaker.call(async { external_service.request().await }).await;
```

### 6. Connection Pool Configuration

Database connection pools now have explicit configuration. Before (v0.x):
```rust
let pool = Pool::new(database_url);
```
After (v1.0):
```rust
use dx_www_db::pool::PoolConfig;
let config = PoolConfig { min_connections: 5, max_connections: 100, acquire_timeout: Duration::from_secs(30), idle_timeout: Duration::from_secs(600), max_lifetime: Duration::from_secs(1800), };
let pool = Pool::with_config(database_url, config);
```

## Deprecated APIs

+-------------------+---------------------+---------+---------------+
| Deprecated        | Replacement         | Removal | Version       |
+===================+=====================+=========+===============+
| `Server::run\(\)` | `Server::serve\(\)` | v2.0    | `Config::from |
+-------------------+---------------------+---------+---------------+



## New Features

### Metrics Endpoint

Prometheus metrics are now available at `/metrics`:
```


# HELP dx_request_count Total number of requests



# TYPE dx_request_count counter


dx_request_count{method="GET",path="/api/users"} 1234


# HELP dx_request_duration_seconds Request duration histogram



# TYPE dx_request_duration_seconds histogram


dx_request_duration_seconds_bucket{le="0.01"} 100 ```


### Structured Logging


All logs now include trace correlation:
```json
{ "timestamp": "2026-01-16T10:30:00Z", "level": "INFO", "message": "Request completed", "trace_id": "abc123", "span_id": "def456", "duration_ms": 42 }
```


## Checklist


- Update Rust toolchain to 1.85+
- Add observability configuration
- Configure health check dependencies
- Set graceful shutdown timeout
- Wrap external calls with circuit breaker
- Update connection pool configuration
- Replace deprecated API calls
- Test with new metrics endpoint
- Verify structured logging output


## Getting Help


- Documentation (../getting-started.md)
- API Reference (../api/README.md)
- GitHub Issues
