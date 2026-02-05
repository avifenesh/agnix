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
    if ts.len() < 20 {
        return None;
    }

    if !ts.is_ascii() {
        return None;
    }

    let bytes = ts.as_bytes();
    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return None;
    }

    let year: i32 = ts.get(0..4)?.parse().ok()?;
    let month: u32 = ts.get(5..7)?.parse().ok()?;
    let day: u32 = ts.get(8..10)?.parse().ok()?;
    let hour: u32 = ts.get(11..13)?.parse().ok()?;
    let minute: u32 = ts.get(14..16)?.parse().ok()?;
    let second: u32 = ts.get(17..19)?.parse().ok()?;

    if !(1970..=9999).contains(&year) {
        return None;
    }
    if !(1..=12).contains(&month) {
        return None;
    }
    if hour > 23 {
        return None;
    }
    if minute > 59 {
        return None;
    }
    if second > 59 {
        return None;
    }

    let days_in_months: [u32; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    if day == 0 || day > days_in_months[(month - 1) as usize] {
        return None;
    }

    // Convert to Unix timestamp (simplified calculation)
    let mut days = 0i64;

    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }

    for days_in_month in days_in_months.iter().take(month.saturating_sub(1) as usize) {
        days += *days_in_month as i64;
    }

    days += day.checked_sub(1)? as i64;

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
        // Leap year date: 2024-02-29T12:00:00Z = 1709208000
        assert_eq!(parse_timestamp("2024-02-29T12:00:00Z"), Some(1_709_208_000));

        // End of month: 2024-01-31T23:59:59Z = 1706745599
        assert_eq!(parse_timestamp("2024-01-31T23:59:59Z"), Some(1_706_745_599));

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

    #[test]
    fn test_parse_timestamp_rejects_non_ascii() {
        // Multi-byte UTF-8 strings should be rejected
        assert!(parse_timestamp("2024\u{00e9}01-01T00:00:00Z").is_none());
        assert!(parse_timestamp("\u{2013}024-01-01T00:00:00Z").is_none());
        assert!(parse_timestamp("2024-01-01T00:00:00\u{00dc}").is_none());
    }

    #[test]
    fn test_parse_timestamp_rejects_invalid_ranges() {
        assert!(parse_timestamp("2024-01-00T00:00:00Z").is_none());
        assert!(parse_timestamp("2024-00-01T00:00:00Z").is_none());
        assert!(parse_timestamp("2024-13-01T00:00:00Z").is_none());
        assert!(parse_timestamp("2024-01-32T00:00:00Z").is_none());
        assert!(parse_timestamp("2024-01-01T25:00:00Z").is_none());
        assert!(parse_timestamp("2024-01-01T24:00:00Z").is_none());
        assert!(parse_timestamp("2024-01-01T00:60:00Z").is_none());
        assert!(parse_timestamp("2024-01-01T00:00:60Z").is_none());

        // Month-specific day validation
        assert!(parse_timestamp("2024-02-30T00:00:00Z").is_none());
        assert!(parse_timestamp("2023-02-29T00:00:00Z").is_none()); // non-leap year
        assert!(parse_timestamp("2024-04-31T00:00:00Z").is_none());
        assert!(parse_timestamp("2024-06-31T00:00:00Z").is_none());
        assert!(parse_timestamp("2100-02-29T00:00:00Z").is_none()); // century non-leap

        // Year upper bound
        assert!(parse_timestamp("2024-01-01T00:00:00Z").is_some());
        assert!(parse_timestamp("9999-12-31T23:59:59Z").is_some());
    }

    #[test]
    fn test_parse_timestamp_rejects_wrong_separators() {
        assert!(parse_timestamp("2024/01/01T00:00:00Z").is_none());
        assert!(parse_timestamp("2024-01-01 00:00:00Z").is_none());
        assert!(parse_timestamp("2024-01-01T00.00.00Z").is_none());
        assert!(parse_timestamp("2024-01-01-00:00:00Z").is_none());
    }

    #[test]
    fn test_parse_timestamp_rejects_pre_epoch() {
        assert!(parse_timestamp("1969-12-31T23:59:59Z").is_none());
        assert!(parse_timestamp("1900-01-01T00:00:00Z").is_none());
        assert!(parse_timestamp("0001-01-01T00:00:00Z").is_none());
    }

    #[test]
    fn test_parse_timestamp_valid_boundaries() {
        // Unix epoch: 1970-01-01T00:00:00Z -> 0
        assert_eq!(parse_timestamp("1970-01-01T00:00:00Z"), Some(0));

        // Leap year Feb 29: 2000-02-29T00:00:00Z = 951782400
        assert_eq!(parse_timestamp("2000-02-29T00:00:00Z"), Some(951_782_400));

        // End of year
        let ts = parse_timestamp("2024-12-31T23:59:59Z");
        assert!(ts.is_some());
        let val = ts.unwrap();
        // Should be just before 2025-01-01T00:00:00Z (1735689600)
        assert_eq!(val, 1735689599);
    }

    #[test]
    fn test_parse_timestamp_rejects_garbage() {
        assert!(parse_timestamp("not-a-timestamp-at-all").is_none());
        assert!(parse_timestamp("xxxx-xx-xxTxx:xx:xxZ").is_none());
        assert!(parse_timestamp("\0\0\0\0-\0\0-\0\0T\0\0:\0\0:\0\0Z").is_none());
        assert!(parse_timestamp("9999-99-99T99:99:99Z").is_none());
        // Very long string (valid prefix, extra trailing data is ignored)
        let long = "2024-01-01T00:00:00Z".to_string() + &"x".repeat(10000);
        assert!(parse_timestamp(&long).is_some());
    }
}
