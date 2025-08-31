use crate::{BulbInfo, Manager};
use lifx_rs::lan::{PowerLevel, HSBK};
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct StateUpdate {
    pub selector: String,
    pub power: Option<String>,
    pub color: Option<String>,
    pub brightness: Option<f64>,
    pub duration: Option<f64>,
    pub infrared: Option<f64>,
    pub fast: Option<bool>,
}

// Custom deserializer for StateUpdate with validation
impl<'de> Deserialize<'de> for StateUpdate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Selector,
            Power,
            Color,
            Brightness,
            Duration,
            Infrared,
            Fast,
        }

        struct StateUpdateVisitor;

        impl<'de> Visitor<'de> for StateUpdateVisitor {
            type Value = StateUpdate;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct StateUpdate")
            }

            fn visit_map<V>(self, mut map: V) -> Result<StateUpdate, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut selector = None;
                let mut power = None;
                let mut color = None;
                let mut brightness = None;
                let mut duration = None;
                let mut infrared = None;
                let mut fast = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Selector => {
                            if selector.is_some() {
                                return Err(de::Error::duplicate_field("selector"));
                            }
                            selector = Some(map.next_value()?);
                        }
                        Field::Power => {
                            if power.is_some() {
                                return Err(de::Error::duplicate_field("power"));
                            }
                            let value: String = map.next_value()?;
                            if value != "on" && value != "off" {
                                return Err(de::Error::custom(format!(
                                    "power must be 'on' or 'off', got '{}'",
                                    value
                                )));
                            }
                            power = Some(Some(value));
                        }
                        Field::Color => {
                            if color.is_some() {
                                return Err(de::Error::duplicate_field("color"));
                            }
                            color = Some(map.next_value()?);
                        }
                        Field::Brightness => {
                            if brightness.is_some() {
                                return Err(de::Error::duplicate_field("brightness"));
                            }
                            let value: f64 = map.next_value()?;
                            if !value.is_finite() {
                                return Err(de::Error::custom(format!(
                                    "brightness must be a finite number, got {}",
                                    value
                                )));
                            }
                            if value < 0.0 || value > 1.0 {
                                return Err(de::Error::custom(format!(
                                    "brightness must be between 0.0 and 1.0, got {}",
                                    value
                                )));
                            }
                            brightness = Some(Some(value));
                        }
                        Field::Duration => {
                            if duration.is_some() {
                                return Err(de::Error::duplicate_field("duration"));
                            }
                            let value: f64 = map.next_value()?;
                            if !value.is_finite() {
                                return Err(de::Error::custom(format!(
                                    "duration must be a finite number, got {}",
                                    value
                                )));
                            }
                            if value < 0.0 || value > 3155760000.0 {
                                return Err(de::Error::custom(
                                    "duration must be between 0 and 3155760000 seconds",
                                ));
                            }
                            duration = Some(Some(value));
                        }
                        Field::Infrared => {
                            if infrared.is_some() {
                                return Err(de::Error::duplicate_field("infrared"));
                            }
                            let value: f64 = map.next_value()?;
                            if !value.is_finite() {
                                return Err(de::Error::custom(format!(
                                    "infrared must be a finite number, got {}",
                                    value
                                )));
                            }
                            if value < 0.0 || value > 1.0 {
                                return Err(de::Error::custom(format!(
                                    "infrared must be between 0.0 and 1.0, got {}",
                                    value
                                )));
                            }
                            infrared = Some(Some(value));
                        }
                        Field::Fast => {
                            if fast.is_some() {
                                return Err(de::Error::duplicate_field("fast"));
                            }
                            fast = Some(map.next_value()?);
                        }
                    }
                }

                let selector = selector.ok_or_else(|| de::Error::missing_field("selector"))?;

                Ok(StateUpdate {
                    selector,
                    power: power.unwrap_or(None),
                    color: color.unwrap_or(None),
                    brightness: brightness.unwrap_or(None),
                    duration: duration.unwrap_or(None),
                    infrared: infrared.unwrap_or(None),
                    fast: fast.unwrap_or(None),
                })
            }
        }

        const FIELDS: &'static [&'static str] = &[
            "selector",
            "power",
            "color",
            "brightness",
            "duration",
            "infrared",
            "fast",
        ];
        deserializer.deserialize_struct("StateUpdate", FIELDS, StateUpdateVisitor)
    }
}

