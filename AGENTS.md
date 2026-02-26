# Agent Rules

See `.rules` for full coding guidelines.

## 🚨 MANDATORY BUILD RULE

The ONLY allowed build command is:
```bash
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=1 CARGO_PROFILE_DEV_CODEGEN_UNITS=1 CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS=1 cargo run -p zed --locked
```

NEVER use `cargo build -p <crate>`, `cargo check -p <crate>`, or `cargo test -p <crate>`. Each creates a separate artifact tree wasting gigabytes of disk space on this low-resource system.