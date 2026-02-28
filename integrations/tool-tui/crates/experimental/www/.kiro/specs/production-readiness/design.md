
# Design Document: Production Readiness

## Overview

This design document outlines the technical approach for making dx-www production-ready. The design focuses on six key areas: authentication hardening, error handling improvements, WASM client testing, security middleware, ecosystem module completion, and comprehensive testing infrastructure. The implementation follows a layered approach where foundational changes (error handling) are made first, followed by security features, then ecosystem completion, and finally testing and documentation.

## Architecture

The production-ready architecture extends the existing dx-www structure with new middleware layers and improved error handling throughout: @tree[]

## Components and Interfaces

### 1. Authentication System

#### Token Structure

```rust
/// Production-ready authentication token


#[derive(Debug, Clone, Serialize, Deserialize)]


pub struct AuthToken { /// Unique token identifier for revocation pub jti: String, /// User identifier pub sub: String, /// Issued at timestamp (Unix seconds)
pub iat: i64, /// Expiration timestamp (Unix seconds)
pub exp: i64, /// Token type (access, refresh)
pub typ: TokenType, /// Ed25519 signature pub sig: [u8; 64], }


#[derive(Debug, Clone, Copy, Serialize, Deserialize)]


pub enum TokenType { Access, Refresh, }
/// Token generator with Ed25519 signing pub struct TokenGenerator { signing_key: ed25519_dalek::SigningKey, access_ttl: Duration, refresh_ttl: Duration, }
impl TokenGenerator { /// Generate a new access token for a user pub fn generate_access(&self, user_id: &str) -> Result<AuthToken, AuthError>;
/// Generate a refresh token for token renewal pub fn generate_refresh(&self, user_id: &str) -> Result<AuthToken, AuthError>;
/// Verify token signature and expiration pub fn verify(&self, token: &AuthToken) -> Result<(), AuthError>;
}
```

#### Credential Verification

```rust
/// Password hasher using Argon2id pub struct PasswordHasher { config: argon2::Config<'static>, }
impl PasswordHasher { /// Hash a password for storage pub fn hash(&self, password: &str) -> Result<String, AuthError>;
/// Verify a password against stored hash pub fn verify(&self, password: &str, hash: &str) -> Result<bool, AuthError>;
}
/// User credential store interface


#[async_trait]


pub trait CredentialStore: Send + Sync { /// Get password hash for user async fn get_password_hash(&self, email: &str) -> Result<Option<String>, AuthError>;
/// Check if token is revoked async fn is_token_revoked(&self, jti: &str) -> Result<bool, AuthError>;
/// Revoke a token async fn revoke_token(&self, jti: &str) -> Result<(), AuthError>;
}
```

### 2. Error Handling System

#### Safe Mutex Wrapper

```rust
/// A mutex wrapper that handles poisoning gracefully pub struct SafeMutex<T> { inner: std::sync::Mutex<T>, }
impl<T> SafeMutex<T> { pub fn new(value: T) -> Self { Self { inner: std::sync::Mutex::new(value) }
}
/// Lock the mutex, recovering from poisoning if necessary pub fn lock(&self) -> Result<SafeMutexGuard<'_, T>, LockError> { match self.inner.lock() { Ok(guard) => Ok(SafeMutexGuard { guard }), Err(poisoned) => { // Log the poisoning event tracing::warn!("Mutex was poisoned, recovering");
// Recover by taking the inner value Ok(SafeMutexGuard { guard: poisoned.into_inner() })
}
}
}
}
/// Error type for lock operations


#[derive(Debug, thiserror::Error)]


pub enum LockError {


#[error("Lock acquisition failed: {0}")]


AcquisitionFailed(String), }
```

#### Structured Error Types

