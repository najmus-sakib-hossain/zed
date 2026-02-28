//! Cart System — The Gateway Drug to dx

use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartItem {
    pub id: String,
    pub package_id: String,
    pub variant: Option<String>,
    pub files: Vec<PathBuf>,
    pub config: serde_json::Value,
}

static CART: OnceLock<Arc<RwLock<Vec<CartItem>>>> = OnceLock::new();

fn get_cart() -> Arc<RwLock<Vec<CartItem>>> {
    CART.get_or_init(|| Arc::new(RwLock::new(Vec::new()))).clone()
}

pub fn stage_item_in_cart(item: CartItem) -> Result<()> {
    let cart = get_cart();
    let mut cart = cart.write();

    tracing::info!("🛒 Staging item in cart: {}", item.package_id);
    cart.push(item);

    Ok(())
}

pub fn commit_entire_cart() -> Result<Vec<PathBuf>> {
    let cart = get_cart();
    let cart_items = cart.read().clone();

    tracing::info!("✅ Committing cart with {} items", cart_items.len());

    let mut installed_files = Vec::new();

    for item in &cart_items {
        // Install each item
        installed_files.extend(item.files.clone());
    }

    // Clear cart after commit
    drop(cart_items);
    cart.write().clear();

    Ok(installed_files)
}

pub fn commit_cart_immediately() -> Result<Vec<PathBuf>> {
    commit_entire_cart()
}

pub fn clear_cart_completely() -> Result<()> {
    let cart = get_cart();
    let mut cart = cart.write();

    tracing::info!("🗑️  Clearing cart ({} items)", cart.len());
    cart.clear();

    Ok(())
}

pub fn remove_specific_cart_item(item_id: &str) -> Result<()> {
    let cart = get_cart();
    let mut cart = cart.write();

    cart.retain(|item| item.id != item_id);
    tracing::info!("➖ Removed item from cart: {}", item_id);

    Ok(())
}

pub fn get_current_cart_contents() -> Result<Vec<CartItem>> {
    let cart = get_cart();
    let items = cart.read().clone();
    Ok(items)
}

pub fn export_cart_as_shareable_json() -> Result<String> {
    let cart = get_cart();
    let cart = cart.read();

    Ok(serde_json::to_string_pretty(&*cart)?)
}

pub fn import_cart_from_json(json: &str) -> Result<()> {
    let items: Vec<CartItem> = serde_json::from_str(json)?;

    let cart = get_cart();
    let mut cart = cart.write();

    tracing::info!("📥 Importing {} items into cart", items.len());
    cart.extend(items);

    Ok(())
}
