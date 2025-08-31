use crate::{parse_f64_safe, BulbInfo, Manager};
use lifx_rs::lan::{BuildOptions, Message, RawMessage, Waveform, HSBK};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize, Debug, Clone)]
pub struct EffectRequest {
    pub color: Option<String>,
    pub from_color: Option<String>,
    pub period: Option<f64>,
    pub cycles: Option<f64>,
    pub persist: Option<bool>,
    pub power_on: Option<bool>,
    pub peak: Option<f64>,
}

#[derive(Serialize, Debug, Clone)]
pub struct EffectResult {
    pub id: String,
    pub label: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct EffectsResponse {
    pub results: Vec<EffectResult>,
}

pub struct EffectsHandler;

impl EffectsHandler {
    pub fn new() -> Self {
        EffectsHandler
    }

    pub fn handle_pulse(
        &self,
        mgr: &Manager,
        bulbs: &[&BulbInfo],
        request: EffectRequest,
    ) -> EffectsResponse {
        let mut results = Vec::new();

        for bulb in bulbs {
            let result = self.apply_pulse_effect(mgr, bulb, &request);
            results.push(EffectResult {
                id: bulb.id.clone(),
                label: bulb.label.clone(),
                status: if result.is_ok() {
                    "ok".to_string()
                } else {
                    "error".to_string()
                },
            });
        }

        EffectsResponse { results }
    }

    pub fn handle_breathe(
        &self,
        mgr: &Manager,
        bulbs: &[&BulbInfo],
        request: EffectRequest,
    ) -> EffectsResponse {
        let mut results = Vec::new();

        for bulb in bulbs {
            let result = self.apply_breathe_effect(mgr, bulb, &request);
            results.push(EffectResult {
                id: bulb.id.clone(),
                label: bulb.label.clone(),
                status: if result.is_ok() {
                    "ok".to_string()
                } else {
                    "error".to_string()
                },
            });
        }

        EffectsResponse { results }
    }

    pub fn handle_strobe(
        &self,
        mgr: &Manager,
        bulbs: &[&BulbInfo],
        request: EffectRequest,
    ) -> EffectsResponse {
        let mut results = Vec::new();

        for bulb in bulbs {
            let result = self.apply_strobe_effect(mgr, bulb, &request);
            results.push(EffectResult {
                id: bulb.id.clone(),
                label: bulb.label.clone(),
                status: if result.is_ok() {
                    "ok".to_string()
                } else {
                    "error".to_string()
                },
            });
        }

        EffectsResponse { results }
    }

    fn apply_pulse_effect(
        &self,
        mgr: &Manager,
        bulb: &BulbInfo,
        request: &EffectRequest,
    ) -> Result<(), String> {
        let period = (request.period.unwrap_or(1.0) * 1000.0) as u32;
        let cycles = request.cycles.unwrap_or(5.0) as f32;
        let peak = request.peak.unwrap_or(0.5);

        let current_color = bulb.lifx_color.as_ref();
        let from_color =
            self.parse_color_or_current(request.from_color.as_deref(), current_color)?;
        let to_color = self.parse_color_or_default(request.color.as_deref(), current_color)?;

        let transient = !request.persist.unwrap_or(false);
        let skew_ratio = self.peak_to_skew_ratio(peak);

        let options = BuildOptions {
            target: Some(bulb.target),
            res_required: true,
            source: bulb.source,
            ..Default::default()
        };

        let message = Message::SetWaveform {
            reserved: 0,
            transient,
            color: to_color,
            period,
            cycles,
            skew_ratio,
            waveform: Waveform::Pulse,
        };

        let raw_message = RawMessage::build(&options, message)
            .map_err(|e| format!("Failed to build message: {:?}", e))?;

        mgr.sock
            .send_to(
                &raw_message
                    .pack()
                    .map_err(|e| format!("Failed to pack message: {:?}", e))?,
                bulb.addr,
            )
            .map_err(|e| format!("Failed to send message: {:?}", e))?;

        Ok(())
    }

    fn apply_breathe_effect(
        &self,
        mgr: &Manager,
        bulb: &BulbInfo,
        request: &EffectRequest,
    ) -> Result<(), String> {
        let period = (request.period.unwrap_or(1.0) * 1000.0) as u32;
        let cycles = request.cycles.unwrap_or(5.0) as f32;
        let peak = request.peak.unwrap_or(0.5);

        let current_color = bulb.lifx_color.as_ref();
        let to_color = self.parse_color_or_default(request.color.as_deref(), current_color)?;

        let transient = !request.persist.unwrap_or(false);
        let skew_ratio = self.peak_to_skew_ratio(peak);

        let options = BuildOptions {
            target: Some(bulb.target),
            res_required: true,
            source: bulb.source,
            ..Default::default()
        };

        let message = Message::SetWaveform {
            reserved: 0,
            transient,
            color: to_color,
            period,
            cycles,
            skew_ratio,
            waveform: Waveform::Sine,
        };

        let raw_message = RawMessage::build(&options, message)
            .map_err(|e| format!("Failed to build message: {:?}", e))?;

        mgr.sock
            .send_to(
                &raw_message
                    .pack()
                    .map_err(|e| format!("Failed to pack message: {:?}", e))?,
                bulb.addr,
            )
            .map_err(|e| format!("Failed to send message: {:?}", e))?;

        Ok(())
    }

