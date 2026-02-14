//! Gemini CLI settings validation rules (GM-004, GM-009)
//!
//! Validates:
//! - GM-009: Settings.json parse error (HIGH) - must be valid JSON/JSONC
//! - GM-004: Invalid hooks configuration (MEDIUM) - unknown events, missing fields

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::{Validator, ValidatorMetadata},
    schemas::gemini_settings::{
        GeminiHook, VALID_HOOK_EVENTS, parse_gemini_settings,
    },
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &["GM-004", "GM-009"];

pub struct GeminiSettingsValidator;

impl Validator for GeminiSettingsValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let path_buf = path.to_path_buf();

        let parsed = parse_gemini_settings(content);

        // GM-009: Parse error (ERROR)
        if let Some(ref error) = parsed.parse_error {
            if config.is_rule_enabled("GM-009") {
                diagnostics.push(
                    Diagnostic::error(
                        path_buf.clone(),
                        error.line,
                        error.column,
                        "GM-009",
                        t!("rules.gm_009.message", error = error.message.as_str()),
                    )
                    .with_suggestion(t!("rules.gm_009.suggestion")),
                );
            }
            return diagnostics;
        }

        let schema = match parsed.schema {
            Some(s) => s,
            None => return diagnostics,
        };

        // GM-009: Unknown top-level keys (WARNING)
        if config.is_rule_enabled("GM-009") {
            for key in &parsed.unknown_top_keys {
                let line = find_key_line(content, key).unwrap_or(1);
                diagnostics.push(
                    Diagnostic::warning(
                        path_buf.clone(),
                        line,
                        0,
                        "GM-009",
                        t!("rules.gm_009.unknown_key", key = key.as_str()),
                    )
                    .with_suggestion(t!("rules.gm_009.suggestion")),
                );
            }
        }

        // GM-004: Invalid hooks configuration (WARNING)
        if config.is_rule_enabled("GM-004") {
            if let Some(ref hooks_config) = schema.hooks_config {
                if let Some(hooks_obj) = hooks_config.as_object() {
                    for (event_name, hooks_array) in hooks_obj {
                        // Check if event name is valid
                        if !VALID_HOOK_EVENTS.contains(&event_name.as_str()) {
                            let line = find_key_line(content, event_name).unwrap_or(1);
                            diagnostics.push(
                                Diagnostic::warning(
                                    path_buf.clone(),
                                    line,
                                    0,
                                    "GM-004",
                                    t!(
                                        "rules.gm_004.message",
                                        description =
                                            t!("rules.gm_004.unknown_event", event = event_name.as_str())
                                    ),
                                )
                                .with_suggestion(t!("rules.gm_004.suggestion")),
                            );
                            continue;
                        }

                        // Validate each hook in the array
                        if let Some(arr) = hooks_array.as_array() {
                            for hook_value in arr {
                                let hook: GeminiHook = match serde_json::from_value(hook_value.clone()) {
                                    Ok(h) => h,
                                    Err(_) => {
                                        let line = find_key_line(content, event_name).unwrap_or(1);
                                        diagnostics.push(
                                            Diagnostic::warning(
                                                path_buf.clone(),
                                                line,
                                                0,
                                                "GM-004",
                                                t!(
                                                    "rules.gm_004.message",
                                                    description = t!(
                                                        "rules.gm_004.malformed_hook",
                                                        event = event_name.as_str()
                                                    )
                                                ),
                                            )
                                            .with_suggestion(t!("rules.gm_004.suggestion")),
                                        );
                                        continue;
                                    }
                                };

                                // Check required field: type
                                match &hook.type_ {
                                    None => {
                                        let line = find_key_line(content, event_name).unwrap_or(1);
                                        diagnostics.push(
                                            Diagnostic::warning(
                                                path_buf.clone(),
                                                line,
                                                0,
                                                "GM-004",
                                                t!(
                                                    "rules.gm_004.message",
                                                    description = t!(
                                                        "rules.gm_004.missing_field",
                                                        event = event_name.as_str(),
                                                        field = "type"
                                                    )
                                                ),
                                            )
                                            .with_suggestion(t!("rules.gm_004.suggestion")),
                                        );
                                    }
                                    Some(type_val) if type_val != "command" => {
                                        let line = find_key_line(content, event_name).unwrap_or(1);
                                        diagnostics.push(
                                            Diagnostic::warning(
                                                path_buf.clone(),
                                                line,
                                                0,
                                                "GM-004",
                                                t!(
                                                    "rules.gm_004.message",
                                                    description = t!(
                                                        "rules.gm_004.invalid_type",
                                                        value = type_val.as_str()
                                                    )
                                                ),
                                            )
                                            .with_suggestion(t!("rules.gm_004.suggestion")),
                                        );
                                    }
                                    _ => {}
                                }

                                // Check required field: command (only if type is "command")
                                if hook.type_.as_deref() == Some("command") {
                                    let command_missing = match &hook.command {
                                        None => true,
                                        Some(cmd) => cmd.is_empty(),
                                    };
                                    if command_missing {
                                        let line = find_key_line(content, event_name).unwrap_or(1);
                                        diagnostics.push(
                                            Diagnostic::warning(
                                                path_buf.clone(),
                                                line,
                                                0,
                                                "GM-004",
                                                t!(
                                                    "rules.gm_004.message",
                                                    description = t!(
                                                        "rules.gm_004.missing_field",
                                                        event = event_name.as_str(),
                                                        field = "command"
                                                    )
                                                ),
                                            )
                                            .with_suggestion(t!("rules.gm_004.suggestion")),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        diagnostics
    }
}

/// Find the 1-indexed line number of a JSON key in the content.
/// Checks for a colon after the quoted key to avoid matching string values.
fn find_key_line(content: &str, key: &str) -> Option<usize> {
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
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;

    fn validate(content: &str) -> Vec<Diagnostic> {
        let validator = GeminiSettingsValidator;
        validator.validate(
            Path::new(".gemini/settings.json"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = GeminiSettingsValidator;
        validator.validate(Path::new(".gemini/settings.json"), content, config)
    }

    // ===== GM-009: Parse Error =====

    #[test]
    fn test_gm_009_invalid_json() {
        let diagnostics = validate("{ invalid }");
        let gm_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-009").collect();
        assert_eq!(gm_009.len(), 1);
        assert_eq!(gm_009[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_gm_009_empty_content() {
        let diagnostics = validate("");
        let gm_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-009").collect();
        assert_eq!(gm_009.len(), 1);
    }

    #[test]
    fn test_gm_009_unknown_top_key() {
        let content = r#"{"general": {}, "badKey": true}"#;
        let diagnostics = validate(content);
        let gm_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-009").collect();
        assert_eq!(gm_009.len(), 1);
        assert_eq!(gm_009[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_gm_009_valid_json_no_issues() {
        let diagnostics = validate(r#"{"general": {}}"#);
        let gm_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-009").collect();
        assert!(gm_009.is_empty());
    }

    #[test]
    fn test_gm_009_blocks_further_rules() {
        let diagnostics = validate("{ invalid }");
        assert!(diagnostics.iter().all(|d| d.rule == "GM-009"));
    }

    // ===== GM-004: Invalid Hooks Configuration =====

    #[test]
    fn test_gm_004_unknown_event() {
        let content = r#"{
  "hooksConfig": {
    "InvalidEvent": [
      {"type": "command", "command": "echo test"}
    ]
  }
}"#;
        let diagnostics = validate(content);
        let gm_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-004").collect();
        assert_eq!(gm_004.len(), 1);
        assert_eq!(gm_004[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_gm_004_missing_type() {
        let content = r#"{
  "hooksConfig": {
    "BeforeAgent": [
      {"command": "echo test"}
    ]
  }
}"#;
        let diagnostics = validate(content);
        let gm_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-004").collect();
        assert_eq!(gm_004.len(), 1);
    }

    #[test]
    fn test_gm_004_invalid_type() {
        let content = r#"{
  "hooksConfig": {
    "BeforeAgent": [
      {"type": "prompt", "command": "echo test"}
    ]
  }
}"#;
        let diagnostics = validate(content);
        let gm_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-004").collect();
        assert_eq!(gm_004.len(), 1);
    }

    #[test]
    fn test_gm_004_valid_hooks() {
        let content = r#"{
  "hooksConfig": {
    "BeforeAgent": [
      {"type": "command", "command": "echo test"}
    ],
    "AfterAgent": [
      {"type": "command", "command": "echo done", "name": "cleanup"}
    ]
  }
}"#;
        let diagnostics = validate(content);
        let gm_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-004").collect();
        assert!(gm_004.is_empty());
    }

    #[test]
    fn test_gm_004_no_hooks_no_error() {
        let diagnostics = validate(r#"{"general": {}}"#);
        let gm_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-004").collect();
        assert!(gm_004.is_empty());
    }

    // ===== Config Integration =====

    #[test]
    fn test_config_disabled_gemini_md_category() {
        let mut config = LintConfig::default();
        config.rules_mut().gemini_md = false;

        let diagnostics = validate_with_config("{ invalid }", &config);
        let gm_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("GM-"))
            .collect();
        assert!(gm_rules.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["GM-009".to_string()];

        let diagnostics = validate_with_config("{ invalid }", &config);
        let gm_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-009").collect();
        assert!(gm_009.is_empty());
    }

    // ===== Valid Settings =====

    #[test]
    fn test_valid_settings_no_issues() {
        let content = r#"{
  "general": {},
  "model": {},
  "hooksConfig": {
    "SessionStart": [
      {"type": "command", "command": "echo hello"}
    ]
  }
}"#;
        let diagnostics = validate(content);
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics, got: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_gm_004_missing_command() {
        let content = r#"{
      "hooksConfig": {
        "BeforeAgent": [
          {"type": "command"}
        ]
      }
    }"#;
        let diagnostics = validate(content);
        let gm_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-004").collect();
        assert!(!gm_004.is_empty(), "GM-004 should fire for missing command field");
    }

    #[test]
    fn test_gm_004_empty_command() {
        let content = r#"{
      "hooksConfig": {
        "BeforeAgent": [
          {"type": "command", "command": ""}
        ]
      }
    }"#;
        let diagnostics = validate(content);
        let gm_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-004").collect();
        assert!(!gm_004.is_empty(), "GM-004 should fire for empty command field");
    }
}
