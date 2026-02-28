# dx-www-build

Build pipeline orchestrator for DX-WWW applications.

## Features

- **Asset Processing Coordination**: Orchestrates media, styles, icons, fonts, i18n, and serialization processing
- **Content-Based Caching**: Implements caching layer with BLAKE3 content hashing to avoid redundant work
- **Modular Architecture**: Extensible design for adding new asset processors

## Usage

```rust
use build::{BuildPipeline, BuildConfig};

let config = BuildConfig::default();
let mut pipeline = BuildPipeline::new("./my-project", config)?;
let result = pipeline.build()?;

println!("Processed {} artifacts", result.artifacts.len());
println!("Build time: {}ms", result.stats.build_time_ms);
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