    fn apply_strobe_effect(
        &self,
        mgr: &Manager,
        bulb: &BulbInfo,
        request: &EffectRequest,
    ) -> Result<(), String> {
        let period = (request.period.unwrap_or(0.1) * 1000.0) as u32;
        let cycles = request.cycles.unwrap_or(10.0) as f32;

        let current_color = bulb.lifx_color.as_ref();
        let to_color = self.parse_color_or_default(request.color.as_deref(), current_color)?;

        let transient = !request.persist.unwrap_or(false);
        let skew_ratio = 0i16;

        let options = BuildOptions {
            target: Some(bulb.target),
            res_required: true,
            source: bulb.source,
            ..Default::default()
        };

        let message = Message::SetWaveform {
            reserved: 0,
            transient,
            color: to_color,
            period,
            cycles,
            skew_ratio,
            waveform: Waveform::Pulse,
        };

        let raw_message = RawMessage::build(&options, message)
            .map_err(|e| format!("Failed to build message: {:?}", e))?;

        mgr.sock
            .send_to(
                &raw_message
                    .pack()
                    .map_err(|e| format!("Failed to pack message: {:?}", e))?,
                bulb.addr,
            )
            .map_err(|e| format!("Failed to send message: {:?}", e))?;

        Ok(())
    }

    fn parse_color_or_current(
        &self,
        color_str: Option<&str>,
        current: Option<&crate::LifxColor>,
    ) -> Result<HSBK, String> {
        if let Some(color) = color_str {
            self.parse_color_string(color, current)
        } else {
            Ok(HSBK {
                hue: current.map_or(0, |c| c.hue),
                saturation: current.map_or(0, |c| c.saturation),
                brightness: current.map_or(65535, |c| c.brightness),
                kelvin: current.map_or(3500, |c| c.kelvin),
            })
        }
    }

    fn parse_color_or_default(
        &self,
        color_str: Option<&str>,
        current: Option<&crate::LifxColor>,
    ) -> Result<HSBK, String> {
        if let Some(color) = color_str {
            self.parse_color_string(color, current)
        } else {
            Ok(HSBK {
                hue: 0,
                saturation: 0,
                brightness: 65535,
                kelvin: 3500,
            })
        }
    }

    fn parse_color_string(
        &self,
        color_str: &str,
        current: Option<&crate::LifxColor>,
    ) -> Result<HSBK, String> {
        let mut hue = current.map_or(0, |c| c.hue);
        let mut saturation = current.map_or(0, |c| c.saturation);
        let mut brightness = current.map_or(65535, |c| c.brightness);
        let mut kelvin = current.map_or(3500, |c| c.kelvin);

        match color_str {
            "white" => {
                saturation = 0;
                hue = 0;
            }
            "red" => {
                hue = 0;
                saturation = 65535;
            }
            "orange" => {
                hue = 7098;
                saturation = 65535;
            }
            "yellow" => {
                hue = 10920;
                saturation = 65535;
            }
            "cyan" => {
                hue = 32760;
                saturation = 65535;
            }
            "green" => {
                hue = 21840;
                saturation = 65535;
            }
            "blue" => {
                hue = 43680;
                saturation = 65535;
            }
            "purple" => {
                hue = 50050;
                saturation = 65535;
            }
            "pink" => {
                hue = 63700;
                saturation = 25000;
            }
            s if s.starts_with("kelvin:") => {
                let k = s
                    .strip_prefix("kelvin:")
                    .and_then(|v| v.parse::<u16>().ok())
                    .ok_or_else(|| "Invalid kelvin value".to_string())?;
                kelvin = k.clamp(1500, 9000);
                saturation = 0;
            }
            s if s.starts_with("hue:") => {
                let h = s
                    .strip_prefix("hue:")
                    .and_then(|v| v.parse::<f64>().ok())
                    .ok_or_else(|| "Invalid hue value".to_string())?;
                hue = ((h * 65535.0 / 360.0) as u16).min(65535);
            }
            s if s.starts_with("saturation:") => {
                let sat = s
                    .strip_prefix("saturation:")
                    .and_then(|v| v.parse::<f64>().ok())
                    .ok_or_else(|| "Invalid saturation value".to_string())?;
                saturation = ((sat * 65535.0) as u16).min(65535);
            }
            s if s.starts_with("brightness:") => {
                let br = s
                    .strip_prefix("brightness:")
                    .and_then(|v| v.parse::<f64>().ok())
                    .ok_or_else(|| "Invalid brightness value".to_string())?;
                brightness = ((br * 65535.0) as u16).min(65535);
            }
            s if s.starts_with("#") => {
                let hex = s.strip_prefix("#").unwrap_or("");
                if hex.len() != 6 {
                    return Err("Hex color must be 6 characters".to_string());
                }

                let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "Invalid hex color")?;
                let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "Invalid hex color")?;
                let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "Invalid hex color")?;

