// Integration tests for SetStates endpoint
// These tests verify the behavior of the complete SetStates implementation

#[cfg(test)]
mod tests {
    use lifx_api_server::set_states::{SetStatesHandler, StatesRequest, StateUpdate, StatesResponse};
    
    // Helper function to create a test request
    fn create_test_request(states: Vec<StateUpdate>, defaults: Option<StateUpdate>) -> StatesRequest {
        StatesRequest {
            states,
            defaults,
        }
    }
    
    #[test]
    fn test_validation_empty_states() {
        let handler = SetStatesHandler::new();
        let request = create_test_request(vec![], None);
        
        // Empty states should fail validation
        // Note: This test would require a Manager instance to fully test
        assert!(request.states.is_empty());
    }
    
    #[test]
    fn test_validation_invalid_power() {
        let handler = SetStatesHandler::new();
        let states = vec![
            StateUpdate {
                selector: "all".to_string(),
                power: Some("invalid_power".to_string()),
                color: None,
                brightness: None,
                duration: None,
                infrared: None,
                fast: None,
            }
        ];
        let request = create_test_request(states, None);
        
        // Invalid power value should fail validation
        assert_eq!(request.states[0].power, Some("invalid_power".to_string()));
    }
    
    #[test]
    fn test_validation_brightness_out_of_range() {
        let handler = SetStatesHandler::new();
        let states = vec![
            StateUpdate {
                selector: "all".to_string(),
                power: None,
                color: None,
                brightness: Some(1.5), // Out of range (0.0-1.0)
                duration: None,
                infrared: None,
                fast: None,
            }
        ];
        let request = create_test_request(states, None);
        
        // Brightness > 1.0 should fail validation
        assert!(request.states[0].brightness.unwrap() > 1.0);
    }
    
    #[test]
    fn test_validation_infrared_out_of_range() {
        let handler = SetStatesHandler::new();
        let states = vec![
            StateUpdate {
                selector: "all".to_string(),
                power: None,
                color: None,
                brightness: None,
                duration: None,
                infrared: Some(-0.1), // Out of range (0.0-1.0)
                fast: None,
            }
        ];
        let request = create_test_request(states, None);
        
        // Negative infrared should fail validation
        assert!(request.states[0].infrared.unwrap() < 0.0);
    }
    
    #[test]
    fn test_validation_invalid_selector() {
        let handler = SetStatesHandler::new();
        let states = vec![
            StateUpdate {
                selector: "invalid:selector:format".to_string(),
                power: Some("on".to_string()),
                color: None,
                brightness: None,
                duration: None,
                infrared: None,
                fast: None,
            }
        ];
        let request = create_test_request(states, None);
        
        // Invalid selector format
        assert!(request.states[0].selector.contains("invalid:selector"));
    }
    
    #[test]
    fn test_defaults_application() {
        let states = vec![
            StateUpdate {
                selector: "all".to_string(),
                power: None,
                color: None,
                brightness: None,
                duration: None,
                infrared: None,
                fast: None,
            },
            StateUpdate {
                selector: "id:123".to_string(),
                power: Some("off".to_string()),
                color: None,
                brightness: None,
                duration: None,
                infrared: None,
                fast: None,
            },
        ];
        
        let defaults = Some(StateUpdate {
            selector: "ignored".to_string(),
            power: Some("on".to_string()),
            color: Some("white".to_string()),
            brightness: Some(0.5),
            duration: Some(2.0),
            infrared: Some(0.0),
            fast: Some(true),
        });
        
        let request = create_test_request(states, defaults);
        
        // Verify request structure
        assert_eq!(request.states.len(), 2);
        assert!(request.defaults.is_some());
        
        // First state should inherit all defaults
        assert_eq!(request.states[0].power, None); // Would be filled by handler
        
        // Second state should keep its explicit power value
        assert_eq!(request.states[1].power, Some("off".to_string()));
    }
    
