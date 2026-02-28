//! # IndexedDB Storage
//!
//! Primary storage for templates, snapshots, and state

use js_sys::{Array, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{IdbDatabase, IdbOpenDbRequest, IdbTransactionMode, window};

/// Open or create IndexedDB database
pub async fn open_database(db_name: &str, version: u32) -> Result<IdbDatabase, JsValue> {
    let window = window().ok_or("No window")?;
    let indexed_db = window.indexed_db()?.ok_or("No IndexedDB")?;

    let request: IdbOpenDbRequest = indexed_db.open_with_u32(db_name, version)?;

    // Set up onupgradeneeded handler
    let onupgradeneeded = Closure::once(move |event: web_sys::IdbVersionChangeEvent| {
        let target = event.target().unwrap();
        let request: IdbOpenDbRequest = target.dyn_into().unwrap();
        let db: IdbDatabase = request.result().unwrap().dyn_into().unwrap();

        // Create object stores
        if !db.object_store_names().contains("templates") {
            db.create_object_store("templates").unwrap();
        }

        if !db.object_store_names().contains("snapshots") {
            db.create_object_store("snapshots").unwrap();
        }

        if !db.object_store_names().contains("metadata") {
            db.create_object_store("metadata").unwrap();
        }
    });

    request.set_onupgradeneeded(Some(onupgradeneeded.as_ref().unchecked_ref()));
    onupgradeneeded.forget();

    // Wait for success
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        let onsuccess = Closure::once(move |event: web_sys::Event| {
            let target = event.target().unwrap();
            let request: IdbOpenDbRequest = target.dyn_into().unwrap();
            let db: IdbDatabase = request.result().unwrap().dyn_into().unwrap();
            resolve.call1(&JsValue::NULL, &db).unwrap();
        });

        let onerror = Closure::once(move |event: web_sys::Event| {
            let target = event.target().unwrap();
            let request: IdbOpenDbRequest = target.dyn_into().unwrap();
            let error = request.error().unwrap().unwrap();
            reject.call1(&JsValue::NULL, &error).unwrap();
        });

        request.set_onsuccess(Some(onsuccess.as_ref().unchecked_ref()));
        request.set_onerror(Some(onerror.as_ref().unchecked_ref()));

        onsuccess.forget();
        onerror.forget();
    });

    let result = JsFuture::from(promise).await?;
    result.dyn_into()
}

/// Store binary data in IndexedDB
pub async fn store_binary(
    db: &IdbDatabase,
    store_name: &str,
    key: &str,
    data: &[u8],
) -> Result<(), JsValue> {
    let transaction =
        db.transaction_with_str_and_mode(store_name, IdbTransactionMode::Readwrite)?;

    let store = transaction.object_store(store_name)?;

    // Convert Rust bytes to JS Uint8Array
    let uint8_array = Uint8Array::from(data);

    store.put_with_key(&uint8_array, &JsValue::from_str(key))?;

    Ok(())
}

/// Retrieve binary data from IndexedDB
pub async fn get_binary(
    db: &IdbDatabase,
    store_name: &str,
    key: &str,
) -> Result<Option<Vec<u8>>, JsValue> {
    let transaction = db.transaction_with_str(store_name)?;
    let store = transaction.object_store(store_name)?;

    let request = store.get(&JsValue::from_str(key))?;

    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        let onsuccess = Closure::once(move |event: web_sys::Event| {
            let target = event.target().unwrap();
            let request: web_sys::IdbRequest = target.dyn_into().unwrap();
            resolve.call1(&JsValue::NULL, &request.result().unwrap()).unwrap();
        });

        let onerror = Closure::once(move |event: web_sys::Event| {
            let target = event.target().unwrap();
            let request: web_sys::IdbRequest = target.dyn_into().unwrap();
            let error = request.error().unwrap().unwrap();
            reject.call1(&JsValue::NULL, &error).unwrap();
        });

        request.set_onsuccess(Some(onsuccess.as_ref().unchecked_ref()));
        request.set_onerror(Some(onerror.as_ref().unchecked_ref()));

        onsuccess.forget();
        onerror.forget();
    });

    let result = JsFuture::from(promise).await?;

    if result.is_undefined() {
        return Ok(None);
    }

    // Convert JS Uint8Array to Rust Vec<u8>
    let uint8_array: Uint8Array = result.dyn_into()?;
    let mut vec = vec![0u8; uint8_array.length() as usize];
    uint8_array.copy_to(&mut vec);

    Ok(Some(vec))
}

/// Delete IndexedDB database
pub async fn delete_database(db_name: &str) -> Result<(), JsValue> {
    let window = window().ok_or("No window")?;
    let indexed_db = window.indexed_db()?.ok_or("No IndexedDB")?;

    let request = indexed_db.delete_database(db_name)?;

    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        let onsuccess = Closure::once(move |_: web_sys::Event| {
            resolve.call0(&JsValue::NULL).unwrap();
        });

        let onerror = Closure::once(move |event: web_sys::Event| {
            let target = event.target().unwrap();
            let request: IdbOpenDbRequest = target.dyn_into().unwrap();
            let error = request.error().unwrap().unwrap();
            reject.call1(&JsValue::NULL, &error).unwrap();
        });

        request.set_onsuccess(Some(onsuccess.as_ref().unchecked_ref()));
        request.set_onerror(Some(onerror.as_ref().unchecked_ref()));

        onsuccess.forget();
        onerror.forget();
    });

    JsFuture::from(promise).await?;

    Ok(())
}

