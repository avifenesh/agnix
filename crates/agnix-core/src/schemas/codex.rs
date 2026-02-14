//! Codex CLI configuration file schema helpers
//!
//! Provides parsing and validation for `.codex/config.toml` configuration files.
//!
//! Validates:
//! - `approvalMode` field values (suggest, auto-edit, full-auto)
//! - `fullAutoErrorMode` field values (ask-user, ignore-and-continue)
//! - Unknown config keys (CDX-004)
//! - `project_doc_max_bytes` limits (CDX-005)

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Valid values for the `approvalMode` field
pub const VALID_APPROVAL_MODES: &[&str] = &["suggest", "auto-edit", "full-auto"];

/// Valid values for the `fullAutoErrorMode` field
pub const VALID_FULL_AUTO_ERROR_MODES: &[&str] = &["ask-user", "ignore-and-continue"];

/// Known valid top-level keys for .codex/config.toml
/// Sourced from <https://developers.openai.com/codex/> sample config
pub const KNOWN_TOP_LEVEL_KEYS: &[&str] = &[
    // Core model settings
    "model",
    "personality",
    "review_model",
    "model_provider",
    "oss_provider",
    "model_context_window",
    "model_auto_compact_token_limit",
    "tool_output_token_limit",
    "log_dir",
    "model_reasoning_effort",
    "model_reasoning_summary",
    "model_verbosity",
    "model_supports_reasoning_summaries",
    // Instructions
    "developer_instructions",
    "instructions",
    "compact_prompt",
    "model_instructions_file",
    "experimental_compact_prompt_file",
    "include_apply_patch_tool",
    // Notifications
    "notify",
    // Approval & sandbox
    "approval_policy",
    "sandbox_mode",
    // Authentication
    "cli_auth_credentials_store",
    "chatgpt_base_url",
    "forced_chatgpt_workspace_id",
    "forced_login_method",
    "mcp_oauth_credentials_store",
    "mcp_oauth_callback_port",
    // Project docs
    "project_doc_max_bytes",
    "project_doc_fallback_filenames",
    "project_root_markers",
    // UI
    "file_opener",
    "hide_agent_reasoning",
    "show_raw_agent_reasoning",
    "disable_paste_burst",
    "windows_wsl_setup_acknowledged",
    "check_for_update_on_startup",
    // Web search
    "web_search",
    // Profiles
    "profile",
    // Experimental
    "experimental_use_unified_exec_tool",
    "experimental_use_freeform_apply_patch",
    // Legacy camelCase keys (backwards compat)
    "approvalMode",
    "fullAutoErrorMode",
];

/// Known valid TOML table names (sections like `[sandbox_workspace_write]`)
pub const KNOWN_TABLE_KEYS: &[&str] = &[
    "sandbox_workspace_write",
    "shell_environment_policy",
    "history",
    "tui",
    "features",
    "mcp_servers",
    "model_providers",
    "profiles",
    "projects",
    "otel",
    "skills",
    "feedback",
    "notice",
];

/// An unknown key found in config
#[derive(Debug, Clone)]
pub struct UnknownKey {
    pub key: String,
    pub line: usize,
    pub column: usize,
}