#[derive(Deserialize, Debug)]
pub struct StatesRequest {
    pub states: Vec<StateUpdate>,
    pub defaults: Option<StateUpdate>,
}

#[derive(Serialize, Clone, Debug)]
pub struct StateResult {
    pub id: String,
    pub label: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct StatesResponse {
    pub results: Vec<StateResult>,
}

#[derive(Debug)]
struct BulbUpdate {
    bulb_info: BulbInfo,
    state_update: StateUpdate,
    attempt: u32,
}

#[derive(Debug)]
struct UpdateResult {
    id: String,
    label: String,
    success: bool,
    error: Option<String>,
}

pub struct SetStatesHandler {
    max_retries: u32,
    concurrent_workers: usize,
}

impl SetStatesHandler {
    pub fn new() -> Self {
        SetStatesHandler {
            max_retries: 3,
            concurrent_workers: 4,
        }
    }

    pub fn handle_request(&self, mgr: &mut Manager, request: StatesRequest) -> StatesResponse {
        let bulbs = match mgr.bulbs.lock() {
            Ok(guard) => guard,
            Err(e) => {
                eprintln!("Failed to acquire bulbs lock in SetStatesHandler: {}", e);
                return StatesResponse {
                    results: vec![StateResult {
                        id: "mutex_error".to_string(),
                        label: "Internal Error".to_string(),
                        status: "error".to_string(),
                        error: Some("Failed to acquire bulbs lock".to_string()),
                    }],
                };
            }
        };

        // Validate request first
        if let Err(e) = self.validate_request(&request) {
            return StatesResponse {
                results: vec![StateResult {
                    id: "validation_error".to_string(),
                    label: "Request Validation".to_string(),
                    status: "error".to_string(),
                    error: Some(e),
                }],
            };
        }

        // Apply defaults to states if provided
        let states_with_defaults = self.apply_defaults(request.states, request.defaults);

        // Collect all bulb updates to be performed
        let mut all_updates: Vec<BulbUpdate> = Vec::new();

        for state_update in states_with_defaults {
            let filtered_bulbs = self.filter_bulbs_by_selector(&bulbs, &state_update.selector);

            for bulb in filtered_bulbs {
                all_updates.push(BulbUpdate {
                    bulb_info: bulb.clone(),
                    state_update: state_update.clone(),
                    attempt: 0,
                });
            }
        }

        // If no bulbs match any selector, return empty results
        if all_updates.is_empty() {
            return StatesResponse { results: vec![] };
        }

        // Execute updates concurrently with retry logic
        let results = self.execute_concurrent_updates(mgr, all_updates);

        // Convert results to response format
        let mut response_results = Vec::new();
        for result in results {
            response_results.push(StateResult {
                id: result.id,
                label: result.label,
                status: if result.success {
                    "ok".to_string()
                } else {
                    "error".to_string()
                },
                error: result.error,
            });
        }

        StatesResponse {
            results: response_results,
        }
    }