```rust
/// Production error type with full context


#[derive(Debug, thiserror::Error)]


pub enum DxError {


#[error("Authentication failed: {message}")]


Auth { message: String, code: AuthErrorCode,


#[source]


source: Option<Box<dyn std::error::Error + Send + Sync>>, },


#[error("Database error: {message}")]


Database { message: String, query_context: Option<String>,


#[source]


source: Option<Box<dyn std::error::Error + Send + Sync>>, },


#[error("Configuration error: {message}")]


Config { message: String, key: String, },


#[error("Internal error: {message}")]


Internal { message: String,


#[source]


source: Option<Box<dyn std::error::Error + Send + Sync>>, }, }


#[derive(Debug, Clone, Copy)]


pub enum AuthErrorCode { InvalidCredentials = 1001, TokenExpired = 1002, TokenInvalid = 1003, TokenRevoked = 1004, RateLimited = 1005, CsrfInvalid = 1006, }
```

### 3. Security Middleware

#### Security Headers Configuration

```rust
/// Security headers configuration


#[derive(Debug, Clone)]


pub struct SecurityConfig { /// Content Security Policy directives pub csp: ContentSecurityPolicy, /// HSTS max-age in seconds pub hsts_max_age: u64, /// Include subdomains in HSTS pub hsts_include_subdomains: bool, /// X-Frame-Options value pub frame_options: FrameOptions, /// Referrer policy pub referrer_policy: ReferrerPolicy, /// Whether to use relaxed settings for development pub development_mode: bool, }


#[derive(Debug, Clone)]


pub struct ContentSecurityPolicy { pub default_src: Vec<String>, pub script_src: Vec<String>, pub style_src: Vec<String>, pub img_src: Vec<String>, pub connect_src: Vec<String>, pub frame_ancestors: Vec<String>, }
impl SecurityConfig { /// Create production-safe defaults pub fn production() -> Self;
/// Create development-friendly defaults pub fn development() -> Self;
/// Build the CSP header value pub fn build_csp_header(&self) -> String;
}
```

#### Rate Limiter

```rust
/// Sliding window rate limiter pub struct RateLimiter { /// Storage backend for rate limit counters store: Arc<dyn RateLimitStore>, /// Default limits by endpoint category limits: HashMap<EndpointCategory, RateLimit>, }


#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]


pub enum EndpointCategory { Auth, Api, Static, }


#[derive(Debug, Clone)]


pub struct RateLimit { /// Maximum requests allowed pub max_requests: u32, /// Time window in seconds pub window_seconds: u64, }


#[async_trait]


pub trait RateLimitStore: Send + Sync { /// Increment counter and return current count async fn increment(&self, key: &str, window: u64) -> Result<u32, RateLimitError>;
/// Get time until window resets async fn ttl(&self, key: &str) -> Result<u64, RateLimitError>;
}
impl RateLimiter { /// Check if request should be rate limited pub async fn check(&self, ip: &str, category: EndpointCategory) -> Result<RateLimitResult, RateLimitError>;
}
pub enum RateLimitResult { Allowed { remaining: u32 }, Limited { retry_after: u64 }, }
```

#### CSRF Protection

```rust
/// CSRF token manager pub struct CsrfManager { /// Secret key for token generation secret: [u8; 32], /// Token TTL ttl: Duration, }
impl CsrfManager { /// Generate a new CSRF token bound to a session pub fn generate(&self, session_id: &str) -> String;
/// Validate a CSRF token pub fn validate(&self, token: &str, session_id: &str) -> Result<(), CsrfError>;
}


#[derive(Debug, thiserror::Error)]


pub enum CsrfError {


#[error("CSRF token missing")]


Missing,


#[error("CSRF token invalid")]


Invalid,


#[error("CSRF token expired")]


Expired, }
```

### 4. WASM Client Test Infrastructure

#### Test Harness

