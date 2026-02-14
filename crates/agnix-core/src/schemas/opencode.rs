//! OpenCode configuration file schema helpers
//!
//! Provides parsing and validation for opencode.json configuration files.
//!
//! Validates:
//! - `share` field values (manual, auto, disabled)
//! - `instructions` array paths existence
//! - Unknown config keys (OC-004)
//! - Remote URLs in instructions (OC-006)
//! - Agent definitions (OC-007)
//! - Permission configuration (OC-008)
//! - Variable substitution syntax (OC-009)

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Valid values for the `share` field
pub const VALID_SHARE_MODES: &[&str] = &["manual", "auto", "disabled"];

/// Known valid top-level keys for opencode.json
pub const KNOWN_TOP_LEVEL_KEYS: &[&str] = &[
    "$schema",
    "theme",
    "model",
    "small_model",
    "provider",
    "autoupdate",
    "tools",
    "tui",
    "server",
    "agent",
    "default_agent",
    "command",
    "keybinds",
    "share",
    "formatter",
    "permission",
    "compaction",
    "watcher",
    "mcp",
    "plugin",
    "instructions",
    "disabled_providers",
    "enabled_providers",
    "experimental",
];

/// Valid permission mode values
pub const VALID_PERMISSION_MODES: &[&str] = &["allow", "ask", "deny"];

/// An unknown key found in config
#[derive(Debug, Clone)]
pub struct UnknownKey {
    pub key: String,
    pub line: usize,
    pub column: usize,
}