/// Partial schema for .codex/config.toml (only fields we validate)
///
/// Note: The actual TOML keys use camelCase (`approvalMode`, `fullAutoErrorMode`).
/// We use manual `toml::Value` parsing in `parse_codex_toml` rather than serde
/// deserialization so that type mismatches (e.g. `approvalMode = true`) are
/// reported as CDX-001/CDX-002 diagnostics instead of generic parse errors.
/// The `#[serde(rename)]` attributes are kept for documentation and in case
/// the struct is ever deserialized directly.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodexConfigSchema {
    /// Approval mode for Codex CLI (TOML key: `approvalMode`)
    #[serde(default, rename = "approvalMode")]
    pub approval_mode: Option<String>,

    /// Error handling mode for full-auto mode (TOML key: `fullAutoErrorMode`)
    #[serde(default, rename = "fullAutoErrorMode")]
    pub full_auto_error_mode: Option<String>,

    /// Maximum size for project documentation files in bytes
    #[serde(default)]
    pub project_doc_max_bytes: Option<i64>,
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
    /// Whether `project_doc_max_bytes` key exists but has wrong type (not an integer)
    pub project_doc_max_bytes_wrong_type: bool,
    /// Unknown top-level keys found in config
    pub unknown_keys: Vec<UnknownKey>,
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
///
/// # Input size
///
/// Callers are expected to enforce file size limits before calling this function.
/// In production, `file_utils::safe_read_file` enforces a 1 MiB limit upstream,
/// so content passed here is already bounded.
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
                project_doc_max_bytes_wrong_type: false,
                unknown_keys: Vec::new(),
            };
        }
    };

    // Second pass: extract typed fields permissively, tracking type mismatches
    // TOML keys use camelCase: approvalMode, fullAutoErrorMode
    let table = value.as_table();

    let approval_mode_value = table.and_then(|t| t.get("approvalMode"));
    let approval_mode_wrong_type = approval_mode_value.is_some_and(|v| !v.is_str());
    let approval_mode = approval_mode_value
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let full_auto_error_mode_value = table.and_then(|t| t.get("fullAutoErrorMode"));
    let full_auto_error_mode_wrong_type = full_auto_error_mode_value.is_some_and(|v| !v.is_str());
    let full_auto_error_mode = full_auto_error_mode_value
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Extract project_doc_max_bytes (CDX-005)
    let project_doc_max_bytes_value = table.and_then(|t| t.get("project_doc_max_bytes"));
    let project_doc_max_bytes_wrong_type =
        project_doc_max_bytes_value.is_some_and(|v| !v.is_integer());
    let project_doc_max_bytes = project_doc_max_bytes_value.and_then(|v| v.as_integer());

    // Detect unknown top-level keys (CDX-004)
    let unknown_keys = detect_unknown_keys(table, content);

    ParsedCodexConfig {
        schema: Some(CodexConfigSchema {
            approval_mode,
            full_auto_error_mode,
            project_doc_max_bytes,
        }),
        parse_error: None,
        approval_mode_wrong_type,
        full_auto_error_mode_wrong_type,
        project_doc_max_bytes_wrong_type,
        unknown_keys,
    }
}

/// Detect unknown top-level keys by comparing TOML table keys against the known sets.
fn detect_unknown_keys(
    table: Option<&toml::map::Map<String, toml::Value>>,
    content: &str,
) -> Vec<UnknownKey> {
    let Some(table) = table else {
        return Vec::new();
    };

    let known_top: HashSet<&str> = KNOWN_TOP_LEVEL_KEYS.iter().copied().collect();
    let known_tables: HashSet<&str> = KNOWN_TABLE_KEYS.iter().copied().collect();

    let mut unknown = Vec::new();
    for key in table.keys() {
        if !known_top.contains(key.as_str()) && !known_tables.contains(key.as_str()) {
            unknown.push(UnknownKey {
                key: key.clone(),
                line: find_toml_key_line(content, key).unwrap_or(1),
                column: 0,
            });
        }
    }
    unknown
}

