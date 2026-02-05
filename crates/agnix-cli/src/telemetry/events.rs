//! Privacy-safe telemetry event definitions.
//!
//! All events are designed to collect only aggregate statistics
//! that cannot identify users or reveal their code.
//!
//! # Privacy Guarantees
//!
//! - **No paths**: File paths are NEVER included
//! - **No contents**: File contents are NEVER included
//! - **No identity**: No user-identifying information
//! - **Aggregate only**: Only counts and durations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Telemetry event types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TelemetryEvent {
    /// A validation run completed.
    #[serde(rename = "validation_run")]
    ValidationRun(ValidationRunEvent),
}

/// Event recorded when a validation run completes.
///
/// Privacy: Contains only aggregate counts, no identifying information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRunEvent {
    /// Count of files validated per file type.
    /// Keys are file type names (e.g., "skill", "claude_md", "mcp").
    /// Values are counts.
    ///
    /// Privacy: File types only, not paths or contents.
    pub file_type_counts: HashMap<String, u32>,

    /// Count of rule triggers per rule ID.
    /// Keys are rule IDs (e.g., "AS-001", "CC-HK-002").
    /// Values are counts.
    ///
    /// Privacy: Rule IDs only, not diagnostic details.
    pub rule_trigger_counts: HashMap<String, u32>,

    /// Total number of errors found.
    pub error_count: u32,

    /// Total number of warnings found.
    pub warning_count: u32,

    /// Total number of info diagnostics found.
    pub info_count: u32,

    /// Validation duration in milliseconds.
    pub duration_ms: u64,

    /// When the validation occurred (ISO 8601).
    pub timestamp: String,
}

impl TelemetryEvent {
    /// Get the event type name.
    pub fn event_type(&self) -> &'static str {
        match self {
            TelemetryEvent::ValidationRun(_) => "validation_run",
        }
    }

    /// Get the timestamp for this event.
    pub fn timestamp(&self) -> &str {
        match self {
            TelemetryEvent::ValidationRun(e) => &e.timestamp,
        }
    }

    /// Validate that this event contains no privacy-sensitive data.
    ///
    /// This is a defense-in-depth check to ensure we never accidentally
    /// include paths or other sensitive data.
    pub fn validate_privacy(&self) -> Result<(), PrivacyViolation> {
        match self {
            TelemetryEvent::ValidationRun(e) => {
                // Check file type keys don't look like paths
                for key in e.file_type_counts.keys() {
                    if looks_like_path(key) {
                        return Err(PrivacyViolation::PathLikeKey(key.clone()));
                    }
                }

                // Check rule IDs are valid format
                for key in e.rule_trigger_counts.keys() {
                    if !is_valid_rule_id(key) {
                        return Err(PrivacyViolation::InvalidRuleId(key.clone()));
                    }
                }

                Ok(())
            }
        }
    }
}

/// Privacy violation detected in telemetry event.
#[derive(Debug, Clone)]
pub enum PrivacyViolation {
    /// Key looks like a file path.
    PathLikeKey(String),
    /// Invalid rule ID format.
    InvalidRuleId(String),
}

impl std::fmt::Display for PrivacyViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrivacyViolation::PathLikeKey(key) => {
                write!(f, "Key looks like a path (privacy violation): {}", key)
            }
            PrivacyViolation::InvalidRuleId(id) => {
                write!(f, "Invalid rule ID format: {}", id)
            }
        }
    }
}

impl std::error::Error for PrivacyViolation {}

/// Check if a string looks like a file path.
fn looks_like_path(s: &str) -> bool {
    // Check for path separators (strongest indicators)
    s.contains('/')
        || s.contains('\\')
        // Check for file extensions at end of string (not contains, to avoid false positives
        // like "claude_md_parser" being flagged as a path)
        || s.ends_with(".md")
        || s.ends_with(".json")
        || s.ends_with(".toml")
        || s.ends_with(".yaml")
        || s.ends_with(".yml")
        // Hidden files/directories
        || s.starts_with('.')
        // Home directory reference
        || s.starts_with('~')
        // Windows drive letter (e.g., "C:")
        || (s.len() > 1 && s.chars().nth(1) == Some(':'))
}