```rust
/// Mock host functions for WASM testing pub struct MockHost { /// Cloned templates pub cloned_templates: Vec<u32>, /// Cached templates pub cached_templates: HashMap<u32, Vec<u8>>, /// DOM operations log pub dom_ops: Vec<DomOp>, /// Event listeners pub listeners: Vec<(u32, u32, u32)>, /// Cached base data for delta patching pub cache: HashMap<u32, Vec<u8>>, }


#[derive(Debug, Clone)]


pub enum DomOp { Append { parent: u32, child: u32 }, Remove { node: u32 }, SetText { node: u32, text: String }, SetAttr { node: u32, key: String, value: String }, ToggleClass { node: u32, class: String, enable: bool }, }
impl MockHost { /// Create HTIP stream for testing pub fn create_htip_stream(&self, ops: &[HtipOp]) -> Vec<u8>;
/// Verify expected DOM operations occurred pub fn verify_ops(&self, expected: &[DomOp]) -> bool;
}
```

### 5. Query Module Integration

```rust
/// Production query client with database integration pub struct ProductionQueryClient { /// Connection pool pool: sqlx::PgPool, /// Query cache cache: QueryCache<Vec<u8>>, /// Query timeout timeout: Duration, }
impl ProductionQueryClient { /// Execute a parameterized query pub async fn query<T: DeserializeOwned>( &self, sql: &str, params: &[&(dyn sqlx::Encode<'_, sqlx::Postgres> + Sync)], ) -> Result<Vec<T>, QueryError>;
/// Execute with caching pub async fn query_cached<T: DeserializeOwned + Serialize>( &self, key: QueryKey, sql: &str, params: &[&(dyn sqlx::Encode<'_, sqlx::Postgres> + Sync)], ttl: Duration, ) -> Result<Vec<T>, QueryError>;
}
```

### 6. Sync Module Integration

```rust
/// Production WebSocket manager pub struct WebSocketManager { /// Active connections connections: DashMap<ConnectionId, WebSocketConnection>, /// Channel subscriptions channels: ChannelManager, /// Message buffer for reconnection buffer: MessageBuffer, /// Presence tracker presence: PresenceTracker, }
impl WebSocketManager { /// Handle new WebSocket connection pub async fn handle_connection(&self, ws: WebSocket, user_id: String) -> Result<(), SyncError>;
/// Send message to channel pub async fn publish(&self, channel: ChannelId, message: BinaryMessage) -> Result<(), SyncError>;
/// Get presence for channel pub fn get_presence(&self, channel: ChannelId) -> Vec<String>;
}
/// Message buffer for offline support pub struct MessageBuffer { /// Buffered messages per connection buffers: DashMap<ConnectionId, VecDeque<BinaryMessage>>, /// Max buffer size max_size: usize, }
```

## Data Models

### Authentication Data

```rust
/// User record for authentication pub struct User { pub id: String, pub email: String, pub password_hash: String, pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc>, }
/// Token revocation record pub struct RevokedToken { pub jti: String, pub revoked_at: DateTime<Utc>, pub expires_at: DateTime<Utc>, }
/// Rate limit record pub struct RateLimitRecord { pub key: String, pub count: u32, pub window_start: DateTime<Utc>, }
```

### Error Response Format

```rust
/// Standardized error response


#[derive(Debug, Serialize)]


pub struct ErrorResponse { pub error: ErrorDetail, pub request_id: String, pub timestamp: DateTime<Utc>, }


#[derive(Debug, Serialize)]


pub struct ErrorDetail { pub code: String, pub message: String,


#[serde(skip_serializing_if = "Option::is_none")]


pub details: Option<serde_json::Value>,


#[serde(skip_serializing_if = "Option::is_none")]


pub suggestion: Option<String>, }
```

## Correctness Properties

A property is a characteristic or behavior that should hold true across all valid executions of a system—, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees. Based on the prework analysis, the following properties have been identified after eliminating redundancy:

### Authentication Properties

