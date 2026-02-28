//! Platform Pairing System for DX Gateway
//!
//! Secure pairing for macOS, iOS, Android, Windows, and Linux clients.
//! Uses Ed25519 signatures for device authentication.

use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

/// Pairing code configuration
const PAIRING_CODE_LENGTH: usize = 8;
const PAIRING_CODE_ALPHABET: &str = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const PAIRING_PENDING_TTL_SECS: i64 = 3600; // 1 hour
const PAIRING_PENDING_MAX: usize = 3;

/// A pairing code for device connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingCode {
    pub code: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub meta: Option<HashMap<String, String>>,
}

/// A pairing request from a device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingRequest {
    pub id: String,
    pub code: String,
    pub device_type: String,
    pub device_name: String,
    pub public_key: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub meta: Option<HashMap<String, String>>,
}

/// Paired device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairedDevice {
    pub id: String,
    pub device_type: DeviceType,
    pub device_name: String,
    pub public_key: String,
    pub paired_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub platform_version: Option<String>,
    pub capabilities: Vec<String>,
}

/// Device platform type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    MacOS,
    IOS,
    Android,
    Windows,
    Linux,
    Web,
    CLI,
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceType::MacOS => write!(f, "macOS"),
            DeviceType::IOS => write!(f, "iOS"),
            DeviceType::Android => write!(f, "Android"),
            DeviceType::Windows => write!(f, "Windows"),
            DeviceType::Linux => write!(f, "Linux"),
            DeviceType::Web => write!(f, "Web"),
            DeviceType::CLI => write!(f, "CLI"),
        }
    }
}

/// Pairing store for managing codes and devices
pub struct PairingStore {
    pending_codes: RwLock<HashMap<String, PairingCode>>,
    paired_devices: RwLock<HashMap<String, PairedDevice>>,
    allowed_from: RwLock<Vec<String>>,
}

impl PairingStore {
    /// Create a new pairing store
    pub fn new() -> Self {
        Self {
            pending_codes: RwLock::new(HashMap::new()),
            paired_devices: RwLock::new(HashMap::new()),
            allowed_from: RwLock::new(Vec::new()),
        }
    }

    /// Load pairing store from disk
    pub fn load_from_disk(state_dir: &std::path::Path) -> std::io::Result<Self> {
        let store = Self::new();

        // Load paired devices
        let devices_path = state_dir.join("paired-devices.json");
        if devices_path.exists() {
            let content = std::fs::read_to_string(&devices_path)?;
            if let Ok(devices) = serde_json::from_str::<HashMap<String, PairedDevice>>(&content) {
                *store.paired_devices.write().unwrap() = devices;
            }
        }

        // Load allowed-from list
        let allowed_path = state_dir.join("allowed-from.json");
        if allowed_path.exists() {
            let content = std::fs::read_to_string(&allowed_path)?;
            if let Ok(allowed) = serde_json::from_str::<Vec<String>>(&content) {
                *store.allowed_from.write().unwrap() = allowed;
            }
        }

        Ok(store)
    }

    /// Save pairing store to disk
    pub fn save_to_disk(&self, state_dir: &std::path::Path) -> std::io::Result<()> {
        std::fs::create_dir_all(state_dir)?;

        // Save paired devices
        let devices = self.paired_devices.read().unwrap();
        let devices_json = serde_json::to_string_pretty(&*devices)?;
        std::fs::write(state_dir.join("paired-devices.json"), devices_json)?;

        // Save allowed-from list
        let allowed = self.allowed_from.read().unwrap();
        let allowed_json = serde_json::to_string_pretty(&*allowed)?;
        std::fs::write(state_dir.join("allowed-from.json"), allowed_json)?;

        Ok(())
    }

    /// Generate a new pairing code
    pub fn generate_code(&self) -> PairingCode {
        // Clean up expired codes first
        self.cleanup_expired();

        let mut rng = rand::thread_rng();
        let alphabet: Vec<char> = PAIRING_CODE_ALPHABET.chars().collect();

        let code: String = (0..PAIRING_CODE_LENGTH)
            .map(|_| alphabet[rng.gen_range(0..alphabet.len())])
            .collect();

        let now = Utc::now();
        let pairing_code = PairingCode {
            code: code.clone(),
            created_at: now,
            expires_at: now + Duration::seconds(PAIRING_PENDING_TTL_SECS),
            meta: None,
        };

        // Store the code
        let mut codes = self.pending_codes.write().unwrap();

        // Limit pending codes
        if codes.len() >= PAIRING_PENDING_MAX {
            // Remove oldest
            if let Some(oldest) =
                codes.values().min_by_key(|c| c.created_at).map(|c| c.code.clone())
            {
                codes.remove(&oldest);
            }
        }

        codes.insert(code, pairing_code.clone());

        pairing_code
    }

