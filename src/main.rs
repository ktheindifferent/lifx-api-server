extern crate lifx_api_server;
use std::env;
use log::{info, warn, error};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

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

 
    let (secret_key, auth_required) = match env::var("SECRET_KEY") {
        Ok(key) if !key.is_empty() => {
            info!("Authentication enabled with provided SECRET_KEY");
            (Some(key), true)
        },
        Ok(_) => {
            // Empty SECRET_KEY means auth disabled
            warn!("SECRET_KEY is empty - authentication disabled!");
            warn!("WARNING: API is accessible without authentication. Use only in trusted environments.");
            (None, false)
        },
        Err(_) => {
            // Check if we're in development mode
            match env::var("LIFX_API_MODE") {
                Ok(mode) if mode == "development" => {
                    // Generate a random key for development
                    let random_key: String = thread_rng()
                        .sample_iter(&Alphanumeric)
                        .take(32)
                        .map(char::from)
                        .collect();
                    warn!("SECRET_KEY not set - generated random key for development mode");
                    warn!("Generated key: {}", random_key);
                    warn!("Set SECRET_KEY environment variable for production use");
                    (Some(random_key), true)
                },
                _ => {
                    // Production mode - authentication disabled with warning
                    warn!("SECRET_KEY not set - authentication disabled!");
                    warn!("WARNING: API is accessible without authentication.");
                    warn!("For production use, set SECRET_KEY environment variable.");
                    warn!("To enable development mode with auto-generated key, set LIFX_API_MODE=development");
                    (None, false)
                }
            }
        }
    };

    let config = lifx_api_server::Config { 
        secret_key,
        port: 8000,
        auth_required
    };

    info!("Starting LIFX API server on port {}", config.port);
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

    loop {
        
    }
}