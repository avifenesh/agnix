//! OpenCode configuration file schema helpers
//!
//! Provides parsing and validation for opencode.json configuration files.
//!
//! Validates:
//! - `share` field values (manual, auto, disabled)
//! - `instructions` array paths existence

use serde::{Deserialize, Serialize};

/// Valid values for the `share` field
pub const VALID_SHARE_MODES: &[&str] = &["manual", "auto", "disabled"];

/// Partial schema for opencode.json (only fields we validate)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenCodeSchema {
    /// Conversation sharing mode
    #[serde(default)]
    pub share: Option<String>,

    /// Array of paths/globs to instruction files
    #[serde(default)]
    pub instructions: Option<Vec<String>>,
}

/// Result of parsing opencode.json
#[derive(Debug, Clone)]
pub struct ParsedOpenCodeConfig {
    /// The parsed schema (if valid JSON)
    pub schema: Option<OpenCodeSchema>,
    /// Parse error if JSON is invalid
    pub parse_error: Option<ParseError>,
    /// Whether `share` key exists but has wrong type (not a string)
    pub share_wrong_type: bool,
    /// Whether `instructions` key exists but has wrong type (not an array of strings)
    pub instructions_wrong_type: bool,
}

/// A JSON parse error with location information
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

/// Parse opencode.json content
///
/// Uses a two-pass approach: first validates JSON syntax with `serde_json::Value`,
/// then extracts the typed schema. This ensures that type mismatches (e.g.,
/// `"share": true`) are reported as OC-001/OC-002 issues rather than OC-003.
pub fn parse_opencode_json(content: &str) -> ParsedOpenCodeConfig {
    // Try to strip JSONC comments before parsing
    let stripped = strip_jsonc_comments(content);

    // First pass: validate JSON syntax
    let value: serde_json::Value = match serde_json::from_str(&stripped) {
        Ok(v) => v,
        Err(e) => {
            let line = e.line();
            let column = e.column();
            return ParsedOpenCodeConfig {
                schema: None,
                parse_error: Some(ParseError {
                    message: e.to_string(),
                    line,
                    column,
                }),
                share_wrong_type: false,
                instructions_wrong_type: false,
            };
        }
    };

    // Second pass: extract typed fields permissively, tracking type mismatches
    let share_value = value.get("share");
    let share_wrong_type = share_value.is_some_and(|v| !v.is_string() && !v.is_null());
    let share = share_value.and_then(|v| v.as_str()).map(|s| s.to_string());

    let instructions_value = value.get("instructions");
    let instructions_wrong_type = instructions_value.is_some_and(|v| {
        if v.is_null() {
            return false;
        }
        match v.as_array() {
            None => true,                                          // not an array
            Some(arr) => arr.iter().any(|item| !item.is_string()), // array with non-string elements
        }
    });
    let instructions = instructions_value.and_then(|v| {
        v.as_array().map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(|s| s.to_string()))
                .collect()
        })
    });

    ParsedOpenCodeConfig {
        schema: Some(OpenCodeSchema {
            share,
            instructions,
        }),
        parse_error: None,
        share_wrong_type,
        instructions_wrong_type,
    }
}

/// Strip single-line (//) and multi-line (/* */) comments from JSONC content
fn strip_jsonc_comments(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_string = false;

    while i < len {
        if in_string {
            result.push(chars[i]);
            if chars[i] == '\\' && i + 1 < len {
                i += 1;
                result.push(chars[i]);
            } else if chars[i] == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if chars[i] == '"' {
            in_string = true;
            result.push(chars[i]);
            i += 1;
            continue;
        }

        if chars[i] == '/' && i + 1 < len {
            if chars[i + 1] == '/' {
                // Single-line comment: skip until end of line
                i += 2;
                while i < len && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            } else if chars[i + 1] == '*' {
                // Multi-line comment: skip until */
                i += 2;
                while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                    // Preserve newlines for line counting
                    if chars[i] == '\n' {
                        result.push('\n');
                    }
                    i += 1;
                }
                if i + 1 < len {
                    i += 2; // skip */
                }
                continue;
            }
        }

        result.push(chars[i]);
        i += 1;
    }

    result
}

