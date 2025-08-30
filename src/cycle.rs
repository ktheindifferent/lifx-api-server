use serde::{Deserialize, Serialize};
use lifx_rs::lan::{Waveform, HSBK, Message, BuildOptions, RawMessage};
use crate::{BulbInfo, Manager};

#[derive(Deserialize, Debug, Clone)]
pub struct CycleRequest {
    pub states: Vec<CycleState>,
    pub defaults: Option<CycleDefaults>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CycleState {
    pub color: Option<String>,
    pub brightness: Option<f64>,
    pub duration: Option<f64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CycleDefaults {
    pub power: Option<String>,
    pub saturation: Option<f64>,
    pub brightness: Option<f64>,
    pub duration: Option<f64>,
}

#[derive(Serialize, Debug)]
pub struct CycleResult {
    pub id: String,
    pub label: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct CycleResponse {
    pub results: Vec<CycleResult>,
}

pub struct CycleHandler;

impl CycleHandler {
    pub fn new() -> Self {
        CycleHandler
    }

    pub fn handle_cycle(
        &self,
        mgr: &Manager,
        bulbs: &[&BulbInfo],
        request: CycleRequest,
    ) -> CycleResponse {
        let mut results = Vec::new();
        
        for bulb in bulbs {
            let result = self.apply_cycle(mgr, bulb, &request);
            results.push(CycleResult {
                id: bulb.id.clone(),
                label: bulb.label.clone(),
                status: if result.is_ok() { "ok".to_string() } else { "error".to_string() },
            });
        }
        
        CycleResponse { results }
    }

    fn apply_cycle(
        &self,
        mgr: &Manager,
        bulb: &BulbInfo,
        request: &CycleRequest,
    ) -> Result<(), String> {
        if request.states.is_empty() {
            return Err("Cycle states cannot be empty".to_string());
        }
        
        let defaults = request.defaults.as_ref();
        let default_duration = defaults.and_then(|d| d.duration).unwrap_or(1.0);
        let default_brightness = defaults.and_then(|d| d.brightness);
        
        if let Some(ref defaults) = request.defaults {
            if let Some(ref power) = defaults.power {
                let power_level = if power == "on" {
                    lifx_rs::lan::PowerLevel::Enabled
                } else {
                    lifx_rs::lan::PowerLevel::Standby
                };
                
                bulb.set_power(&mgr.sock, power_level)
                    .map_err(|e| format!("Failed to set power: {:?}", e))?;
            }
        }
        
        let total_duration: f64 = request.states.iter()
            .map(|s| s.duration.unwrap_or(default_duration))
            .sum();
        
        let period = (total_duration * 1000.0) as u32;
        let cycles = 1.0;
        
        let current = bulb.lifx_color.as_ref();
        let first_state = &request.states[0];
        let target_color = self.parse_cycle_state(first_state, current, default_brightness)?;
        
        let options = BuildOptions {
            target: Some(bulb.target),
            res_required: true,
            source: bulb.source,
            ..Default::default()
        };
        
        let message = Message::SetWaveform {
            reserved: 0,
            transient: false,
            color: target_color,
            period,
            cycles,
            skew_ratio: 0,
            waveform: Waveform::Triangle,
        };
        
        let raw_message = RawMessage::build(&options, message)
            .map_err(|e| format!("Failed to build message: {:?}", e))?;
        
        mgr.sock.send_to(&raw_message.pack().map_err(|e| format!("Failed to pack message: {:?}", e))?, bulb.addr)
            .map_err(|e| format!("Failed to send message: {:?}", e))?;
        
        Ok(())
    }

    fn parse_cycle_state(
        &self,
        state: &CycleState,
        current: Option<&crate::LifxColor>,
        default_brightness: Option<f64>,
    ) -> Result<HSBK, String> {
        let mut hue = current.map_or(0, |c| c.hue);
        let mut saturation = current.map_or(0, |c| c.saturation);
        let mut brightness = state.brightness
            .or(default_brightness)
            .map(|b| (b * 65535.0) as u16)
            .or_else(|| current.map(|c| c.brightness))
            .unwrap_or(65535);
        let kelvin = current.map_or(3500, |c| c.kelvin);
        
        if let Some(ref color) = state.color {
            match color.as_str() {
                "white" => {
                    saturation = 0;
                    hue = 0;
                },
                "red" => {
                    hue = 0;
                    saturation = 65535;
                },
                "orange" => {
                    hue = 7098;
                    saturation = 65535;
                },
                "yellow" => {
                    hue = 10920;
                    saturation = 65535;
                },
                "cyan" => {
                    hue = 32760;
                    saturation = 65535;
                },
                "green" => {
                    hue = 21840;
                    saturation = 65535;
                },
                "blue" => {
                    hue = 43680;
                    saturation = 65535;
                },
                "purple" => {
                    hue = 50050;
                    saturation = 65535;
                },
                "pink" => {
                    hue = 63700;
                    saturation = 25000;
                },
                s if s.starts_with("hue:") => {
                    let h = s.strip_prefix("hue:")
                        .and_then(|v| v.parse::<f64>().ok())
                        .ok_or_else(|| "Invalid hue value".to_string())?;
                    hue = ((h * 65535.0 / 360.0) as u16).min(65535);
                },
                s if s.starts_with("saturation:") => {
                    let sat = s.strip_prefix("saturation:")
                        .and_then(|v| v.parse::<f64>().ok())
                        .ok_or_else(|| "Invalid saturation value".to_string())?;
                    saturation = ((sat * 65535.0) as u16).min(65535);
                },
                s if s.starts_with("kelvin:") => {
                    let k = s.strip_prefix("kelvin:")
                        .and_then(|v| v.parse::<u16>().ok())
                        .ok_or_else(|| "Invalid kelvin value".to_string())?;
                    return Ok(HSBK {
                        hue: 0,
                        saturation: 0,
                        brightness,
                        kelvin: k.clamp(1500, 9000),
                    });
                },
                _ => return Err(format!("Unknown color: {}", color)),
            }
        }
        
        Ok(HSBK { hue, saturation, brightness, kelvin })
    }
}

impl Default for CycleHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cycle_request_creation() {
        let request = CycleRequest {
            states: vec![
                CycleState {
                    color: Some("red".to_string()),
                    brightness: Some(1.0),
                    duration: Some(1.0),
                },
                CycleState {
                    color: Some("blue".to_string()),
                    brightness: Some(0.5),
                    duration: Some(2.0),
                },
            ],
            defaults: Some(CycleDefaults {
                power: Some("on".to_string()),
                saturation: Some(1.0),
                brightness: Some(0.8),
                duration: Some(1.5),
            }),
        };
        
        assert_eq!(request.states.len(), 2);
        assert_eq!(request.states[0].color.as_ref().unwrap(), "red");
        assert_eq!(request.defaults.as_ref().unwrap().power.as_ref().unwrap(), "on");
    }
    
    #[test]
    fn test_parse_cycle_state() {
        let handler = CycleHandler::new();
        
        let state = CycleState {
            color: Some("green".to_string()),
            brightness: Some(0.75),
            duration: Some(1.0),
        };
        
        let hsbk = handler.parse_cycle_state(&state, None, None).unwrap();
        assert_eq!(hsbk.hue, 21840); // Green hue
        assert_eq!(hsbk.saturation, 65535);
        assert_eq!(hsbk.brightness, (0.75 * 65535.0) as u16);
    }
    
    #[test]
    fn test_parse_cycle_state_with_defaults() {
        let handler = CycleHandler::new();
        
        let state = CycleState {
            color: Some("red".to_string()),
            brightness: None,
            duration: Some(1.0),
        };
        
        let hsbk = handler.parse_cycle_state(&state, None, Some(0.6)).unwrap();
        assert_eq!(hsbk.hue, 0); // Red hue
        assert_eq!(hsbk.brightness, (0.6 * 65535.0) as u16);
    }
    
    #[test]
    fn test_parse_cycle_state_kelvin() {
        let handler = CycleHandler::new();
        
        let state = CycleState {
            color: Some("kelvin:4500".to_string()),
            brightness: Some(1.0),
            duration: Some(1.0),
        };
        
        let hsbk = handler.parse_cycle_state(&state, None, None).unwrap();
        assert_eq!(hsbk.kelvin, 4500);
        assert_eq!(hsbk.saturation, 0);
        assert_eq!(hsbk.brightness, 65535);
    }
    
    #[test]
    fn test_parse_cycle_state_invalid() {
        let handler = CycleHandler::new();
        
        let state = CycleState {
            color: Some("invalid_color".to_string()),
            brightness: Some(1.0),
            duration: Some(1.0),
        };
        
        let result = handler.parse_cycle_state(&state, None, None);
        assert!(result.is_err());
    }
}