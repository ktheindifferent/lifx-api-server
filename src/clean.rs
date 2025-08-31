use crate::{BulbInfo, Manager};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone)]
pub struct CleanRequest {
    pub duration: Option<u32>,
    pub stop: Option<bool>,
}

#[derive(Serialize, Debug)]
pub struct CleanResult {
    pub id: String,
    pub label: String,
    pub status: String,
    pub message: Option<String>,
}

#[derive(Serialize)]
pub struct CleanResponse {
    pub results: Vec<CleanResult>,
}

pub struct CleanHandler;

impl CleanHandler {
    pub fn new() -> Self {
        CleanHandler
    }

    pub fn handle_clean(
        &self,
        _mgr: &Manager,
        bulbs: &[&BulbInfo],
        _request: CleanRequest,
    ) -> CleanResponse {
        let mut results = Vec::new();

        for bulb in bulbs {
            let has_hev = bulb
                .product
                .as_ref()
                .map_or(false, |p| p.capabilities.has_hev);

            if !has_hev {
                results.push(CleanResult {
                    id: bulb.id.clone(),
                    label: bulb.label.clone(),
                    status: "error".to_string(),
                    message: Some("Device does not support HEV/Clean mode".to_string()),
                });
                continue;
            }

            results.push(CleanResult {
                id: bulb.id.clone(),
                label: bulb.label.clone(),
                status: "ok".to_string(),
                message: Some("Clean mode operation acknowledged (HEV message type not yet implemented in lifx-rs)".to_string()),
            });
        }

        CleanResponse { results }
    }
}

impl Default for CleanHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_request_creation() {
        let request = CleanRequest {
            duration: Some(3600),
            stop: Some(false),
        };

        assert_eq!(request.duration.unwrap(), 3600);
        assert_eq!(request.stop.unwrap(), false);
    }

    #[test]
    fn test_clean_request_stop() {
        let request = CleanRequest {
            duration: None,
            stop: Some(true),
        };

        assert!(request.duration.is_none());
        assert_eq!(request.stop.unwrap(), true);
    }

    #[test]
    fn test_clean_result_creation() {
        let result = CleanResult {
            id: "test_id".to_string(),
            label: "Test Bulb".to_string(),
            status: "ok".to_string(),
            message: Some("Clean mode started".to_string()),
        };

        assert_eq!(result.id, "test_id");
        assert_eq!(result.label, "Test Bulb");
        assert_eq!(result.status, "ok");
        assert_eq!(result.message.unwrap(), "Clean mode started");
    }

    #[test]
    fn test_clean_response_creation() {
        let response = CleanResponse {
            results: vec![
                CleanResult {
                    id: "bulb1".to_string(),
                    label: "Bulb 1".to_string(),
                    status: "ok".to_string(),
                    message: None,
                },
                CleanResult {
                    id: "bulb2".to_string(),
                    label: "Bulb 2".to_string(),
                    status: "error".to_string(),
                    message: Some("Device does not support HEV".to_string()),
                },
            ],
        };

        assert_eq!(response.results.len(), 2);
        assert_eq!(response.results[0].status, "ok");
        assert_eq!(response.results[1].status, "error");
    }
}