/// Check if a path looks like a valid glob pattern (contains glob characters)
pub fn is_glob_pattern(path: &str) -> bool {
    path.contains('*') || path.contains('?') || path.contains('[')
}

/// Validate a glob pattern syntax
pub fn validate_glob_pattern(pattern: &str) -> bool {
    glob::Pattern::new(pattern).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_config() {
        let content = r#"{
  "share": "manual",
  "instructions": ["CONTRIBUTING.md", "docs/guidelines.md"]
}"#;
        let result = parse_opencode_json(content);
        assert!(result.schema.is_some());
        assert!(result.parse_error.is_none());
        let schema = result.schema.unwrap();
        assert_eq!(schema.share, Some("manual".to_string()));
        assert_eq!(schema.instructions.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_parse_minimal_config() {
        let content = "{}";
        let result = parse_opencode_json(content);
        assert!(result.schema.is_some());
        assert!(result.parse_error.is_none());
        let schema = result.schema.unwrap();
        assert!(schema.share.is_none());
        assert!(schema.instructions.is_none());
    }

    #[test]
    fn test_parse_invalid_json() {
        let content = "{ invalid json }";
        let result = parse_opencode_json(content);
        assert!(result.schema.is_none());
        assert!(result.parse_error.is_some());
    }

    #[test]
    fn test_parse_jsonc_with_comments() {
        let content = r#"{
  // This is a comment
  "share": "auto",
  /* Multi-line
     comment */
  "instructions": ["README.md"]
}"#;
        let result = parse_opencode_json(content);
        assert!(result.schema.is_some());
        assert!(result.parse_error.is_none());
        let schema = result.schema.unwrap();
        assert_eq!(schema.share, Some("auto".to_string()));
    }

    #[test]
    fn test_strip_jsonc_single_line_comment() {
        let input = r#"{
  // comment
  "key": "value"
}"#;
        let stripped = strip_jsonc_comments(input);
        assert!(!stripped.contains("comment"));
        assert!(stripped.contains("\"key\""));
    }

    #[test]
    fn test_strip_jsonc_multi_line_comment() {
        let input = r#"{
  /* multi
     line */
  "key": "value"
}"#;
        let stripped = strip_jsonc_comments(input);
        assert!(!stripped.contains("multi"));
        assert!(stripped.contains("\"key\""));
    }

    #[test]
    fn test_strip_jsonc_preserves_strings() {
        let input = r#"{"key": "value with // not a comment"}"#;
        let stripped = strip_jsonc_comments(input);
        assert!(stripped.contains("// not a comment"));
    }

    #[test]
    fn test_valid_share_modes() {
        for mode in VALID_SHARE_MODES {
            let content = format!(r#"{{"share": "{}"}}"#, mode);
            let result = parse_opencode_json(&content);
            assert!(result.schema.is_some());
            assert_eq!(result.schema.unwrap().share, Some(mode.to_string()));
        }
    }

    #[test]
    fn test_is_glob_pattern() {
        assert!(is_glob_pattern("**/*.md"));
        assert!(is_glob_pattern("docs/*.txt"));
        assert!(is_glob_pattern("file[0-9].md"));
        assert!(!is_glob_pattern("README.md"));
        assert!(!is_glob_pattern("docs/guide.md"));
    }

    #[test]
    fn test_validate_glob_pattern() {
        assert!(validate_glob_pattern("**/*.md"));
        assert!(validate_glob_pattern("docs/*.txt"));
        assert!(!validate_glob_pattern("[unclosed"));
    }

    #[test]
    fn test_parse_extra_fields_ignored() {
        // opencode.json has many fields we don't validate; they should not cause parse errors
        let content = r#"{
  "share": "manual",
  "instructions": ["README.md"],
  "tui": {"theme": "dark"},
  "model": "claude-sonnet-4-5-20250929"
}"#;
        let result = parse_opencode_json(content);
        assert!(result.schema.is_some());
        assert!(result.parse_error.is_none());
    }

    #[test]
    fn test_parse_error_location() {
        let content = "{\n  \"share\": \n}";
        let result = parse_opencode_json(content);
        assert!(result.parse_error.is_some());
        let err = result.parse_error.unwrap();
        assert!(err.line > 0);
    }
}
