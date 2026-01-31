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
    let (frontmatter, body) = extract_frontmatter(content).map_err(LintError::Other)?;
    let parsed: T = serde_yaml::from_str(&frontmatter).map_err(|e| LintError::Other(e.into()))?;
    Ok((parsed, body))
}

/// Extract frontmatter and body from content
fn extract_frontmatter(content: &str) -> anyhow::Result<(String, String)> {
    let content = content.trim_start();

    // Check for opening ---
    if !content.starts_with("---") {
        return Ok((String::new(), content.to_string()));
    }

    let content = &content[3..];

    // Find closing ---
    if let Some(end_pos) = content.find("\n---") {
        let frontmatter = &content[..end_pos];
        let body = &content[end_pos + 4..]; // Skip \n---
        Ok((frontmatter.to_string(), body.trim_start().to_string()))
    } else {
        // No closing marker - treat entire file as body
        Ok((String::new(), content.to_string()))
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
