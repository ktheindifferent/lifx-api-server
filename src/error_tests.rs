#[cfg(test)]
mod tests {
    use super::super::error::{LifxError, Result};
    use std::sync::{Arc, Mutex};
    
    #[test]
    fn test_missing_field_error() {
        let err = LifxError::MissingField("brightness".to_string());
        assert_eq!(err.to_string(), "Missing required field: brightness");
    }
    
    #[test]
    fn test_validation_error() {
        let err = LifxError::ValidationError("Invalid color format".to_string());
        assert_eq!(err.to_string(), "Validation error: Invalid color format");
    }
    
    #[test]
    fn test_scene_not_found_error() {
        let err = LifxError::SceneNotFound("scene-123".to_string());
        assert_eq!(err.to_string(), "Scene not found: scene-123");
    }
    
    #[test]
    fn test_device_not_found_error() {
        let err = LifxError::DeviceNotFound("device-456".to_string());
        assert_eq!(err.to_string(), "Device not found: device-456");
    }
    
    #[test]
    fn test_mutex_poisoned_error() {
        let err = LifxError::MutexPoisoned("Test mutex error".to_string());
        assert_eq!(err.to_string(), "Mutex poisoned: Test mutex error");
    }
    
    #[test]
    fn test_parse_error() {
        let err = LifxError::ParseError("Failed to parse integer".to_string());
        assert_eq!(err.to_string(), "Parse error: Failed to parse integer");
    }
    
    #[test]
    fn test_config_error() {
        let err = LifxError::ConfigError("Missing SECRET_KEY".to_string());
        assert_eq!(err.to_string(), "Configuration error: Missing SECRET_KEY");
    }
    
    #[test]
    fn test_result_type_alias() {
        fn test_function() -> Result<String> {
            Ok("Success".to_string())
        }
        
        let result = test_function();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success");
    }
    
    #[test]
    fn test_result_error_propagation() {
        fn test_function() -> Result<String> {
            Err(LifxError::ValidationError("Test error".to_string()))
        }
        
        let result = test_function();
        assert!(result.is_err());
        
        if let Err(e) = result {
            assert_eq!(e.to_string(), "Validation error: Test error");
        }
    }
    
    #[test]
    fn test_from_poison_error() {
        // Simulate a poisoned mutex scenario
        let mutex = Arc::new(Mutex::new(42));
        let mutex_clone = Arc::clone(&mutex);
        
        // Force a panic in a thread to poison the mutex
        let handle = std::thread::spawn(move || {
            let _lock = mutex_clone.lock().unwrap();
            panic!("Intentional panic to poison mutex");
        });
        
        // Wait for the thread to panic
        let _ = handle.join();
        
        // Now try to lock the poisoned mutex
        let result = mutex.lock();
        assert!(result.is_err());
        
        // Convert the PoisonError to LifxError
        let lifx_error: LifxError = result.unwrap_err().into();
        assert!(lifx_error.to_string().contains("Mutex poisoned"));
    }
    
    #[test]
    fn test_error_chaining_with_io() {
        use std::io;
        
        fn simulate_io_error() -> Result<()> {
            let io_err = io::Error::new(io::ErrorKind::NotFound, "File not found");
            Err(io_err.into())
        }
        
        let result = simulate_io_error();
        assert!(result.is_err());
        
        if let Err(e) = result {
            assert!(e.to_string().contains("Network error"));
        }
    }
    
    #[test]
    fn test_error_chaining_with_json() {
        fn simulate_json_error() -> Result<serde_json::Value> {
            let invalid_json = "{ invalid json }";
            serde_json::from_str(invalid_json)
                .map_err(|e| e.into())
        }
        
        let result = simulate_json_error();
        assert!(result.is_err());
        
        if let Err(e) = result {
            assert!(e.to_string().contains("JSON error"));
        }
    }
    
    #[test]
    fn test_error_with_env_var() {
        use std::env;
        
        fn get_env_var() -> Result<String> {
            env::var("NON_EXISTENT_VAR_FOR_TEST")
                .map_err(|e| e.into())
        }
        
        let result = get_env_var();
        assert!(result.is_err());
        
        if let Err(e) = result {
            assert!(e.to_string().contains("Environment variable error"));
        }
    }
}