/// Check if a string is a valid rule ID format.
///
/// Rule IDs are in format: XX-NNN or XX-YY-NNN
/// Examples: AS-001, CC-HK-001, MCP-002
pub fn is_valid_rule_id(s: &str) -> bool {
    // Rule IDs are in format: XX-NNN or XX-YY-NNN
    // Examples: AS-001, CC-HK-001, MCP-002

    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return false;
    }

    // First part(s) should be uppercase letters
    for part in &parts[..parts.len() - 1] {
        if part.is_empty() || !part.chars().all(|c| c.is_ascii_uppercase()) {
            return false;
        }
    }

    // Last part should be digits
    let last = parts.last().unwrap();
    if last.is_empty() || !last.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_looks_like_path() {
        assert!(looks_like_path("/home/user/file.md"));
        assert!(looks_like_path("C:\\Users\\file.json"));
        assert!(looks_like_path("./relative/path"));
        assert!(looks_like_path("CLAUDE.md"));
        assert!(looks_like_path("~/.config"));

        assert!(!looks_like_path("skill"));
        assert!(!looks_like_path("claude_md"));
        assert!(!looks_like_path("mcp"));
    }

    #[test]
    fn test_is_valid_rule_id() {
        assert!(is_valid_rule_id("AS-001"));
        assert!(is_valid_rule_id("CC-HK-001"));
        assert!(is_valid_rule_id("MCP-002"));
        assert!(is_valid_rule_id("XP-001"));

        assert!(!is_valid_rule_id(""));
        assert!(!is_valid_rule_id("invalid"));
        assert!(!is_valid_rule_id("AS-"));
        assert!(!is_valid_rule_id("-001"));
        assert!(!is_valid_rule_id("as-001")); // lowercase
        assert!(!is_valid_rule_id("AS-abc")); // letters in number
    }

    #[test]
    fn test_validation_run_event_serialization() {
        let event = ValidationRunEvent {
            file_type_counts: [("skill".to_string(), 5), ("mcp".to_string(), 3)]
                .into_iter()
                .collect(),
            rule_trigger_counts: [("AS-001".to_string(), 2)].into_iter().collect(),
            error_count: 1,
            warning_count: 2,
            info_count: 0,
            duration_ms: 150,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"error_count\":1"));
        assert!(json.contains("\"duration_ms\":150"));

        let parsed: ValidationRunEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.error_count, 1);
        assert_eq!(parsed.duration_ms, 150);
    }

    #[test]
    fn test_telemetry_event_serialization() {
        let event = TelemetryEvent::ValidationRun(ValidationRunEvent {
            file_type_counts: HashMap::new(),
            rule_trigger_counts: HashMap::new(),
            error_count: 0,
            warning_count: 0,
            info_count: 0,
            duration_ms: 100,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        });

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"validation_run\""));
    }

    #[test]
    fn test_privacy_validation_passes() {
        let event = TelemetryEvent::ValidationRun(ValidationRunEvent {
            file_type_counts: [("skill".to_string(), 5)].into_iter().collect(),
            rule_trigger_counts: [("AS-001".to_string(), 2)].into_iter().collect(),
            error_count: 1,
            warning_count: 0,
            info_count: 0,
            duration_ms: 100,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        });

        assert!(event.validate_privacy().is_ok());
    }

    #[test]
    fn test_privacy_validation_catches_path() {
        let event = TelemetryEvent::ValidationRun(ValidationRunEvent {
            file_type_counts: [("/home/user/SKILL.md".to_string(), 1)]
                .into_iter()
                .collect(),
            rule_trigger_counts: HashMap::new(),
            error_count: 0,
            warning_count: 0,
            info_count: 0,
            duration_ms: 100,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        });

        assert!(matches!(
            event.validate_privacy(),
            Err(PrivacyViolation::PathLikeKey(_))
        ));
    }

    #[test]
    fn test_privacy_validation_catches_invalid_rule() {
        let event = TelemetryEvent::ValidationRun(ValidationRunEvent {
            file_type_counts: HashMap::new(),
            rule_trigger_counts: [("not-a-rule".to_string(), 1)].into_iter().collect(),
            error_count: 0,
            warning_count: 0,
            info_count: 0,
            duration_ms: 100,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        });

        assert!(matches!(
            event.validate_privacy(),
            Err(PrivacyViolation::InvalidRuleId(_))
        ));
    }

    #[test]
    fn test_privacy_validation_with_empty_hashmaps() {
        let event = TelemetryEvent::ValidationRun(ValidationRunEvent {
            file_type_counts: HashMap::new(),
            rule_trigger_counts: HashMap::new(),
            error_count: 0,
            warning_count: 0,
            info_count: 0,
            duration_ms: 0,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        });

        assert!(
            event.validate_privacy().is_ok(),
            "Empty hashmaps should pass validation"
        );
    }

    #[test]
    fn test_looks_like_path_edge_cases() {
        // Windows UNC paths
        assert!(looks_like_path("\\\\server\\share"));
        // Backslash paths
        assert!(looks_like_path("path\\to\\file"));
        // Single drive letter without path
        assert!(looks_like_path("C:"));
        // Hidden files (starts with .)
        assert!(looks_like_path(".gitignore"));

        // Valid file type names that should NOT trigger (false positive prevention)
        assert!(!looks_like_path("rust"));
        assert!(!looks_like_path("typescript"));
        assert!(!looks_like_path("json_schema")); // Contains "json" but doesn't end with .json
        assert!(!looks_like_path("markdown_file")); // Contains "md" but doesn't end with .md
        assert!(!looks_like_path("claude_md_parser")); // Contains ".md" but doesn't END with it
        assert!(!looks_like_path("my_json_handler")); // Contains "json" substring
                                                      // Note: "file.tar.gz" would contain ".gz" but our detector doesn't catch that
                                                      // This is acceptable as .gz isn't a path, it's an extension
    }

    #[test]
    fn test_is_valid_rule_id_edge_cases() {
        // Single character prefix
        assert!(is_valid_rule_id("A-001"));
        // Very long ID
        assert!(is_valid_rule_id("ABCDEF-12345"));
        // Three-part ID
        assert!(is_valid_rule_id("CC-SK-001"));

        // Invalid: no number part
        assert!(!is_valid_rule_id("AS"));
        // Invalid: leading zero is still valid format
        assert!(is_valid_rule_id("AS-001")); // 001 is valid
                                             // Invalid: too many dashes
        assert!(!is_valid_rule_id("A-B-C-001"));
        // Invalid: empty parts
        assert!(!is_valid_rule_id("--001"));
        assert!(!is_valid_rule_id("AS--001"));
    }

    #[test]
    fn test_event_type_method() {
        let validation_event = TelemetryEvent::ValidationRun(ValidationRunEvent {
            file_type_counts: HashMap::new(),
            rule_trigger_counts: HashMap::new(),
            error_count: 0,
            warning_count: 0,
            info_count: 0,
            duration_ms: 0,
            timestamp: "".to_string(),
        });
        assert_eq!(validation_event.event_type(), "validation_run");
    }
}
