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
mod shared;

#[cfg(feature = "telemetry")]
mod client;

mod queue;

pub use config::TelemetryConfig;
pub use events::{TelemetryEvent, ValidationRunEvent, is_valid_rule_id};
pub use queue::EventQueue;

#[cfg(feature = "telemetry")]
pub use client::TelemetryClient;

use std::collections::HashMap;

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
    shared::chrono_timestamp()
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
        assert!((2020..=2100).contains(&year));
    }
}
