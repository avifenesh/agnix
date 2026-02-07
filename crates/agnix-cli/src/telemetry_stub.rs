//! Stub telemetry facade for builds without the `telemetry` feature.
//!
//! This module keeps the CLI telemetry UX (status/enable/disable/config parsing)
//! while avoiding compilation of HTTP submission code and queue machinery.

use std::collections::HashMap;

#[path = "telemetry/config.rs"]
mod config;
#[path = "telemetry/shared.rs"]
mod shared;

pub use config::TelemetryConfig;
pub use shared::is_valid_rule_id;

pub fn record_validation(
    _file_type_counts: HashMap<String, u32>,
    _rule_trigger_counts: HashMap<String, u32>,
    _error_count: u32,
    _warning_count: u32,
    _info_count: u32,
    _duration_ms: u64,
) {
    // No-op when telemetry submission is not compiled in.
}

// Used by telemetry/config.rs for consent timestamps.
fn chrono_timestamp() -> String {
    shared::chrono_timestamp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_format_is_iso_8601() {
        let ts = chrono_timestamp();
        assert_eq!(ts.len(), 20);
        assert!(ts.ends_with('Z'));
    }

    #[test]
    fn record_validation_is_safe_noop() {
        record_validation(HashMap::new(), HashMap::new(), 0, 0, 0, 10);
    }

    #[test]
    fn rule_id_validation_matches_expected_shape() {
        assert!(is_valid_rule_id("AS-001"));
        assert!(is_valid_rule_id("CC-HK-001"));
        assert!(!is_valid_rule_id("as-001"));
        assert!(!is_valid_rule_id("AS"));
    }
}
