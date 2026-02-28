@echo off
cargo run --example wisprflow_silent --features whisper,wisprflow -p dx-i18n --release 2>nul
