use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use lifx_rs::lan::HSBK;
use crate::{BulbInfo, Manager, LifxColor};
use crate::error::{LifxError, Result};
use crate::mutex_utils::{safe_lock, safe_lock_monitored};
use log::error;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Scene {
    pub uuid: String,
    pub name: String,
    pub states: Vec<SceneState>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SceneState {
    pub selector: String,
    pub power: Option<String>,
    pub color: Option<SceneColor>,
    pub brightness: Option<f64>,
    pub kelvin: Option<u16>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SceneColor {
    pub hue: u16,
    pub saturation: u16,
    pub brightness: u16,
    pub kelvin: u16,
}

#[derive(Deserialize, Debug)]
pub struct CreateSceneRequest {
    pub name: String,
    pub states: Vec<SceneState>,
}

#[derive(Deserialize, Debug)]
pub struct ActivateSceneRequest {
    pub duration: Option<f64>,
    pub fast: Option<bool>,
}

#[derive(Serialize, Debug)]
pub struct SceneResponse {
    pub scene: Scene,
}

#[derive(Serialize, Debug)]
pub struct ScenesListResponse {
    pub scenes: Vec<Scene>,
}

#[derive(Serialize, Debug)]
pub struct ActivateSceneResponse {
    pub results: Vec<ActivateResult>,
}

#[derive(Serialize, Debug)]
pub struct ActivateResult {
    pub id: String,
    pub label: String,
    pub status: String,
}

pub struct ScenesHandler {
    scenes: Arc<Mutex<HashMap<String, Scene>>>,
}

impl ScenesHandler {
    pub fn new() -> Self {
        ScenesHandler {
            scenes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn create_scene(&self, request: CreateSceneRequest) -> Result<SceneResponse> {
        let uuid = self.generate_uuid();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| LifxError::ValidationError(format!("Time error: {}", e)))?
            .as_secs();
        
        let scene = Scene {
            uuid: uuid.clone(),
            name: request.name,
            states: request.states,
            created_at: now,
            updated_at: now,
        };
        
        let mut scenes = self.scenes.lock()?
        scenes.insert(uuid, scene.clone());
        
        Ok(SceneResponse { scene })
    }

    pub fn list_scenes(&self) -> Result<ScenesListResponse> {
        let scenes = self.scenes.lock()?
        let scenes_list: Vec<Scene> = scenes.values().cloned().collect();
        
        Ok(ScenesListResponse { scenes: scenes_list })
    }

    pub fn get_scene(&self, uuid: &str) -> Result<Option<Scene>> {
        let scenes = self.scenes.lock()?;
        Ok(scenes.get(uuid).cloned())
    }

    pub fn delete_scene(&self, uuid: &str) -> Result<bool> {
        let mut scenes = self.scenes.lock()?;
        Ok(scenes.remove(uuid).is_some())
    }

    pub fn activate_scene(
        &self,
        mgr: &Manager,
        uuid: &str,
        request: ActivateSceneRequest,
    ) -> Result<ActivateSceneResponse> {
        let scene = self.get_scene(uuid)?
            .ok_or_else(|| LifxError::SceneNotFound(uuid.to_string()))?;
        
        let duration = (request.duration.unwrap_or(1.0) * 1000.0) as u32;
        let mut results = Vec::new();
        
        let bulbs = mgr.bulbs.lock()?
        
        for state in &scene.states {
            let matching_bulbs = self.filter_bulbs_by_selector(&bulbs, &state.selector);
            
            for bulb in matching_bulbs {
                let result = self.apply_scene_state(mgr, bulb, state, duration);
                
                results.push(ActivateResult {
                    id: bulb.id.clone(),
                    label: bulb.label.clone(),
                    status: if result.is_ok() { "ok".to_string() } else { "error".to_string() },
                });
            }
        }
        
        Ok(ActivateSceneResponse { results })
    }

    pub fn capture_current_state(&self, mgr: &Manager, name: String) -> Result<SceneResponse> {
        let bulbs = mgr.bulbs.lock()?
        let mut states = Vec::new();
        
        for bulb in bulbs.values() {
            let state = SceneState {
                selector: format!("id:{}", bulb.id),
                power: Some(bulb.power.clone()),
                color: bulb.lifx_color.as_ref().map(|c| SceneColor {
                    hue: c.hue,
                    saturation: c.saturation,
                    brightness: c.brightness,
                    kelvin: c.kelvin,
                }),
                brightness: Some(bulb.brightness),
                kelvin: bulb.lifx_color.as_ref().map(|c| c.kelvin),
            };
            states.push(state);
        }
        
        let request = CreateSceneRequest { name, states };
        self.create_scene(request)
    }

    fn apply_scene_state(
        &self,
        mgr: &Manager,
        bulb: &BulbInfo,
        state: &SceneState,
        duration: u32,
    ) -> Result<()> {
        if let Some(ref power) = state.power {
            let power_level = if power == "on" {
                lifx_rs::lan::PowerLevel::Enabled
            } else {
                lifx_rs::lan::PowerLevel::Standby
            };
            
            bulb.set_power(&mgr.sock, power_level)
                .map_err(|e| LifxError::FailureError(format!("Failed to set power: {:?}", e)))?;
        }
        
        if let Some(ref color) = state.color {
            let hsbk = HSBK {
                hue: color.hue,
                saturation: color.saturation,
                brightness: color.brightness,
                kelvin: color.kelvin,
            };
            
            bulb.set_color(&mgr.sock, hsbk, duration)
                .map_err(|e| LifxError::FailureError(format!("Failed to set color: {:?}", e)))?;
        } else if state.brightness.is_some() || state.kelvin.is_some() {
            let current = bulb.lifx_color.as_ref();
            let hsbk = HSBK {
                hue: current.map_or(0, |c| c.hue),
                saturation: current.map_or(0, |c| c.saturation),
                brightness: state.brightness
                    .map(|b| (b * 65535.0) as u16)
                    .or_else(|| current.map(|c| c.brightness))
                    .unwrap_or(65535),
                kelvin: state.kelvin
                    .or_else(|| current.map(|c| c.kelvin))
                    .unwrap_or(3500),
            };
            
            bulb.set_color(&mgr.sock, hsbk, duration)
                .map_err(|e| LifxError::FailureError(format!("Failed to set color: {:?}", e)))?;
        }
        
        Ok(())
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
                },
                s if s.starts_with("group_id:") => {
                    let group_id = s.strip_prefix("group_id:").unwrap_or("");
                    bulb.lifx_group.as_ref().map_or(false, |g| g.id.contains(group_id))
                },
                s if s.starts_with("group:") => {
                    let group_name = s.strip_prefix("group:").unwrap_or("");
                    bulb.lifx_group.as_ref().map_or(false, |g| g.name.contains(group_name))
                },
                s if s.starts_with("location_id:") => {
                    let location_id = s.strip_prefix("location_id:").unwrap_or("");
                    bulb.lifx_location.as_ref().map_or(false, |l| l.id.contains(location_id))
                },
                s if s.starts_with("location:") => {
                    let location_name = s.strip_prefix("location:").unwrap_or("");
                    bulb.lifx_location.as_ref().map_or(false, |l| l.name.contains(location_name))
                },
                s if s.starts_with("label:") => {
                    let label = s.strip_prefix("label:").unwrap_or("");
                    bulb.label.contains(label)
                },
                _ => false,
            };
            
            if matches {
                filtered.push(bulb);
            }
        }
        
        filtered
    }

    fn generate_uuid(&self) -> String {
        use rand::{thread_rng, Rng};
        use rand::distributions::Alphanumeric;
        
        let uuid: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        
        format!("{}-{}-{}-{}-{}",
            &uuid[0..8],
            &uuid[8..12],
            &uuid[12..16],
            &uuid[16..20],
            &uuid[20..32]
        )
    }
}

