//! Opt-in telemetry and usage analytics for agnix.
//!
//! This module provides privacy-first telemetry that helps improve agnix
//! by collecting aggregate usage statistics. Key privacy guarantees:
//!
//! - **Opt-in only**: Telemetry is disabled by default
//! - **No file paths**: Never collects paths, file names, or directory structure
//! - **No file contents**: Never collects any file contents or code
//! - **No identity**: Uses random installation ID, not tied to user identity
//! - **Aggregate counts only**: Collects file type counts, rule trigger counts
//! - **Environment-aware**: Respects DO_NOT_TRACK, CI, GITHUB_ACTIONS env vars
//!
//! # Usage
//!
//! ```no_run
//! use agnix_cli::telemetry::{TelemetryConfig, TelemetryEvent, record_validation};
//!
//! // Check if telemetry is enabled
//! let config = TelemetryConfig::load();
//! if config.is_enabled() {
//!     // Record a validation event
//!     record_validation(file_type_counts, rule_trigger_counts, duration);
//! }
//! ```

mod config;
mod events;

#[cfg(feature = "telemetry")]
mod client;

mod queue;

pub use config::TelemetryConfig;
pub use events::{is_valid_rule_id, TelemetryEvent, ValidationRunEvent};
pub use queue::EventQueue;

#[cfg(feature = "telemetry")]
pub use client::TelemetryClient;

use std::collections::HashMap;
use std::time::Duration;

#[cfg(feature = "telemetry")]
use std::thread;

/// Record a validation run event (non-blocking).
///
/// This function is safe to call even when telemetry is disabled -
/// it will simply return early without doing anything.
///
/// Events are saved to a local queue synchronously (fast file write),
/// then HTTP submission is attempted in a background thread. This ensures
/// events are never lost even if the CLI exits immediately.
pub fn record_validation(
    file_type_counts: HashMap<String, u32>,
    rule_trigger_counts: HashMap<String, u32>,
    error_count: u32,
    warning_count: u32,
    info_count: u32,
    duration_ms: u64,
) {
    // Check if telemetry is enabled first (fast path)
    let config = match TelemetryConfig::load() {
        Ok(c) => c,
        Err(_) => return,
    };

    if !config.is_enabled() {
        return;
    }

    // Create the event
    let event = TelemetryEvent::ValidationRun(ValidationRunEvent {
        file_type_counts,
        rule_trigger_counts,
        error_count,
        warning_count,
        info_count,
        duration_ms,
        timestamp: chrono_timestamp(),
    });

    // IMPORTANT: Save to queue synchronously to prevent event loss on CLI exit.
    // This is a fast file write (~1ms), so it won't noticeably block the CLI.
    let mut queue = match EventQueue::load() {
        Ok(q) => q,
        Err(_) => return,
    };

    // Push event to queue - if this fails, we can't do anything more
    if queue.push(event).is_ok() {
        // HTTP submission happens in background thread (only with telemetry feature).
        // If CLI exits before this completes, events remain safely queued for next run.
        #[cfg(feature = "telemetry")]
        {
            thread::spawn(move || {
                try_submit_queued_events(&config, &mut queue);
            });
        }
    }
}

/// Try to submit queued events via HTTP (called from background thread).
/// Events are already safely persisted to the queue before this is called.
#[cfg(feature = "telemetry")]
fn try_submit_queued_events(config: &TelemetryConfig, queue: &mut EventQueue) {
    if let Ok(client) = TelemetryClient::new(config) {
        let events = queue.take_batch(10);
        if !events.is_empty() {
            match client.submit_batch(&events) {
                Ok(_) => {
                    // Successfully submitted, remove events from queue
                    queue.remove_batch(events.len());
                    let _ = queue.save();
                }
                Err(_) => {
                    // Failed to submit, events stay in queue for retry on next run
                }
            }
        }
    }
}

/// Get current timestamp as ISO 8601 string.
fn chrono_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();

    // Format as ISO 8601 (simplified without external deps)
    let secs_per_day = 86400u64;
    let secs_per_hour = 3600u64;
    let secs_per_minute = 60u64;

    // Days since Unix epoch
    let days = now / secs_per_day;
    let remaining = now % secs_per_day;

    // Time components
    let hours = remaining / secs_per_hour;
    let remaining = remaining % secs_per_hour;
    let minutes = remaining / secs_per_minute;
    let seconds = remaining % secs_per_minute;

    // Calculate year, month, day from days since epoch
    // This is a simplified calculation
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
    fn test_chrono_timestamp_format() {
        let ts = chrono_timestamp();
        // Should match ISO 8601 format: YYYY-MM-DDTHH:MM:SSZ
        assert!(ts.len() == 20);
        assert!(ts.ends_with('Z'));
        assert!(ts.contains('T'));
        // Year should be reasonable (between 2020 and 2100)
        let year: i32 = ts[0..4].parse().unwrap();
        assert!(year >= 2020 && year <= 2100);
    }

    #[test]
    fn test_leap_year() {
        assert!(is_leap_year(2000)); // Divisible by 400
        assert!(!is_leap_year(1900)); // Divisible by 100 but not 400
        assert!(is_leap_year(2024)); // Divisible by 4
        assert!(!is_leap_year(2023)); // Not divisible by 4
    }
}