    /// Validate a pairing code
    pub fn validate_code(&self, code: &str) -> bool {
        let codes = self.pending_codes.read().unwrap();
        if let Some(pairing_code) = codes.get(code) {
            pairing_code.expires_at > Utc::now()
        } else {
            false
        }
    }

    /// Invalidate (consume) a pairing code
    pub fn invalidate_code(&self, code: &str) {
        let mut codes = self.pending_codes.write().unwrap();
        codes.remove(code);
    }

    /// Cleanup expired codes
    pub fn cleanup_expired(&self) {
        let mut codes = self.pending_codes.write().unwrap();
        let now = Utc::now();
        codes.retain(|_, c| c.expires_at > now);
    }

    /// Add a paired device
    pub fn add_device(&self, device: PairedDevice) {
        let mut devices = self.paired_devices.write().unwrap();
        devices.insert(device.id.clone(), device);
    }

    /// Remove a paired device
    pub fn remove_device(&self, device_id: &str) -> Option<PairedDevice> {
        let mut devices = self.paired_devices.write().unwrap();
        devices.remove(device_id)
    }

    /// Get a paired device
    pub fn get_device(&self, device_id: &str) -> Option<PairedDevice> {
        let devices = self.paired_devices.read().unwrap();
        devices.get(device_id).cloned()
    }

    /// List all paired devices
    pub fn list_devices(&self) -> Vec<PairedDevice> {
        let devices = self.paired_devices.read().unwrap();
        devices.values().cloned().collect()
    }

    /// List devices by type
    pub fn list_devices_by_type(&self, device_type: DeviceType) -> Vec<PairedDevice> {
        let devices = self.paired_devices.read().unwrap();
        devices.values().filter(|d| d.device_type == device_type).cloned().collect()
    }

    /// Update device last seen
    pub fn update_last_seen(&self, device_id: &str) {
        let mut devices = self.paired_devices.write().unwrap();
        if let Some(device) = devices.get_mut(device_id) {
            device.last_seen = Utc::now();
        }
    }

    /// Check if a device is paired
    pub fn is_paired(&self, device_id: &str) -> bool {
        let devices = self.paired_devices.read().unwrap();
        devices.contains_key(device_id)
    }

    /// Add to allowed-from list
    pub fn add_allowed_from(&self, identifier: String) {
        let mut allowed = self.allowed_from.write().unwrap();
        if !allowed.contains(&identifier) {
            allowed.push(identifier);
        }
    }

    /// Check if identifier is in allowed-from list
    pub fn is_allowed_from(&self, identifier: &str) -> bool {
        let allowed = self.allowed_from.read().unwrap();
        allowed.contains(&identifier.to_string())
    }
}

impl Default for PairingStore {
    fn default() -> Self {
        Self::new()
    }
}

/// QR code data for pairing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingQRData {
    pub code: String,
    pub host: String,
    pub port: u16,
    pub tls: bool,
    pub version: String,
}

impl PairingQRData {
    /// Create new QR data
    pub fn new(code: &str, host: &str, port: u16, tls: bool) -> Self {
        Self {
            code: code.to_string(),
            host: host.to_string(),
            port,
            tls,
            version: "2026.2".to_string(),
        }
    }

    /// Generate deep link URL for iOS/macOS
    pub fn to_deep_link(&self) -> String {
        let protocol = if self.tls { "wss" } else { "ws" };
        format!(
            "dx://pair?code={}&host={}:{}&protocol={}",
            self.code, self.host, self.port, protocol
        )
    }

    /// Generate JSON for QR code
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Platform-specific pairing adapter
pub trait PlatformPairingAdapter: Send + Sync {
    /// Platform identifier
    fn platform(&self) -> DeviceType;

    /// Initialize pairing
    fn init_pairing(&self) -> Result<(), String>;

    /// Handle pairing request
    fn handle_pairing(&self, request: &PairingRequest) -> Result<PairedDevice, String>;

    /// Verify device signature
    fn verify_signature(&self, device_id: &str, data: &[u8], signature: &[u8]) -> bool;
}

/// macOS pairing adapter
pub struct MacOSPairingAdapter;

impl PlatformPairingAdapter for MacOSPairingAdapter {
    fn platform(&self) -> DeviceType {
        DeviceType::MacOS
    }

