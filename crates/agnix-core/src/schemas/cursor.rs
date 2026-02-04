//! Cursor project rules schema helpers
//!
//! Provides parsing and validation for:
//! - .mdc files in .cursor/rules/ directory
//! - Legacy .cursorrules files
//!
//! .mdc files use a YAML frontmatter format with optional fields:
//! - description: Rule description
//! - globs: File patterns to apply the rule to
//! - alwaysApply: Whether to always apply the rule

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Known valid keys for .mdc frontmatter
const KNOWN_KEYS: &[&str] = &["description", "globs", "alwaysApply"];

/// Frontmatter schema for Cursor .mdc files
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorRuleSchema {
    /// Description of what the rule does
    #[serde(default)]
    pub description: Option<String>,
    /// Glob patterns specifying which files this rule applies to
    #[serde(default)]
    pub globs: Option<GlobsField>,
    /// Whether to always apply this rule regardless of file patterns
    #[serde(default)]
    pub always_apply: Option<bool>,
}

/// Globs field can be a single string or an array of strings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GlobsField {
    Single(String),
    Multiple(Vec<String>),
}

impl GlobsField {
    /// Get all glob patterns as a vector
    pub fn patterns(&self) -> Vec<&str> {
        match self {
            GlobsField::Single(s) => vec![s.as_str()],
            GlobsField::Multiple(v) => v.iter().map(|s| s.as_str()).collect(),
        }
    }
}

