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

/// Trait for frontmatter types that support value range finding.
/// Both ParsedFrontmatter (copilot) and ParsedMdcFrontmatter (cursor) implement this.
pub(crate) trait FrontmatterRanges {
    fn raw_content(&self) -> &str;
    fn start_line(&self) -> usize;
}

/// Find the byte range of a line in content (1-indexed line numbers).
/// Returns (start_byte, end_byte) including the newline character.
pub(crate) fn line_byte_range(content: &str, line_number: usize) -> Option<(usize, usize)> {
    if line_number == 0 {
        return None;
    }

    let mut current_line = 1usize;
    let mut line_start = 0usize;

    for (idx, ch) in content.char_indices() {
        if current_line == line_number && ch == '\n' {
            return Some((line_start, idx + 1));
        }
        if ch == '\n' {
            current_line += 1;
            line_start = idx + 1;
        }
    }

    if current_line == line_number {
        Some((line_start, content.len()))
    } else {
        None
    }
}

/// Find the byte range of a YAML value for a given key in frontmatter.
/// Returns the range including quotes if the value is quoted.
/// Handles `#` comments correctly (ignores them inside quotes).
pub(crate) fn find_yaml_value_range<T: FrontmatterRanges>(
    full_content: &str,
    parsed: &T,
    key: &str,
    include_quotes: bool,
) -> Option<(usize, usize)> {
    for (idx, line) in parsed.raw_content().lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(key) {
            if let Some(after_colon) = rest.trim_start().strip_prefix(':') {
                let after_colon_trimmed = after_colon.trim();

                // Handle quoted values (# inside quotes is literal, not a comment)
                let value_str = if let Some(inner) = after_colon_trimmed.strip_prefix('"') {
                    if let Some(end_quote_idx) = inner.find('"') {
                        let quoted = &after_colon_trimmed[..end_quote_idx + 2];
                        if include_quotes {
                            quoted
                        } else {
                            &quoted[1..quoted.len() - 1]
                        }
                    } else {
                        after_colon_trimmed
                    }
                } else if let Some(inner) = after_colon_trimmed.strip_prefix('\'') {
                    if let Some(end_quote_idx) = inner.find('\'') {
                        let quoted = &after_colon_trimmed[..end_quote_idx + 2];
                        if include_quotes {
                            quoted
                        } else {
                            &quoted[1..quoted.len() - 1]
                        }
                    } else {
                        after_colon_trimmed
                    }
                } else {
                    // Unquoted value: strip comments
                    after_colon_trimmed.split('#').next().unwrap_or("").trim()
                };

                if value_str.is_empty() {
                    continue;
                }
                let line_num = parsed.start_line() + 1 + idx;
                let (line_start, _) = line_byte_range(full_content, line_num)?;
                let line_content = &full_content[line_start..];
                let val_offset = line_content.find(value_str)?;
                let abs_start = line_start + val_offset;
                let abs_end = abs_start + value_str.len();
                return Some((abs_start, abs_end));
            }
        }
    }
    None
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