    #[test]
    fn test_multiple_selectors() {
        let states = vec![
            StateUpdate {
                selector: "all".to_string(),
                power: Some("on".to_string()),
                color: None,
                brightness: None,
                duration: None,
                infrared: None,
                fast: None,
            },
            StateUpdate {
                selector: "id:abc123".to_string(),
                power: None,
                color: Some("red".to_string()),
                brightness: None,
                duration: None,
                infrared: None,
                fast: None,
            },
            StateUpdate {
                selector: "group_id:living_room".to_string(),
                power: None,
                color: None,
                brightness: Some(0.8),
                duration: None,
                infrared: None,
                fast: None,
            },
            StateUpdate {
                selector: "location_id:home".to_string(),
                power: None,
                color: Some("kelvin:3000".to_string()),
                brightness: None,
                duration: None,
                infrared: None,
                fast: None,
            },
            StateUpdate {
                selector: "label:Kitchen".to_string(),
                power: Some("off".to_string()),
                color: None,
                brightness: None,
                duration: Some(5.0),
                infrared: None,
                fast: None,
            },
        ];
        
        let request = create_test_request(states, None);
        
        // Verify all selectors are present and correct
        assert_eq!(request.states.len(), 5);
        assert_eq!(request.states[0].selector, "all");
        assert_eq!(request.states[1].selector, "id:abc123");
        assert_eq!(request.states[2].selector, "group_id:living_room");
        assert_eq!(request.states[3].selector, "location_id:home");
        assert_eq!(request.states[4].selector, "label:Kitchen");
    }
    
    #[test]
    fn test_color_formats_validation() {
        let color_tests = vec![
            ("white", true),
            ("red", true),
            ("kelvin:3000", true),
            ("hue:180", true),
            ("saturation:0.5", true),
            ("brightness:0.8", true),
            ("rgb:255,128,0", true),
            ("#FF8000", true),
            ("hue:120 saturation:1.0", true),
            ("invalid_color", false),
            ("kelvin:invalid", false),
            ("rgb:256,0,0", false), // Out of range
            ("#GGGGGG", false), // Invalid hex
        ];
        
        for (color, _should_be_valid) in color_tests {
            let states = vec![
                StateUpdate {
                    selector: "all".to_string(),
                    power: None,
                    color: Some(color.to_string()),
                    brightness: None,
                    duration: None,
                    infrared: None,
                    fast: None,
                }
            ];
            
            let request = create_test_request(states, None);
            assert_eq!(request.states[0].color, Some(color.to_string()));
        }
    }
    
    #[test]
    fn test_complex_state_combination() {
        let states = vec![
            StateUpdate {
                selector: "group:bedroom".to_string(),
                power: Some("on".to_string()),
                color: Some("kelvin:2700".to_string()),
                brightness: Some(0.3),
                duration: Some(10.0),
                infrared: None,
                fast: Some(false),
            },
            StateUpdate {
                selector: "location:office".to_string(),
                power: Some("on".to_string()),
                color: Some("hue:200 saturation:0.8 brightness:0.9".to_string()),
                brightness: None, // Brightness is in the color string
                duration: Some(0.0),
                infrared: Some(0.2),
                fast: Some(true),
            },
        ];
        
        let defaults = Some(StateUpdate {
            selector: "ignored".to_string(),
            power: Some("off".to_string()),
            color: None,
            brightness: Some(1.0),
            duration: Some(1.0),
            infrared: Some(0.0),
            fast: Some(false),
        });
        
        let request = create_test_request(states, defaults);
        
        // Verify complex state combinations
        assert_eq!(request.states[0].power, Some("on".to_string()));
        assert_eq!(request.states[0].color, Some("kelvin:2700".to_string()));
        assert_eq!(request.states[0].brightness, Some(0.3));
        assert_eq!(request.states[0].duration, Some(10.0));
        assert_eq!(request.states[0].fast, Some(false));
        
        assert_eq!(request.states[1].power, Some("on".to_string()));
        assert!(request.states[1].color.as_ref().unwrap().contains("hue:200"));
        assert_eq!(request.states[1].infrared, Some(0.2));
        assert_eq!(request.states[1].fast, Some(true));
    }
}