extern crate lifx_api_server;
use log::{error, info};
use std::env;

fn main() {
    // Initialize logger with environment variable control
    env_logger::init();

    if let Err(e) = sudo::with_env(&["SECRET_KEY"]) {
        error!("Failed to preserve SECRET_KEY environment variable: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = sudo::escalate_if_needed() {
        error!("Failed to escalate privileges: {}", e);
        std::process::exit(1);
    }

    let secret_key = env::var("SECRET_KEY").expect("$SECRET_KEY is not set");

    // Get discovery refresh interval from env var, default to 300 seconds (5 minutes)
    let discovery_refresh_interval = env::var("DISCOVERY_REFRESH_INTERVAL")
        .unwrap_or_else(|_| "300".to_string())
        .parse::<u64>()
        .unwrap_or_else(|e| {
            error!(
                "Invalid DISCOVERY_REFRESH_INTERVAL, using default 300 seconds: {}",
                e
            );
            300
        });

    // Get auto discovery enabled flag from env var, default to true
    let auto_discovery_enabled = env::var("AUTO_DISCOVERY_ENABLED")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or_else(|e| {
            error!("Invalid AUTO_DISCOVERY_ENABLED, using default true: {}", e);
            true
        });

    let config = lifx_api_server::Config {
        secret_key: secret_key.to_string(),
        port: 8000,
        discovery_refresh_interval,
        auto_discovery_enabled,
    };

    info!("Starting LIFX API server on port {}", config.port);
    info!(
        "Discovery refresh interval: {} seconds",
        discovery_refresh_interval
    );
    info!("Auto discovery enabled: {}", auto_discovery_enabled);
    lifx_api_server::start(config);

    // Now you can use curl to access the api
    // curl -X PUT "http://localhost:8089/v1/lights/all/state"      -H "Authorization: Bearer xxx"      -d "color=kelvin:9000"
    // or rust

    // extern crate lifx_rs as lifx;

    // fn main() {

    //     let key = "xxx".to_string();
    //     let mut api_endpoints: Vec<String> = Vec::new();

    //     api_endpoints.push(format!("http://localhost:8089"));

    //     let config = lifx::LifxConfig{
    //         access_token: key.clone(),
    //         api_endpoints: api_endpoints
    //     };

    //     let mut off_state = lifx::State::new();
    //     off_state.power = Some(format!("off"));

    //     // Turn off all lights
    //     lifx::Light::set_state_by_selector(config.clone(), format!("all"), off_state);

    // }

    info!("Server started, entering main loop");

    loop {}
}
