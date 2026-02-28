pub fn init() {
    // DX_LOG examples:
    // - "info"
    // - "dx_gpui_hello=debug,wry=info"
    // - "warn"
    let filter = std::env::var("DX_LOG").unwrap_or_else(|_| "info".to_string());

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_level(true)
        .with_ansi(true)
        .compact()
        .try_init();
}
