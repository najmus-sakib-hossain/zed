//! # Smart Home Integration
//!
//! Smart home control via HomeKit, Home Assistant, and Philips Hue.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::smarthome::{SmartHomeClient, SmartHomeConfig};
//!
//! let config = SmartHomeConfig::from_file("~/.dx/config/smarthome.sr")?;
//! let client = SmartHomeClient::new(&config)?;
//!
//! // Turn on lights
//! client.set_light("Living Room", true).await?;
//!
//! // Set thermostat
//! client.set_thermostat(72.0).await?;
//! ```

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Smart home configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartHomeConfig {
    /// Whether smart home integration is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Home Assistant configuration
    pub home_assistant: Option<HomeAssistantConfig>,
    /// Philips Hue configuration
    pub hue: Option<HueConfig>,
    /// Provider preference order
    #[serde(default)]
    pub provider_order: Vec<SmartHomeProvider>,
}

fn default_true() -> bool {
    true
}

impl Default for SmartHomeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            home_assistant: None,
            hue: None,
            provider_order: vec![SmartHomeProvider::HomeAssistant, SmartHomeProvider::Hue],
        }
    }
}

impl SmartHomeConfig {
    /// Load from .sr config file
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| DrivenError::Io(e))?;
        Self::parse_sr(&content)
    }

    fn parse_sr(_content: &str) -> Result<Self> {
        Ok(Self::default())
    }
}

/// Home Assistant configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeAssistantConfig {
    /// Home Assistant URL
    pub url: String,
    /// Long-lived access token
    pub token: String,
}

impl HomeAssistantConfig {
    /// Resolve environment variables
    pub fn resolve_env_vars(&mut self) {
        if self.url.is_empty() || self.url.starts_with('$') {
            self.url = std::env::var("HASS_URL")
                .or_else(|_| std::env::var("HOME_ASSISTANT_URL"))
                .unwrap_or_default();
        }
        if self.token.is_empty() || self.token.starts_with('$') {
            self.token = std::env::var("HASS_TOKEN")
                .or_else(|_| std::env::var("HOME_ASSISTANT_TOKEN"))
                .unwrap_or_default();
        }
    }
}

/// Philips Hue configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HueConfig {
    /// Bridge IP address
    pub bridge_ip: String,
    /// API username/key
    pub username: String,
}

impl HueConfig {
    /// Resolve environment variables
    pub fn resolve_env_vars(&mut self) {
        if self.bridge_ip.is_empty() || self.bridge_ip.starts_with('$') {
            self.bridge_ip = std::env::var("HUE_BRIDGE_IP").unwrap_or_default();
        }
        if self.username.is_empty() || self.username.starts_with('$') {
            self.username = std::env::var("HUE_USERNAME")
                .or_else(|_| std::env::var("HUE_API_KEY"))
                .unwrap_or_default();
        }
    }
}

/// Smart home provider
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmartHomeProvider {
    HomeAssistant,
    Hue,
    HomeKit,
}

/// Device type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    Light,
    Switch,
    Thermostat,
    Lock,
    Sensor,
    Camera,
    Fan,
    Cover,
    Climate,
    MediaPlayer,
    Vacuum,
    Other,
}

/// Smart device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    /// Device ID
    pub id: String,
    /// Device name
    pub name: String,
    /// Device type
    pub device_type: DeviceType,
    /// Current state
    pub state: DeviceState,
    /// Room/area
    pub room: Option<String>,
    /// Provider
    pub provider: SmartHomeProvider,
    /// Additional attributes
    #[serde(default)]
    pub attributes: HashMap<String, serde_json::Value>,
}

/// Device state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceState {
    On,
    Off,
    Unavailable,
    Unknown,
    /// Numeric value (temperature, brightness, etc.)
    Value(f64),
    /// String state
    State(String),
}

impl DeviceState {
    pub fn is_on(&self) -> bool {
        matches!(self, DeviceState::On)
    }

