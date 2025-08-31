use lifx_api_server::scenes::{ScenesHandler, CreateSceneRequest, SceneState, ActivateSceneRequest};
use lifx_api_server::error::LifxError;

#[test]
fn test_scene_not_found_error() {
    let handler = ScenesHandler::new();
    
    // Try to activate a non-existent scene
    let mgr = create_test_manager();
    let result = handler.activate_scene(&mgr, "non-existent-uuid", ActivateSceneRequest {
        duration: Some(1.0),
        fast: Some(false),
    });
    
    assert!(result.is_err());
    
    if let Err(e) = result {
        // Check that it's specifically a SceneNotFound error
        match e {
            LifxError::SceneNotFound(uuid) => {
                assert_eq!(uuid, "non-existent-uuid");
            }
            _ => panic!("Expected SceneNotFound error, got: {:?}", e),
        }
    }
}

#[test]
fn test_scene_crud_error_recovery() {
    let handler = ScenesHandler::new();
    
    // Create a scene
    let request = CreateSceneRequest {
        name: "Test Scene".to_string(),
        states: vec![
            SceneState {
                selector: "id:test".to_string(),
                power: Some("on".to_string()),
                color: None,
                brightness: Some(0.5),
                kelvin: Some(3500),
            },
        ],
    };
    
    let result = handler.create_scene(request);
    assert!(result.is_ok());
    
    let scene_uuid = result.unwrap().scene.uuid;
    
    // Test getting the scene
    let get_result = handler.get_scene(&scene_uuid);
    assert!(get_result.is_ok());
    assert!(get_result.unwrap().is_some());
    
    // Delete the scene
    let delete_result = handler.delete_scene(&scene_uuid);
    assert!(delete_result.is_ok());
    assert!(delete_result.unwrap());
    
    // Try to get the deleted scene
    let get_deleted = handler.get_scene(&scene_uuid);
    assert!(get_deleted.is_ok());
    assert!(get_deleted.unwrap().is_none());
    
    // Try to delete the already deleted scene
    let delete_again = handler.delete_scene(&scene_uuid);
    assert!(delete_again.is_ok());
    assert!(!delete_again.unwrap()); // Should return false
}

#[test]
fn test_concurrent_scene_access() {
    use std::sync::Arc;
    use std::thread;
    
    let handler = Arc::new(ScenesHandler::new());
    let mut handles = vec![];
    
    // Create multiple threads that create scenes concurrently
    for i in 0..10 {
        let handler_clone = Arc::clone(&handler);
        let handle = thread::spawn(move || {
            let request = CreateSceneRequest {
                name: format!("Concurrent Scene {}", i),
                states: vec![
                    SceneState {
                        selector: format!("id:bulb{}", i),
                        power: Some("on".to_string()),
                        color: None,
                        brightness: Some(1.0),
                        kelvin: Some(6500),
                    },
                ],
            };
            
            handler_clone.create_scene(request)
        });
        handles.push(handle);
    }
    
    // Wait for all threads to complete and check results
    for handle in handles {
        let result = handle.join().unwrap();
        assert!(result.is_ok());
    }
    
    // Verify all scenes were created
    let list_result = handler.list_scenes();
    assert!(list_result.is_ok());
    let scenes = list_result.unwrap().scenes;
    assert_eq!(scenes.len(), 10);
}

#[test]
fn test_malformed_scene_state() {
    let handler = ScenesHandler::new();
    
    // Create a scene with potentially problematic values
    let request = CreateSceneRequest {
        name: "Edge Case Scene".to_string(),
        states: vec![
            SceneState {
                selector: "".to_string(), // Empty selector
                power: Some("invalid_power_state".to_string()), // Invalid power state
                color: None,
                brightness: Some(-1.0), // Invalid brightness (negative)
                kelvin: Some(100000), // Invalid kelvin (too high)
            },
        ],
    };
    
    // The scene creation should succeed (validation happens during activation)
    let result = handler.create_scene(request);
    assert!(result.is_ok());
    
    // But activation should handle these gracefully
    let mgr = create_test_manager();
    let scene_uuid = result.unwrap().scene.uuid;
    let activate_result = handler.activate_scene(&mgr, &scene_uuid, ActivateSceneRequest {
        duration: Some(1.0),
        fast: Some(false),
    });
    
    // Should handle gracefully even with invalid values
    assert!(activate_result.is_ok());
}

// Helper function to create a test Manager
fn create_test_manager() -> lifx_api_server::Manager {
    use std::net::UdpSocket;
    use std::sync::{Arc, Mutex};
    use std::collections::HashMap;
    
    lifx_api_server::Manager {
        sock: Arc::new(UdpSocket::bind("127.0.0.1:0").unwrap()),
        bulbs: Arc::new(Mutex::new(HashMap::new())),
        source: rand::random(),
        sequence: Arc::new(Mutex::new(0)),
        rate_limiter: Arc::new(lifx_api_server::RateLimiter::new(20, std::time::Duration::from_secs(1))),
    }
}