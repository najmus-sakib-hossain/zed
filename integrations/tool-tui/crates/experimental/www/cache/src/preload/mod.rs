//! # Preload Module
//!
//! Service Worker interceptor for instant cache serving

use wasm_bindgen::prelude::*;

/// Check if Service Worker is supported
#[wasm_bindgen]
pub fn is_service_worker_supported() -> bool {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return false,
    };

    let navigator = window.navigator();

    js_sys::Reflect::has(&navigator, &"serviceWorker".into()).unwrap_or(false)
}

/// Register Service Worker
#[wasm_bindgen]
pub async fn register_service_worker(script_url: &str) -> Result<JsValue, JsValue> {
    let window = web_sys::window().ok_or("No window")?;
    let navigator = window.navigator();

    let service_worker = js_sys::Reflect::get(&navigator, &"serviceWorker".into())?;

    if service_worker.is_undefined() {
        return Err("Service Worker not supported".into());
    }

    // Call navigator.serviceWorker.register(script_url)
    let register_fn = js_sys::Reflect::get(&service_worker, &"register".into())?;
    let register_fn: js_sys::Function = register_fn.dyn_into()?;

    let promise = register_fn.call1(&service_worker, &JsValue::from_str(script_url))?;
    let promise: js_sys::Promise = promise.dyn_into()?;

    wasm_bindgen_futures::JsFuture::from(promise).await
}

/// Service Worker script content
pub const SERVICE_WORKER_SCRIPT: &str = r#"
// dx-cache Service Worker
// Intercepts requests and serves from IndexedDB instantly

const CACHE_NAME = 'dx-cache-v1';
const DB_NAME = 'dx-cache';

// Install event
self.addEventListener('install', (event) => {
    console.log('[dx-cache SW] Installing...');
    self.skipWaiting();
});

// Activate event
self.addEventListener('activate', (event) => {
    console.log('[dx-cache SW] Activating...');
    event.waitUntil(self.clients.claim());
});

// Fetch event - THE MAGIC HAPPENS HERE
self.addEventListener('fetch', (event) => {
    const url = new URL(event.request.url);
    
    // Only intercept dx-binary requests
    if (!url.pathname.endsWith('.dxb') && !url.pathname.endsWith('.wasm')) {
        return;
    }
    
    event.respondWith(
        (async () => {
            // Try cache first
            const cache = await caches.open(CACHE_NAME);
            const cached = await cache.match(event.request);
            
            if (cached) {
                console.log('[dx-cache SW] Cache HIT:', url.pathname);
                return cached;
            }
            
            console.log('[dx-cache SW] Cache MISS - fetching:', url.pathname);
            
            // Fetch from network
            const response = await fetch(event.request);
            
            // Cache for next time
            cache.put(event.request, response.clone());
            
            return response;
        })()
    );
});
"#;
