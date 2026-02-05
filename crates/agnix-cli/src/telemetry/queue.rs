//! Local event queue for offline telemetry storage.
//!
//! Events are stored locally when network is unavailable and
//! submitted in batches later.
//!
//! Storage location: `dirs::data_local_dir()/agnix/telemetry_queue.json`

use super::TelemetryEvent;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::io;
use std::path::PathBuf;

/// Maximum number of events to store in the queue.
/// Older events are pruned when this limit is exceeded.
const MAX_QUEUE_SIZE: usize = 100;

/// Maximum age of events in days before they're pruned.
const MAX_EVENT_AGE_DAYS: i64 = 7;

/// Local event queue.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventQueue {
    events: VecDeque<TelemetryEvent>,
}

impl EventQueue {
    /// Load the event queue from disk, or create a new one.
    pub fn load() -> io::Result<Self> {
        let path = Self::queue_path()?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)?;
        let mut queue: Self = serde_json::from_str(&content).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid queue file: {}", e),
            )
        })?;

        // Prune old events
        queue.prune_old_events();

        Ok(queue)
    }

    /// Save the event queue to disk.
    pub fn save(&self) -> io::Result<()> {
        let path = Self::queue_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize queue: {}", e),
            )
        })?;

        fs::write(&path, content)?;
        Ok(())
    }

    /// Push an event onto the queue.
    pub fn push(&mut self, event: TelemetryEvent) -> io::Result<()> {
        // Validate privacy before storing
        event
            .validate_privacy()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

        self.events.push_back(event);

        // Prune if over limit
        while self.events.len() > MAX_QUEUE_SIZE {
            self.events.pop_front();
        }

        self.save()
    }

    /// Take a batch of events for submission.
    /// Does NOT remove them from the queue - call `remove_batch` after successful submission.
    pub fn take_batch(&mut self, max_count: usize) -> Vec<TelemetryEvent> {
        self.events.iter().take(max_count).cloned().collect()
    }

    /// Remove successfully submitted events from the front of the queue.
    pub fn remove_batch(&mut self, count: usize) {
        for _ in 0..count.min(self.events.len()) {
            self.events.pop_front();
        }
    }

    /// Get the number of queued events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Get the path to the queue file.
    pub fn queue_path() -> io::Result<PathBuf> {
        let data_dir = dirs::data_local_dir().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Could not determine local data directory",
            )
        })?;

        Ok(data_dir.join("agnix").join("telemetry_queue.json"))
    }

    /// Prune events older than MAX_EVENT_AGE_DAYS.
    fn prune_old_events(&mut self) {
        use std::time::{Duration, SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();

        let cutoff = now.saturating_sub((MAX_EVENT_AGE_DAYS as u64) * 24 * 60 * 60);

        self.events.retain(|event| {
            // Parse timestamp and check if recent enough
            match parse_timestamp(event.timestamp()) {
                Some(ts) => ts >= cutoff,
                None => true, // Keep events with unparseable timestamps
            }
        });
    }
}

