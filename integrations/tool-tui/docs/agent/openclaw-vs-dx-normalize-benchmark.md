# OpenClaw vs DX WhatsApp Normalize Benchmark

- Timestamp: 2026-02-14T09:14:10.914Z
- Iterations per implementation: 200000
- Corpus size: 19
- Parity mismatches: 0

## Results

| Impl | Avg ns/op | Total ms |
|---|---:|---:|
| DX | 276.36 | 55.27 |
| OpenClaw reference | 270.97 | 54.19 |

- Winner: **openclaw** (1.02x faster)

## Notes

- OpenClaw reference implementation is adapted from source files in integrations/openclaw.
- This benchmark measures target normalization path only.
- End-to-end channel throughput should be measured separately.
