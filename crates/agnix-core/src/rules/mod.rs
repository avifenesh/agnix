//! Validation rules

pub mod agent;
pub mod agents_md;
pub mod claude_md;
pub mod claude_rules;
pub mod cline;
pub mod codex;
pub mod copilot;
pub mod cross_platform;
pub mod cursor;
pub mod gemini_md;
pub mod hooks;
pub mod imports;
pub mod mcp;
pub mod opencode;
pub mod plugin;
pub mod prompt;
pub mod skill;
pub mod xml;

use crate::{config::LintConfig, diagnostics::Diagnostic};
use std::path::Path;

/// Trait for file validators
pub trait Validator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic>;
}

/// Find the closest valid value for an invalid input.
/// Returns an exact case-insensitive match first, then a substring match,
/// or None if no plausible match is found.
pub(crate) fn find_closest_value<'a>(invalid: &str, valid_values: &[&'a str]) -> Option<&'a str> {
    let lower = invalid.to_lowercase();
    // Skip empty strings - no meaningful match possible
    if lower.is_empty() {
        return None;
    }
    // Case-insensitive exact match
    for &v in valid_values {
        if v.to_lowercase() == lower {
            return Some(v);
        }
    }
    // Substring match (invalid contains valid or valid contains invalid)
    valid_values
        .iter()
        .find(|&&v| v.to_lowercase().contains(&lower) || lower.contains(&v.to_lowercase()))
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_closest_value_exact_case_insensitive() {
        assert_eq!(
            find_closest_value("Stdio", &["stdio", "http", "sse"]),
            Some("stdio")
        );
        assert_eq!(
            find_closest_value("HTTP", &["stdio", "http", "sse"]),
            Some("http")
        );
    }

    #[test]
    fn test_find_closest_value_substring_match() {
        assert_eq!(
            find_closest_value("code", &["code-review", "coding-agent"]),
            Some("code-review")
        );
        assert_eq!(
            find_closest_value("coding-agent-v2", &["code-review", "coding-agent"]),
            Some("coding-agent")
        );
    }

    #[test]
    fn test_find_closest_value_no_match() {
        assert_eq!(
            find_closest_value("nonsense", &["stdio", "http", "sse"]),
            None
        );
        assert_eq!(
            find_closest_value("xyz", &["code-review", "coding-agent"]),
            None
        );
    }

    #[test]
    fn test_find_closest_value_empty_input() {
        assert_eq!(find_closest_value("", &["stdio", "http", "sse"]), None);
    }

    #[test]
    fn test_find_closest_value_exact_preferred_over_substring() {
        // "user" matches exactly, not as substring of "user-project"
        assert_eq!(
            find_closest_value("User", &["user", "project", "local"]),
            Some("user")
        );
    }
}
