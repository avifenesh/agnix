//! YAML frontmatter parser

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
}
