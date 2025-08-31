// Comprehensive validation tests for StateUpdate
// These tests verify that all validation rules are properly enforced

#[cfg(test)]
mod validation_tests {
    use lifx_api_server::set_states::{StateUpdate, StatesRequest};
    use serde_json;

    #[test]
    fn test_valid_brightness_values() {
        // Test valid brightness values
        let valid_values = vec![0.0, 0.5, 1.0, 0.25, 0.75];

        for value in valid_values {
            let json = format!(
                r#"{{
                "selector": "all",
                "brightness": {}
            }}"#,
                value
            );

            let result: Result<StateUpdate, _> = serde_json::from_str(&json);
            assert!(result.is_ok(), "Should accept brightness value {}", value);
            assert_eq!(result.unwrap().brightness, Some(value));
        }
    }

    #[test]
    fn test_invalid_brightness_values() {
        // Test out of range brightness values
        let invalid_values = vec![
            ("-0.1", "negative value"),
            ("1.1", "value > 1.0"),
            ("2.0", "value > 1.0"),
            ("-1.0", "negative value"),
            ("\"NaN\"", "NaN"),
            ("\"Infinity\"", "Infinity"),
            ("\"-Infinity\"", "-Infinity"),
        ];

        for (value, description) in invalid_values {
            let json = format!(
                r#"{{
                "selector": "all",
                "brightness": {}
            }}"#,
                value
            );

            let result: Result<StateUpdate, _> = serde_json::from_str(&json);
            assert!(
                result.is_err(),
                "Should reject brightness {} ({})",
                value,
                description
            );
        }
    }

    #[test]
    fn test_valid_infrared_values() {
        // Test valid infrared values
        let valid_values = vec![0.0, 0.5, 1.0, 0.25, 0.75];

        for value in valid_values {
            let json = format!(
                r#"{{
                "selector": "all",
                "infrared": {}
            }}"#,
                value
            );

            let result: Result<StateUpdate, _> = serde_json::from_str(&json);
            assert!(result.is_ok(), "Should accept infrared value {}", value);
            assert_eq!(result.unwrap().infrared, Some(value));
        }
    }

    #[test]
    fn test_invalid_infrared_values() {
        // Test out of range infrared values
        let invalid_values = vec![
            ("-0.1", "negative value"),
            ("1.1", "value > 1.0"),
            ("2.0", "value > 1.0"),
            ("-1.0", "negative value"),
            ("\"NaN\"", "NaN"),
            ("\"Infinity\"", "Infinity"),
            ("\"-Infinity\"", "-Infinity"),
        ];

        for (value, description) in invalid_values {
            let json = format!(
                r#"{{
                "selector": "all",
                "infrared": {}
            }}"#,
                value
            );

            let result: Result<StateUpdate, _> = serde_json::from_str(&json);
            assert!(
                result.is_err(),
                "Should reject infrared {} ({})",
                value,
                description
            );
        }
    }

    #[test]
    fn test_valid_duration_values() {
        // Test valid duration values
        let valid_values = vec![0.0, 1.0, 60.0, 3600.0, 86400.0, 3155760000.0];

        for value in valid_values {
            let json = format!(
                r#"{{
                "selector": "all",
                "duration": {}
            }}"#,
                value
            );

            let result: Result<StateUpdate, _> = serde_json::from_str(&json);
            assert!(result.is_ok(), "Should accept duration value {}", value);
            assert_eq!(result.unwrap().duration, Some(value));
        }
    }

    #[test]
    fn test_invalid_duration_values() {
        // Test out of range duration values
        let invalid_values = vec![
            ("-1.0", "negative value"),
            ("3155760001.0", "value > max"),
            ("\"NaN\"", "NaN"),
            ("\"Infinity\"", "Infinity"),
            ("\"-Infinity\"", "-Infinity"),
        ];

        for (value, description) in invalid_values {
            let json = format!(
                r#"{{
                "selector": "all",
                "duration": {}
            }}"#,
                value
            );

            let result: Result<StateUpdate, _> = serde_json::from_str(&json);
            assert!(
                result.is_err(),
                "Should reject duration {} ({})",
                value,
                description
            );
        }
    }

    #[test]
    fn test_valid_power_values() {
        // Test valid power values
        let valid_values = vec!["on", "off"];

        for value in valid_values {
            let json = format!(
                r#"{{
                "selector": "all",
                "power": "{}"
            }}"#,
                value
            );

            let result: Result<StateUpdate, _> = serde_json::from_str(&json);
            assert!(result.is_ok(), "Should accept power value {}", value);
            assert_eq!(result.unwrap().power, Some(value.to_string()));
        }
    }

    #[test]
    fn test_invalid_power_values() {
        // Test invalid power values
        let invalid_values = vec![
            "ON", "OFF", "true", "false", "1", "0", "enabled", "disabled",
        ];

        for value in invalid_values {
            let json = format!(
                r#"{{
                "selector": "all",
                "power": "{}"
            }}"#,
                value
            );

            let result: Result<StateUpdate, _> = serde_json::from_str(&json);
            assert!(result.is_err(), "Should reject power value {}", value);
        }
    }

    #[test]
    fn test_edge_case_brightness() {
        // Test exact boundary values for brightness
        let json_zero = r#"{"selector": "all", "brightness": 0.0}"#;
        let json_one = r#"{"selector": "all", "brightness": 1.0}"#;

        let result_zero: Result<StateUpdate, _> = serde_json::from_str(json_zero);
        let result_one: Result<StateUpdate, _> = serde_json::from_str(json_one);

        assert!(result_zero.is_ok(), "Should accept brightness 0.0");
        assert!(result_one.is_ok(), "Should accept brightness 1.0");

        assert_eq!(result_zero.unwrap().brightness, Some(0.0));
        assert_eq!(result_one.unwrap().brightness, Some(1.0));
    }

    #[test]
    fn test_edge_case_infrared() {
        // Test exact boundary values for infrared
        let json_zero = r#"{"selector": "all", "infrared": 0.0}"#;
        let json_one = r#"{"selector": "all", "infrared": 1.0}"#;

        let result_zero: Result<StateUpdate, _> = serde_json::from_str(json_zero);
        let result_one: Result<StateUpdate, _> = serde_json::from_str(json_one);

        assert!(result_zero.is_ok(), "Should accept infrared 0.0");
        assert!(result_one.is_ok(), "Should accept infrared 1.0");

        assert_eq!(result_zero.unwrap().infrared, Some(0.0));
        assert_eq!(result_one.unwrap().infrared, Some(1.0));
    }

    #[test]
    fn test_multiple_invalid_fields() {
        // Test multiple invalid fields at once
        let json = r#"{
            "selector": "all",
            "power": "invalid",
            "brightness": 1.5,
            "infrared": -0.5
        }"#;

        let result: Result<StateUpdate, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "Should reject when multiple fields are invalid"
        );
    }

    #[test]
    fn test_complete_valid_state_update() {
        // Test a complete valid StateUpdate with all fields
        let json = r#"{
            "selector": "all",
            "power": "on",
            "color": "red",
            "brightness": 0.75,
            "duration": 5.0,
            "infrared": 0.25,
            "fast": true
        }"#;

        let result: Result<StateUpdate, _> = serde_json::from_str(json);
        assert!(result.is_ok(), "Should accept complete valid StateUpdate");

        let state = result.unwrap();
        assert_eq!(state.selector, "all");
        assert_eq!(state.power, Some("on".to_string()));
        assert_eq!(state.color, Some("red".to_string()));
        assert_eq!(state.brightness, Some(0.75));
        assert_eq!(state.duration, Some(5.0));
        assert_eq!(state.infrared, Some(0.25));
        assert_eq!(state.fast, Some(true));
    }

    #[test]
    fn test_states_request_with_defaults() {
        // Test StatesRequest with defaults containing invalid values
        let json = r#"{
            "states": [
                {
                    "selector": "all",
                    "power": "on"
                }
            ],
            "defaults": {
                "selector": "ignored",
                "brightness": 0.5,
                "infrared": 0.0
            }
        }"#;

        let result: Result<StatesRequest, _> = serde_json::from_str(json);
        assert!(result.is_ok(), "Should accept valid defaults");
    }

    #[test]
    fn test_missing_selector() {
        // Test that selector is required
        let json = r#"{
            "power": "on",
            "brightness": 0.5
        }"#;

        let result: Result<StateUpdate, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "Should reject StateUpdate without selector"
        );
    }

    #[test]
    fn test_very_small_positive_values() {
        // Test very small positive values that should be accepted
        let json = r#"{
            "selector": "all",
            "brightness": 0.000001,
            "infrared": 0.000001
        }"#;

        let result: Result<StateUpdate, _> = serde_json::from_str(json);
        assert!(result.is_ok(), "Should accept very small positive values");
    }

    #[test]
    fn test_precision_edge_cases() {
        // Test floating point precision edge cases
        let json = r#"{
            "selector": "all",
            "brightness": 0.9999999999,
            "infrared": 0.9999999999
        }"#;

        let result: Result<StateUpdate, _> = serde_json::from_str(json);
        assert!(result.is_ok(), "Should accept values very close to 1.0");
    }
}
