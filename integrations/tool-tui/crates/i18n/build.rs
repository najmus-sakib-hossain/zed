fn main() {
    // Set environment variables to disable whisper logging
    println!("cargo:rustc-env=GGML_LOG_DISABLE=1");
    println!("cargo:rustc-env=WHISPER_LOG_DISABLE=1");

    // Enable all CPU optimizations
    println!("cargo:rustc-link-arg=-march=native");
    println!("cargo:rustc-link-arg=-mtune=native");
}