    pub fn as_value(&self) -> Option<f64> {
        match self {
            DeviceState::Value(v) => Some(*v),
            _ => None,
        }
    }
}

/// Light state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightState {
    /// Is on
    pub on: bool,
    /// Brightness (0-255)
    pub brightness: Option<u8>,
    /// Color temperature in mireds
    pub color_temp: Option<u16>,
    /// RGB color
    pub rgb: Option<(u8, u8, u8)>,
    /// Hue (0-65535)
    pub hue: Option<u16>,
    /// Saturation (0-254)
    pub saturation: Option<u8>,
}

impl Default for LightState {
    fn default() -> Self {
        Self {
            on: false,
            brightness: None,
            color_temp: None,
            rgb: None,
            hue: None,
            saturation: None,
        }
    }
}

/// Thermostat state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermostatState {
    /// Current temperature
    pub current_temp: f64,
    /// Target temperature
    pub target_temp: f64,
    /// Mode (heat, cool, auto, off)
    pub mode: String,
    /// Current action (heating, cooling, idle)
    pub action: Option<String>,
    /// Humidity
    pub humidity: Option<f64>,
}

/// Room/Area
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    /// Room ID
    pub id: String,
    /// Room name
    pub name: String,
    /// Devices in room
    pub devices: Vec<String>,
}

/// Scene
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    /// Scene ID
    pub id: String,
    /// Scene name
    pub name: String,
    /// Provider
    pub provider: SmartHomeProvider,
}

/// Smart home client
pub struct SmartHomeClient {
    config: SmartHomeConfig,
}

impl SmartHomeClient {
    /// Create a new smart home client
    pub fn new(config: &SmartHomeConfig) -> Result<Self> {
        let mut config = config.clone();
        
        if let Some(ref mut ha) = config.home_assistant {
            ha.resolve_env_vars();
        }
        if let Some(ref mut hue) = config.hue {
            hue.resolve_env_vars();
        }

        Ok(Self { config })
    }

    /// Check if any provider is configured
    pub fn is_configured(&self) -> bool {
        self.config.enabled
            && (self.has_home_assistant() || self.has_hue())
    }

    fn has_home_assistant(&self) -> bool {
        self.config.home_assistant.as_ref()
            .map(|h| !h.url.is_empty() && !h.token.is_empty())
            .unwrap_or(false)
    }

    fn has_hue(&self) -> bool {
        self.config.hue.as_ref()
            .map(|h| !h.bridge_ip.is_empty() && !h.username.is_empty())
            .unwrap_or(false)
    }

    // Device operations

    /// List all devices
    pub async fn list_devices(&self) -> Result<Vec<Device>> {
        let mut devices = Vec::new();

        if self.has_home_assistant() {
            let ha_devices = self.ha_list_devices().await?;
            devices.extend(ha_devices);
        }

        if self.has_hue() {
            let hue_devices = self.hue_list_lights().await?;
            devices.extend(hue_devices);
        }

        Ok(devices)
    }

    /// Get device by name
    pub async fn get_device(&self, name: &str) -> Result<Device> {
        let devices = self.list_devices().await?;
        let name_lower = name.to_lowercase();

        devices
            .into_iter()
            .find(|d| d.name.to_lowercase().contains(&name_lower))
            .ok_or_else(|| DrivenError::NotFound(format!("Device '{}' not found", name)))
    }

    // Light operations

    /// Turn light on/off
    pub async fn set_light(&self, name: &str, on: bool) -> Result<()> {
        let device = self.get_device(name).await?;

        match device.provider {
            SmartHomeProvider::HomeAssistant => {
                self.ha_turn_on_off(&device.id, on).await
            }
            SmartHomeProvider::Hue => {
                self.hue_set_light(&device.id, on, None).await
            }
            _ => Err(DrivenError::Unsupported("Provider not supported".into())),
        }
    }

