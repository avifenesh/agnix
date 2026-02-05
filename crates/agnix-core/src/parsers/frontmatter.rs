//! YAML frontmatter parser
//!
//! ## Security: YAML Bomb Protection
//!
//! While this module doesn't implement explicit depth limits, YAML bombs (deeply
//! nested structures) are mitigated by:
//!
//! 1. **File Size Limit**: DEFAULT_MAX_FILE_SIZE (1 MiB) in file_utils.rs prevents
//!    extremely large YAML payloads from being read.
//!
//! 2. **Parser Library**: `serde_yaml` has internal protections against excessive
//!    memory usage and stack overflow from deeply nested structures.
//!
//! 3. **Memory Limit**: The entire file is bounded at 1 MiB, limiting total
//!    memory consumption regardless of structure complexity.
//!
//! **Known Limitation**: Within the 1 MiB file size, deeply nested YAML (e.g.,
//! 10,000 levels of nesting) could cause high memory usage or slow parsing.
//! This is acceptable for a local linter with bounded input size.
//!
//! **Future Enhancement**: Consider adding explicit depth tracking if memory
//! profiling reveals issues with pathological YAML structures.

use crate::diagnostics::{LintError, LintResult};
use serde::de::DeserializeOwned;

/// Parse YAML frontmatter from markdown content
///
/// Expects content in format:
/// ```markdown
/// ---
/// key: value
/// ---
/// body content
/// ```
///
/// # Security
///
/// Protected against YAML bombs by file size limit (1 MiB) and serde_yaml's
/// internal protections. See module documentation for details.
pub fn parse_frontmatter<T: DeserializeOwned>(content: &str) -> LintResult<(T, String)> {
    let parts = split_frontmatter(content);
    let parsed: T =
        serde_yaml::from_str(&parts.frontmatter).map_err(|e| LintError::Other(e.into()))?;
    Ok((parsed, parts.body.trim_start().to_string()))
}

/// Extract frontmatter and body from content with offsets.
#[derive(Debug, Clone)]
pub struct FrontmatterParts {
    pub has_frontmatter: bool,
    pub has_closing: bool,
    pub frontmatter: String,
    pub body: String,
    pub frontmatter_start: usize,
    pub body_start: usize,
}

/// Split frontmatter and body from content.
pub fn split_frontmatter(content: &str) -> FrontmatterParts {
    let trimmed = content.trim_start();
    let trim_offset = content.len() - trimmed.len();

    // Check for opening ---
    if !trimmed.starts_with("---") {
        return FrontmatterParts {
            has_frontmatter: false,
            has_closing: false,
            frontmatter: String::new(),
            body: trimmed.to_string(),
            frontmatter_start: trim_offset,
            body_start: trim_offset,
        };
    }

    let rest = &trimmed[3..];
    let frontmatter_start = trim_offset + 3;

    // Find closing ---
    if let Some(end_pos) = rest.find("\n---") {
        let frontmatter = &rest[..end_pos];
        let body = &rest[end_pos + 4..]; // Skip \n---
        FrontmatterParts {
            has_frontmatter: true,
            has_closing: true,
            frontmatter: frontmatter.to_string(),
            body: body.to_string(),
            frontmatter_start,
            body_start: frontmatter_start + end_pos + 4,
        }
    } else {
        // No closing marker - treat entire file as body
        FrontmatterParts {
            has_frontmatter: true,
            has_closing: false,
            frontmatter: String::new(),
            body: rest.to_string(),
            frontmatter_start,
            body_start: frontmatter_start,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestFrontmatter {
        name: String,
        description: String,
    }

    #[test]
    fn test_parse_frontmatter() {
        let content = r#"---
name: test-skill
description: A test skill
---
Body content here"#;

        let (fm, body): (TestFrontmatter, String) = parse_frontmatter(content).unwrap();
        assert_eq!(fm.name, "test-skill");
        assert_eq!(fm.description, "A test skill");
        assert_eq!(body, "Body content here");
    }

    #[test]
    fn test_no_frontmatter() {
        let content = "Just body content";
        let result: LintResult<(TestFrontmatter, String)> = parse_frontmatter(content);
        assert!(result.is_err()); // Should fail to deserialize empty frontmatter
    }

    #[test]
    fn test_split_frontmatter_basic() {
        let content = "---\nname: test\n---\nbody";
        let parts = split_frontmatter(content);
        assert!(parts.has_frontmatter);
        assert!(parts.has_closing);
        // Frontmatter excludes the \n before closing --- (it's part of the delimiter)
        assert_eq!(parts.frontmatter, "\nname: test");
        assert_eq!(parts.body, "\nbody");
    }

    #[test]
    fn test_split_frontmatter_no_closing() {
        let content = "---\nname: test";
        let parts = split_frontmatter(content);
        assert!(parts.has_frontmatter);
        assert!(!parts.has_closing);
        assert!(parts.frontmatter.is_empty());
    }

    #[test]
    fn test_split_frontmatter_empty() {
        let content = "";
        let parts = split_frontmatter(content);
        assert!(!parts.has_frontmatter);
        assert!(!parts.has_closing);
    }

    #[test]
    fn test_split_frontmatter_whitespace_prefix() {
        let content = "  \n---\nkey: val\n---\nbody";
        let parts = split_frontmatter(content);
        assert!(parts.has_frontmatter);
        assert!(parts.has_closing);
    }

    #[test]
    fn test_split_frontmatter_multiple_dashes() {
        let content = "---\nfirst: 1\n---\nmiddle\n---\nlast";
        let parts = split_frontmatter(content);
        assert!(parts.has_frontmatter);
        assert!(parts.has_closing);
        // Should split at first closing ---
        assert!(parts.body.contains("middle"));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn split_frontmatter_never_panics(content in ".*") {
            // split_frontmatter should never panic on any input
            let _ = split_frontmatter(&content);
        }

        #[test]
        fn split_frontmatter_valid_offsets(content in ".*") {
            let parts = split_frontmatter(&content);
            // Offsets should be within content bounds
            prop_assert!(parts.frontmatter_start <= content.len());
            prop_assert!(parts.body_start <= content.len());
        }

        #[test]
        fn frontmatter_with_dashes_detected(
            yaml in "[a-z]+: [a-z]+",
        ) {
            let content = format!("---\n{}\n---\nbody", yaml);
            let parts = split_frontmatter(&content);
            prop_assert!(parts.has_frontmatter);
            prop_assert!(parts.has_closing);
        }

        #[test]
        fn no_frontmatter_without_leading_dashes(
            content in "[^-].*"
        ) {
            let parts = split_frontmatter(&content);
            prop_assert!(!parts.has_frontmatter);
        }

        #[test]
        fn unclosed_frontmatter_has_empty_frontmatter(
            yaml in "[a-z]+: [a-z]+"
        ) {
            // Content with --- but no closing ---
            let content = format!("---\n{}", yaml);
            let parts = split_frontmatter(&content);
            prop_assert!(parts.has_frontmatter);
            prop_assert!(!parts.has_closing);
            prop_assert!(parts.frontmatter.is_empty());
        }
    }
}