    fn validate_request(&self, request: &StatesRequest) -> Result<(), String> {
        // Validate states array is not empty
        if request.states.is_empty() {
            return Err("States array cannot be empty".to_string());
        }

        // Validate each state update
        for (i, state) in request.states.iter().enumerate() {
            // Validate selector format
            if state.selector.is_empty() {
                return Err(format!("State[{}]: selector cannot be empty", i));
            }

            // Validate selector format
            if !self.is_valid_selector(&state.selector) {
                return Err(format!(
                    "State[{}]: invalid selector format '{}'",
                    i, state.selector
                ));
            }

            // Validate power value
            if let Some(ref power) = state.power {
                if power != "on" && power != "off" {
                    return Err(format!(
                        "State[{}]: power must be 'on' or 'off', got '{}'",
                        i, power
                    ));
                }
            }

            // Validate brightness range
            if let Some(brightness) = state.brightness {
                if !brightness.is_finite() {
                    return Err(format!(
                        "State[{}]: brightness must be a finite number, got {}",
                        i, brightness
                    ));
                }
                if brightness < 0.0 || brightness > 1.0 {
                    return Err(format!(
                        "State[{}]: brightness must be between 0.0 and 1.0, got {}",
                        i, brightness
                    ));
                }
            }

            // Validate infrared range
            if let Some(infrared) = state.infrared {
                if !infrared.is_finite() {
                    return Err(format!(
                        "State[{}]: infrared must be a finite number, got {}",
                        i, infrared
                    ));
                }
                if infrared < 0.0 || infrared > 1.0 {
                    return Err(format!(
                        "State[{}]: infrared must be between 0.0 and 1.0, got {}",
                        i, infrared
                    ));
                }
            }

            // Validate duration
            if let Some(duration) = state.duration {
                if !duration.is_finite() {
                    return Err(format!(
                        "State[{}]: duration must be a finite number, got {}",
                        i, duration
                    ));
                }
                if duration < 0.0 || duration > 3155760000.0 {
                    return Err(format!(
                        "State[{}]: duration must be between 0 and 3155760000 seconds",
                        i
                    ));
                }
            }

            // Validate color format
            if let Some(ref color) = state.color {
                if !self.is_valid_color(color) {
                    return Err(format!("State[{}]: invalid color format '{}'", i, color));
                }
            }
        }

        // Validate defaults if present
        if let Some(ref defaults) = request.defaults {
            if let Some(ref power) = defaults.power {
                if power != "on" && power != "off" {
                    return Err(format!(
                        "Defaults: power must be 'on' or 'off', got '{}'",
                        power
                    ));
                }
            }

            if let Some(brightness) = defaults.brightness {
                if !brightness.is_finite() {
                    return Err(format!(
                        "Defaults: brightness must be a finite number, got {}",
                        brightness
                    ));
                }
                if brightness < 0.0 || brightness > 1.0 {
                    return Err(format!(
                        "Defaults: brightness must be between 0.0 and 1.0, got {}",
                        brightness
                    ));
                }
            }

            if let Some(infrared) = defaults.infrared {
                if !infrared.is_finite() {
                    return Err(format!(
                        "Defaults: infrared must be a finite number, got {}",
                        infrared
                    ));
                }
                if infrared < 0.0 || infrared > 1.0 {
                    return Err(format!(
                        "Defaults: infrared must be between 0.0 and 1.0, got {}",
                        infrared
                    ));
                }
            }

            if let Some(duration) = defaults.duration {
                if !duration.is_finite() {
                    return Err(format!(
                        "Defaults: duration must be a finite number, got {}",
                        duration
                    ));
                }
                if duration < 0.0 || duration > 3155760000.0 {
                    return Err(format!(
                        "Defaults: duration must be between 0 and 3155760000 seconds, got {}",
                        duration
                    ));
                }
            }

            if let Some(ref color) = defaults.color {
                if !self.is_valid_color(color) {
                    return Err(format!("Defaults: invalid color format '{}'", color));
                }
            }
        }

        Ok(())
    }

    fn is_valid_selector(&self, selector: &str) -> bool {
        selector == "all"
            || selector.starts_with("id:")
            || selector.starts_with("group_id:")
            || selector.starts_with("location_id:")
            || selector.starts_with("label:")
            || selector.starts_with("group:")
            || selector.starts_with("location:")
    }

