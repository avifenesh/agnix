//! Shared telemetry helpers used by both feature-enabled and stub builds.
//!
//! These remain dependency-free to avoid adding crate weight to the default
//! CLI binary while keeping behavior consistent across telemetry paths.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Get current timestamp as ISO 8601 string (YYYY-MM-DDTHH:MM:SSZ).
pub fn chrono_timestamp() -> String {
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
    for days_in_month in &days_in_months {
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

/// Check if a string is a valid rule ID format.
///
/// Rule IDs are in format: XX-NNN or XX-YY-NNN
/// Examples: AS-001, CC-HK-001, MCP-002
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

    let last = parts.last().expect("guaranteed by bounds check above");
    !last.is_empty() && last.chars().all(|c| c.is_ascii_digit())
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
        assert!(ts.contains('T'));
    }

    #[test]
    fn leap_year_logic_is_correct() {
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2023));
    }

    #[test]
    fn rule_id_validation_matches_expected_shape() {
        assert!(is_valid_rule_id("AS-001"));
        assert!(is_valid_rule_id("CC-HK-001"));
        assert!(is_valid_rule_id("MCP-002"));
        assert!(!is_valid_rule_id("as-001"));
        assert!(!is_valid_rule_id("AS"));
    }
}