                let (h, s, _) = self.rgb_to_hsl(r, g, b);
                hue = (h * 65535.0 / 360.0) as u16;
                saturation = (s * 65535.0) as u16;
            }
            _ => return Err(format!("Unknown color: {}", color_str)),
        }

        Ok(HSBK {
            hue,
            saturation,
            brightness,
            kelvin,
        })
    }

    fn peak_to_skew_ratio(&self, peak: f64) -> i16 {
        let clamped = peak.max(0.0).min(1.0);
        ((clamped - 0.5) * 65535.0) as i16
    }

    fn rgb_to_hsl(&self, r: u8, g: u8, b: u8) -> (f64, f64, f64) {
        let r = r as f64 / 255.0;
        let g = g as f64 / 255.0;
        let b = b as f64 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let diff = max - min;

        let l = (max + min) / 2.0;

        if diff == 0.0 {
            return (0.0, 0.0, l);
        }

        let s = if l < 0.5 {
            diff / (max + min)
        } else {
            diff / (2.0 - max - min)
        };

        let h = if max == r {
            ((g - b) / diff + if g < b { 6.0 } else { 0.0 }) / 6.0
        } else if max == g {
            ((b - r) / diff + 2.0) / 6.0
        } else {
            ((r - g) / diff + 4.0) / 6.0
        };

        (h * 360.0, s, l)
    }
}

impl Default for EffectsHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peak_to_skew_ratio() {
        let handler = EffectsHandler::new();

        // Test edge cases
        assert_eq!(handler.peak_to_skew_ratio(0.0), -32767);
        assert_eq!(handler.peak_to_skew_ratio(0.5), 0);
        assert_eq!(handler.peak_to_skew_ratio(1.0), 32767);

        // Test clamping
        assert_eq!(handler.peak_to_skew_ratio(-0.5), -32767);
        assert_eq!(handler.peak_to_skew_ratio(1.5), 32767);
    }

    #[test]
    fn test_rgb_to_hsl() {
        let handler = EffectsHandler::new();

        // Test pure red
        let (h, s, l) = handler.rgb_to_hsl(255, 0, 0);
        assert!((h - 0.0).abs() < 0.1);
        assert!((s - 1.0).abs() < 0.01);
        assert!((l - 0.5).abs() < 0.01);

        // Test pure green
        let (h, s, l) = handler.rgb_to_hsl(0, 255, 0);
        assert!((h - 120.0).abs() < 0.1);
        assert!((s - 1.0).abs() < 0.01);
        assert!((l - 0.5).abs() < 0.01);

        // Test pure blue
        let (h, s, l) = handler.rgb_to_hsl(0, 0, 255);
        assert!((h - 240.0).abs() < 0.1);
        assert!((s - 1.0).abs() < 0.01);
        assert!((l - 0.5).abs() < 0.01);

        // Test white
        let (_, s, l) = handler.rgb_to_hsl(255, 255, 255);
        assert_eq!(s, 0.0);
        assert!((l - 1.0).abs() < 0.01);

        // Test black
        let (_, s, l) = handler.rgb_to_hsl(0, 0, 0);
        assert_eq!(s, 0.0);
        assert_eq!(l, 0.0);
    }

    #[test]
    fn test_parse_color_string() {
        let handler = EffectsHandler::new();

        // Test named colors
        let red = handler.parse_color_string("red", None).unwrap();
        assert_eq!(red.hue, 0);
        assert_eq!(red.saturation, 65535);

        let green = handler.parse_color_string("green", None).unwrap();
        assert_eq!(green.hue, 21840);
        assert_eq!(green.saturation, 65535);

        let white = handler.parse_color_string("white", None).unwrap();
        assert_eq!(white.hue, 0);
        assert_eq!(white.saturation, 0);

        // Test kelvin
        let kelvin = handler.parse_color_string("kelvin:3500", None).unwrap();
        assert_eq!(kelvin.kelvin, 3500);
        assert_eq!(kelvin.saturation, 0);

        // Test hue
        let hue = handler.parse_color_string("hue:180", None).unwrap();
        assert_eq!(hue.hue, 32767); // 180 degrees = 32767 in LIFX scale

        // Test hex color
        let hex = handler.parse_color_string("#FF0000", None).unwrap();
        assert_eq!(hex.hue, 0); // Red
        assert_eq!(hex.saturation, 65535);

        // Test invalid color
        assert!(handler.parse_color_string("invalid", None).is_err());
    }

    #[test]
    fn test_effect_request_creation() {
        let request = EffectRequest {
            color: Some("red".to_string()),
            from_color: Some("blue".to_string()),
            period: Some(1.0),
            cycles: Some(5.0),
            persist: Some(false),
            power_on: Some(true),
            peak: Some(0.5),
        };

        assert_eq!(request.color.unwrap(), "red");
        assert_eq!(request.period.unwrap(), 1.0);
        assert_eq!(request.cycles.unwrap(), 5.0);
    }
}
