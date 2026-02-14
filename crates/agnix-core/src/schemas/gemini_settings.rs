//! Gemini CLI settings file schema helpers
//!
//! Provides parsing and validation for .gemini/settings.json configuration files.
//!
//! Validates:
//! - JSON/JSONC syntax
//! - Top-level keys against known schema
//! - Hook event names and hook object structure

use serde::Deserialize;

/// Valid hook event types in Gemini CLI hooksConfig
pub const VALID_HOOK_EVENTS: &[&str] = &[
    "SessionStart",
    "SessionEnd",
    "BeforeAgent",
    "AfterAgent",
    "BeforeModel",
    "AfterModel",
    "BeforeToolSelection",
    "BeforeTool",
    "AfterTool",
    "PreCompress",
    "Notification",
];

/// Valid top-level keys in .gemini/settings.json
pub const VALID_TOP_LEVEL_KEYS: &[&str] = &[
    "general",
    "output",
    "ui",
    "ide",
    "model",
    "context",
    "tools",
    "security",
    "advanced",
    "experimental",
    "skills",
    "hooksConfig",
];

/// A JSON parse error with location information
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

/// Result of parsing .gemini/settings.json
#[derive(Debug, Clone)]
pub struct ParsedGeminiSettings {
    /// Parse error if JSON is invalid
    pub parse_error: Option<ParseError>,
    /// The parsed schema (if valid JSON)
    pub schema: Option<GeminiSettingsSchema>,
    /// Top-level keys not in the known set
    pub unknown_top_keys: Vec<String>,
}

/// Partial schema for .gemini/settings.json (only fields we validate)
#[derive(Debug, Clone, Default, Deserialize)]
pub struct GeminiSettingsSchema {
    /// Hooks configuration object
    #[serde(rename = "hooksConfig")]
    pub hooks_config: Option<serde_json::Value>,
}

/// A single hook definition in hooksConfig
#[derive(Debug, Clone, Deserialize)]
pub struct GeminiHook {
    /// Hook type (must be "command")
    #[serde(rename = "type")]
    pub type_: Option<String>,
    /// Command to execute
    pub command: Option<String>,
    /// Optional hook name
    pub name: Option<String>,
    /// Optional timeout in seconds
    pub timeout: Option<serde_json::Value>,
    /// Optional description
    pub description: Option<String>,
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

/// Parse .gemini/settings.json content
///
/// Uses a two-pass approach: first validates JSON syntax with `serde_json::Value`,
/// then extracts the typed schema. Supports JSONC comments.
pub fn parse_gemini_settings(content: &str) -> ParsedGeminiSettings {
    let stripped = strip_jsonc_comments(content);

    // First pass: validate JSON syntax
    let value: serde_json::Value = match serde_json::from_str(&stripped) {
        Ok(v) => v,
        Err(e) => {
            return ParsedGeminiSettings {
                parse_error: Some(ParseError {
                    message: e.to_string(),
                    line: e.line(),
                    column: e.column(),
                }),
                schema: None,
                unknown_top_keys: Vec::new(),
            };
        }
    };

    // Check for unknown top-level keys
    let unknown_top_keys = if let Some(obj) = value.as_object() {
        obj.keys()
            .filter(|k| !VALID_TOP_LEVEL_KEYS.contains(&k.as_str()))
            .cloned()
            .collect()
    } else {
        Vec::new()
    };

    // Extract hooks_config
    let hooks_config = value.get("hooksConfig").cloned();

    ParsedGeminiSettings {
        parse_error: None,
        schema: Some(GeminiSettingsSchema { hooks_config }),
        unknown_top_keys,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_settings() {
        let content = r#"{
  "general": {},
  "model": {},
  "hooksConfig": {
    "BeforeAgent": [
      {
        "type": "command",
        "command": "echo hello"
      }
    ]
  }
}"#;
        let result = parse_gemini_settings(content);
        assert!(result.parse_error.is_none());
        assert!(result.schema.is_some());
        assert!(result.unknown_top_keys.is_empty());
        let schema = result.schema.unwrap();
        assert!(schema.hooks_config.is_some());
    }

    #[test]
    fn test_parse_empty_object() {
        let result = parse_gemini_settings("{}");
        assert!(result.parse_error.is_none());
        assert!(result.schema.is_some());
        assert!(result.unknown_top_keys.is_empty());
    }

    #[test]
    fn test_parse_invalid_json() {
        let result = parse_gemini_settings("{ invalid }");
        assert!(result.parse_error.is_some());
        assert!(result.schema.is_none());
    }

    #[test]
    fn test_parse_jsonc_comments() {
        let content = r#"{
  // This is a comment
  "general": {},
  /* multi-line
     comment */
  "model": {}
}"#;
        let result = parse_gemini_settings(content);
        assert!(result.parse_error.is_none());
        assert!(result.schema.is_some());
    }

    #[test]
    fn test_unknown_top_level_keys() {
        let content = r#"{
  "general": {},
  "unknownKey": true,
  "anotherBadKey": 42
}"#;
        let result = parse_gemini_settings(content);
        assert!(result.parse_error.is_none());
        assert_eq!(result.unknown_top_keys.len(), 2);
        assert!(result.unknown_top_keys.contains(&"unknownKey".to_string()));
        assert!(result
            .unknown_top_keys
            .contains(&"anotherBadKey".to_string()));
    }

    #[test]
    fn test_all_valid_top_level_keys() {
        let content = r#"{
  "general": {},
  "output": {},
  "ui": {},
  "ide": {},
  "model": {},
  "context": {},
  "tools": {},
  "security": {},
  "advanced": {},
  "experimental": {},
  "skills": {},
  "hooksConfig": {}
}"#;
        let result = parse_gemini_settings(content);
        assert!(result.parse_error.is_none());
        assert!(result.unknown_top_keys.is_empty());
    }

    #[test]
    fn test_hooks_config_extracted() {
        let content = r#"{
  "hooksConfig": {
    "SessionStart": [
      {"type": "command", "command": "echo start"}
    ]
  }
}"#;
        let result = parse_gemini_settings(content);
        let schema = result.schema.unwrap();
        assert!(schema.hooks_config.is_some());
        let hooks = schema.hooks_config.unwrap();
        assert!(hooks.get("SessionStart").is_some());
    }

    #[test]
    fn test_parse_error_location() {
        let content = "{\n  \"general\": \n}";
        let result = parse_gemini_settings(content);
        assert!(result.parse_error.is_some());
        let err = result.parse_error.unwrap();
        assert!(err.line > 0);
    }

    #[test]
    fn test_valid_hook_events_count() {
        assert_eq!(VALID_HOOK_EVENTS.len(), 11);
    }

    #[test]
    fn test_valid_top_level_keys_count() {
        assert_eq!(VALID_TOP_LEVEL_KEYS.len(), 12);
    }
}
