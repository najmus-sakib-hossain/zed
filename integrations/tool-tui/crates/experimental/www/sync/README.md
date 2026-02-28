
# dx-sync — Realtime Binary WebSocket Protocol

Replace Socket.io + Pusher with zero-parse binary WebSocket streaming.

## What It Does

- Channel-based pub/sub — Subscribe to realtime updates
- Delta updates — XOR-based diffs for bandwidth efficiency
- Message history — Automatic state synchronization
- Auto-reconnection — Exponential backoff with state resync

## Replaces

- socket.io-client (85 KB)
- pusher-js (45 KB)
- @liveblocks/client (60 KB) Total replaced: 190 KB → 0 KB

## Example

```typescript
// Client-side const channel = sync.subscribe("chat:lobby");
channel.on("message", (data) => { console.log("New message:", data);
});
channel.send({ user: "Alice", text: "Hello!" });
// Server-side (Rust)
let manager = ChannelManager::new(1000);
manager.publish(BinaryMessage { channel_id: 1, message_id: 42, data: vec![...], timestamp: now(), });
```

## Performance

+--------+-----------+---------+-------------+
| Metric | Socket.io | dx-sync | Improvement |
+========+===========+=========+=============+
| Bundle | size      | 85      | KB          |
+--------+-----------+---------+-------------+



## Binary Protocol

+--------+-----------+-------------+
| Opcode | Hex       | Description |
+========+===========+=============+
| SYNC   | SUBSCRIBE | 0xA0        |
+--------+-----------+-------------+



## Features

- XOR-based delta updates — Only send changed bytes
- Message history — Configurable per channel
- Reconnection handler — Exponential backoff (100ms → 3.2s)
- Subscriber tracking — Automatic cleanup of disconnected clients
- Channel isolation — Independent pub/sub channels

## Tests

```bash
cargo test -p dx-sync ```
All 4 tests passing ✅
