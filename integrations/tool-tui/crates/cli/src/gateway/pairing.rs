//! Device pairing with QR codes and one-time codes

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingCode {
    pub code: String,
    pub device_id: String,
    pub expires_at: i64,
    pub used: bool,
}

impl PairingCode {
    pub fn new(device_id: String) -> Self {
        let code = format!("{:06}", rand::random::<u32>() % 1_000_000);
        let expires_at = chrono::Utc::now().timestamp() + 300; // 5 minutes

        Self {
            code,
            device_id,
            expires_at,
            used: false,
        }
    }

    pub fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp() > self.expires_at
    }

    pub fn is_valid(&self) -> bool {
        !self.used && !self.is_expired()
    }
}

pub struct PairingManager {
    codes: Arc<RwLock<HashMap<String, PairingCode>>>,
    paired_devices: Arc<RwLock<HashMap<String, String>>>,
}

impl PairingManager {
    pub fn new() -> Self {
        Self {
            codes: Arc::new(RwLock::new(HashMap::new())),
            paired_devices: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn generate_code(&self, device_id: String) -> Result<PairingCode> {
        let code = PairingCode::new(device_id.clone());
        let mut codes = self.codes.write().await;
        codes.insert(code.code.clone(), code.clone());
        Ok(code)
    }

    pub async fn verify_code(&self, code: &str) -> Result<Option<String>> {
        let mut codes = self.codes.write().await;

        if let Some(pairing) = codes.get_mut(code) {
            if pairing.is_valid() {
                pairing.used = true;
                let device_id = pairing.device_id.clone();

                let mut devices = self.paired_devices.write().await;
                devices.insert(device_id.clone(), code.to_string());

                return Ok(Some(device_id));
            }
        }

        Ok(None)
    }

    pub async fn is_paired(&self, device_id: &str) -> bool {
        let devices = self.paired_devices.read().await;
        devices.contains_key(device_id)
    }

    pub async fn unpair(&self, device_id: &str) -> Result<()> {
        let mut devices = self.paired_devices.write().await;
        devices.remove(device_id);
        Ok(())
    }

    pub async fn cleanup_expired(&self) {
        let mut codes = self.codes.write().await;
        codes.retain(|_, code| !code.is_expired());
    }
}

impl Default for PairingManager {
    fn default() -> Self {
        Self::new()
    }
}
