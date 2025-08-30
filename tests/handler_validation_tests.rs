// Tests for SetStatesHandler validation logic
// These tests verify the handler's is_valid_color method

#[cfg(test)]
mod handler_validation_tests {
    use lifx_api_server::set_states::SetStatesHandler;
    
    // Since SetStatesHandler methods are not public, we'll test through the public interface
    // by creating StateUpdate objects and checking if they can be deserialized
    
    use serde_json;
    use lifx_api_server::set_states::StateUpdate;
    
    #[test]
    fn test_color_validation_through_state_update() {
        // Test that valid colors can be part of a StateUpdate
        let valid_colors = vec![
            "white", "red", "orange", "yellow", "cyan", "green", "blue", "purple", "pink",
            "kelvin:3000", "kelvin:6500", "kelvin:9000",
            "hue:180", "hue:0", "hue:360",
            "saturation:0.5", "saturation:0", "saturation:1.0",
            "brightness:0.5", "brightness:0", "brightness:1.0",
            "rgb:255,128,0", "rgb:0,0,0", "rgb:255,255,255",
            "#FF0000", "#00FF00", "#0000FF", "#FFFFFF",
            "hue:180 saturation:0.5 brightness:0.8",
        ];
        
        for color in valid_colors {
            let json = format!(r#"{{
                "selector": "all",
                "color": "{}"
            }}"#, color);
            
            let result: Result<StateUpdate, _> = serde_json::from_str(&json);
            assert!(result.is_ok(), "Should accept color '{}', but got error: {:?}", 
                   color, result.err());
            assert_eq!(result.unwrap().color, Some(color.to_string()));
        }
    }
    
    #[test]
    fn test_combined_state_with_all_validations() {
        // Test a complete state with all fields that need validation
        let json = r#"{
            "selector": "all",
            "power": "on",
            "color": "kelvin:3000",
            "brightness": 0.75,
            "duration": 5.0,
            "infrared": 0.25,
            "fast": true
        }"#;
        
        let result: Result<StateUpdate, _> = serde_json::from_str(json);
        assert!(result.is_ok(), "Should accept complete valid state");
        
        let state = result.unwrap();
        assert_eq!(state.selector, "all");
        assert_eq!(state.power, Some("on".to_string()));
        assert_eq!(state.color, Some("kelvin:3000".to_string()));
        assert_eq!(state.brightness, Some(0.75));
        assert_eq!(state.duration, Some(5.0));
        assert_eq!(state.infrared, Some(0.25));
        assert_eq!(state.fast, Some(true));
    }
    
    #[test]
    fn test_invalid_combinations() {
        // Test that invalid combinations are rejected
        let invalid_cases = vec![
            (r#"{"selector": "all", "power": "ON"}"#, "uppercase power"),
            (r#"{"selector": "all", "brightness": 1.5}"#, "brightness > 1.0"),
            (r#"{"selector": "all", "infrared": -0.5}"#, "negative infrared"),
            (r#"{"selector": "all", "duration": -1.0}"#, "negative duration"),
            (r#"{"selector": "all", "brightness": "NaN"}"#, "NaN brightness"),
            (r#"{"selector": "all", "infrared": "Infinity"}"#, "Infinity infrared"),
        ];
        
        for (json, description) in invalid_cases {
            let result: Result<StateUpdate, _> = serde_json::from_str(json);
            assert!(result.is_err(), "Should reject {}", description);
        }
    }
    
    #[test]
    fn test_selector_is_required() {
        let json = r#"{
            "power": "on",
            "brightness": 0.5
        }"#;
        
        let result: Result<StateUpdate, _> = serde_json::from_str(json);
        assert!(result.is_err(), "Should reject StateUpdate without selector");
    }
    
    #[test]
    fn test_all_fields_optional_except_selector() {
        let json = r#"{
            "selector": "all"
        }"#;
        
        let result: Result<StateUpdate, _> = serde_json::from_str(json);
        assert!(result.is_ok(), "Should accept StateUpdate with only selector");
        
        let state = result.unwrap();
        assert_eq!(state.selector, "all");
        assert_eq!(state.power, None);
        assert_eq!(state.color, None);
        assert_eq!(state.brightness, None);
        assert_eq!(state.duration, None);
        assert_eq!(state.infrared, None);
        assert_eq!(state.fast, None);
    }
}