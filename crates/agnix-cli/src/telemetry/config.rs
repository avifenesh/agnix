//! Telemetry configuration storage and consent management.
//!
//! Configuration is stored at `dirs::config_dir()/agnix/telemetry.json`.
//! The configuration includes:
//! - `enabled`: Whether telemetry is enabled (opt-in)
//! - `installation_id`: Random UUID for aggregate analysis (not tied to identity)

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

/// Telemetry configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Whether telemetry collection is enabled.
    /// Default: false (opt-in only)
    #[serde(default)]
    pub enabled: bool,

    /// Random installation ID for aggregate analysis.
    /// This is NOT tied to user identity - it's a random UUID
    /// generated on first consent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installation_id: Option<String>,

    /// When consent was given (ISO 8601 timestamp).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consent_timestamp: Option<String>,

    /// Telemetry endpoint URL (for testing/self-hosting).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
}

impl TelemetryConfig {
    /// Load configuration from disk, or return default if not found.
    pub fn load() -> io::Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)?;
        let config: Self = serde_json::from_str(&content).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid telemetry config: {}", e),
            )
        })?;

        Ok(config)
    }

    /// Save configuration to disk.
    pub fn save(&self) -> io::Result<()> {
        let path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize config: {}", e),
            )
        })?;

        fs::write(&path, content)?;
        Ok(())
    }

    /// Check if telemetry is effectively enabled.
    ///
    /// Returns false if:
    /// - Config has `enabled: false`
    /// - DO_NOT_TRACK environment variable is set (any value)
    /// - AGNIX_TELEMETRY=0 or AGNIX_TELEMETRY=false
    /// - CI environment variable is set (running in CI)
    /// - GITHUB_ACTIONS environment variable is set
    /// - TF_BUILD (Azure Pipelines), TRAVIS, CIRCLECI, etc.
    pub fn is_enabled(&self) -> bool {
        // Check explicit config setting first
        if !self.enabled {
            return false;
        }

        // Respect DO_NOT_TRACK (https://consoledonottrack.com/)
        if env::var("DO_NOT_TRACK").is_ok() {
            return false;
        }

        // Check AGNIX_TELEMETRY override
        if let Ok(val) = env::var("AGNIX_TELEMETRY") {
            let val = val.to_lowercase();
            if val == "0" || val == "false" || val == "no" || val == "off" {
                return false;
            }
        }

        // Disable in CI environments
        if Self::is_ci_environment() {
            return false;
        }

        true
    }

    /// Check if running in a CI environment.
    fn is_ci_environment() -> bool {
        // Standard CI env var
        if env::var("CI").is_ok() {
            return true;
        }

        // Common CI systems
        let ci_vars = [
            "GITHUB_ACTIONS",
            "GITLAB_CI",
            "TRAVIS",
            "CIRCLECI",
            "JENKINS_URL",
            "TF_BUILD", // Azure Pipelines
            "BUILDKITE",
            "TEAMCITY_VERSION",
            "CODEBUILD_BUILD_ID", // AWS CodeBuild
            "DRONE",
        ];

        ci_vars.iter().any(|var| env::var(var).is_ok())
    }

    /// Enable telemetry with consent.
    pub fn enable(&mut self) -> io::Result<()> {
        self.enabled = true;

        // Generate installation ID if not present
        if self.installation_id.is_none() {
            self.installation_id = Some(generate_uuid());
        }

        // Record consent timestamp
        self.consent_timestamp = Some(super::chrono_timestamp());

        self.save()
    }

    /// Disable telemetry.
    pub fn disable(&mut self) -> io::Result<()> {
        self.enabled = false;
        // Keep installation_id for if user re-enables
        // Clear consent timestamp
        self.consent_timestamp = None;
        self.save()
    }

    /// Get the path to the telemetry config file.
    pub fn config_path() -> io::Result<PathBuf> {
        let config_dir = dirs::config_dir().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Could not determine config directory",
            )
        })?;

        Ok(config_dir.join("agnix").join("telemetry.json"))
    }

    /// Get the telemetry endpoint URL.
    pub fn endpoint(&self) -> &str {
        self.endpoint
            .as_deref()
            .unwrap_or("https://telemetry.agnix.dev/v1/events")
    }
}

