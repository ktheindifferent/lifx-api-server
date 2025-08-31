use lifx_api_server::set_states::{SetStatesHandler, StateUpdate, StatesRequest};
use serde_json;

#[test]
fn test_valid_state_request() {
    let json = r#"{
        "states": [
            {
                "selector": "all",
                "power": "on",
                "brightness": 0.8
            },
            {
                "selector": "id:abc123",
                "color": "red",
                "duration": 2.0
            }
        ]
    }"#;

    let request: StatesRequest =
        serde_json::from_str(json).expect("Failed to parse valid StatesRequest JSON");
    assert_eq!(request.states.len(), 2);
    assert_eq!(request.states[0].selector, "all");
    assert_eq!(request.states[0].power, Some("on".to_string()));
    assert_eq!(request.states[0].brightness, Some(0.8));
}

#[test]
fn test_state_request_with_defaults() {
    let json = r#"{
        "states": [
            {
                "selector": "all"
            },
            {
                "selector": "group_id:123",
                "power": "off"
            }
        ],
        "defaults": {
            "selector": "ignored",
            "power": "on",
            "brightness": 0.5,
            "duration": 1.0
        }
    }"#;

    let request: StatesRequest =
        serde_json::from_str(json).expect("Failed to parse StatesRequest JSON with defaults");
    assert_eq!(request.states.len(), 2);
    assert!(request.defaults.is_some());

    let defaults = request
        .defaults
        .expect("Expected defaults to be present in request");
    assert_eq!(defaults.power, Some("on".to_string()));
    assert_eq!(defaults.brightness, Some(0.5));
    assert_eq!(defaults.duration, Some(1.0));
}

#[test]
fn test_invalid_power_value() {
    let json = r#"{
        "states": [
            {
                "selector": "all",
                "power": "invalid"
            }
        ]
    }"#;

    // With the custom deserializer, invalid power values should be rejected at deserialization time
    let result: Result<StatesRequest, _> = serde_json::from_str(json);
    assert!(
        result.is_err(),
        "Should reject invalid power value at deserialization"
    );

    // The error message should mention the invalid power value
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("power") && error_msg.contains("'on' or 'off'"),
        "Error message should mention power validation: {}",
        error_msg
    );
}

#[test]
fn test_color_formats() {
    let test_cases = vec![
        (
            r#"{"selector": "all", "color": "red"}"#,
            Some("red".to_string()),
        ),
        (
            r#"{"selector": "all", "color": "kelvin:3500"}"#,
            Some("kelvin:3500".to_string()),
        ),
        (
            r#"{"selector": "all", "color": "hue:120"}"#,
            Some("hue:120".to_string()),
        ),
        (
            r#"{"selector": "all", "color": "saturation:0.5"}"#,
            Some("saturation:0.5".to_string()),
        ),
        (
            r#"{"selector": "all", "color": "brightness:0.8"}"#,
            Some("brightness:0.8".to_string()),
        ),
        (
            r#"{"selector": "all", "color": "rgb:255,0,128"}"#,
            Some("rgb:255,0,128".to_string()),
        ),
        (
            r##"{"selector": "all", "color": "#FF0080"}"##,
            Some("#FF0080".to_string()),
        ),
        (
            r#"{"selector": "all", "color": "hue:120 saturation:1.0 brightness:0.5"}"#,
            Some("hue:120 saturation:1.0 brightness:0.5".to_string()),
        ),
    ];

    for (json_str, expected) in test_cases {
        let state: StateUpdate = serde_json::from_str(json_str).expect(&format!(
            "Failed to parse StateUpdate JSON for color format test: {}",
            json_str
        ));
        assert_eq!(state.color, expected);
    }
}

#[test]
fn test_selector_formats() {
    let test_cases = vec![
        r#"{"selector": "all"}"#,
        r#"{"selector": "id:abc123"}"#,
        r#"{"selector": "group_id:xyz789"}"#,
        r#"{"selector": "location_id:home"}"#,
        r#"{"selector": "label:Kitchen"}"#,
        r#"{"selector": "group:Living Room"}"#,
        r#"{"selector": "location:Upstairs"}"#,
    ];

    for json_str in test_cases {
        let state: StateUpdate = serde_json::from_str(json_str).expect(&format!(
            "Failed to parse StateUpdate JSON for color format test: {}",
            json_str
        ));
        assert!(!state.selector.is_empty());
    }
}

#[test]
fn test_brightness_and_infrared_ranges() {
    let valid_json = r#"{
        "selector": "all",
        "brightness": 0.5,
        "infrared": 0.3
    }"#;

    let state: StateUpdate = serde_json::from_str(valid_json)
        .expect("Failed to parse StateUpdate JSON with brightness and infrared");
    assert_eq!(state.brightness, Some(0.5));
    assert_eq!(state.infrared, Some(0.3));
}

#[test]
fn test_duration_field() {
    let json = r#"{
        "selector": "all",
        "power": "on",
        "duration": 5.5
    }"#;

    let state: StateUpdate =
        serde_json::from_str(json).expect("Failed to parse StateUpdate JSON with duration field");
    assert_eq!(state.duration, Some(5.5));
}

#[test]
fn test_fast_field() {
    let json = r#"{
        "selector": "all",
        "power": "on",
        "fast": true
    }"#;

    let state: StateUpdate =
        serde_json::from_str(json).expect("Failed to parse StateUpdate JSON with fast field");
    assert_eq!(state.fast, Some(true));
}

#[test]
fn test_empty_states_array() {
    let json = r#"{
        "states": []
    }"#;

    let request: StatesRequest = serde_json::from_str(json)
        .expect("Failed to parse StatesRequest JSON with empty states array");
    assert_eq!(request.states.len(), 0);
}

#[test]
fn test_multiple_state_updates() {
    let json = r#"{
        "states": [
            {
                "selector": "group_id:bedroom",
                "power": "off",
                "duration": 2.0
            },
            {
                "selector": "group_id:living_room",
                "power": "on",
                "color": "white",
                "brightness": 1.0
            },
            {
                "selector": "label:Kitchen",
                "color": "kelvin:2700",
                "brightness": 0.6
            },
            {
                "selector": "id:abc123def456",
                "infrared": 0.5
            }
        ]
    }"#;

    let request: StatesRequest = serde_json::from_str(json)
        .expect("Failed to parse StatesRequest JSON with multiple state updates");
    assert_eq!(request.states.len(), 4);

    // Verify each state
    assert_eq!(request.states[0].selector, "group_id:bedroom");
    assert_eq!(request.states[0].power, Some("off".to_string()));
    assert_eq!(request.states[0].duration, Some(2.0));

    assert_eq!(request.states[1].selector, "group_id:living_room");
    assert_eq!(request.states[1].power, Some("on".to_string()));
    assert_eq!(request.states[1].color, Some("white".to_string()));
    assert_eq!(request.states[1].brightness, Some(1.0));

    assert_eq!(request.states[2].selector, "label:Kitchen");
    assert_eq!(request.states[2].color, Some("kelvin:2700".to_string()));
    assert_eq!(request.states[2].brightness, Some(0.6));

    assert_eq!(request.states[3].selector, "id:abc123def456");
    assert_eq!(request.states[3].infrared, Some(0.5));
}