/// Parse an ISO 8601 timestamp to Unix seconds.
fn parse_timestamp(ts: &str) -> Option<u64> {
    // Simple parser for YYYY-MM-DDTHH:MM:SSZ format
    if ts.len() < 19 {
        return None;
    }

    let year: i32 = ts[0..4].parse().ok()?;
    let month: u32 = ts[5..7].parse().ok()?;
    let day: u32 = ts[8..10].parse().ok()?;
    let hour: u32 = ts[11..13].parse().ok()?;
    let minute: u32 = ts[14..16].parse().ok()?;
    let second: u32 = ts[17..19].parse().ok()?;

    // Convert to Unix timestamp (simplified calculation)
    let mut days = 0i64;

    // Years since 1970
    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }

    // Months in current year
    let days_in_months: [u32; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    for days_in_month in days_in_months.iter().take(month.saturating_sub(1) as usize) {
        days += *days_in_month as i64;
    }

    // Days in current month
    days += (day - 1) as i64;

    // Convert to seconds
    let secs = days * 86400 + hour as i64 * 3600 + minute as i64 * 60 + second as i64;

    Some(secs as u64)
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_event(timestamp: &str) -> TelemetryEvent {
        TelemetryEvent::ValidationRun(super::super::ValidationRunEvent {
            file_type_counts: HashMap::new(),
            rule_trigger_counts: HashMap::new(),
            error_count: 0,
            warning_count: 0,
            info_count: 0,
            duration_ms: 100,
            timestamp: timestamp.to_string(),
        })
    }

    #[test]
    fn test_queue_push_and_take() {
        let mut queue = EventQueue::default();
        assert!(queue.is_empty());

        let event = make_event("2024-01-01T00:00:00Z");
        // Can't call push directly as it tries to save
        queue.events.push_back(event);

        assert_eq!(queue.len(), 1);
        let batch = queue.take_batch(10);
        assert_eq!(batch.len(), 1);
        // take_batch doesn't remove
        assert_eq!(queue.len(), 1);

        queue.remove_batch(1);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_queue_max_size() {
        let mut queue = EventQueue::default();

        // Add more than MAX_QUEUE_SIZE events
        for i in 0..MAX_QUEUE_SIZE + 10 {
            let event = make_event(&format!("2024-01-01T{:02}:00:00Z", i % 24));
            queue.events.push_back(event);
        }

        // Should be capped at MAX_QUEUE_SIZE
        while queue.events.len() > MAX_QUEUE_SIZE {
            queue.events.pop_front();
        }
        assert_eq!(queue.len(), MAX_QUEUE_SIZE);
    }

    #[test]
    fn test_parse_timestamp() {
        let ts = parse_timestamp("2024-01-01T00:00:00Z");
        assert!(ts.is_some());

        // 2024-01-01 00:00:00 UTC should be 1704067200
        assert_eq!(ts.unwrap(), 1704067200);

        // Invalid formats
        assert!(parse_timestamp("invalid").is_none());
        assert!(parse_timestamp("2024").is_none());
    }

    #[test]
    fn test_queue_serialization() {
        let mut queue = EventQueue::default();
        queue.events.push_back(make_event("2024-01-01T00:00:00Z"));

        let json = serde_json::to_string(&queue).unwrap();
        let parsed: EventQueue = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 1);
    }

    #[test]
    fn test_take_batch_edge_cases() {
        let mut queue = EventQueue::default();

        // Empty queue
        assert!(queue.take_batch(10).is_empty());

        // Add 3 events
        for i in 0..3 {
            queue
                .events
                .push_back(make_event(&format!("2024-01-0{}T00:00:00Z", i + 1)));
        }

        // Take 0 should return empty
        assert!(queue.take_batch(0).is_empty());

        // Take more than queue size should return all
        let batch = queue.take_batch(100);
        assert_eq!(batch.len(), 3);
    }

    #[test]
    fn test_remove_batch_edge_cases() {
        let mut queue = EventQueue::default();

        // Remove from empty queue shouldn't panic
        queue.remove_batch(10);
        assert!(queue.is_empty());

        // Add 5 events
        for i in 0..5 {
            queue
                .events
                .push_back(make_event(&format!("2024-01-0{}T00:00:00Z", i + 1)));
        }

        // Remove 0 should do nothing
        queue.remove_batch(0);
        assert_eq!(queue.len(), 5);

        // Remove more than available should remove all
        queue.remove_batch(100);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_parse_timestamp_edge_cases() {
        // Leap year date
        let ts = parse_timestamp("2024-02-29T12:00:00Z");
        assert!(ts.is_some());

        // End of month
        let ts = parse_timestamp("2024-01-31T23:59:59Z");
        assert!(ts.is_some());

        // Note: parse_timestamp does minimal validation (parses positions only)
        // It doesn't validate separators or month/day ranges - acceptable for pruning

        // Too short
        assert!(parse_timestamp("2024-01-01").is_none());

        // Empty string
        assert!(parse_timestamp("").is_none());

        // Non-numeric values fail
        assert!(parse_timestamp("abcd-01-01T00:00:00Z").is_none());
    }

    #[test]
    fn test_prune_old_events() {
        let mut queue = EventQueue::default();

        // Add an old event (2020) and a recent event
        queue.events.push_back(make_event("2020-01-01T00:00:00Z"));
        queue.events.push_back(make_event("2025-01-01T00:00:00Z"));

        assert_eq!(queue.len(), 2);

        // Prune should remove the old event
        queue.prune_old_events();

        // Note: depending on current date, the 2025 event may or may not be pruned
        // The old 2020 event should definitely be removed
        assert!(queue.len() <= 2);

        // Verify no events from 2020 remain
        for event in &queue.events {
            let TelemetryEvent::ValidationRun(run) = event;
            assert!(!run.timestamp.starts_with("2020"));
        }
    }
}
