//! # PWA Module - Progressive Web App Engine
//!
//! Compiles `pwa/` folder into a Binary Service Worker for offline-first apps.
//!
//! ## Features
//! - Parses `pwa/manifest.dx` for PWA metadata
//! - Generates binary service worker with offline caching
//! - Supports background sync and push notifications
//!
//! ## Example pwa/manifest.dx
//! ```dx
//! name "My App"
//! short_name "App"
//! theme_color "#1a1a2e"
//! background_color "#16213e"
//! display "standalone"
//!
//! icons {
//!     "icon:app-192" 192
//!     "icon:app-512" 512
//! }
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// PWA Manifest configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PwaManifest {
    pub name: String,
    pub short_name: String,
    pub description: String,
    pub theme_color: String,
    pub background_color: String,
    pub display: String,
    pub start_url: String,
    pub icons: Vec<PwaIcon>,
    pub scope: String,
}

/// PWA Icon definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PwaIcon {
    pub src: String,
    pub sizes: String,
    pub icon_type: String,
    pub purpose: String,
}

impl PwaManifest {
    pub fn new() -> Self {
        Self {
            display: "standalone".to_string(),
            start_url: "/".to_string(),
            scope: "/".to_string(),
            ..Default::default()
        }
    }

    /// Convert to JSON for standard manifest.json
    pub fn to_json(&self) -> Result<String> {
        let manifest = serde_json::json!({
            "name": self.name,
            "short_name": self.short_name,
            "description": self.description,
            "theme_color": self.theme_color,
            "background_color": self.background_color,
            "display": self.display,
            "start_url": self.start_url,
            "scope": self.scope,
            "icons": self.icons.iter().map(|i| serde_json::json!({
                "src": i.src,
                "sizes": i.sizes,
                "type": i.icon_type,
                "purpose": i.purpose
            })).collect::<Vec<_>>()
        });

        serde_json::to_string_pretty(&manifest).context("Failed to serialize PWA manifest")
    }
}

/// Parse pwa/manifest.dx
pub fn parse_manifest(root: &Path, verbose: bool) -> Result<Option<PwaManifest>> {
    let manifest_path = root.join("pwa").join("manifest.dx");

    if !manifest_path.exists() {
        if verbose {
            println!("  📱 PWA: No pwa/manifest.dx found, skipping");
        }
        return Ok(None);
    }

    if verbose {
        println!("  📱 PWA: Parsing manifest...");
    }

    let source = fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read {}", manifest_path.display()))?;

    let manifest = parse_manifest_content(&source, verbose)?;

    if verbose {
        println!("    Name: {}", manifest.name);
        println!("    Theme: {}", manifest.theme_color);
    }

    Ok(Some(manifest))
}

/// Parse manifest content
fn parse_manifest_content(source: &str, _verbose: bool) -> Result<PwaManifest> {
    let mut manifest = PwaManifest::new();

    for line in source.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
            continue;
        }

        // Parse name: name "My App"
        if line.starts_with("name") && !line.starts_with("short_name") {
            if let Some(value) = extract_quoted(line) {
                manifest.name = value;
            }
        }

        // Parse short_name: short_name "App"
        if line.starts_with("short_name") {
            if let Some(value) = extract_quoted(line) {
                manifest.short_name = value;
            }
        }

        // Parse theme_color: theme_color "#1a1a2e"
        if line.starts_with("theme_color") {
            if let Some(value) = extract_quoted(line) {
                manifest.theme_color = value;
            }
        }

        // Parse background_color: background_color "#16213e"
        if line.starts_with("background_color") {
            if let Some(value) = extract_quoted(line) {
                manifest.background_color = value;
            }
        }

        // Parse display: display "standalone"
        if line.starts_with("display") {
            if let Some(value) = extract_quoted(line) {
                manifest.display = value;
            }
        }

        // Parse description: description "A great app"
        if line.starts_with("description") {
            if let Some(value) = extract_quoted(line) {
                manifest.description = value;
            }
        }
    }

    Ok(manifest)
}

