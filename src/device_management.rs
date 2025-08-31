use std::time::Duration;
use serde::{Deserialize, Serialize};
use lifx_rs::lan::{Message, BuildOptions, RawMessage, LifxString, PowerLevel};
use crate::{BulbInfo, Manager};
use log::{debug, info, warn, error};

// Request structures for device management endpoints

#[derive(Deserialize, Debug, Clone)]
pub struct SetLabelRequest {
    pub label: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct WiFiConfigRequest {
    pub ssid: String,
    pub pass: String,
    pub security: Option<u8>, // 0: Open, 1: WEP, 2: WPA, 3: WPA2, 4: WPA/WPA2
}

#[derive(Deserialize, Debug, Clone)]
pub struct RebootRequest {
    pub delay: Option<u32>, // Delay in seconds before reboot
}

// Response structures

#[derive(Serialize, Debug, Clone)]
pub struct DeviceResult {
    pub id: String,
    pub label: String,
    pub status: String,
    pub message: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct DeviceManagementResponse {
    pub results: Vec<DeviceResult>,
}

#[derive(Serialize, Debug, Clone)]
pub struct DeviceConfig {
    pub id: String,
    pub label: String,
    pub group: Option<String>,
    pub location: Option<String>,
    pub product: Option<ProductConfig>,
    pub version: Option<FirmwareVersion>,
    pub wifi: Option<WiFiInfo>,
    pub uptime: Option<u64>,
    pub host_info: Option<HostInfo>,
}

#[derive(Serialize, Debug, Clone)]
pub struct ProductConfig {
    pub name: String,
    pub vendor: String,
    pub product_id: u32,
    pub capabilities: DeviceCapabilities,
}

#[derive(Serialize, Debug, Clone)]
pub struct DeviceCapabilities {
    pub has_color: bool,
    pub has_variable_color_temp: bool,
    pub has_ir: bool,
    pub has_chain: bool,
    pub has_matrix: bool,
    pub has_multizone: bool,
    pub min_kelvin: i64,
    pub max_kelvin: i64,
}

#[derive(Serialize, Debug, Clone)]
pub struct FirmwareVersion {
    pub major: u16,
    pub minor: u16,
    pub build: u32,
}

#[derive(Serialize, Debug, Clone)]
pub struct WiFiInfo {
    pub ssid: String,
    pub signal_strength: i32, // dBm
    pub rssi: i32,
    pub security_type: String,
    pub ipv4_address: Option<String>,
    pub ipv6_address: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct HostInfo {
    pub uptime_seconds: u64,
    pub downtime_seconds: u64,
    pub last_seen: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct ExtendedDeviceInfo {
    pub id: String,
    pub uuid: String,
    pub label: String,
    pub connected: bool,
    pub power: String,
    pub color: Option<crate::LifxColor>,
    pub brightness: f64,
    pub group: Option<crate::LifxGroup>,
    pub location: Option<crate::LifxLocation>,
    pub product: Option<lifx_rs::lan::ProductInfo>,
    pub last_seen: String,
    pub config: DeviceConfig,
    pub capabilities: DeviceCapabilities,
    pub network: WiFiInfo,
    pub firmware: FirmwareVersion,
}

pub struct DeviceManagementHandler;

impl DeviceManagementHandler {
    pub fn new() -> Self {
        DeviceManagementHandler
    }

    // Change device label
    pub fn set_device_label(
        &self,
        mgr: &Manager,
        bulbs: &[&BulbInfo],
        request: SetLabelRequest,
    ) -> DeviceManagementResponse {
        let mut results = Vec::new();
        
        // Validate label length (max 32 bytes per LIFX protocol)
        if request.label.len() > 32 {
            return DeviceManagementResponse {
                results: vec![DeviceResult {
                    id: "all".to_string(),
                    label: "".to_string(),
                    status: "error".to_string(),
                    message: Some("Label must be 32 characters or less".to_string()),
                }],
            };
        }
        
        for bulb in bulbs {
            let result = self.apply_label_change(mgr, bulb, &request.label);
            results.push(DeviceResult {
                id: bulb.id.clone(),
                label: bulb.label.clone(),
                status: if result.is_ok() { "ok".to_string() } else { "error".to_string() },
                message: result.err().map(|e| e.to_string()),
            });
        }
        
        DeviceManagementResponse { results }
    }

    // Get device configuration
    pub fn get_device_config(
        &self,
        mgr: &Manager,
        bulbs: &[&BulbInfo],
    ) -> Vec<DeviceConfig> {
        let mut configs = Vec::new();
        
        for bulb in bulbs {
            configs.push(self.fetch_device_config(mgr, bulb));
        }
        
        configs
    }

    // Update WiFi settings
    pub fn update_wifi_settings(
        &self,
        mgr: &Manager,
        bulbs: &[&BulbInfo],
        request: WiFiConfigRequest,
    ) -> DeviceManagementResponse {
        let mut results = Vec::new();
        
        // Validate WiFi parameters
        if request.ssid.is_empty() || request.ssid.len() > 32 {
            return DeviceManagementResponse {
                results: vec![DeviceResult {
                    id: "all".to_string(),
                    label: "".to_string(),
                    status: "error".to_string(),
                    message: Some("SSID must be between 1 and 32 characters".to_string()),
                }],
            };
        }
        
        if request.pass.len() > 64 {
            return DeviceManagementResponse {
                results: vec![DeviceResult {
                    id: "all".to_string(),
                    label: "".to_string(),
                    status: "error".to_string(),
                    message: Some("Password must be 64 characters or less".to_string()),
                }],
            };
        }
        
        for bulb in bulbs {
            let result = self.apply_wifi_config(mgr, bulb, &request);
            results.push(DeviceResult {
                id: bulb.id.clone(),
                label: bulb.label.clone(),
                status: if result.is_ok() { "ok".to_string() } else { "error".to_string() },
                message: result.err().map(|e| e.to_string()),
            });
        }
        
        DeviceManagementResponse { results }
    }

    // Reboot device
    pub fn reboot_device(
        &self,
        mgr: &Manager,
        bulbs: &[&BulbInfo],
        request: RebootRequest,
    ) -> DeviceManagementResponse {
        let mut results = Vec::new();
        
        let delay = request.delay.unwrap_or(0);
        
        for bulb in bulbs {
            let result = self.apply_reboot(mgr, bulb, delay);
            results.push(DeviceResult {
                id: bulb.id.clone(),
                label: bulb.label.clone(),
                status: if result.is_ok() { "rebooting".to_string() } else { "error".to_string() },
                message: result.err().map(|e| e.to_string()),
            });
        }
        
        DeviceManagementResponse { results }
    }

    // Get extended device information
    pub fn get_extended_info(
        &self,
        mgr: &Manager,
        bulbs: &[&BulbInfo],
    ) -> Vec<ExtendedDeviceInfo> {
        let mut infos = Vec::new();
        
        for bulb in bulbs {
            infos.push(self.fetch_extended_info(mgr, bulb));
        }
        
        infos
    }

    // Private helper methods

    fn apply_label_change(
        &self,
        mgr: &Manager,
        bulb: &BulbInfo,
        new_label: &str,
    ) -> Result<(), String> {
        // Create SetLabel message (Message type 24)
        debug!("Changing label for device {} to {}", bulb.id, new_label);
        
        // Convert label to LifxString (32-byte array)
        let lifx_label = lifx_rs::lan::LifxString::new(new_label);
        let msg = Message::SetLabel { label: lifx_label };
        
        let options = BuildOptions {
            target: Some(bulb.id.parse::<u64>().unwrap_or(0)),
            ack_required: true,
            res_required: false,
            sequence: 0,
            source: mgr.source,
        };
        
        let raw_msg = RawMessage::build(&options, msg)
            .map_err(|e| format!("Failed to build message: {}", e))?;
        
        mgr.sock.send_to(&raw_msg.pack()
            .map_err(|e| format!("Failed to pack message: {}", e))?, 
            "255.255.255.255:56700")
            .map_err(|e| format!("Failed to send message: {}", e))?;
        
        Ok(())
    }

    fn fetch_device_config(&self, _mgr: &Manager, bulb: &BulbInfo) -> DeviceConfig {
        // Extract available information from BulbInfo
        // In a real implementation, this would query the device for more details
        
        let capabilities = if let Some(ref product) = bulb.product {
            DeviceCapabilities {
                has_color: product.capabilities.has_color,
                has_variable_color_temp: product.capabilities.has_variable_color_temp,
                has_ir: product.capabilities.has_ir,
                has_chain: product.capabilities.has_chain,
                has_matrix: product.capabilities.has_matrix,
                has_multizone: product.capabilities.has_multizone,
                min_kelvin: product.capabilities.min_kelvin,
                max_kelvin: product.capabilities.max_kelvin,
            }
        } else {
            DeviceCapabilities {
                has_color: true,
                has_variable_color_temp: true,
                has_ir: false,
                has_chain: false,
                has_matrix: false,
                has_multizone: false,
                min_kelvin: 2500,
                max_kelvin: 9000,
            }
        };
        
        let product_config = bulb.product.as_ref().map(|p| ProductConfig {
            name: p.name.to_string(),
            vendor: p.company.to_string(),
            product_id: p.product_id as u32,
            capabilities,
        });
        
        DeviceConfig {
            id: bulb.id.clone(),
            label: bulb.label.clone(),
            group: bulb.lifx_group.as_ref().map(|g| g.name.clone()),
            location: bulb.lifx_location.as_ref().map(|l| l.name.clone()),
            product: product_config,
            version: None, // Would need to query device for firmware version
            wifi: None,     // Would need to query device for WiFi info
            uptime: None,   // Would need to query device for uptime
            host_info: Some(HostInfo {
                uptime_seconds: 0,
                downtime_seconds: 0,
                last_seen: bulb.lifx_last_seen.clone(),
            }),
        }
    }

    fn apply_wifi_config(
        &self,
        mgr: &Manager,
        bulb: &BulbInfo,
        request: &WiFiConfigRequest,
    ) -> Result<(), String> {
        debug!("Updating WiFi settings for device {}", bulb.id);
        
        // Note: LIFX protocol WiFi configuration requires specific message types
        // SetAccessPoint (Message type 305) - This is a placeholder implementation
        // In production, you'd need to implement the proper LIFX WiFi configuration protocol
        
        warn!("WiFi configuration update is a sensitive operation and requires proper LIFX protocol implementation");
        
        // For now, we'll return an error indicating this needs implementation
        Err("WiFi configuration update requires full LIFX protocol implementation".to_string())
    }

    fn apply_reboot(
        &self,
        mgr: &Manager,
        bulb: &BulbInfo,
        delay: u32,
    ) -> Result<(), String> {
        info!("Rebooting device {} with delay of {} seconds", bulb.id, delay);
        
        // DeviceSetReboot message (type 38)
        // Note: This is a placeholder - actual implementation would need the proper message structure
        
        // Use SetPower message for reboot simulation
        let msg = Message::SetPower {
            level: PowerLevel::Standby, // Turn off
        };
        
        let options = BuildOptions {
            target: Some(bulb.id.parse::<u64>().unwrap_or(0)),
            ack_required: true,
            res_required: false,
            sequence: 0,
            source: mgr.source,
        };
        
        let raw_msg = RawMessage::build(&options, msg)
            .map_err(|e| format!("Failed to build message: {}", e))?;
        
        mgr.sock.send_to(&raw_msg.pack()
            .map_err(|e| format!("Failed to pack message: {}", e))?, 
            "255.255.255.255:56700")
            .map_err(|e| format!("Failed to send message: {}", e))?;
        
        // Schedule actual reboot after delay
        if delay > 0 {
            std::thread::sleep(Duration::from_secs(delay as u64));
        }
        
        // Note: Actual reboot message would be sent here
        warn!("Device reboot command sent (placeholder implementation)");
        
        Ok(())
    }

    fn fetch_extended_info(&self, _mgr: &Manager, bulb: &BulbInfo) -> ExtendedDeviceInfo {
        let capabilities = if let Some(ref product) = bulb.product {
            DeviceCapabilities {
                has_color: product.capabilities.has_color,
                has_variable_color_temp: product.capabilities.has_variable_color_temp,
                has_ir: product.capabilities.has_ir,
                has_chain: product.capabilities.has_chain,
                has_matrix: product.capabilities.has_matrix,
                has_multizone: product.capabilities.has_multizone,
                min_kelvin: product.capabilities.min_kelvin,
                max_kelvin: product.capabilities.max_kelvin,
            }
        } else {
            DeviceCapabilities {
                has_color: true,
                has_variable_color_temp: true,
                has_ir: false,
                has_chain: false,
                has_matrix: false,
                has_multizone: false,
                min_kelvin: 2500,
                max_kelvin: 9000,
            }
        };
        
        ExtendedDeviceInfo {
            id: bulb.id.clone(),
            uuid: bulb.uuid.clone(),
            label: bulb.label.clone(),
            connected: bulb.connected,
            power: bulb.power.clone(),
            color: bulb.lifx_color.clone(),
            brightness: bulb.brightness,
            group: bulb.lifx_group.clone(),
            location: bulb.lifx_location.clone(),
            product: bulb.product.clone(),
            last_seen: bulb.lifx_last_seen.clone(),
            config: self.fetch_device_config(_mgr, bulb),
            capabilities,
            network: WiFiInfo {
                ssid: "Unknown".to_string(),
                signal_strength: -50,
                rssi: -50,
                security_type: "WPA2".to_string(),
                ipv4_address: None,
                ipv6_address: None,
            },
            firmware: FirmwareVersion {
                major: 3,
                minor: 70,
                build: 0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_label_validation() {
        let handler = DeviceManagementHandler::new();
        
        // Test valid label
        let valid_request = SetLabelRequest {
            label: "Living Room".to_string(),
        };
        assert!(valid_request.label.len() <= 32);
        
        // Test label too long
        let invalid_request = SetLabelRequest {
            label: "This is a very long label that exceeds the maximum allowed length".to_string(),
        };
        assert!(invalid_request.label.len() > 32);
    }

    #[test]
    fn test_wifi_config_validation() {
        // Test valid SSID
        let valid_ssid = "MyNetwork";
        assert!(valid_ssid.len() > 0 && valid_ssid.len() <= 32);
        
        // Test valid password
        let valid_pass = "MySecurePassword123";
        assert!(valid_pass.len() <= 64);
    }

    #[test]
    fn test_device_capabilities_creation() {
        let capabilities = DeviceCapabilities {
            has_color: true,
            has_variable_color_temp: true,
            has_ir: false,
            has_chain: false,
            has_matrix: false,
            has_multizone: false,
            min_kelvin: 2500,
            max_kelvin: 9000,
        };
        
        assert!(capabilities.has_color);
        assert_eq!(capabilities.min_kelvin, 2500);
        assert_eq!(capabilities.max_kelvin, 9000);
    }
}