    fn is_valid_color(&self, color: &str) -> bool {
        // Named colors
        let named_colors = [
            "white", "red", "orange", "yellow", "cyan", "green", "blue", "purple", "pink",
        ];
        if named_colors.contains(&color) {
            return true;
        }

        // Validate kelvin value
        if let Some(kelvin_str) = color.strip_prefix("kelvin:") {
            if let Ok(kelvin) = kelvin_str.parse::<u16>() {
                return kelvin >= 1500 && kelvin <= 9000;
            }
            return false;
        }

        // Validate hue value
        if let Some(hue_str) = color.strip_prefix("hue:") {
            if let Ok(hue) = hue_str.parse::<f64>() {
                return hue.is_finite() && hue >= 0.0 && hue <= 360.0;
            }
            return false;
        }

        // Validate saturation value
        if let Some(sat_str) = color.strip_prefix("saturation:") {
            if let Ok(sat) = sat_str.parse::<f64>() {
                return sat.is_finite() && sat >= 0.0 && sat <= 1.0;
            }
            return false;
        }

        // Validate brightness value
        if let Some(bright_str) = color.strip_prefix("brightness:") {
            if let Ok(bright) = bright_str.parse::<f64>() {
                return bright.is_finite() && bright >= 0.0 && bright <= 1.0;
            }
            return false;
        }

        // Validate RGB format
        if let Some(rgb_str) = color.strip_prefix("rgb:") {
            let parts: Vec<&str> = rgb_str.split(',').collect();
            if parts.len() != 3 {
                return false;
            }
            for part in parts {
                if part.trim().parse::<u8>().is_err() {
                    return false;
                }
            }
            return true;
        }

        // Validate hex color
        if let Some(hex) = color.strip_prefix("#") {
            if hex.len() != 6 {
                return false;
            }
            return hex.chars().all(|c| c.is_ascii_hexdigit());
        }

        // HSB format: "hue:120 saturation:1.0 brightness:0.5"
        if color.contains(" ")
            && (color.contains("hue:")
                || color.contains("saturation:")
                || color.contains("brightness:")
                || color.contains("kelvin:"))
        {
            let parts: Vec<&str> = color.split_whitespace().collect();
            for part in parts {
                if let Some(hue_str) = part.strip_prefix("hue:") {
                    if let Ok(hue) = hue_str.parse::<f64>() {
                        if !hue.is_finite() || hue < 0.0 || hue > 360.0 {
                            return false;
                        }
                    } else {
                        return false;
                    }
                } else if let Some(sat_str) = part.strip_prefix("saturation:") {
                    if let Ok(sat) = sat_str.parse::<f64>() {
                        if !sat.is_finite() || sat < 0.0 || sat > 1.0 {
                            return false;
                        }
                    } else {
                        return false;
                    }
                } else if let Some(bright_str) = part.strip_prefix("brightness:") {
                    if let Ok(bright) = bright_str.parse::<f64>() {
                        if !bright.is_finite() || bright < 0.0 || bright > 1.0 {
                            return false;
                        }
                    } else {
                        return false;
                    }
                } else if let Some(kelvin_str) = part.strip_prefix("kelvin:") {
                    if let Ok(kelvin) = kelvin_str.parse::<u16>() {
                        if kelvin < 1500 || kelvin > 9000 {
                            return false;
                        }
                    } else {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            return true;
        }

        false
    }

    fn apply_defaults(
        &self,
        mut states: Vec<StateUpdate>,
        defaults: Option<StateUpdate>,
    ) -> Vec<StateUpdate> {
        if let Some(defaults) = defaults {
            for state in &mut states {
                if state.power.is_none() && defaults.power.is_some() {
                    state.power = defaults.power.clone();
                }
                if state.color.is_none() && defaults.color.is_some() {
                    state.color = defaults.color.clone();
                }
                if state.brightness.is_none() && defaults.brightness.is_some() {
                    state.brightness = defaults.brightness;
                }
                if state.duration.is_none() && defaults.duration.is_some() {
                    state.duration = defaults.duration;
                }
                if state.infrared.is_none() && defaults.infrared.is_some() {
                    state.infrared = defaults.infrared;
                }
                if state.fast.is_none() && defaults.fast.is_some() {
                    state.fast = defaults.fast;
                }
            }
        }
        states
    }

    fn filter_bulbs_by_selector<'a>(
        &self,
        bulbs: &'a HashMap<u64, BulbInfo>,
        selector: &str,
    ) -> Vec<&'a BulbInfo> {
        let mut filtered = Vec::new();

        for bulb in bulbs.values() {
            let matches = match selector {
                "all" => true,
                s if s.starts_with("id:") => {
                    let id = s.strip_prefix("id:").unwrap_or("");
                    bulb.id.contains(id)
                }
                s if s.starts_with("group_id:") => {
                    let group_id = s.strip_prefix("group_id:").unwrap_or("");
                    bulb.lifx_group
                        .as_ref()
                        .map_or(false, |g| g.id.contains(group_id))
                }
                s if s.starts_with("group:") => {
                    let group_name = s.strip_prefix("group:").unwrap_or("");
                    bulb.lifx_group
                        .as_ref()
                        .map_or(false, |g| g.name.contains(group_name))
                }
                s if s.starts_with("location_id:") => {
                    let location_id = s.strip_prefix("location_id:").unwrap_or("");
                    bulb.lifx_location
                        .as_ref()
                        .map_or(false, |l| l.id.contains(location_id))
                }
                s if s.starts_with("location:") => {
                    let location_name = s.strip_prefix("location:").unwrap_or("");
                    bulb.lifx_location
                        .as_ref()
                        .map_or(false, |l| l.name.contains(location_name))
                }
                s if s.starts_with("label:") => {
                    let label = s.strip_prefix("label:").unwrap_or("");
                    bulb.label.contains(label)
                }
                _ => false,
            };

            if matches {
                filtered.push(bulb);
            }
        }

        filtered
    }

    fn execute_concurrent_updates(
        &self,
        mgr: &Manager,
        updates: Vec<BulbUpdate>,
    ) -> Vec<UpdateResult> {
        let mut results = Vec::new();

        // Process updates sequentially with retry logic
        // Note: True concurrent updates would require refactoring the Manager to be thread-safe
        for mut update in updates {
            let mut success = false;
            let mut error_msg = None;

            // Retry logic
            while update.attempt < self.max_retries && !success {
                update.attempt += 1;

                match Self::apply_state_to_bulb(mgr, &update.bulb_info, &update.state_update) {
                    Ok(_) => {
                        success = true;
                    }
                    Err(e) => {
                        error_msg = Some(format!("Attempt {}: {}", update.attempt, e));
                        if update.attempt < self.max_retries {
                            // Wait before retry with exponential backoff
                            thread::sleep(Duration::from_millis(
                                100 * (2_u64.pow(update.attempt - 1)),
                            ));
                        }
                    }
                }
            }

            results.push(UpdateResult {
                id: update.bulb_info.id.clone(),
                label: update.bulb_info.label.clone(),
                success,
                error: if success { None } else { error_msg },
            });
        }

        results
    }

    fn apply_state_to_bulb(
        mgr: &Manager,
        bulb: &BulbInfo,
        state: &StateUpdate,
    ) -> Result<(), String> {
        // Apply power state
        if let Some(ref power) = state.power {
            let power_level = if power == "on" {
                PowerLevel::Enabled
            } else {
                PowerLevel::Standby
            };

            bulb.set_power(&mgr.sock, power_level)
                .map_err(|e| format!("Failed to set power: {:?}", e))?;
        }

        // Parse and apply color
        if let Some(ref color_str) = state.color {
            let hsbk = Self::parse_color(color_str, bulb)?;
            let duration = state.duration.unwrap_or(0.0) as u32;

            bulb.set_color(&mgr.sock, hsbk, duration)
                .map_err(|e| format!("Failed to set color: {:?}", e))?;
        }

        // Apply brightness independently if no color was specified
        if state.color.is_none() && state.brightness.is_some() {
            let brightness_val = state.brightness.unwrap();
            let duration = state.duration.unwrap_or(0.0) as u32;

            let current_color = bulb.lifx_color.as_ref();
            let hsbk = HSBK {
                hue: current_color.map_or(0, |c| c.hue),
                saturation: current_color.map_or(0, |c| c.saturation),
                brightness: (brightness_val * 65535.0) as u16,
                kelvin: current_color.map_or(6500, |c| c.kelvin),
            };

            bulb.set_color(&mgr.sock, hsbk, duration)
                .map_err(|e| format!("Failed to set brightness: {:?}", e))?;
        }

        // Apply infrared
        if let Some(infrared) = state.infrared {
            let ir_brightness = (infrared * 65535.0) as u16;
            bulb.set_infrared(&mgr.sock, ir_brightness)
                .map_err(|e| format!("Failed to set infrared: {:?}", e))?;
        }

        Ok(())
    }

    fn parse_color(color_str: &str, bulb: &BulbInfo) -> Result<HSBK, String> {
        let current_color = bulb.lifx_color.as_ref();
        let mut hue = current_color.map_or(0, |c| c.hue);
        let mut saturation = current_color.map_or(0, |c| c.saturation);
        let mut brightness = current_color.map_or(65535, |c| c.brightness);
        let mut kelvin = current_color.map_or(6500, |c| c.kelvin);

        // Parse different color formats
        if color_str.starts_with("kelvin:") {
            let k = color_str
                .strip_prefix("kelvin:")
                .and_then(|s| s.parse::<u16>().ok())
                .ok_or_else(|| "Invalid kelvin value".to_string())?;
            kelvin = k.clamp(1500, 9000);
            saturation = 0;
        } else if color_str.starts_with("hue:") {
            let h = color_str
                .strip_prefix("hue:")
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or_else(|| "Invalid hue value".to_string())?;
            hue = ((h * 65535.0 / 360.0) as u16).min(65535);
        } else if color_str.starts_with("saturation:") {
            let s = color_str
                .strip_prefix("saturation:")
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or_else(|| "Invalid saturation value".to_string())?;
            saturation = ((s * 65535.0) as u16).min(65535);
        } else if color_str.starts_with("brightness:") {
            let b = color_str
                .strip_prefix("brightness:")
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or_else(|| "Invalid brightness value".to_string())?;
            brightness = ((b * 65535.0) as u16).min(65535);
        } else if color_str.starts_with("rgb:") {
            // Parse RGB format "rgb:255,0,128"
            let rgb_str = color_str.strip_prefix("rgb:").unwrap_or("");
            let parts: Vec<&str> = rgb_str.split(',').collect();
            if parts.len() != 3 {
                return Err("RGB format must be 'rgb:r,g,b'".to_string());
            }

            let r = parts[0].parse::<u8>().map_err(|_| "Invalid red value")?;
            let g = parts[1].parse::<u8>().map_err(|_| "Invalid green value")?;
            let b = parts[2].parse::<u8>().map_err(|_| "Invalid blue value")?;

            let (h, s, l) = Self::rgb_to_hsl(r, g, b);
            hue = (h * 65535.0 / 360.0) as u16;
            saturation = (s * 65535.0) as u16;
            brightness = (l * 65535.0) as u16;
        } else if color_str.starts_with("#") {
            // Parse hex color
            let hex = color_str.strip_prefix("#").unwrap_or("");
            if hex.len() != 6 {
                return Err("Hex color must be 6 characters".to_string());
            }

            let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "Invalid hex color")?;
            let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "Invalid hex color")?;
            let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "Invalid hex color")?;