    /// Set light brightness (0-100)
    pub async fn set_brightness(&self, name: &str, brightness: u8) -> Result<()> {
        let device = self.get_device(name).await?;
        let brightness = brightness.min(100);

        match device.provider {
            SmartHomeProvider::HomeAssistant => {
                self.ha_set_brightness(&device.id, brightness).await
            }
            SmartHomeProvider::Hue => {
                let bri = (brightness as u16 * 254 / 100) as u8;
                self.hue_set_light(&device.id, true, Some(bri)).await
            }
            _ => Err(DrivenError::Unsupported("Provider not supported".into())),
        }
    }

    /// Set light color (RGB)
    pub async fn set_color(&self, name: &str, r: u8, g: u8, b: u8) -> Result<()> {
        let device = self.get_device(name).await?;

        match device.provider {
            SmartHomeProvider::HomeAssistant => {
                self.ha_set_color(&device.id, r, g, b).await
            }
            SmartHomeProvider::Hue => {
                self.hue_set_color(&device.id, r, g, b).await
            }
            _ => Err(DrivenError::Unsupported("Provider not supported".into())),
        }
    }

    // Thermostat operations

    /// Set thermostat temperature
    pub async fn set_thermostat(&self, temperature: f64) -> Result<()> {
        if self.has_home_assistant() {
            self.ha_set_temperature(temperature).await
        } else {
            Err(DrivenError::Unsupported("No thermostat provider".into()))
        }
    }

    /// Get thermostat state
    pub async fn get_thermostat(&self) -> Result<ThermostatState> {
        if self.has_home_assistant() {
            self.ha_get_thermostat().await
        } else {
            Err(DrivenError::Unsupported("No thermostat provider".into()))
        }
    }

    // Scene operations

    /// List scenes
    pub async fn list_scenes(&self) -> Result<Vec<Scene>> {
        let mut scenes = Vec::new();

        if self.has_home_assistant() {
            let ha_scenes = self.ha_list_scenes().await?;
            scenes.extend(ha_scenes);
        }

        if self.has_hue() {
            let hue_scenes = self.hue_list_scenes().await?;
            scenes.extend(hue_scenes);
        }

        Ok(scenes)
    }

    /// Activate a scene
    pub async fn activate_scene(&self, name: &str) -> Result<()> {
        let scenes = self.list_scenes().await?;
        let name_lower = name.to_lowercase();

        let scene = scenes
            .into_iter()
            .find(|s| s.name.to_lowercase().contains(&name_lower))
            .ok_or_else(|| DrivenError::NotFound(format!("Scene '{}' not found", name)))?;

        match scene.provider {
            SmartHomeProvider::HomeAssistant => {
                self.ha_activate_scene(&scene.id).await
            }
            SmartHomeProvider::Hue => {
                self.hue_activate_scene(&scene.id).await
            }
            _ => Err(DrivenError::Unsupported("Provider not supported".into())),
        }
    }

    // Home Assistant API

