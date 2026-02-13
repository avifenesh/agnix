//! Cline rules schema helpers
//!
//! Provides parsing and validation for:
//! - `.clinerules` single file (plain text, no frontmatter)
//! - `.clinerules/*.md` folder files (optional `paths` frontmatter)
//!
//! Folder files support YAML frontmatter with a `paths` field
//! containing glob patterns for scoped rule application.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Known valid keys for .clinerules/*.md frontmatter
const KNOWN_KEYS: &[&str] = &["paths"];

/// Paths field can be a single string (scalar) or an array of strings.
/// Cline expects an array - scalar values are silently ignored.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PathsField {
    Scalar(String),
    Array(Vec<String>),
}

impl PathsField {
    /// Returns true if this is a scalar string (not an array)
    pub fn is_scalar(&self) -> bool {
        matches!(self, PathsField::Scalar(_))
    }

    /// Returns the scalar value if this is a scalar, None if array
    pub fn as_scalar(&self) -> Option<&str> {
        match self {
            PathsField::Scalar(s) => Some(s.as_str()),
            PathsField::Array(_) => None,
        }
    }

    /// Get all patterns as a vector
    pub fn patterns(&self) -> Vec<&str> {
        match self {
            PathsField::Scalar(s) => vec![s.as_str()],
            PathsField::Array(v) => v.iter().map(|s| s.as_str()).collect(),
        }
    }
}

/// Frontmatter schema for Cline .clinerules/*.md files
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClineRuleSchema {
    /// Glob patterns specifying which files this rule applies to
    #[serde(default)]
    pub paths: Option<PathsField>,
}

/// Result of parsing Cline rule file frontmatter
#[derive(Debug, Clone)]
pub struct ParsedClineFrontmatter {
    /// The parsed schema (if valid YAML)
    pub schema: Option<ClineRuleSchema>,
    /// Raw frontmatter string (between --- markers)
    pub raw: String,
    /// Line number where frontmatter starts (1-indexed)
    pub start_line: usize,
    /// Line number where frontmatter ends (1-indexed)
    pub end_line: usize,
    /// Body content after frontmatter
    pub body: String,
    /// Unknown keys found in frontmatter
    pub unknown_keys: Vec<UnknownKey>,
    /// Line number where the `paths` key appears (1-indexed)
    pub paths_line: Option<usize>,
    /// Parse error if YAML is invalid
    pub parse_error: Option<String>,
}

/// An unknown key found in frontmatter
#[derive(Debug, Clone)]
pub struct UnknownKey {
    pub key: String,
    pub line: usize,
    pub column: usize,
}

/// Result of validating a glob pattern
#[derive(Debug, Clone)]
pub struct GlobValidation {
    pub valid: bool,
    pub pattern: String,
    pub error: Option<String>,
}

/// Parse frontmatter from a Cline .clinerules/*.md file
///
/// Returns parsed frontmatter if present, or None if no frontmatter exists.
pub fn parse_frontmatter(content: &str) -> Option<ParsedClineFrontmatter> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return None;
    }

    // Only treat as frontmatter if the first line is exactly '---' (after trim)
    if lines[0].trim() != "---" {
        return None;
    }

    // Find closing ---
    let mut end_idx = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            end_idx = Some(i);
            break;
        }
    }

    // If we have an opening --- but no closing ---,
    // treat this as invalid frontmatter rather than missing frontmatter.
    if end_idx.is_none() {
        let frontmatter_lines: Vec<&str> = lines[1..].to_vec();
        let raw = frontmatter_lines.join("\n");

        return Some(ParsedClineFrontmatter {
            schema: None,
            raw,
            start_line: 1,
            end_line: lines.len(),
            body: String::new(),
            unknown_keys: Vec::new(),
            paths_line: None,
            parse_error: Some("missing closing ---".to_string()),
        });
    }

    let end_idx = end_idx.unwrap();

    // Extract frontmatter content (between --- markers)
    let frontmatter_lines: Vec<&str> = lines[1..end_idx].to_vec();
    let raw = frontmatter_lines.join("\n");

    // Extract body (after closing ---)
    let body_lines: Vec<&str> = lines[end_idx + 1..].to_vec();
    let body = body_lines.join("\n");

    // Try to parse as YAML
    let (schema, parse_error) = match serde_yaml::from_str::<ClineRuleSchema>(&raw) {
        Ok(s) => (Some(s), None),
        Err(e) => (None, Some(e.to_string())),
    };

    // Find unknown keys
    let unknown_keys = find_unknown_keys(&raw, 2); // Start at line 2 (after first ---)

    // Find the line number of the `paths:` key (1-indexed)
    let paths_line = frontmatter_lines
        .iter()
        .position(|line| line.trim_start().starts_with("paths:"))
        .map(|i| i + 2); // +2 because line 1 is `---`, and i is 0-indexed

    Some(ParsedClineFrontmatter {
        schema,
        raw,
        start_line: 1,
        end_line: end_idx + 1,
        body,
        unknown_keys,
        paths_line,
        parse_error,
    })
}