/// Result of parsing Cursor .mdc file frontmatter
#[derive(Debug, Clone)]
pub struct ParsedMdcFrontmatter {
    /// The parsed schema (if valid YAML)
    pub schema: Option<CursorRuleSchema>,
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

/// Parse frontmatter from a Cursor .mdc file
///
/// Returns parsed frontmatter if present, or None if no frontmatter exists.
pub fn parse_mdc_frontmatter(content: &str) -> Option<ParsedMdcFrontmatter> {
    if !content.starts_with("---") {
        return None;
    }

    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
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

        return Some(ParsedMdcFrontmatter {
            schema: None,
            raw,
            start_line: 1,
            end_line: lines.len(),
            body: String::new(),
            unknown_keys: Vec::new(),
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
    let (schema, parse_error) = match serde_yaml::from_str::<CursorRuleSchema>(&raw) {
        Ok(s) => (Some(s), None),
        Err(e) => (None, Some(e.to_string())),
    };

    // Find unknown keys
    let unknown_keys = find_unknown_keys(&raw, 2); // Start at line 2 (after first ---)

    Some(ParsedMdcFrontmatter {
        schema,
        raw,
        start_line: 1,
        end_line: end_idx + 1,
        body,
        unknown_keys,
        parse_error,
    })
}

/// Find unknown keys in frontmatter YAML
fn find_unknown_keys(yaml: &str, start_line: usize) -> Vec<UnknownKey> {
    let known: HashSet<&str> = KNOWN_KEYS.iter().copied().collect();
    let mut unknown = Vec::new();

    for (i, line) in yaml.lines().enumerate() {
        // Heuristic: top-level keys in YAML frontmatter are not indented.
        // This helps avoid parsing content from multi-line strings.
        if line.starts_with(' ') || line.starts_with('\t') {
            continue;
        }

        if let Some(colon_idx) = line.find(':') {
            let key_raw = &line[..colon_idx];
            // Trim whitespace and quotes to get the actual key.
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
///
/// Uses the glob crate to validate pattern syntax.
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

/// Check if content is empty (for files without frontmatter)
pub fn is_content_empty(content: &str) -> bool {
    content.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Frontmatter Parsing =====

    #[test]
    fn test_parse_valid_frontmatter() {
        let content = r#"---
description: TypeScript coding standards
globs: "**/*.ts"
---
# TypeScript Rules

Use strict mode.
"#;
        let result = parse_mdc_frontmatter(content).unwrap();
        assert!(result.schema.is_some());
        let schema = result.schema.as_ref().unwrap();
        assert_eq!(
            schema.description,
            Some("TypeScript coding standards".to_string())
        );
        assert!(result.parse_error.is_none());
        assert!(result.body.contains("TypeScript Rules"));
    }

    #[test]
    fn test_parse_frontmatter_with_globs_array() {
        let content = r#"---
description: Web files
globs:
  - "**/*.ts"
  - "**/*.tsx"
  - "**/*.js"
---
# Web Rules
"#;
        let result = parse_mdc_frontmatter(content).unwrap();
        assert!(result.schema.is_some());
        let schema = result.schema.as_ref().unwrap();
        if let Some(GlobsField::Multiple(patterns)) = &schema.globs {
            assert_eq!(patterns.len(), 3);
            assert!(patterns.contains(&"**/*.ts".to_string()));
            assert!(patterns.contains(&"**/*.tsx".to_string()));
            assert!(patterns.contains(&"**/*.js".to_string()));
        } else {
            panic!("Expected multiple glob patterns");
        }
    }

    #[test]
    fn test_parse_frontmatter_with_always_apply() {
        let content = r#"---
description: Always applied rule
alwaysApply: true
---
# Always Rules
"#;
        let result = parse_mdc_frontmatter(content).unwrap();
        assert!(result.schema.is_some());
        let schema = result.schema.as_ref().unwrap();
        assert_eq!(schema.always_apply, Some(true));
    }

    #[test]
    fn test_parse_frontmatter_empty_body() {
        let content = r#"---
description: Empty body test
---
"#;
        let result = parse_mdc_frontmatter(content).unwrap();
        assert!(result.schema.is_some());
        assert!(is_body_empty(&result.body));
    }

    #[test]
    fn test_parse_no_frontmatter() {
        let content = "# Just markdown without frontmatter";
        let result = parse_mdc_frontmatter(content);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_unclosed_frontmatter() {
        let content = r#"---
description: Unclosed
# Missing closing ---
"#;
        let result = parse_mdc_frontmatter(content).unwrap();
        // Unclosed frontmatter should be treated as invalid, not missing
        assert!(result.parse_error.is_some());
        assert_eq!(result.parse_error.as_ref().unwrap(), "missing closing ---");
    }

    #[test]
    fn test_parse_invalid_yaml() {
        let content = r#"---
globs: [unclosed
---
# Body
"#;
        let result = parse_mdc_frontmatter(content).unwrap();
        assert!(result.schema.is_none());
        assert!(result.parse_error.is_some());
    }

    // ===== Unknown Keys Detection =====

    #[test]
    fn test_detect_unknown_keys() {
        let content = r#"---
description: Valid key
unknownKey: value
anotherUnknown: 123
---
# Body
"#;
        let result = parse_mdc_frontmatter(content).unwrap();
        assert_eq!(result.unknown_keys.len(), 2);
        assert!(result.unknown_keys.iter().any(|k| k.key == "unknownKey"));
        assert!(result
            .unknown_keys
            .iter()
            .any(|k| k.key == "anotherUnknown"));
    }

    #[test]
    fn test_no_unknown_keys() {
        let content = r#"---
description: Valid
globs: "**/*.rs"
alwaysApply: false
---
# Body
"#;
        let result = parse_mdc_frontmatter(content).unwrap();
        assert!(result.unknown_keys.is_empty());
    }

    // ===== Glob Pattern Validation =====

    #[test]
    fn test_valid_glob_patterns() {
        let patterns = vec![
            "**/*.ts",
            "*.rs",
            "src/**/*.js",
            "tests/unit/*.test.ts",
            "[abc].txt",
            "file?.md",
        ];

        for pattern in patterns {
            let result = validate_glob_pattern(pattern);
            assert!(result.valid, "Pattern '{}' should be valid", pattern);
        }
    }

    #[test]
    fn test_invalid_glob_pattern() {
        // Unclosed bracket is invalid
        let result = validate_glob_pattern("[unclosed");
        assert!(!result.valid);
        assert!(result.error.is_some());
    }

    // ===== Empty Content Detection =====

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

    // ===== Line Number Tracking =====

    #[test]
    fn test_frontmatter_line_numbers() {
        let content = r#"---
description: Test
globs: "**/*.ts"
---
# Body
"#;
        let result = parse_mdc_frontmatter(content).unwrap();
        assert_eq!(result.start_line, 1);
        assert_eq!(result.end_line, 4);
    }

    #[test]
    fn test_unknown_key_line_numbers() {
        let content = r#"---
description: Test
unknownKey: value
---
# Body
"#;
        let result = parse_mdc_frontmatter(content).unwrap();
        assert_eq!(result.unknown_keys.len(), 1);
        // unknownKey is on line 3 (line 1 is ---, line 2 is description, line 3 is unknownKey)
        assert_eq!(result.unknown_keys[0].line, 3);
    }

    // ===== GlobsField Helper =====

    #[test]
    fn test_globs_field_single() {
        let globs = GlobsField::Single("**/*.ts".to_string());
        let patterns = globs.patterns();
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0], "**/*.ts");
    }

    #[test]
    fn test_globs_field_multiple() {
        let globs = GlobsField::Multiple(vec!["**/*.ts".to_string(), "**/*.tsx".to_string()]);
        let patterns = globs.patterns();
        assert_eq!(patterns.len(), 2);
        assert!(patterns.contains(&"**/*.ts"));
        assert!(patterns.contains(&"**/*.tsx"));
    }
}
