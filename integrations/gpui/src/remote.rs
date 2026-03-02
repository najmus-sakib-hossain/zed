//! Remote file fetching — thin ureq wrapper used by all media views.
//!
//! Always call `fetch_bytes` from inside `smol::unblock` — it blocks the thread.

use std::io::Read;

/// Download a remote URL and return its bytes.
///
/// # Errors
/// Returns an error on non-2xx status or I/O failure.
pub fn fetch_bytes(url: &str) -> anyhow::Result<Vec<u8>> {
    let resp = ureq::get(url)
        .set("User-Agent", "gpui-media-app/0.1 (pure-rust)")
        .call()
        .map_err(|e| anyhow::anyhow!("HTTP fetch failed for {url}: {e}"))?;

    // Pre-allocate from Content-Length if available
    let capacity = resp
        .header("Content-Length")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(4 * 1024 * 1024);

    let mut buf = Vec::with_capacity(capacity);
    resp.into_reader().read_to_end(&mut buf)?;
    Ok(buf)
}

/// True for `http://` or `https://` strings.
#[inline]
pub fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

/// Resolve a source string to raw bytes:
/// - HTTP/HTTPS URL  → download
/// - Non-empty path  → read from disk
/// - Empty string    → return `Ok(vec![])` (caller should skip processing)
pub fn resolve(src: &str) -> anyhow::Result<Vec<u8>> {
    if is_url(src) {
        fetch_bytes(src)
    } else if src.is_empty() {
        Ok(Vec::new())
    } else {
        Ok(std::fs::read(src)?)
    }
}
