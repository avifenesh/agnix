//! HTTP client for submitting telemetry events.
//!
//! This module is only compiled when the `telemetry` feature is enabled.
//! When disabled, events are queued locally but not submitted.

use super::{TelemetryConfig, TelemetryEvent};
use std::io;
use std::time::Duration;

/// Telemetry submission client.
pub struct TelemetryClient {
    endpoint: String,
    installation_id: String,
    client: reqwest::blocking::Client,
}

impl TelemetryClient {
    /// Create a new telemetry client.
    pub fn new(config: &TelemetryConfig) -> io::Result<Self> {
        let installation_id = config.installation_id.clone().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "No installation ID configured")
        })?;

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(5))
            .connect_timeout(Duration::from_secs(3))
            .user_agent(format!("agnix/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("Failed to create HTTP client: {}", e),
                )
            })?;

        Ok(Self {
            endpoint: config.endpoint().to_string(),
            installation_id,
            client,
        })
    }

    /// Submit a batch of events.
    pub fn submit_batch(&self, events: &[TelemetryEvent]) -> io::Result<()> {
        if events.is_empty() {
            return Ok(());
        }

        // Validate privacy before submission
        for event in events {
            event
                .validate_privacy()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        }

        let payload = BatchPayload {
            installation_id: &self.installation_id,
            events,
        };

        let response = self
            .client
            .post(&self.endpoint)
            .json(&payload)
            .send()
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::ConnectionRefused,
                    format!("Failed to submit telemetry: {}", e),
                )
            })?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Telemetry server returned error: {}", response.status()),
            ))
        }
    }
}

/// Batch payload for submission.
#[derive(serde::Serialize)]
struct BatchPayload<'a> {
    installation_id: &'a str,
    events: &'a [TelemetryEvent],
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_client_requires_installation_id() {
        let config = TelemetryConfig::default();
        let result = TelemetryClient::new(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_batch_payload_serialization() {
        let events = vec![TelemetryEvent::ValidationRun(
            super::super::ValidationRunEvent {
                file_type_counts: HashMap::new(),
                rule_trigger_counts: HashMap::new(),
                error_count: 0,
                warning_count: 0,
                info_count: 0,
                duration_ms: 100,
                timestamp: "2024-01-01T00:00:00Z".to_string(),
            },
        )];

        let payload = BatchPayload {
            installation_id: "test-id",
            events: &events,
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"installation_id\":\"test-id\""));
        assert!(json.contains("\"events\":["));
    }
}