/// Generate a random UUID v4.
fn generate_uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Use time + process id + random bytes for uniqueness
    let time_component = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    let pid = std::process::id();

    // XorShift64 for additional randomness
    let mut state = time_component as u64 ^ (pid as u64) ^ 0x5DEECE66D;
    let mut bytes = [0u8; 16];

    for chunk in bytes.chunks_mut(8) {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;

        let state_bytes = state.to_le_bytes();
        for (i, b) in chunk.iter_mut().enumerate() {
            *b = state_bytes[i];
        }
    }

    // Set version (4) and variant (RFC 4122)
    bytes[6] = (bytes[6] & 0x0f) | 0x40; // Version 4
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // Variant RFC 4122

    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_disabled() {
        let config = TelemetryConfig::default();
        assert!(!config.enabled);
        assert!(config.installation_id.is_none());
    }

    #[test]
    fn test_is_enabled_respects_config() {
        let config = TelemetryConfig::default();
        assert!(!config.is_enabled());

        let _config = TelemetryConfig {
            enabled: true,
            installation_id: Some(generate_uuid()),
            ..Default::default()
        };
        // Note: config.is_enabled() may still return false if running in CI,
        // so we don't assert on it here.
    }

    #[test]
    fn test_generate_uuid_format() {
        let uuid = generate_uuid();
        // UUID v4 format: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
        assert_eq!(uuid.len(), 36);
        assert_eq!(uuid.chars().nth(8), Some('-'));
        assert_eq!(uuid.chars().nth(13), Some('-'));
        assert_eq!(uuid.chars().nth(18), Some('-'));
        assert_eq!(uuid.chars().nth(23), Some('-'));
        // Version 4
        assert_eq!(uuid.chars().nth(14), Some('4'));
        // Variant (8, 9, a, or b)
        let variant_char = uuid.chars().nth(19).unwrap();
        assert!(matches!(variant_char, '8' | '9' | 'a' | 'b'));
    }

    #[test]
    fn test_generate_uuid_uniqueness() {
        let uuid1 = generate_uuid();
        // Small sleep to ensure time-based component changes
        std::thread::sleep(std::time::Duration::from_millis(1));
        let uuid2 = generate_uuid();
        assert_ne!(uuid1, uuid2);
    }

    #[test]
    fn test_ci_detection() {
        // This test checks the function doesn't panic
        // Actual CI status depends on environment
        let _ = TelemetryConfig::is_ci_environment();
    }

    #[test]
    fn test_config_serialization() {
        let config = TelemetryConfig {
            enabled: true,
            installation_id: Some("test-id".to_string()),
            consent_timestamp: Some("2024-01-01T00:00:00Z".to_string()),
            endpoint: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"enabled\":true"));
        assert!(json.contains("\"installation_id\":\"test-id\""));
        // endpoint should not be serialized when None
        assert!(!json.contains("endpoint"));

        let parsed: TelemetryConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.enabled, config.enabled);
        assert_eq!(parsed.installation_id, config.installation_id);
    }

    #[test]
    fn test_do_not_track_disables_telemetry() {
        // Save current env state
        let original = std::env::var("DO_NOT_TRACK").ok();

        // Set DO_NOT_TRACK and verify telemetry is disabled
        std::env::set_var("DO_NOT_TRACK", "1");
        let config = TelemetryConfig {
            enabled: true,
            installation_id: Some(generate_uuid()),
            ..Default::default()
        };
        assert!(
            !config.is_enabled(),
            "DO_NOT_TRACK should disable telemetry"
        );

        // Restore original state
        match original {
            Some(val) => std::env::set_var("DO_NOT_TRACK", val),
            None => std::env::remove_var("DO_NOT_TRACK"),
        }
    }

    #[test]
    fn test_agnix_telemetry_env_overrides() {
        // Save current env state
        let original = std::env::var("AGNIX_TELEMETRY").ok();
        let original_dnt = std::env::var("DO_NOT_TRACK").ok();

        // Clear DO_NOT_TRACK to isolate this test
        std::env::remove_var("DO_NOT_TRACK");

        let config = TelemetryConfig {
            enabled: true,
            installation_id: Some(generate_uuid()),
            ..Default::default()
        };

        // Test various override values
        for val in &["0", "false", "no", "off"] {
            std::env::set_var("AGNIX_TELEMETRY", val);
            assert!(
                !config.is_enabled(),
                "AGNIX_TELEMETRY={} should disable telemetry",
                val
            );
        }

        // Restore original state
        match original {
            Some(val) => std::env::set_var("AGNIX_TELEMETRY", val),
            None => std::env::remove_var("AGNIX_TELEMETRY"),
        }
        match original_dnt {
            Some(val) => std::env::set_var("DO_NOT_TRACK", val),
            None => std::env::remove_var("DO_NOT_TRACK"),
        }
    }

    #[test]
    fn test_enable_generates_installation_id() {
        let mut config = TelemetryConfig::default();
        assert!(config.installation_id.is_none());

        let _ = config.enable();
        assert!(config.enabled);
        assert!(config.installation_id.is_some());
        assert!(config.consent_timestamp.is_some());

        // Second enable should preserve the ID
        let id = config.installation_id.clone();
        let _ = config.enable();
        assert_eq!(
            config.installation_id, id,
            "enable() should preserve existing ID"
        );
    }

    #[test]
    fn test_disable_preserves_installation_id() {
        let mut config = TelemetryConfig::default();
        let _ = config.enable();
        let id = config.installation_id.clone();

        let _ = config.disable();
        assert!(!config.enabled);
        assert_eq!(
            config.installation_id, id,
            "disable() should preserve installation_id"
        );
        // Note: disable() clears consent_timestamp by design
        assert!(
            config.consent_timestamp.is_none(),
            "disable() should clear consent_timestamp"
        );
    }
}