/// Partial schema for opencode.json (only fields we validate)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenCodeSchema {
    /// Conversation sharing mode
    #[serde(default)]
    pub share: Option<String>,

    /// Array of paths/globs to instruction files
    #[serde(default)]
    pub instructions: Option<Vec<String>>,

    /// Agent definitions
    #[serde(default, skip_serializing)]
    pub agent: Option<serde_json::Value>,

    /// Permission configuration
    #[serde(default, skip_serializing)]
    pub permission: Option<serde_json::Value>,
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
    /// Whether `agent` key exists but has wrong type (not an object)
    pub agent_wrong_type: bool,
    /// Whether `permission` key exists but has wrong type (not an object or string)
    pub permission_wrong_type: bool,
    /// Unknown top-level keys found in config
    pub unknown_keys: Vec<UnknownKey>,
    /// The raw parsed JSON value (for OC-009 substitution scanning)
    pub raw_value: Option<serde_json::Value>,
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
                agent_wrong_type: false,
                permission_wrong_type: false,
                unknown_keys: Vec::new(),
                raw_value: None,
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

    // Extract agent field (OC-007)
    let agent_value = value.get("agent");
    let agent_wrong_type = agent_value.is_some_and(|v| !v.is_object() && !v.is_null());
    let agent = agent_value.cloned();

    // Extract permission field (OC-008)
    let permission_value = value.get("permission");
    let permission_wrong_type =
        permission_value.is_some_and(|v| !v.is_object() && !v.is_string() && !v.is_null());
    let permission = permission_value.cloned();

    // Detect unknown top-level keys (OC-004)
    let unknown_keys = detect_unknown_keys(&value, content);

    ParsedOpenCodeConfig {
        schema: Some(OpenCodeSchema {
            share,
            instructions,
            agent,
            permission,
        }),
        parse_error: None,
        share_wrong_type,
        instructions_wrong_type,
        agent_wrong_type,
        permission_wrong_type,
        unknown_keys,
        raw_value: Some(value),
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

/// Detect unknown top-level keys by comparing JSON object keys against the known set.
fn detect_unknown_keys(value: &serde_json::Value, content: &str) -> Vec<UnknownKey> {
    let Some(obj) = value.as_object() else {
        return Vec::new();
    };

    let known: HashSet<&str> = KNOWN_TOP_LEVEL_KEYS.iter().copied().collect();

    let mut unknown = Vec::new();
    for key in obj.keys() {
        if !known.contains(key.as_str()) {
            unknown.push(UnknownKey {
                key: key.clone(),
                line: find_json_key_line(content, key).unwrap_or(1),
                column: 0,
            });
        }
    }
    unknown
}

/// Find the 1-indexed line number of a JSON key in the content.
///
/// Looks for `"key"` followed by `:` to avoid matching the key name
/// when it appears as a string value rather than an object key.
fn find_json_key_line(content: &str, key: &str) -> Option<usize> {
    let needle = format!("\"{}\"", key);
    for (i, line) in content.lines().enumerate() {
        if let Some(pos) = line.find(&needle) {
            let after = &line[pos + needle.len()..];
            if after.trim_start().starts_with(':') {
                return Some(i + 1);
            }
        }
    }
    None
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

    // ===== Unknown Keys Detection =====

    #[test]
    fn test_unknown_keys_detected() {
        let content = r#"{"totally_unknown": true, "share": "manual"}"#;
        let result = parse_opencode_json(content);
        assert_eq!(result.unknown_keys.len(), 1);
        assert_eq!(result.unknown_keys[0].key, "totally_unknown");
    }

    #[test]
    fn test_known_keys_not_flagged() {
        let content = r#"{
  "share": "manual",
  "instructions": ["**/*.md"],
  "model": "claude-sonnet-4-5",
  "agent": {},
  "permission": {}
}"#;
        let result = parse_opencode_json(content);
        assert!(result.unknown_keys.is_empty(), "All keys are known");
    }

    #[test]
    fn test_unknown_keys_empty_on_parse_error() {
        let content = "{ invalid }";
        let result = parse_opencode_json(content);
        assert!(result.parse_error.is_some());
        assert!(result.unknown_keys.is_empty());
    }

    // ===== Agent Parsing =====

    #[test]
    fn test_agent_parsed() {
        let content = r#"{"agent": {"my-agent": {"description": "test"}}}"#;
        let result = parse_opencode_json(content);
        assert!(result.schema.is_some());
        assert!(result.schema.unwrap().agent.is_some());
        assert!(!result.agent_wrong_type);
    }

    #[test]
    fn test_agent_wrong_type() {
        let content = r#"{"agent": "not an object"}"#;
        let result = parse_opencode_json(content);
        assert!(result.agent_wrong_type);
    }

    #[test]
    fn test_agent_null_ok() {
        let content = r#"{"agent": null}"#;
        let result = parse_opencode_json(content);
        assert!(!result.agent_wrong_type);
    }

    // ===== Permission Parsing =====

    #[test]
    fn test_permission_object_parsed() {
        let content = r#"{"permission": {"read": "allow", "edit": "ask"}}"#;
        let result = parse_opencode_json(content);
        assert!(result.schema.is_some());
        assert!(result.schema.unwrap().permission.is_some());
        assert!(!result.permission_wrong_type);
    }

    #[test]
    fn test_permission_string_parsed() {
        let content = r#"{"permission": "allow"}"#;
        let result = parse_opencode_json(content);
        assert!(!result.permission_wrong_type);
    }

    #[test]
    fn test_permission_wrong_type() {
        let content = r#"{"permission": 42}"#;
        let result = parse_opencode_json(content);
        assert!(result.permission_wrong_type);
    }

    #[test]
    fn test_permission_null_ok() {
        let content = r#"{"permission": null}"#;
        let result = parse_opencode_json(content);
        assert!(!result.permission_wrong_type);
    }

    // ===== find_json_key_line =====

    #[test]
    fn test_find_json_key_line_basic() {
        let content = "{\n  \"share\": \"manual\",\n  \"unknown\": true\n}";
        assert_eq!(find_json_key_line(content, "share"), Some(2));
        assert_eq!(find_json_key_line(content, "unknown"), Some(3));
        assert_eq!(find_json_key_line(content, "nonexistent"), None);
    }
}