            let (h, s, l) = Self::rgb_to_hsl(r, g, b);
            hue = (h * 65535.0 / 360.0) as u16;
            saturation = (s * 65535.0) as u16;
            brightness = (l * 65535.0) as u16;
        } else if color_str.contains(" ") {
            // Parse space-separated HSB values
            let parts: Vec<&str> = color_str.split_whitespace().collect();
            for part in parts {
                if part.starts_with("hue:") {
                    let h = part
                        .strip_prefix("hue:")
                        .and_then(|s| s.parse::<f64>().ok())
                        .ok_or_else(|| "Invalid hue value".to_string())?;
                    hue = ((h * 65535.0 / 360.0) as u16).min(65535);
                } else if part.starts_with("saturation:") {
                    let s = part
                        .strip_prefix("saturation:")
                        .and_then(|s| s.parse::<f64>().ok())
                        .ok_or_else(|| "Invalid saturation value".to_string())?;
                    saturation = ((s * 65535.0) as u16).min(65535);
                } else if part.starts_with("brightness:") {
                    let b = part
                        .strip_prefix("brightness:")
                        .and_then(|s| s.parse::<f64>().ok())
                        .ok_or_else(|| "Invalid brightness value".to_string())?;
                    brightness = ((b * 65535.0) as u16).min(65535);
                } else if part.starts_with("kelvin:") {
                    let k = part
                        .strip_prefix("kelvin:")
                        .and_then(|s| s.parse::<u16>().ok())
                        .ok_or_else(|| "Invalid kelvin value".to_string())?;
                    kelvin = k.clamp(1500, 9000);
                }
            }
        } else {
            // Handle named colors
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
                _ => return Err(format!("Unknown color: {}", color_str)),
            }
        }

        Ok(HSBK {
            hue,
            saturation,
            brightness,
            kelvin,
        })
    }

    fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
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

impl Default for SetStatesHandler {
    fn default() -> Self {
        Self::new()
    }
}