Property 1: Password Hash Round-Trip For any valid password string, hashing with Argon2id and then verifying the same password against the hash SHALL return true. Validates: Requirements 1.1 Property 2: Token Signature Integrity For any generated Ed25519 token, the signature SHALL verify successfully with the corresponding public key, and any modification to the token payload SHALL cause verification to fail. Validates: Requirements 1.2, 1.3, 1.5 Property 3: Token Expiration Enforcement For any token with expiration time T, verification SHALL succeed when current_time < T and SHALL fail when current_time >= T. Validates: Requirements 1.3, 1.4 Property 4: Token Refresh Validity For any valid refresh token within its grace period, refreshing SHALL produce a new valid access token with a fresh expiration time. Validates: Requirements 1.6 Property 5: Token Revocation Effectiveness For any revoked token, subsequent verification attempts SHALL fail regardless of the token's expiration status. Validates: Requirements 1.7

### Error Handling Properties

Property 6: Mutex Poisoning Recovery For any SafeMutex that has been poisoned by a panicking thread, subsequent lock() calls SHALL return Ok with the recovered value instead of panicking. Validates: Requirements 2.1 Property 7: Error Isolation For any component failure within an ErrorBoundary, other components outside that boundary SHALL continue to operate normally. Validates: Requirements 2.4 Property 8: Structured Error Completeness For any DxError instance, it SHALL contain a non-empty error code, a non-empty message, and optionally a source error and recovery suggestion. Validates: Requirements 2.5

### WASM Client Properties

Property 9: Delta Patch Round-Trip For any valid base data and delta patch, applying the patch and then generating a new patch from the result to the original SHALL produce an identity patch (or equivalent data). Validates: Requirements 3.2 Property 10: Allocator Non-Overlap For any sequence of allocations from the bump allocator, no two allocated regions SHALL overlap in memory. Validates: Requirements 3.3 Property 11: Malformed Stream Resilience For any byte sequence (including random/malformed data), processing it as an HTIP stream SHALL either succeed or return an error code without crashing or causing undefined behavior. Validates: Requirements 3.4, 3.5

### Security Headers Properties

Property 12: Security Headers Presence For any HTTP response from the server, it SHALL contain all required security headers: Content-Security-Policy, Strict-Transport-Security, X-Frame-Options, X-Content-Type-Options, X-XSS-Protection, and Referrer-Policy. Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5, 4.6

### Rate Limiting Properties

Property 13: Rate Limit Counting Accuracy For any sequence of N requests from the same IP within a time window, the rate limiter SHALL report a count of exactly N. Validates: Requirements 5.1 Property 14: Rate Limit Enforcement For any IP that has made max_requests requests within the window, the next request SHALL receive HTTP 429 with a valid Retry-After header. Validates: Requirements 5.2, 5.3 Property 15: Sliding Window Correctness For any request made at time T, it SHALL only count against windows that include time T, preventing burst attacks at window boundaries. Validates: Requirements 5.5

### CSRF Properties

Property 16: CSRF Token Uniqueness For any two CSRF tokens generated for different sessions, they SHALL be cryptographically distinct (collision probability < 2^-128). Validates: Requirements 6.1 Property 17: CSRF Validation Strictness For any POST/PUT/DELETE request, it SHALL be rejected with HTTP 403 if the CSRF token is missing, invalid, expired, or bound to a different session. Validates: Requirements 6.3, 6.4, 6.5 Property 18: CSRF Token Location Flexibility For any valid CSRF token, it SHALL be accepted whether provided in a form field or a custom header. Validates: Requirements 6.6

### Query Module Properties

Property 19: SQL Injection Prevention For any user-provided input containing SQL injection patterns (quotes, semicolons, comments), parameterized queries SHALL escape or reject the input without executing malicious SQL. Validates: Requirements 8.2 Property 20: Connection Pool Reuse For any sequence of queries, the number of database connections created SHALL be bounded by the pool size, regardless of query count. Validates: Requirements 8.3 Property 21: Query Error Structure For any failed query, the returned error SHALL contain the error type, a sanitized message (no credentials), and query context (table/operation). Validates: Requirements 8.4