    async fn ha_list_devices(&self) -> Result<Vec<Device>> {
        let ha = self.config.home_assistant.as_ref()
            .ok_or_else(|| DrivenError::Config("Home Assistant not configured".into()))?;

        let url = format!("{}/api/states", ha.url.trim_end_matches('/'));
        let response: Vec<serde_json::Value> = self.ha_get(&url).await?;

        let devices = response
            .into_iter()
            .filter_map(|state| {
                let entity_id = state["entity_id"].as_str()?;
                let parts: Vec<&str> = entity_id.split('.').collect();
                if parts.len() != 2 {
                    return None;
                }

                let device_type = match parts[0] {
                    "light" => DeviceType::Light,
                    "switch" => DeviceType::Switch,
                    "climate" => DeviceType::Thermostat,
                    "lock" => DeviceType::Lock,
                    "sensor" => DeviceType::Sensor,
                    "camera" => DeviceType::Camera,
                    "fan" => DeviceType::Fan,
                    "cover" => DeviceType::Cover,
                    "media_player" => DeviceType::MediaPlayer,
                    "vacuum" => DeviceType::Vacuum,
                    _ => return None,
                };

                let state_val = match state["state"].as_str()? {
                    "on" => DeviceState::On,
                    "off" => DeviceState::Off,
                    "unavailable" => DeviceState::Unavailable,
                    s => DeviceState::State(s.to_string()),
                };

                Some(Device {
                    id: entity_id.to_string(),
                    name: state["attributes"]["friendly_name"]
                        .as_str()
                        .unwrap_or(parts[1])
                        .to_string(),
                    device_type,
                    state: state_val,
                    room: state["attributes"]["area"].as_str().map(String::from),
                    provider: SmartHomeProvider::HomeAssistant,
                    attributes: serde_json::from_value(state["attributes"].clone())
                        .unwrap_or_default(),
                })
            })
            .collect();

        Ok(devices)
    }

    async fn ha_turn_on_off(&self, entity_id: &str, on: bool) -> Result<()> {
        let ha = self.config.home_assistant.as_ref()
            .ok_or_else(|| DrivenError::Config("Home Assistant not configured".into()))?;

        let domain = entity_id.split('.').next().unwrap_or("light");
        let service = if on { "turn_on" } else { "turn_off" };
        let url = format!("{}/api/services/{}/{}", ha.url.trim_end_matches('/'), domain, service);

        let body = serde_json::json!({ "entity_id": entity_id });
        self.ha_post(&url, body).await?;
        Ok(())
    }

    async fn ha_set_brightness(&self, entity_id: &str, brightness: u8) -> Result<()> {
        let ha = self.config.home_assistant.as_ref()
            .ok_or_else(|| DrivenError::Config("Home Assistant not configured".into()))?;

        let url = format!("{}/api/services/light/turn_on", ha.url.trim_end_matches('/'));
        let body = serde_json::json!({
            "entity_id": entity_id,
            "brightness_pct": brightness
        });

        self.ha_post(&url, body).await?;
        Ok(())
    }

    async fn ha_set_color(&self, entity_id: &str, r: u8, g: u8, b: u8) -> Result<()> {
        let ha = self.config.home_assistant.as_ref()
            .ok_or_else(|| DrivenError::Config("Home Assistant not configured".into()))?;

        let url = format!("{}/api/services/light/turn_on", ha.url.trim_end_matches('/'));
        let body = serde_json::json!({
            "entity_id": entity_id,
            "rgb_color": [r, g, b]
        });

        self.ha_post(&url, body).await?;
        Ok(())
    }

    async fn ha_set_temperature(&self, temperature: f64) -> Result<()> {
        let ha = self.config.home_assistant.as_ref()
            .ok_or_else(|| DrivenError::Config("Home Assistant not configured".into()))?;

        // Find first climate entity
        let devices = self.ha_list_devices().await?;
        let thermostat = devices
            .iter()
            .find(|d| d.device_type == DeviceType::Thermostat)
            .ok_or_else(|| DrivenError::NotFound("No thermostat found".into()))?;

        let url = format!("{}/api/services/climate/set_temperature", ha.url.trim_end_matches('/'));
        let body = serde_json::json!({
            "entity_id": thermostat.id,
            "temperature": temperature
        });

        self.ha_post(&url, body).await?;
        Ok(())
    }

    async fn ha_get_thermostat(&self) -> Result<ThermostatState> {
        let devices = self.ha_list_devices().await?;
        let thermostat = devices
            .iter()
            .find(|d| d.device_type == DeviceType::Thermostat)
            .ok_or_else(|| DrivenError::NotFound("No thermostat found".into()))?;

        Ok(ThermostatState {
            current_temp: thermostat.attributes
                .get("current_temperature")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            target_temp: thermostat.attributes
                .get("temperature")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            mode: thermostat.attributes
                .get("hvac_mode")
                .and_then(|v| v.as_str())
                .unwrap_or("off")
                .to_string(),
            action: thermostat.attributes
                .get("hvac_action")
                .and_then(|v| v.as_str())
                .map(String::from),
            humidity: thermostat.attributes
                .get("current_humidity")
                .and_then(|v| v.as_f64()),
        })
    }

