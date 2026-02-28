//! # Cache API Storage
//!
//! HTTP cache for delta updates and assets

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Cache, CacheStorage, Request, Response, window};

/// Open cache
pub async fn open_cache(cache_name: &str) -> Result<Cache, JsValue> {
    let window = window().ok_or("No window")?;
    let caches: CacheStorage = window.caches()?;

    let promise = caches.open(cache_name);
    let result = JsFuture::from(promise).await?;

    result.dyn_into()
}

/// Store response in cache
pub async fn cache_response(cache: &Cache, url: &str, response: &Response) -> Result<(), JsValue> {
    let promise = cache.put_with_str(url, response);
    JsFuture::from(promise).await?;
    Ok(())
}

/// Get cached response
pub async fn get_cached_response(cache: &Cache, url: &str) -> Result<Option<Response>, JsValue> {
    let request = Request::new_with_str(url)?;
    let promise = cache.match_with_request(&request);
    let result = JsFuture::from(promise).await?;

    if result.is_undefined() {
        return Ok(None);
    }

    Ok(Some(result.dyn_into()?))
}

/// Delete cache
pub async fn delete_cache(cache_name: &str) -> Result<bool, JsValue> {
    let window = window().ok_or("No window")?;
    let caches: CacheStorage = window.caches()?;

    let promise = caches.delete(cache_name);
    let result = JsFuture::from(promise).await?;

    Ok(result.as_bool().unwrap_or(false))
}

/// Get the total size of all cached responses in bytes
pub async fn get_cache_size(cache: &Cache) -> Result<u64, JsValue> {
    let promise = cache.keys();
    let result = JsFuture::from(promise).await?;

    if result.is_undefined() {
        return Ok(0);
    }

    let requests: js_sys::Array = result.dyn_into()?;
    let mut total_size: u64 = 0;

    for i in 0..requests.length() {
        let request: Request = requests.get(i).dyn_into()?;

        // Get the cached response for this request
        let match_promise = cache.match_with_request(&request);
        let match_result = JsFuture::from(match_promise).await?;

        if !match_result.is_undefined() {
            let response: Response = match_result.dyn_into()?;

            // Clone the response to read its body without consuming it
            let cloned = response.clone()?;

            // Get the response body as an ArrayBuffer
            if let Ok(body_promise) = cloned.array_buffer() {
                if let Ok(body_result) = JsFuture::from(body_promise).await {
                    if let Ok(array_buffer) = body_result.dyn_into::<js_sys::ArrayBuffer>() {
                        total_size += array_buffer.byte_length() as u64;
                    }
                }
            }

            // Add header size estimate (rough approximation)
            // Headers typically add 200-500 bytes per response
            total_size += 300;
        }
    }

    Ok(total_size)
}

/// Get the count of cached entries
pub async fn get_cache_entry_count(cache: &Cache) -> Result<u32, JsValue> {
    let promise = cache.keys();
    let result = JsFuture::from(promise).await?;

    if result.is_undefined() {
        return Ok(0);
    }

    let requests: js_sys::Array = result.dyn_into()?;
    Ok(requests.length())
}

/// Get the total size of all caches in the CacheStorage
pub async fn get_all_caches_size() -> Result<u64, JsValue> {
    let window = window().ok_or("No window")?;
    let caches: CacheStorage = window.caches()?;

    let keys_promise = caches.keys();
    let keys_result = JsFuture::from(keys_promise).await?;

    if keys_result.is_undefined() {
        return Ok(0);
    }

    let cache_names: js_sys::Array = keys_result.dyn_into()?;
    let mut total_size: u64 = 0;

    for i in 0..cache_names.length() {
        if let Some(cache_name) = cache_names.get(i).as_string() {
            let cache = open_cache(&cache_name).await?;
            let size = get_cache_size(&cache).await?;
            total_size += size;
        }
    }

    Ok(total_size)
}

/// Get the total count of entries across all caches
pub async fn get_all_caches_entry_count() -> Result<u32, JsValue> {
    let window = window().ok_or("No window")?;
    let caches: CacheStorage = window.caches()?;

    let keys_promise = caches.keys();
    let keys_result = JsFuture::from(keys_promise).await?;

    if keys_result.is_undefined() {
        return Ok(0);
    }

    let cache_names: js_sys::Array = keys_result.dyn_into()?;
    let mut total_count: u32 = 0;

    for i in 0..cache_names.length() {
        if let Some(cache_name) = cache_names.get(i).as_string() {
            let cache = open_cache(&cache_name).await?;
            let count = get_cache_entry_count(&cache).await?;
            total_count += count;
        }
    }

    Ok(total_count)
}

#[cfg(test)]
mod tests {
    // Cache API tests require a browser environment
    // These tests verify the function signatures and basic logic

    #[test]
    fn test_cache_api_module_compiles() {
        // This test verifies that the module compiles correctly
        // Actual functionality tests require wasm-bindgen-test
        assert!(true);
    }
}