/// Find unknown keys in frontmatter YAML
fn find_unknown_keys(yaml: &str, start_line: usize) -> Vec<UnknownKey> {
    let known: HashSet<&str> = KNOWN_KEYS.iter().copied().collect();
    let mut unknown = Vec::new();

    for (i, line) in yaml.lines().enumerate() {
        // Heuristic: top-level keys in YAML frontmatter are not indented.
        if line.starts_with(' ') || line.starts_with('\t') {
            continue;
        }

        if let Some(colon_idx) = line.find(':') {
            let key_raw = &line[..colon_idx];
            let key = key_raw.trim().trim_matches(|c| c == '\'' || c == '\"');

            if !key.is_empty() && !known.contains(key) {
                unknown.push(UnknownKey {
                    key: key.to_string(),
                    line: start_line + i,
                    column: key_raw.len() - key_raw.trim_start().len(),
                });
            }
        }
    }

    unknown
}

/// Validate a glob pattern
pub fn validate_glob_pattern(pattern: &str) -> GlobValidation {
    match glob::Pattern::new(pattern) {
        Ok(_) => GlobValidation {
            valid: true,
            pattern: pattern.to_string(),
            error: None,
        },
        Err(e) => GlobValidation {
            valid: false,
            pattern: pattern.to_string(),
            error: Some(e.to_string()),
        },
    }
}

/// Check if content body is empty (ignoring whitespace)
pub fn is_body_empty(body: &str) -> bool {
    body.trim().is_empty()
}