    async fn ha_list_scenes(&self) -> Result<Vec<Scene>> {
        let ha = self.config.home_assistant.as_ref()
            .ok_or_else(|| DrivenError::Config("Home Assistant not configured".into()))?;

        let url = format!("{}/api/states", ha.url.trim_end_matches('/'));
        let response: Vec<serde_json::Value> = self.ha_get(&url).await?;

        let scenes = response
            .into_iter()
            .filter_map(|state| {
                let entity_id = state["entity_id"].as_str()?;
                if !entity_id.starts_with("scene.") {
                    return None;
                }

                Some(Scene {
                    id: entity_id.to_string(),
                    name: state["attributes"]["friendly_name"]
                        .as_str()
                        .unwrap_or(entity_id)
                        .to_string(),
                    provider: SmartHomeProvider::HomeAssistant,
                })
            })
            .collect();

        Ok(scenes)
    }

    async fn ha_activate_scene(&self, scene_id: &str) -> Result<()> {
        let ha = self.config.home_assistant.as_ref()
            .ok_or_else(|| DrivenError::Config("Home Assistant not configured".into()))?;

        let url = format!("{}/api/services/scene/turn_on", ha.url.trim_end_matches('/'));
        let body = serde_json::json!({ "entity_id": scene_id });

        self.ha_post(&url, body).await?;
        Ok(())
    }

