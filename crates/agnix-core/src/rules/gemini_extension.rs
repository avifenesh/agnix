//! Gemini CLI extension manifest validation rules (GM-005, GM-008)
//!
//! Validates:
//! - GM-005: Invalid extension manifest (HIGH) - parse errors, missing required fields, invalid name
//! - GM-008: Invalid context file name configuration (LOW) - questionable contextFileName values

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::{Validator, ValidatorMetadata},
    schemas::gemini_extension::{REQUIRED_FIELDS, is_valid_extension_name, parse_gemini_extension},
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &["GM-005", "GM-008"];

pub struct GeminiExtensionValidator;

impl Validator for GeminiExtensionValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let path_buf = path.to_path_buf();

        let parsed = parse_gemini_extension(content);

        // GM-005: Parse error (ERROR)
        if let Some(ref error) = parsed.parse_error {
            if config.is_rule_enabled("GM-005") {
                diagnostics.push(
                    Diagnostic::error(
                        path_buf.clone(),
                        error.line,
                        error.column,
                        "GM-005",
                        t!(
                            "rules.gm_005.message",
                            description =
                                t!("rules.gm_005.parse_error", error = error.message.as_str())
                        ),
                    )
                    .with_suggestion(t!("rules.gm_005.suggestion")),
                );
            }
            return diagnostics;
        }

        let schema = match parsed.schema {
            Some(s) => s,
            None => return diagnostics,
        };

        // GM-005: Check required fields
        if config.is_rule_enabled("GM-005") {
            for &field in REQUIRED_FIELDS {
                let value = match field {
                    "name" => &schema.name,
                    "version" => &schema.version,
                    "description" => &schema.description,
                    _ => continue,
                };

                let is_missing = match value {
                    None => true,
                    Some(v) => v.is_empty(),
                };

                if is_missing {
                    let line = find_key_line(content, field).unwrap_or(1);
                    diagnostics.push(
                        Diagnostic::error(
                            path_buf.clone(),
                            line,
                            0,
                            "GM-005",
                            t!(
                                "rules.gm_005.message",
                                description = t!("rules.gm_005.missing_field", field = field)
                            ),
                        )
                        .with_suggestion(t!("rules.gm_005.suggestion")),
                    );
                }
            }

            // Validate name format
            if let Some(ref name) = schema.name {
                if !name.is_empty() && !is_valid_extension_name(name) {
                    let line = find_key_line(content, "name").unwrap_or(1);
                    diagnostics.push(
                        Diagnostic::error(
                            path_buf.clone(),
                            line,
                            0,
                            "GM-005",
                            t!(
                                "rules.gm_005.message",
                                description = t!("rules.gm_005.invalid_name", name = name.as_str())
                            ),
                        )
                        .with_suggestion(t!("rules.gm_005.suggestion")),
                    );
                }
            }
        }

        // GM-008: Validate contextFileName
        if config.is_rule_enabled("GM-008") {
            if let Some(ref context_file) = schema.context_file_name {
                if !context_file.is_empty() {
                    // Check for path separators (should be just a filename)
                    let has_separator = context_file.contains('/') || context_file.contains('\\');
                    if has_separator {
                        let line = find_key_line(content, "contextFileName").unwrap_or(1);
                        diagnostics.push(
                            Diagnostic::info(
                                path_buf.clone(),
                                line,
                                0,
                                "GM-008",
                                t!(
                                    "rules.gm_008.message",
                                    description = t!("rules.gm_008.path_not_filename")
                                ),
                            )
                            .with_suggestion(t!("rules.gm_008.suggestion")),
                        );
                    }
                }
            }
        }

        diagnostics
    }
}

/// Find the 1-indexed line number of a JSON key in the content.
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
        let validator = GeminiExtensionValidator;
        validator.validate(
            Path::new("gemini-extension.json"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = GeminiExtensionValidator;
        validator.validate(Path::new("gemini-extension.json"), content, config)
    }

    // ===== GM-005: Parse Error =====

    #[test]
    fn test_gm_005_invalid_json() {
        let diagnostics = validate("{ invalid }");
        let gm_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-005").collect();
        assert_eq!(gm_005.len(), 1);
        assert_eq!(gm_005[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_gm_005_missing_required_fields() {
        let diagnostics = validate("{}");
        let gm_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-005").collect();
        // Should report missing name, version, description
        assert_eq!(gm_005.len(), 3);
    }

    #[test]
    fn test_gm_005_missing_name() {
        let content = r#"{"version": "1.0.0", "description": "Test"}"#;
        let diagnostics = validate(content);
        let gm_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-005").collect();
        assert_eq!(gm_005.len(), 1);
    }

    #[test]
    fn test_gm_005_invalid_name_format() {
        let content = r#"{"name": "My Extension", "version": "1.0.0", "description": "Test"}"#;
        let diagnostics = validate(content);
        let gm_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-005").collect();
        assert_eq!(gm_005.len(), 1);
    }

    #[test]
    fn test_gm_005_valid_extension() {
        let content =
            r#"{"name": "my-extension", "version": "1.0.0", "description": "A test extension"}"#;
        let diagnostics = validate(content);
        let gm_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-005").collect();
        assert!(gm_005.is_empty());
    }

    // ===== GM-008: Context File Name =====

    #[test]
    fn test_gm_008_context_file_with_path_separator() {
        let content = r#"{"name": "ext", "version": "1.0.0", "description": "Test", "contextFileName": "docs/context.md"}"#;
        let diagnostics = validate(content);
        let gm_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-008").collect();
        assert_eq!(gm_008.len(), 1);
        assert_eq!(gm_008[0].level, DiagnosticLevel::Info);
    }

    #[test]
    fn test_gm_008_valid_context_file() {
        let content = r#"{"name": "ext", "version": "1.0.0", "description": "Test", "contextFileName": "CONTEXT.md"}"#;
        let diagnostics = validate(content);
        let gm_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-008").collect();
        assert!(gm_008.is_empty());
    }

    #[test]
    fn test_gm_008_no_context_file() {
        let content = r#"{"name": "ext", "version": "1.0.0", "description": "Test"}"#;
        let diagnostics = validate(content);
        let gm_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-008").collect();
        assert!(gm_008.is_empty());
    }

    // ===== GM-005: Empty field edge cases =====

    #[test]
    fn test_gm_005_empty_required_fields() {
        let content = r#"{"name": "", "version": "", "description": ""}"#;
        let diagnostics = validate(content);
        let gm_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-005").collect();
        assert_eq!(
            gm_005.len(),
            3,
            "GM-005 should fire for each empty required field (name, version, description)"
        );
    }

    // ===== GM-008: Windows path separator =====

    #[test]
    fn test_gm_008_windows_path_separator() {
        let content = r#"{"name": "ext", "version": "1.0.0", "description": "Test", "contextFileName": "docs\\context.md"}"#;
        let diagnostics = validate(content);
        let gm_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-008").collect();
        assert_eq!(
            gm_008.len(),
            1,
            "GM-008 should fire for contextFileName containing backslash path separator"
        );
        assert_eq!(gm_008[0].level, DiagnosticLevel::Info);
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
        config.rules_mut().disabled_rules = vec!["GM-005".to_string()];

        let diagnostics = validate_with_config("{ invalid }", &config);
        let gm_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-005").collect();
        assert!(gm_005.is_empty());
    }
}