/// Extract quoted value
fn extract_quoted(line: &str) -> Option<String> {
    let start = line.find('"')?;
    let rest = &line[start + 1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Generate binary service worker
pub fn generate_service_worker(
    manifest: &PwaManifest,
    cache_files: &[String],
    verbose: bool,
) -> Result<String> {
    if verbose {
        println!("  📱 PWA: Generating service worker...");
    }

    let cache_name = format!("{}-v1", manifest.short_name.to_lowercase().replace(' ', "-"));

    let cache_list =
        cache_files.iter().map(|f| format!("  '{}'", f)).collect::<Vec<_>>().join(",\n");

    let sw_code = format!(
        r#"// Auto-generated Service Worker by dx-compiler
// DO NOT EDIT

const CACHE_NAME = '{}';
const urlsToCache = [
  '/',
{}
];

// Install event - cache all static assets
self.addEventListener('install', (event) => {{
  event.waitUntil(
    caches.open(CACHE_NAME)
      .then((cache) => {{
        console.log('Opened cache');
        return cache.addAll(urlsToCache);
      }})
      .then(() => self.skipWaiting())
  );
}});

// Activate event - clean up old caches
self.addEventListener('activate', (event) => {{
  event.waitUntil(
    caches.keys().then((cacheNames) => {{
      return Promise.all(
        cacheNames.filter((name) => name !== CACHE_NAME)
          .map((name) => caches.delete(name))
      );
    }}).then(() => self.clients.claim())
  );
}});

// Fetch event - serve from cache, fallback to network
self.addEventListener('fetch', (event) => {{
  event.respondWith(
    caches.match(event.request)
      .then((response) => {{
        // Cache hit - return response
        if (response) {{
          return response;
        }}

        // Clone the request
        const fetchRequest = event.request.clone();

        return fetch(fetchRequest).then((response) => {{
          // Check if valid response
          if (!response || response.status !== 200 || response.type !== 'basic') {{
            return response;
          }}

          // Clone and cache the response
          const responseToCache = response.clone();
          caches.open(CACHE_NAME)
            .then((cache) => {{
              cache.put(event.request, responseToCache);
            }});

          return response;
        }});
      }})
  );
}});

// Binary RPC handler for dx-www
self.addEventListener('message', (event) => {{
  if (event.data && event.data.type === 'DX_RPC') {{
    // Handle binary RPC calls
    console.log('DX RPC:', event.data.method);
  }}
}});
"#,
        cache_name, cache_list
    );

    if verbose {
        println!("    Generated SW: {} bytes", sw_code.len());
    }

    Ok(sw_code)
}

/// Generate the HTML meta tags for PWA
pub fn generate_pwa_meta(manifest: &PwaManifest) -> String {
    format!(
        r#"
    <meta name="theme-color" content="{theme}">
    <meta name="mobile-web-app-capable" content="yes">
    <meta name="apple-mobile-web-app-capable" content="yes">
    <meta name="apple-mobile-web-app-status-bar-style" content="black-translucent">
    <meta name="apple-mobile-web-app-title" content="{name}">
    <link rel="manifest" href="/manifest.json">
    "#,
        theme = manifest.theme_color,
        name = manifest.short_name,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_manifest_content() {
        let source = r##"
            name "My Awesome App"
            short_name "MyApp"
            theme_color "#1a1a2e"
            background_color "#16213e"
            display "standalone"
        "##;

        let manifest = parse_manifest_content(source, false).unwrap();
        assert_eq!(manifest.name, "My Awesome App");
        assert_eq!(manifest.short_name, "MyApp");
        assert_eq!(manifest.theme_color, "#1a1a2e");
        assert_eq!(manifest.display, "standalone");
    }

    #[test]
    fn test_generate_service_worker() {
        let manifest = PwaManifest {
            name: "Test App".to_string(),
            short_name: "Test".to_string(),
            ..Default::default()
        };

        let cache_files = vec!["/app.js".to_string(), "/style.css".to_string()];
        let sw = generate_service_worker(&manifest, &cache_files, false).unwrap();

        assert!(sw.contains("test-v1"));
        assert!(sw.contains("/app.js"));
        assert!(sw.contains("self.addEventListener('install'"));
    }
}