/// Find the 1-indexed line number of a TOML key in the content.
///
/// Searches for a bare key or quoted key followed by `=` to prevent partial
/// matches and value-position false positives.
fn find_toml_key_line(content: &str, key: &str) -> Option<usize> {
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        // Skip table headers like [section]
        if trimmed.starts_with('[') {
            continue;
        }
        // Try bare key match
        if let Some(after) = trimmed.strip_prefix(key) {
            if after.trim_start().starts_with('=') {
                return Some(i + 1);
            }
        }
        // Try quoted key match
        let quoted = format!("\"{}\"", key);
        if let Some(after) = trimmed.strip_prefix(&quoted) {
            if after.trim_start().starts_with('=') {
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
            assert_eq!(result.schema.unwrap().approval_mode, Some(mode.to_string()));
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

    #[test]
    fn test_parse_error_fallback_line() {
        // When span() returns None the code falls back to (line=1, column=0).
        // In practice the toml crate always provides spans for parse errors,
        // so we verify the fallback indirectly: any parse error must have
        // line >= 1 (the minimum from the fallback path).
        let content = "= value_without_key";
        let result = parse_codex_toml(content);
        assert!(result.parse_error.is_some());
        let err = result.parse_error.unwrap();
        assert!(
            err.line >= 1,
            "Parse error line should be at least 1 (fallback or span-derived)"
        );
    }

    // ===== Unknown Keys Detection =====

    #[test]
    fn test_unknown_keys_detected() {
        let content = "completely_unknown_key = true\nmodel = \"o4-mini\"";
        let result = parse_codex_toml(content);
        assert!(result.parse_error.is_none());
        assert_eq!(result.unknown_keys.len(), 1);
        assert_eq!(result.unknown_keys[0].key, "completely_unknown_key");
        assert_eq!(result.unknown_keys[0].line, 1);
    }

    #[test]
    fn test_known_keys_not_flagged() {
        let content = r#"
model = "o4-mini"
approvalMode = "suggest"
fullAutoErrorMode = "ask-user"
notify = true
project_doc_max_bytes = 32768
"#;
        let result = parse_codex_toml(content);
        assert!(result.unknown_keys.is_empty(), "All keys are known");
    }

    #[test]
    fn test_known_table_keys_not_flagged() {
        let content = r#"
model = "o4-mini"

[mcp_servers]
name = "test"
"#;
        let result = parse_codex_toml(content);
        assert!(
            result.unknown_keys.is_empty(),
            "Known table keys should not be flagged"
        );
    }

    #[test]
    fn test_unknown_keys_empty_on_parse_error() {
        let content = "invalid = [unclosed";
        let result = parse_codex_toml(content);
        assert!(result.parse_error.is_some());
        assert!(result.unknown_keys.is_empty());
    }

    // ===== project_doc_max_bytes Parsing =====

    #[test]
    fn test_project_doc_max_bytes_parsed() {
        let content = "project_doc_max_bytes = 32768";
        let result = parse_codex_toml(content);
        assert!(result.schema.is_some());
        assert_eq!(result.schema.unwrap().project_doc_max_bytes, Some(32768));
        assert!(!result.project_doc_max_bytes_wrong_type);
    }

    #[test]
    fn test_project_doc_max_bytes_wrong_type() {
        let content = "project_doc_max_bytes = \"not a number\"";
        let result = parse_codex_toml(content);
        assert!(result.project_doc_max_bytes_wrong_type);
    }

    #[test]
    fn test_project_doc_max_bytes_absent() {
        let content = "model = \"o4-mini\"";
        let result = parse_codex_toml(content);
        assert!(result.schema.is_some());
        assert!(result.schema.unwrap().project_doc_max_bytes.is_none());
        assert!(!result.project_doc_max_bytes_wrong_type);
    }

    // ===== find_toml_key_line =====

    #[test]
    fn test_find_toml_key_line_basic() {
        let content = "model = \"o4-mini\"\nunknown_key = true";
        assert_eq!(find_toml_key_line(content, "model"), Some(1));
        assert_eq!(find_toml_key_line(content, "unknown_key"), Some(2));
        assert_eq!(find_toml_key_line(content, "nonexistent"), None);
    }

    #[test]
    fn test_find_toml_key_line_skips_table_headers() {
        let content = "[mcp_servers]\nname = \"test\"";
        // Should not match "mcp_servers" in a table header
        assert_eq!(find_toml_key_line(content, "name"), Some(2));
    }
}