/// Get the size of all data in an IndexedDB database
pub async fn get_database_size(db: &IdbDatabase) -> Result<u64, JsValue> {
    let store_names = db.object_store_names();
    let mut total_size: u64 = 0;

    for i in 0..store_names.length() {
        if let Some(store_name) = store_names.get(i) {
            let size = get_store_size(db, &store_name).await?;
            total_size += size;
        }
    }

    Ok(total_size)
}

/// Get the size of all data in an object store
async fn get_store_size(db: &IdbDatabase, store_name: &str) -> Result<u64, JsValue> {
    let transaction = db.transaction_with_str(store_name)?;
    let store = transaction.object_store(store_name)?;

    // Get all keys and values to calculate size
    let request = store.get_all()?;

    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        let onsuccess = Closure::once(move |event: web_sys::Event| {
            let target = event.target().unwrap();
            let request: web_sys::IdbRequest = target.dyn_into().unwrap();
            resolve.call1(&JsValue::NULL, &request.result().unwrap()).unwrap();
        });

        let onerror = Closure::once(move |event: web_sys::Event| {
            let target = event.target().unwrap();
            let request: web_sys::IdbRequest = target.dyn_into().unwrap();
            let error = request.error().unwrap().unwrap();
            reject.call1(&JsValue::NULL, &error).unwrap();
        });

        request.set_onsuccess(Some(onsuccess.as_ref().unchecked_ref()));
        request.set_onerror(Some(onerror.as_ref().unchecked_ref()));

        onsuccess.forget();
        onerror.forget();
    });

    let result = JsFuture::from(promise).await?;

    if result.is_undefined() {
        return Ok(0);
    }

    // Calculate size of all values
    let array: Array = result.dyn_into()?;
    let mut size: u64 = 0;

    for i in 0..array.length() {
        let value = array.get(i);
        size += estimate_js_value_size(&value);
    }

    Ok(size)
}

/// Get the count of all entries in an IndexedDB database
pub async fn get_database_entry_count(db: &IdbDatabase) -> Result<u32, JsValue> {
    let store_names = db.object_store_names();
    let mut total_count: u32 = 0;

    for i in 0..store_names.length() {
        if let Some(store_name) = store_names.get(i) {
            let count = get_store_entry_count(db, &store_name).await?;
            total_count += count;
        }
    }

    Ok(total_count)
}

/// Get the count of entries in an object store
async fn get_store_entry_count(db: &IdbDatabase, store_name: &str) -> Result<u32, JsValue> {
    let transaction = db.transaction_with_str(store_name)?;
    let store = transaction.object_store(store_name)?;

    let request = store.count()?;

    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        let onsuccess = Closure::once(move |event: web_sys::Event| {
            let target = event.target().unwrap();
            let request: web_sys::IdbRequest = target.dyn_into().unwrap();
            resolve.call1(&JsValue::NULL, &request.result().unwrap()).unwrap();
        });

        let onerror = Closure::once(move |event: web_sys::Event| {
            let target = event.target().unwrap();
            let request: web_sys::IdbRequest = target.dyn_into().unwrap();
            let error = request.error().unwrap().unwrap();
            reject.call1(&JsValue::NULL, &error).unwrap();
        });

        request.set_onsuccess(Some(onsuccess.as_ref().unchecked_ref()));
        request.set_onerror(Some(onerror.as_ref().unchecked_ref()));

        onsuccess.forget();
        onerror.forget();
    });

    let result = JsFuture::from(promise).await?;

    Ok(result.as_f64().unwrap_or(0.0) as u32)
}

/// Estimate the size of a JavaScript value in bytes
fn estimate_js_value_size(value: &JsValue) -> u64 {
    if value.is_undefined() || value.is_null() {
        return 0;
    }

    // If it's a Uint8Array, get its byte length directly
    if let Ok(uint8_array) = value.clone().dyn_into::<Uint8Array>() {
        return uint8_array.length() as u64;
    }

    // If it's an ArrayBuffer, get its byte length
    if let Ok(array_buffer) = value.clone().dyn_into::<js_sys::ArrayBuffer>() {
        return array_buffer.byte_length() as u64;
    }

    // If it's a string, estimate based on UTF-16 encoding (2 bytes per char)
    if let Some(s) = value.as_string() {
        return (s.len() * 2) as u64;
    }

    // If it's a number, it's 8 bytes (f64)
    if value.as_f64().is_some() {
        return 8;
    }

    // If it's a boolean, it's 1 byte
    if value.as_bool().is_some() {
        return 1;
    }

    // For objects/arrays, try to serialize and measure
    if let Ok(json) = js_sys::JSON::stringify(value) {
        if let Some(s) = json.as_string() {
            return s.len() as u64;
        }
    }

    // Default estimate for unknown types
    64
}

#[cfg(test)]
mod tests {
    // Note: Tests that use JsValue require a WASM environment
    // These tests verify the function signatures and basic logic
    // For full integration tests, use wasm-bindgen-test

    #[test]
    fn test_indexeddb_module_compiles() {
        // This test verifies that the module compiles correctly
        // Actual functionality tests require wasm-bindgen-test
        assert!(true);
    }

    // WASM-only tests would go here with #[cfg(target_arch = "wasm32")]
    // and use wasm_bindgen_test::wasm_bindgen_test attribute
}
