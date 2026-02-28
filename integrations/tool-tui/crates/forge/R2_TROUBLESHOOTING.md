# R2 Storage Integration - Problem & Solution

## Problem

The R2 storage integration was failing with `SignatureDoesNotMatch` errors when attempting to upload/download blobs. The AWS Signature Version 4 authentication was not working correctly.

## Root Causes

### 1. Third-Party Signing Library Issues

Initially tried using `aws-sign-v4` crate (v0.3.0), which had issues:
- Expected string body instead of binary data
- Signature computation was incorrect for R2's S3-compatible API
- The library was computing different signatures than what R2 expected

Then attempted to use official `aws-sigv4` crate (v1.2.6), which had API complexity issues:
- Required multiple AWS SDK dependencies
- Complex type conversions for identity and credentials
- API mismatch between expected types and what we had

### 2. Inconsistent Body Hash Handling

For GET/HEAD/DELETE requests:
- Initially used `"UNSIGNED-PAYLOAD"` as the content hash header
- But signed the request with empty body hash (`e3b0c44...`)
- This mismatch caused signature verification to fail

## Solution

### Manual AWS Signature V4 Implementation

Implemented a clean, manual AWS Signature V4 signing process:

```rust
fn create_auth_header(&self, method: &str, key: &str, body: &[u8]) -> Result<String> {
    // 1. Create canonical request with proper headers
    let canonical_headers = format!(
        "host:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n",
        host, payload_hash, amz_date
    );
    
    // 2. Create string to sign
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        amz_date, credential_scope, canonical_request_hash
    );
    
    // 3. Calculate signature using HMAC-SHA256 chain
    let k_date = hmac_sha256(format!("AWS4{}", secret_key).as_bytes(), date.as_bytes());
    let k_region = hmac_sha256(&k_date, b"auto");
    let k_service = hmac_sha256(&k_region, b"s3");
    let k_signing = hmac_sha256(&k_service, b"aws4_request");
    let signature = hex::encode(hmac_sha256(&k_signing, string_to_sign.as_bytes()));
    
    // 4. Build authorization header
    format!("AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        access_key_id, credential_scope, signed_headers, signature)
}
```

### Consistent Body Hash Usage

For all HTTP methods:
- **PUT requests**: Pass actual body bytes, compute SHA-256 hash
- **GET/HEAD/DELETE requests**: Pass empty body (`b""`), compute SHA-256 of empty bytes
- Always include `x-amz-content-sha256` header with the computed hash
- Sign the request with the same hash value

### Key Changes

1. Removed `aws-sign-v4` dependency
2. Added manual HMAC-SHA256 helper function
3. Updated all request methods to use `b""` for empty body (not `&[]`)
4. Ensured body hash in header matches body hash used in signature

## Dependencies Used

```toml
# Minimal dependencies for R2 signing
http = "1.2.0"      # For HTTP types (not used in final solution)
hmac = "0.12.1"     # HMAC authentication
sha2 = "0.10.9"     # SHA-256 hashing
hex = "0.4.3"       # Hex encoding for signatures
```

## Test Results

All R2 operations now work correctly:
- ✅ Upload blob (PUT with binary body)
- ✅ Check existence (HEAD with empty body)
- ✅ Download blob (GET with empty body)
- ✅ Delete blob (DELETE with empty body)

## Lessons Learned

1. **Manual implementation > complex libraries**: For S3-compatible APIs, a clean manual implementation is often simpler and more reliable than wrestling with SDK complexity
2. **Consistency is critical**: Body hash in headers must exactly match body hash used in signature computation
3. **Empty body ≠ unsigned payload**: For GET/HEAD/DELETE, use SHA-256 of empty bytes, not "UNSIGNED-PAYLOAD"
4. **R2 specifics**: Uses "auto" as region, standard S3 signature v4 otherwise