impl Default for ScenesHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scene_creation() {
        let handler = ScenesHandler::new();
        
        let request = CreateSceneRequest {
            name: "Test Scene".to_string(),
            states: vec![
                SceneState {
                    selector: "all".to_string(),
                    power: Some("on".to_string()),
                    color: Some(SceneColor {
                        hue: 32768,
                        saturation: 65535,
                        brightness: 32768,
                        kelvin: 3500,
                    }),
                    brightness: Some(0.5),
                    kelvin: Some(3500),
                }
            ],
        };
        
        let response = handler.create_scene(request).unwrap();
        assert_eq!(response.scene.name, "Test Scene");
        assert_eq!(response.scene.states.len(), 1);
        assert!(response.scene.uuid.len() > 0);
    }
    
    #[test]
    fn test_scene_list() {
        let handler = ScenesHandler::new();
        
        // Create multiple scenes
        for i in 0..3 {
            let request = CreateSceneRequest {
                name: format!("Scene {}", i),
                states: vec![],
            };
            handler.create_scene(request).unwrap();
        }
        
        let list = handler.list_scenes().unwrap();
        assert_eq!(list.scenes.len(), 3);
    }
    
    #[test]
    fn test_scene_get_and_delete() {
        let handler = ScenesHandler::new();
        
        let request = CreateSceneRequest {
            name: "Test Scene".to_string(),
            states: vec![],
        };
        
        let response = handler.create_scene(request).unwrap();
        let uuid = response.scene.uuid.clone();
        
        // Test get
        let scene = handler.get_scene(&uuid).unwrap();
        assert!(scene.is_some());
        assert_eq!(scene.unwrap().name, "Test Scene");
        
        // Test delete
        assert!(handler.delete_scene(&uuid).unwrap());
        assert!(handler.get_scene(&uuid).unwrap().is_none());
        assert!(!handler.delete_scene(&uuid).unwrap()); // Should return false for non-existent
    }
    
    #[test]
    fn test_uuid_generation() {
        let handler = ScenesHandler::new();
        
        let uuid1 = handler.generate_uuid();
        let uuid2 = handler.generate_uuid();
        
        // UUIDs should be unique
        assert_ne!(uuid1, uuid2);
        
        // UUID should have correct format
        assert!(uuid1.contains('-'));
        let parts: Vec<&str> = uuid1.split('-').collect();
        assert_eq!(parts.len(), 5);
    }
    
    #[test]
    fn test_scene_state_creation() {
        let state = SceneState {
            selector: "id:123".to_string(),
            power: Some("on".to_string()),
            color: Some(SceneColor {
                hue: 0,
                saturation: 0,
                brightness: 65535,
                kelvin: 6500,
            }),
            brightness: Some(1.0),
            kelvin: Some(6500),
        };
        
        assert_eq!(state.selector, "id:123");
        assert_eq!(state.power.as_ref().unwrap(), "on");
        assert_eq!(state.brightness.as_ref().unwrap(), &1.0);
    }
}