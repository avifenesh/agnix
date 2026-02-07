//! Codex CLI configuration file schema helpers
//!
//! Provides parsing and validation for `.codex/config.toml` configuration files.
//!
//! Validates:
//! - `approvalMode` field values (suggest, auto-edit, full-auto)
//! - `fullAutoErrorMode` field values (ask-user, ignore-and-continue)

use serde::{Deserialize, Serialize};

/// Valid values for the `approvalMode` field
pub const VALID_APPROVAL_MODES: &[&str] = &["suggest", "auto-edit", "full-auto"];

/// Valid values for the `fullAutoErrorMode` field
pub const VALID_FULL_AUTO_ERROR_MODES: &[&str] = &["ask-user", "ignore-and-continue"];

/// Partial schema for .codex/config.toml (only fields we validate)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodexConfigSchema {
    /// Approval mode for Codex CLI
    #[serde(default)]
    pub approval_mode: Option<String>,

    /// Error handling mode for full-auto mode
    #[serde(default)]
    pub full_auto_error_mode: Option<String>,
}

/// Result of parsing .codex/config.toml
#[derive(Debug, Clone)]
pub struct ParsedCodexConfig {
    /// The parsed schema (if valid TOML)
    pub schema: Option<CodexConfigSchema>,
    /// Parse error if TOML is invalid
    pub parse_error: Option<ParseError>,
    /// Whether `approvalMode` key exists but has wrong type (not a string)
    pub approval_mode_wrong_type: bool,
    /// Whether `fullAutoErrorMode` key exists but has wrong type (not a string)
    pub full_auto_error_mode_wrong_type: bool,
}

/// A TOML parse error with location information
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

/// Parse .codex/config.toml content
///
/// Uses a two-pass approach: first validates TOML syntax with `toml::Value`,
/// then extracts the typed schema. This ensures that type mismatches (e.g.,
/// `approvalMode = true`) are reported as CDX-001/CDX-002 issues rather than
/// generic parse errors.
pub fn parse_codex_toml(content: &str) -> ParsedCodexConfig {
    // First pass: validate TOML syntax
    let value: toml::Value = match content.parse::<toml::Value>() {
        Ok(v) => v,
        Err(e) => {
            // toml crate provides span info; extract line/column
            let (line, column) = e
                .span()
                .map(|span| {
                    let mut l = 1usize;
                    let mut c = 1usize;
                    for (i, ch) in content.char_indices() {
                        if i >= span.start {
                            break;
                        }
                        if ch == '\n' {
                            l += 1;
                            c = 1;
                        } else {
                            c += 1;
                        }
                    }
                    (l, c)
                })
                .unwrap_or((1, 0));

            return ParsedCodexConfig {
                schema: None,
                parse_error: Some(ParseError {
                    message: e.message().to_string(),
                    line,
                    column,
                }),
                approval_mode_wrong_type: false,
                full_auto_error_mode_wrong_type: false,
            };
        }
    };

    // Second pass: extract typed fields permissively, tracking type mismatches
    // TOML keys use camelCase: approvalMode, fullAutoErrorMode
    let table = value.as_table();

    let approval_mode_value = table.and_then(|t| t.get("approvalMode"));
    let approval_mode_wrong_type =
        approval_mode_value.is_some_and(|v| !v.is_str());
    let approval_mode = approval_mode_value
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let full_auto_error_mode_value = table.and_then(|t| t.get("fullAutoErrorMode"));
    let full_auto_error_mode_wrong_type =
        full_auto_error_mode_value.is_some_and(|v| !v.is_str());
    let full_auto_error_mode = full_auto_error_mode_value
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    ParsedCodexConfig {
        schema: Some(CodexConfigSchema {
            approval_mode,
            full_auto_error_mode,
        }),
        parse_error: None,
        approval_mode_wrong_type,
        full_auto_error_mode_wrong_type,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_config() {
        let content = r#"
model = "o4-mini"
approvalMode = "suggest"
fullAutoErrorMode = "ask-user"
notify = true
"#;
        let result = parse_codex_toml(content);
        assert!(result.schema.is_some());
        assert!(result.parse_error.is_none());
        let schema = result.schema.unwrap();
        assert_eq!(schema.approval_mode, Some("suggest".to_string()));
        assert_eq!(schema.full_auto_error_mode, Some("ask-user".to_string()));
    }

    #[test]
    fn test_parse_minimal_config() {
        let content = "";
        let result = parse_codex_toml(content);
        assert!(result.schema.is_some());
        assert!(result.parse_error.is_none());
        let schema = result.schema.unwrap();
        assert!(schema.approval_mode.is_none());
        assert!(schema.full_auto_error_mode.is_none());
    }

    #[test]
    fn test_parse_invalid_toml() {
        let content = "invalid = [unclosed";
        let result = parse_codex_toml(content);
        assert!(result.schema.is_none());
        assert!(result.parse_error.is_some());
    }

    #[test]
    fn test_valid_approval_modes() {
        for mode in VALID_APPROVAL_MODES {
            let content = format!("approvalMode = \"{}\"", mode);
            let result = parse_codex_toml(&content);
            assert!(result.schema.is_some());
            assert_eq!(
                result.schema.unwrap().approval_mode,
                Some(mode.to_string())
            );
        }
    }

    #[test]
    fn test_valid_full_auto_error_modes() {
        for mode in VALID_FULL_AUTO_ERROR_MODES {
            let content = format!("fullAutoErrorMode = \"{}\"", mode);
            let result = parse_codex_toml(&content);
            assert!(result.schema.is_some());
            assert_eq!(
                result.schema.unwrap().full_auto_error_mode,
                Some(mode.to_string())
            );
        }
    }

    #[test]
    fn test_parse_extra_fields_ignored() {
        let content = r#"
model = "o4-mini"
approvalMode = "suggest"
fullAutoErrorMode = "ask-user"
notify = true
provider = "openai"
"#;
        let result = parse_codex_toml(content);
        assert!(result.schema.is_some());
        assert!(result.parse_error.is_none());
    }

    #[test]
    fn test_approval_mode_wrong_type() {
        let content = "approvalMode = true";
        let result = parse_codex_toml(content);
        assert!(result.approval_mode_wrong_type);
        assert!(!result.full_auto_error_mode_wrong_type);
        assert!(result.schema.is_some());
        assert!(result.schema.unwrap().approval_mode.is_none());
    }

    #[test]
    fn test_full_auto_error_mode_wrong_type() {
        let content = "fullAutoErrorMode = 123";
        let result = parse_codex_toml(content);
        assert!(!result.approval_mode_wrong_type);
        assert!(result.full_auto_error_mode_wrong_type);
        assert!(result.schema.is_some());
        assert!(result.schema.unwrap().full_auto_error_mode.is_none());
    }

    #[test]
    fn test_parse_error_location() {
        let content = "approvalMode = [unclosed";
        let result = parse_codex_toml(content);
        assert!(result.parse_error.is_some());
        let err = result.parse_error.unwrap();
        assert!(err.line > 0);
    }
}