    async fn ha_get<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        let ha = self.config.home_assistant.as_ref()
            .ok_or_else(|| DrivenError::Config("Home Assistant not configured".into()))?;

        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header("Authorization", format!("Bearer {}", ha.token))
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!("Home Assistant error: {}", error)));
        }

        response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))
    }

    async fn ha_post(&self, url: &str, body: serde_json::Value) -> Result<serde_json::Value> {
        let ha = self.config.home_assistant.as_ref()
            .ok_or_else(|| DrivenError::Config("Home Assistant not configured".into()))?;

        let client = reqwest::Client::new();
        let response = client
            .post(url)
            .header("Authorization", format!("Bearer {}", ha.token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!("Home Assistant error: {}", error)));
        }

        response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))
    }

    // Philips Hue API

    async fn hue_list_lights(&self) -> Result<Vec<Device>> {
        let hue = self.config.hue.as_ref()
            .ok_or_else(|| DrivenError::Config("Hue not configured".into()))?;

        let url = format!("http://{}/api/{}/lights", hue.bridge_ip, hue.username);
        let response: HashMap<String, serde_json::Value> = self.hue_get(&url).await?;

        let devices = response
            .into_iter()
            .map(|(id, light)| {
                let state = &light["state"];
                let on = state["on"].as_bool().unwrap_or(false);

                Device {
                    id,
                    name: light["name"].as_str().unwrap_or("Light").to_string(),
                    device_type: DeviceType::Light,
                    state: if on { DeviceState::On } else { DeviceState::Off },
                    room: None,
                    provider: SmartHomeProvider::Hue,
                    attributes: serde_json::from_value(state.clone()).unwrap_or_default(),
                }
            })
            .collect();

        Ok(devices)
    }

    async fn hue_set_light(&self, light_id: &str, on: bool, brightness: Option<u8>) -> Result<()> {
        let hue = self.config.hue.as_ref()
            .ok_or_else(|| DrivenError::Config("Hue not configured".into()))?;

        let url = format!("http://{}/api/{}/lights/{}/state", hue.bridge_ip, hue.username, light_id);
        
        let mut body = serde_json::json!({ "on": on });
        if let Some(bri) = brightness {
            body["bri"] = serde_json::json!(bri);
        }

        self.hue_put(&url, body).await?;
        Ok(())
    }

    async fn hue_set_color(&self, light_id: &str, r: u8, g: u8, b: u8) -> Result<()> {
        let hue = self.config.hue.as_ref()
            .ok_or_else(|| DrivenError::Config("Hue not configured".into()))?;

        // Convert RGB to XY (simplified)
        let (x, y) = rgb_to_xy(r, g, b);

        let url = format!("http://{}/api/{}/lights/{}/state", hue.bridge_ip, hue.username, light_id);
        let body = serde_json::json!({
            "on": true,
            "xy": [x, y]
        });

        self.hue_put(&url, body).await?;
        Ok(())
    }

    async fn hue_list_scenes(&self) -> Result<Vec<Scene>> {
        let hue = self.config.hue.as_ref()
            .ok_or_else(|| DrivenError::Config("Hue not configured".into()))?;

        let url = format!("http://{}/api/{}/scenes", hue.bridge_ip, hue.username);
        let response: HashMap<String, serde_json::Value> = self.hue_get(&url).await?;

        let scenes = response
            .into_iter()
            .map(|(id, scene)| Scene {
                id,
                name: scene["name"].as_str().unwrap_or("Scene").to_string(),
                provider: SmartHomeProvider::Hue,
            })
            .collect();

        Ok(scenes)
    }

    async fn hue_activate_scene(&self, scene_id: &str) -> Result<()> {
        let hue = self.config.hue.as_ref()
            .ok_or_else(|| DrivenError::Config("Hue not configured".into()))?;

        let url = format!("http://{}/api/{}/groups/0/action", hue.bridge_ip, hue.username);
        let body = serde_json::json!({ "scene": scene_id });

        self.hue_put(&url, body).await?;
        Ok(())
    }

    async fn hue_get<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!("Hue error: {}", error)));
        }

        response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))
    }

    async fn hue_put(&self, url: &str, body: serde_json::Value) -> Result<serde_json::Value> {
        let client = reqwest::Client::new();
        let response = client
            .put(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!("Hue error: {}", error)));
        }

        response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))
    }
}

/// Convert RGB to CIE XY color space (simplified)
fn rgb_to_xy(r: u8, g: u8, b: u8) -> (f64, f64) {
    let r = r as f64 / 255.0;
    let g = g as f64 / 255.0;
    let b = b as f64 / 255.0;

    // Apply gamma correction
    let r = if r > 0.04045 { ((r + 0.055) / 1.055).powf(2.4) } else { r / 12.92 };
    let g = if g > 0.04045 { ((g + 0.055) / 1.055).powf(2.4) } else { g / 12.92 };
    let b = if b > 0.04045 { ((b + 0.055) / 1.055).powf(2.4) } else { b / 12.92 };

    // Convert to XYZ
    let x = r * 0.649926 + g * 0.103455 + b * 0.197109;
    let y = r * 0.234327 + g * 0.743075 + b * 0.022598;
    let z = r * 0.0 + g * 0.053077 + b * 1.035763;

    // Convert to xy
    let sum = x + y + z;
    if sum == 0.0 {
        (0.0, 0.0)
    } else {
        (x / sum, y / sum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SmartHomeConfig::default();
        assert!(config.enabled);
    }

    #[test]
    fn test_rgb_to_xy() {
        let (x, y) = rgb_to_xy(255, 0, 0); // Red
        assert!(x > 0.6); // Red has high x
        assert!(y < 0.4); // Red has low y
    }

    #[test]
    fn test_device_state() {
        assert!(DeviceState::On.is_on());
        assert!(!DeviceState::Off.is_on());
        assert_eq!(DeviceState::Value(72.0).as_value(), Some(72.0));
    }
}
