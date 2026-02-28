
# dx-query — Binary RPC Data Fetching

Replace TanStack Query + Axios with zero-parse binary RPC.

## What It Does

- Binary RPC — Direct memory request/response
- Concurrent cache — DashMap with TTL
- Live subscriptions — WebSocket binary streams
- Zero-copy parsing — No JSON overhead

## Replaces

- @tanstack/react-query (38 KB)
- axios (32 KB)
- swr (18 KB)
- apollo-client (120 KB) Total replaced: 208 KB → 0 KB

## Example

```typescript
// app.dx async function UserProfile() { // Automatic caching, deduplication, retry const user = await query('/api/user/123');
return <div>{user.name}</div>;
}
```

## Performance

+--------+----------+-------+----------+-------------+
| Metric | TanStack | Query | dx-query | Improvement |
+========+==========+=======+==========+=============+
| Bundle | size     | 38    | KB       | 0           |
+--------+----------+-------+----------+-------------+



## Binary Protocol

+--------+---------+---------+
| Opcode | Hex     | Payload |
+========+=========+=========+
| QUERY  | REQUEST | 0x70    |
+--------+---------+---------+



## Features

- Automatic caching with configurable TTL
- Request deduplication via hash-based keys
- Retry with exponential backoff
- Stale-while-revalidate pattern
- Live subscriptions for realtime updates
- Optimistic updates built-in
- Cache invalidation by key or prefix