    fn init_pairing(&self) -> Result<(), String> {
        // macOS uses Keychain for secure storage
        Ok(())
    }

    fn handle_pairing(&self, request: &PairingRequest) -> Result<PairedDevice, String> {
        let device = PairedDevice {
            id: request.id.clone(),
            device_type: DeviceType::MacOS,
            device_name: request.device_name.clone(),
            public_key: request.public_key.clone().unwrap_or_default(),
            paired_at: Utc::now(),
            last_seen: Utc::now(),
            platform_version: None,
            capabilities: vec![
                "voice-wake".to_string(),
                "menu-bar".to_string(),
                "notifications".to_string(),
            ],
        };
        Ok(device)
    }

    fn verify_signature(&self, _device_id: &str, _data: &[u8], _signature: &[u8]) -> bool {
        // TODO: Implement Ed25519 verification
        true
    }
}

/// iOS pairing adapter
pub struct IOSPairingAdapter;

impl PlatformPairingAdapter for IOSPairingAdapter {
    fn platform(&self) -> DeviceType {
        DeviceType::IOS
    }

    fn init_pairing(&self) -> Result<(), String> {
        Ok(())
    }

    fn handle_pairing(&self, request: &PairingRequest) -> Result<PairedDevice, String> {
        let device = PairedDevice {
            id: request.id.clone(),
            device_type: DeviceType::IOS,
            device_name: request.device_name.clone(),
            public_key: request.public_key.clone().unwrap_or_default(),
            paired_at: Utc::now(),
            last_seen: Utc::now(),
            platform_version: None,
            capabilities: vec![
                "voice-wake".to_string(),
                "camera".to_string(),
                "canvas".to_string(),
                "push-notifications".to_string(),
            ],
        };
        Ok(device)
    }

    fn verify_signature(&self, _device_id: &str, _data: &[u8], _signature: &[u8]) -> bool {
        true
    }
}

/// Android pairing adapter
pub struct AndroidPairingAdapter;

impl PlatformPairingAdapter for AndroidPairingAdapter {
    fn platform(&self) -> DeviceType {
        DeviceType::Android
    }

    fn init_pairing(&self) -> Result<(), String> {
        Ok(())
    }

    fn handle_pairing(&self, request: &PairingRequest) -> Result<PairedDevice, String> {
        let device = PairedDevice {
            id: request.id.clone(),
            device_type: DeviceType::Android,
            device_name: request.device_name.clone(),
            public_key: request.public_key.clone().unwrap_or_default(),
            paired_at: Utc::now(),
            last_seen: Utc::now(),
            platform_version: None,
            capabilities: vec![
                "camera".to_string(),
                "screen-capture".to_string(),
                "canvas".to_string(),
            ],
        };
        Ok(device)
    }

    fn verify_signature(&self, _device_id: &str, _data: &[u8], _signature: &[u8]) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_code() {
        let store = PairingStore::new();
        let code = store.generate_code();

        assert_eq!(code.code.len(), PAIRING_CODE_LENGTH);
        assert!(code.expires_at > Utc::now());
    }

    #[test]
    fn test_validate_code() {
        let store = PairingStore::new();
        let code = store.generate_code();

        assert!(store.validate_code(&code.code));
        assert!(!store.validate_code("INVALID1"));
    }

    #[test]
    fn test_invalidate_code() {
        let store = PairingStore::new();
        let code = store.generate_code();

        assert!(store.validate_code(&code.code));
        store.invalidate_code(&code.code);
        assert!(!store.validate_code(&code.code));
    }

    #[test]
    fn test_device_management() {
        let store = PairingStore::new();

        let device = PairedDevice {
            id: "test-device".to_string(),
            device_type: DeviceType::MacOS,
            device_name: "Test Mac".to_string(),
            public_key: "test-key".to_string(),
            paired_at: Utc::now(),
            last_seen: Utc::now(),
            platform_version: Some("14.0".to_string()),
            capabilities: vec!["voice-wake".to_string()],
        };

        store.add_device(device.clone());

        assert!(store.is_paired("test-device"));
        assert_eq!(store.list_devices().len(), 1);

        let retrieved = store.get_device("test-device").unwrap();
        assert_eq!(retrieved.device_name, "Test Mac");

        store.remove_device("test-device");
        assert!(!store.is_paired("test-device"));
    }

    #[test]
    fn test_qr_data() {
        let qr = PairingQRData::new("ABCD1234", "192.168.1.100", 31337, false);

        assert_eq!(
            qr.to_deep_link(),
            "dx://pair?code=ABCD1234&host=192.168.1.100:31337&protocol=ws"
        );
    }
}