/// Check if content is empty
pub fn is_content_empty(content: &str) -> bool {
    content.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_frontmatter() {
        let content = r#"---
paths:
  - "**/*.ts"
---
# TypeScript Rules

Use strict mode.
"#;
        let result = parse_frontmatter(content).unwrap();
        assert!(result.schema.is_some());
        let schema = result.schema.as_ref().unwrap();
        assert!(schema.paths.is_some());
        let paths = schema.paths.as_ref().unwrap();
        assert!(!paths.is_scalar());
        assert_eq!(paths.patterns(), vec!["**/*.ts"]);
        assert!(result.parse_error.is_none());
        assert!(result.body.contains("TypeScript Rules"));
    }

    #[test]
    fn test_parse_scalar_paths() {
        let content = r#"---
paths: "**/*.ts"
---
# TypeScript Rules

Use strict mode.
"#;
        let result = parse_frontmatter(content).unwrap();
        assert!(result.schema.is_some());
        let schema = result.schema.as_ref().unwrap();
        assert!(schema.paths.is_some());
        let paths = schema.paths.as_ref().unwrap();
        assert!(paths.is_scalar());
        assert_eq!(paths.patterns(), vec!["**/*.ts"]);
    }

    #[test]
    fn test_parse_array_paths() {
        let content = r#"---
paths:
  - "**/*.ts"
  - "**/*.tsx"
  - "src/**/*.js"
---
# Web Rules
"#;
        let result = parse_frontmatter(content).unwrap();
        assert!(result.schema.is_some());
        let schema = result.schema.as_ref().unwrap();
        if let Some(PathsField::Array(patterns)) = &schema.paths {
            assert_eq!(patterns.len(), 3);
            assert!(patterns.contains(&"**/*.ts".to_string()));
            assert!(patterns.contains(&"**/*.tsx".to_string()));
            assert!(patterns.contains(&"src/**/*.js".to_string()));
        } else {
            panic!("Expected array paths");
        }
    }

    #[test]
    fn test_paths_field_is_scalar() {
        let scalar = PathsField::Scalar("**/*.ts".to_string());
        assert!(scalar.is_scalar());

        let array = PathsField::Array(vec!["**/*.ts".to_string()]);
        assert!(!array.is_scalar());
    }

    #[test]
    fn test_paths_field_patterns() {
        let scalar = PathsField::Scalar("**/*.ts".to_string());
        assert_eq!(scalar.patterns(), vec!["**/*.ts"]);

        let array = PathsField::Array(vec!["**/*.ts".to_string(), "**/*.tsx".to_string()]);
        let patterns = array.patterns();
        assert_eq!(patterns.len(), 2);
        assert!(patterns.contains(&"**/*.ts"));
        assert!(patterns.contains(&"**/*.tsx"));
    }

    #[test]
    fn test_parse_no_frontmatter() {
        let content = "# Just markdown without frontmatter";
        let result = parse_frontmatter(content);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_unclosed_frontmatter() {
        let content = r#"---
paths: "**/*.ts"
# Missing closing ---
"#;
        let result = parse_frontmatter(content).unwrap();
        assert!(result.parse_error.is_some());
        assert_eq!(result.parse_error.as_ref().unwrap(), "missing closing ---");
    }

    #[test]
    fn test_parse_invalid_yaml() {
        let content = r#"---
paths: [unclosed
---
# Body
"#;
        let result = parse_frontmatter(content).unwrap();
        assert!(result.schema.is_none());
        assert!(result.parse_error.is_some());
    }

    #[test]
    fn test_detect_unknown_keys() {
        let content = r#"---
paths: "**/*.ts"
unknownKey: value
---
# Body
"#;
        let result = parse_frontmatter(content).unwrap();
        assert_eq!(result.unknown_keys.len(), 1);
        assert!(result.unknown_keys.iter().any(|k| k.key == "unknownKey"));
    }

    #[test]
    fn test_no_unknown_keys() {
        let content = r#"---
paths: "**/*.rs"
---
# Body
"#;
        let result = parse_frontmatter(content).unwrap();
        assert!(result.unknown_keys.is_empty());
    }

    #[test]
    fn test_valid_glob_patterns() {
        let patterns = vec!["**/*.ts", "*.rs", "src/**/*.js", "[abc].txt"];
        for pattern in patterns {
            let result = validate_glob_pattern(pattern);
            assert!(result.valid, "Pattern '{}' should be valid", pattern);
        }
    }

    #[test]
    fn test_invalid_glob_pattern() {
        let result = validate_glob_pattern("[unclosed");
        assert!(!result.valid);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_empty_body() {
        assert!(is_body_empty(""));
        assert!(is_body_empty("   "));
        assert!(is_body_empty("\n\n\n"));
        assert!(!is_body_empty("# Content"));
    }

    #[test]
    fn test_empty_content() {
        assert!(is_content_empty(""));
        assert!(is_content_empty("   \n\t  "));
        assert!(!is_content_empty("# Instructions"));
    }

    #[test]
    fn test_frontmatter_line_numbers() {
        let content = r#"---
paths: "**/*.ts"
---
# Body
"#;
        let result = parse_frontmatter(content).unwrap();
        assert_eq!(result.start_line, 1);
        assert_eq!(result.end_line, 3);
    }

    #[test]
    fn test_unknown_key_line_numbers() {
        let content = r#"---
paths: "**/*.ts"
unknownKey: value
---
# Body
"#;
        let result = parse_frontmatter(content).unwrap();
        assert_eq!(result.unknown_keys.len(), 1);
        assert_eq!(result.unknown_keys[0].line, 3);
    }
}
