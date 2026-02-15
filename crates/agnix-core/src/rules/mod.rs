//! Validation rules

pub mod agent;
pub mod agents_md;
pub mod amp;
pub mod claude_md;
pub mod claude_rules;
pub mod cline;
pub mod codex;
pub mod copilot;
pub mod cross_platform;
pub mod cursor;
pub mod gemini_extension;
pub mod gemini_ignore;
pub mod gemini_md;
pub mod gemini_settings;
pub mod hooks;
pub mod imports;
pub mod kiro_steering;
pub mod mcp;
pub mod opencode;
pub mod per_client_skill;
pub mod plugin;
pub mod prompt;
pub mod roo;
pub mod skill;
pub mod windsurf;
pub mod xml;

use crate::{config::LintConfig, diagnostics::Diagnostic};
use std::path::Path;

/// Extract the short (unqualified) type name from `std::any::type_name`.
///
/// Given a fully-qualified path like `"agnix_core::rules::skill::SkillValidator"`,
/// returns `"SkillValidator"`. For generic types like `"Wrapper<foo::Bar>"`,
/// strips the generic suffix first, yielding `"Wrapper"`.
/// Falls back to the full name when no `::` separator is found.
fn short_type_name<T: ?Sized + 'static>() -> &'static str {
    let full = std::any::type_name::<T>();
    // Strip generic suffix (e.g., "Wrapper<foo::Bar>" -> "Wrapper")
    let base = full.split('<').next().unwrap_or(full);
    base.rsplit("::").next().unwrap_or(base)
}

/// Metadata for a validator, providing introspection capabilities.
///
/// Returned by [`Validator::metadata`] to expose the validator's name and
/// the set of rule IDs it can emit during validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidatorMetadata {
    /// Human-readable validator name (e.g. `"SkillValidator"`).
    pub name: &'static str,
    /// Rule IDs this validator can emit (e.g. `&["AS-001", "AS-002"]`).
    pub rule_ids: &'static [&'static str],
}

/// Trait for file validators.
///
/// Implementors define validation logic for specific file types. Each validator
/// is created by a [`ValidatorFactory`](crate::ValidatorFactory) registered in
/// the [`ValidatorRegistry`](crate::ValidatorRegistry).
///
/// The [`name()`](Validator::name) method returns a human-readable identifier
/// used for filtering via `disabled_validators` configuration. The default
/// implementation derives the name from the concrete struct name (e.g.,
/// `"SkillValidator"`).
pub trait Validator: 'static {
    /// Validate the given file content and return any diagnostics.
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic>;

    /// Return a short, human-readable name for this validator.
    ///
    /// Used by [`ValidatorRegistry`](crate::ValidatorRegistry) to support
    /// `disabled_validators` filtering. The default implementation extracts
    /// the unqualified struct name (e.g., `"SkillValidator"`).
    ///
    /// Override this if the auto-derived name is unsuitable (e.g., for
    /// dynamically-generated validators from plugins).
    fn name(&self) -> &'static str {
        short_type_name::<Self>()
    }

    /// Returns metadata describing this validator.
    ///
    /// The default implementation returns the validator's [`name`](Validator::name)
    /// with an empty `rule_ids` slice. Built-in validators override this to
    /// advertise the full set of rule IDs they can emit from their
    /// [`validate`](Validator::validate) method.
    ///
    /// Note: `rule_ids` covers only rules emitted directly by the validator.
    /// Pipeline-level post-processing rules (e.g. `AGM-006`, `XP-004`..`XP-006`,
    /// `VER-001`) are not attributed to any validator.
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: &[],
        }
    }
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