### Sync Module Properties

Property 22: Pub/Sub Delivery For any message published to a channel, all active subscribers to that channel SHALL receive the message exactly once. Validates: Requirements 9.2 Property 23: Message Buffer Replay For any messages sent while a client is disconnected, upon reconnection the client SHALL receive all buffered messages in order. Validates: Requirements 9.3 Property 24: Presence Accuracy For any channel, the presence list SHALL contain exactly the user IDs of currently connected clients subscribed to that channel. Validates: Requirements 9.4 Property 25: Acknowledgment Guarantee For any message with acknowledgment enabled, the sender SHALL receive an ack if and only if the message was successfully delivered to at least one subscriber. Validates: Requirements 9.5

### Server Error Handling Properties

Property 26: Error Response Sanitization For any unhandled exception in production mode, the error response SHALL NOT contain stack traces, internal paths, or sensitive configuration details. Validates: Requirements 7.5

## Error Handling

### Error Categories

- Authentication Errors (1xxx codes)
- 1001: Invalid credentials
- 1002: Token expired
- 1003: Token invalid (signature mismatch)
- 1004: Token revoked
- 1005: Rate limited
- 1006: CSRF validation failed
- Database Errors (2xxx codes)
- 2001: Connection failed
- 2002: Query timeout
- 2003: Constraint violation
- 2004: Transaction failed
- Sync Errors (3xxx codes)
- 3001: Connection lost
- 3002: Channel not found
- 3003: Message delivery failed
- 3004: Buffer overflow
- Internal Errors (5xxx codes)
- 5001: Configuration error
- 5002: Lock acquisition failed
- 5003: Resource exhausted

### Error Recovery Strategies

```rust
/// Error recovery configuration pub struct RecoveryConfig { /// Maximum retry attempts pub max_retries: u32, /// Base delay between retries (exponential backoff)
pub base_delay: Duration, /// Maximum delay cap pub max_delay: Duration, /// Jitter factor (0.0 - 1.0)
pub jitter: f64, }
impl Default for RecoveryConfig { fn default() -> Self { Self { max_retries: 3, base_delay: Duration::from_millis(100), max_delay: Duration::from_secs(10), jitter: 0.1, }
}
}
```

## Testing Strategy

### Dual Testing Approach

The testing strategy employs both unit tests and property-based tests: -Unit tests: Verify specific examples, edge cases, and error conditions -Property tests: Verify universal properties across randomly generated inputs

### Property-Based Testing Configuration

- Framework: `proptest` crate for Rust
- Minimum iterations: 100 per property test
- Tag format: `Feature: production-readiness, Property {number}: {property_text}`

### Test Categories

- Authentication Tests
- Password hashing round-trip (Property 1)
- Token generation and verification (Properties 2, 3)
- Token refresh flow (Property 4)
- Token revocation (Property 5)
- Error Handling Tests
- Mutex poisoning recovery (Property 6)
- Error isolation (Property 7)
- Error structure validation (Property 8)
- WASM Client Tests
- Delta patch round-trip (Property 9)
- Allocator correctness (Property 10)
- Malformed input handling (Property 11)
- Security Middleware Tests
- Header presence (Property 12)
- Rate limiting (Properties 13, 14, 15)
- CSRF validation (Properties 16, 17, 18)
- Query Module Tests
- SQL injection prevention (Property 19)
- Connection pooling (Property 20)
- Error structure (Property 21)
- Sync Module Tests
- Pub/sub delivery (Property 22)
- Message buffering (Property 23)
- Presence tracking (Property 24)
- Acknowledgments (Property 25)
- Integration Tests
- End-to-end authentication flow
- SSR with bot detection
- Binary streaming with delta patching
- Error response format validation

### Coverage Requirements

- Critical modules (auth, error, client): Minimum 80% line coverage
- Security modules (rate limiter, CSRF): Minimum 90% line coverage
- Overall workspace: Minimum 70% line coverage
