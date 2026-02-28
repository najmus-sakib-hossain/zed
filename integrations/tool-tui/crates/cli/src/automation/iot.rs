//! IoT and smart home integration

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub state: DeviceState,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceType {
    Light,
    Switch,
    Thermostat,
    Lock,
    Camera,
    Sensor,
    Speaker,
    Display,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceState {
    pub online: bool,
    pub properties: HashMap<String, String>,
}

pub struct IoTHub {
    devices: HashMap<String, Device>,
}

impl IoTHub {
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
        }
    }

    pub fn register_device(&mut self, device: Device) {
        self.devices.insert(device.id.clone(), device);
    }

    pub fn get_device(&self, id: &str) -> Option<&Device> {
        self.devices.get(id)
    }

    pub fn list_devices(&self) -> Vec<&Device> {
        self.devices.values().collect()
    }

    pub async fn send_command(
        &self,
        device_id: &str,
        command: &str,
        params: HashMap<String, String>,
    ) -> Result<()> {
        println!("Sending command '{}' to device {}: {:?}", command, device_id, params);
        Ok(())
    }

    pub async fn turn_on(&self, device_id: &str) -> Result<()> {
        self.send_command(device_id, "turn_on", HashMap::new()).await
    }

    pub async fn turn_off(&self, device_id: &str) -> Result<()> {
        self.send_command(device_id, "turn_off", HashMap::new()).await
    }

    pub async fn set_brightness(&self, device_id: &str, level: u8) -> Result<()> {
        let mut params = HashMap::new();
        params.insert("brightness".to_string(), level.to_string());
        self.send_command(device_id, "set_brightness", params).await
    }

    pub async fn set_temperature(&self, device_id: &str, temp: f32) -> Result<()> {
        let mut params = HashMap::new();
        params.insert("temperature".to_string(), temp.to_string());
        self.send_command(device_id, "set_temperature", params).await
    }

    pub async fn lock(&self, device_id: &str) -> Result<()> {
        self.send_command(device_id, "lock", HashMap::new()).await
    }

    pub async fn unlock(&self, device_id: &str) -> Result<()> {
        self.send_command(device_id, "unlock", HashMap::new()).await
    }
}

impl Default for IoTHub {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iot_hub() {
        let mut hub = IoTHub::new();

        let device = Device {
            id: "light1".to_string(),
            name: "Living Room Light".to_string(),
            device_type: DeviceType::Light,
            state: DeviceState {
                online: true,
                properties: HashMap::new(),
            },
            capabilities: vec!["on_off".to_string(), "brightness".to_string()],
        };

        hub.register_device(device);
        assert!(hub.get_device("light1").is_some());
    }
}