/// Compute the byte offset where frontmatter content begins - after the opening
/// `---` delimiter and its line ending. This is the correct insertion point for
/// new frontmatter keys. Handles both LF and CRLF line endings.
pub(crate) fn frontmatter_content_offset(content: &str, frontmatter_start: usize) -> usize {
    let mut pos = frontmatter_start;
    let bytes = content.as_bytes();
    if bytes.get(pos) == Some(&b'\r') {
        pos += 1;
    }
    if bytes.get(pos) == Some(&b'\n') {
        pos += 1;
    }
    pos
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

/// Find the byte span of a JSON string value for a unique key/value pair.
/// Returns byte positions of the inner string (without quotes).
/// Returns None if the key/value pair is not found or appears more than once (uniqueness guard).
pub(crate) fn find_unique_json_string_value_span(
    content: &str,
    key: &str,
    current_value: &str,
) -> Option<(usize, usize)> {
    crate::span_utils::find_unique_json_string_inner(content, key, current_value)
}

/// Find the closest valid value for an invalid input.
/// Returns an exact case-insensitive match first, then a substring match,
/// or None if no plausible match is found.
///
/// Uses ASCII case folding — all valid values in agnix are ASCII identifiers
/// (agent names, scope names, transport types). The 3-byte minimum for
/// substring matching uses byte length, which equals char count for ASCII.
pub(crate) fn find_closest_value<'a>(invalid: &str, valid_values: &[&'a str]) -> Option<&'a str> {
    if invalid.is_empty() {
        return None;
    }
    // Case-insensitive exact match (no allocation)
    for &v in valid_values {
        if v.eq_ignore_ascii_case(invalid) {
            return Some(v);
        }
    }
    // Substring match — require minimum 3 chars to avoid spurious matches
    if invalid.len() < 3 {
        return None;
    }
    let lower = invalid.to_ascii_lowercase();
    valid_values
        .iter()
        .find(|&&v| {
            contains_ignore_ascii_case(v.as_bytes(), lower.as_bytes())
                || contains_ignore_ascii_case(lower.as_bytes(), v.as_bytes())
        })
        .copied()
}

/// Check if `haystack` contains `needle` using ASCII case-insensitive comparison.
/// Zero allocations — operates directly on byte slices.
fn contains_ignore_ascii_case(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || needle.len() > haystack.len() {
        return false;
    }
    haystack
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle))
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

    #[test]
    fn test_find_closest_value_short_input_no_substring() {
        // Inputs shorter than 3 chars should only match exactly, not as substrings
        assert_eq!(
            find_closest_value("ss", &["stdio", "http", "sse"]),
            None,
            "2-char input should not substring-match"
        );
        assert_eq!(
            find_closest_value("a", &["coding-agent", "code-review"]),
            None,
            "1-char input should not substring-match"
        );
        // But short exact matches still work
        assert_eq!(
            find_closest_value("SS", &["stdio", "http", "ss"]),
            Some("ss"),
            "2-char exact match (case-insensitive) should still work"
        );
    }

    #[test]
    fn test_validator_metadata_default_has_empty_rule_ids() {
        struct DummyValidator;
        impl Validator for DummyValidator {
            fn validate(&self, _: &Path, _: &str, _: &LintConfig) -> Vec<Diagnostic> {
                vec![]
            }
        }
        let v = DummyValidator;
        let meta = v.metadata();
        assert_eq!(meta.name, "DummyValidator");
        assert!(meta.rule_ids.is_empty());
    }

    #[test]
    fn test_validator_metadata_custom_override() {
        const IDS: &[&str] = &["TEST-001", "TEST-002"];
        struct CustomValidator;
        impl Validator for CustomValidator {
            fn validate(&self, _: &Path, _: &str, _: &LintConfig) -> Vec<Diagnostic> {
                vec![]
            }
            fn metadata(&self) -> ValidatorMetadata {
                ValidatorMetadata {
                    name: "CustomValidator",
                    rule_ids: IDS,
                }
            }
        }
        let v = CustomValidator;
        let meta = v.metadata();
        assert_eq!(meta.name, "CustomValidator");
        assert_eq!(meta.rule_ids, &["TEST-001", "TEST-002"]);
    }

    #[test]
    fn test_validator_metadata_is_copy() {
        let meta = ValidatorMetadata {
            name: "Test",
            rule_ids: &["R-001"],
        };
        let copy = meta;
        assert_eq!(meta, copy);
    }
}
