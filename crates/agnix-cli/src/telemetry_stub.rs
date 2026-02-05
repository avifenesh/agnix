//! Stub telemetry facade for builds without the `telemetry` feature.
//!
//! This module keeps the CLI telemetry UX (status/enable/disable/config parsing)
//! while avoiding compilation of HTTP submission code and queue machinery.

use std::collections::HashMap;
use std::time::Duration;

#[path = "telemetry/config.rs"]
#[allow(dead_code)]
mod config;

pub use config::TelemetryConfig;

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

pub fn is_valid_rule_id(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return false;
    }

    for part in &parts[..parts.len() - 1] {
        if part.is_empty() || !part.chars().all(|c| c.is_ascii_uppercase()) {
            return false;
        }
    }

    let last = parts.last().unwrap_or(&"");
    !last.is_empty() && last.chars().all(|c| c.is_ascii_digit())
}

// Used by telemetry/config.rs for consent timestamps.
fn chrono_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();

    let secs_per_day = 86400u64;
    let secs_per_hour = 3600u64;
    let secs_per_minute = 60u64;

    let days = now / secs_per_day;
    let remaining = now % secs_per_day;

    let hours = remaining / secs_per_hour;
    let remaining = remaining % secs_per_hour;
    let minutes = remaining / secs_per_minute;
    let seconds = remaining % secs_per_minute;

    let mut year = 1970i32;
    let mut remaining_days = days as i32;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let days_in_months: [i32; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for days_in_month in days_in_months.iter() {
        if remaining_days < *days_in_month {
            break;
        }
        remaining_days -= days_in_month;
        month += 1;
    }
    let day = remaining_days + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
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
