fn main() {
    // Simple wrapper that runs dx-style with playground files
    let exe_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("dx-style.exe")))
        .expect("Failed to locate dx-style binary");

    std::process::exit(
        std::process::Command::new(exe_path)
            .args(&["crates/style/playground/index.html", "--watch"])
            .status()
            .map(|s| s.code().unwrap_or(1))
            .unwrap_or(1),
    );
}
