use thiserror::Error;
use std::sync::PoisonError;

#[derive(Error, Debug)]
pub enum LifxError {
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
    
    #[error("Missing required field: {0}")]
    MissingField(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Mutex poisoned: {0}")]
    MutexPoisoned(String),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Scene not found: {0}")]
    SceneNotFound(String),
    
    #[error("Device not found: {0}")]
    DeviceNotFound(String),
    
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("Environment variable error: {0}")]
    EnvVarError(#[from] std::env::VarError),
    
    #[error("Failure error: {0}")]
    FailureError(String),
}

impl<T> From<PoisonError<T>> for LifxError {
    fn from(err: PoisonError<T>) -> Self {
        LifxError::MutexPoisoned(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, LifxError>;