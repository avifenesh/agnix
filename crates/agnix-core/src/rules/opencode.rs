//! OpenCode configuration validation rules (OC-001 to OC-003)
//!
//! Validates:
//! - OC-001: Invalid share mode (HIGH) - must be "manual", "auto", or "disabled"
//! - OC-002: Invalid instruction path (HIGH) - paths must exist or be valid globs
//! - OC-003: opencode.json parse error (HIGH) - must be valid JSON/JSONC

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::Validator,
    schemas::opencode::{
        VALID_SHARE_MODES, is_glob_pattern, parse_opencode_json, validate_glob_pattern,
    },
};
use rust_i18n::t;
use std::path::Path;

pub struct OpenCodeValidator;

impl Validator for OpenCodeValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // OC-003: Parse error (ERROR)
        let parsed = parse_opencode_json(content);
        if let Some(ref error) = parsed.parse_error {
            if config.is_rule_enabled("OC-003") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        error.line,
                        error.column,
                        "OC-003",
                        t!("rules.oc_003.message", error = error.message.as_str()),
                    )
                    .with_suggestion(t!("rules.oc_003.suggestion")),
                );
            }
            // Can't continue if JSON is broken
            return diagnostics;
        }

        let schema = match parsed.schema {
            Some(s) => s,
            None => return diagnostics,
        };

        // OC-001: Invalid share mode (ERROR)
        if config.is_rule_enabled("OC-001") {
            if parsed.share_wrong_type {
                let line = find_key_line(content, "share").unwrap_or(1);
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        0,
                        "OC-001",
                        t!("rules.oc_001.type_error"),
                    )
                    .with_suggestion(t!("rules.oc_001.suggestion")),
                );
            } else if let Some(ref share_value) = schema.share {
                if !VALID_SHARE_MODES.contains(&share_value.as_str()) {
                    let line = find_key_line(content, "share").unwrap_or(1);
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            0,
                            "OC-001",
                            t!("rules.oc_001.message", value = share_value.as_str()),
                        )
                        .with_suggestion(t!("rules.oc_001.suggestion")),
                    );
                }
            }
        }

        // OC-002: Invalid instruction path (ERROR)
        if config.is_rule_enabled("OC-002") {
            if parsed.instructions_wrong_type {
                let instructions_line = find_key_line(content, "instructions").unwrap_or(1);
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        instructions_line,
                        0,
                        "OC-002",
                        t!("rules.oc_002.type_error"),
                    )
                    .with_suggestion(t!("rules.oc_002.suggestion")),
                );
            }
            if let Some(ref instructions) = schema.instructions {
                let config_dir = path.parent().unwrap_or(Path::new("."));
                let instructions_line = find_key_line(content, "instructions").unwrap_or(1);
                let fs = config.fs();

                for instruction_path in instructions {
                    if instruction_path.trim().is_empty() {
                        continue;
                    }

                    // Reject absolute paths and path traversal attempts
                    let p = Path::new(instruction_path);
                    if p.is_absolute()
                        || p.components().any(|c| c == std::path::Component::ParentDir)
                    {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                instructions_line,
                                0,
                                "OC-002",
                                t!("rules.oc_002.traversal", path = instruction_path.as_str()),
                            )
                            .with_suggestion(t!("rules.oc_002.suggestion")),
                        );
                        continue;
                    }

                    // If it's a glob pattern, validate the pattern syntax
                    if is_glob_pattern(instruction_path) {
                        if !validate_glob_pattern(instruction_path) {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    instructions_line,
                                    0,
                                    "OC-002",
                                    t!(
                                        "rules.oc_002.invalid_glob",
                                        path = instruction_path.as_str()
                                    ),
                                )
                                .with_suggestion(t!("rules.oc_002.suggestion")),
                            );
                        }
                        // Valid glob patterns are allowed even if no files match yet
                        continue;
                    }

                    // For non-glob paths, check if the file exists relative to config dir
                    let resolved = config_dir.join(instruction_path);
                    if !fs.exists(&resolved) {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                instructions_line,
                                0,
                                "OC-002",
                                t!("rules.oc_002.not_found", path = instruction_path.as_str()),
                            )
                            .with_suggestion(t!("rules.oc_002.suggestion")),
                        );
                    }
                }
            }
        }

        diagnostics
    }
}

/// Find the 1-indexed line number of a JSON key in the content.
///
/// Looks for `"key"` followed by `:` to avoid matching the key name
/// when it appears as a string value rather than an object key.
fn find_key_line(content: &str, key: &str) -> Option<usize> {
    let needle = format!("\"{}\"", key);
    for (i, line) in content.lines().enumerate() {
        if let Some(pos) = line.find(&needle) {
            // Check that a colon follows the key (possibly with whitespace)
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
        let validator = OpenCodeValidator;
        validator.validate(Path::new("opencode.json"), content, &LintConfig::default())
    }

    fn validate_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = OpenCodeValidator;
        validator.validate(Path::new("opencode.json"), content, config)
    }

    // ===== OC-003: Parse Error =====

    #[test]
    fn test_oc_003_invalid_json() {
        let diagnostics = validate("{ invalid json }");
        let oc_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-003").collect();
        assert_eq!(oc_003.len(), 1);
        assert_eq!(oc_003[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_oc_003_empty_content() {
        let diagnostics = validate("");
        let oc_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-003").collect();
        assert_eq!(oc_003.len(), 1);
    }

    #[test]
    fn test_oc_003_trailing_comma() {
        let diagnostics = validate(r#"{"share": "manual",}"#);
        let oc_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-003").collect();
        assert_eq!(oc_003.len(), 1);
    }

    #[test]
    fn test_oc_003_valid_json() {
        let diagnostics = validate(r#"{"share": "manual"}"#);
        let oc_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-003").collect();
        assert!(oc_003.is_empty());
    }

    #[test]
    fn test_oc_003_jsonc_comments_allowed() {
        let content = r#"{
  // This is a JSONC comment
  "share": "manual"
}"#;
        let diagnostics = validate(content);
        let oc_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-003").collect();
        assert!(oc_003.is_empty());
    }

    #[test]
    fn test_oc_003_blocks_further_rules() {
        // When JSON is invalid, no OC-001/OC-002 should fire
        let diagnostics = validate("{ invalid }");
        assert!(diagnostics.iter().all(|d| d.rule == "OC-003"));
    }

    // ===== OC-001: Invalid Share Mode =====

    #[test]
    fn test_oc_001_invalid_share_mode() {
        let diagnostics = validate(r#"{"share": "public"}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1);
        assert_eq!(oc_001[0].level, DiagnosticLevel::Error);
        assert!(oc_001[0].message.contains("public"));
    }

    #[test]
    fn test_oc_001_valid_manual() {
        let diagnostics = validate(r#"{"share": "manual"}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert!(oc_001.is_empty());
    }

    #[test]
    fn test_oc_001_valid_auto() {
        let diagnostics = validate(r#"{"share": "auto"}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert!(oc_001.is_empty());
    }

    #[test]
    fn test_oc_001_valid_disabled() {
        let diagnostics = validate(r#"{"share": "disabled"}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert!(oc_001.is_empty());
    }

    #[test]
    fn test_oc_001_absent_share() {
        // No share field at all should not trigger OC-001
        let diagnostics = validate(r#"{}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert!(oc_001.is_empty());
    }

    #[test]
    fn test_oc_001_empty_string() {
        let diagnostics = validate(r#"{"share": ""}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1);
    }

    #[test]
    fn test_oc_001_case_sensitive() {
        let diagnostics = validate(r#"{"share": "Manual"}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1, "Share mode should be case-sensitive");
    }

    #[test]
    fn test_oc_001_line_number() {
        let content = "{\n  \"share\": \"invalid\"\n}";
        let diagnostics = validate(content);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1);
        assert_eq!(oc_001[0].line, 2);
    }

    // ===== OC-002: Invalid Instruction Path =====

    #[test]
    fn test_oc_002_nonexistent_path() {
        let diagnostics =
            validate(r#"{"instructions": ["nonexistent-file-that-does-not-exist.md"]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert_eq!(oc_002.len(), 1);
        assert_eq!(oc_002[0].level, DiagnosticLevel::Error);
        assert!(oc_002[0].message.contains("nonexistent-file"));
    }

    #[test]
    fn test_oc_002_valid_glob_pattern() {
        // Valid glob patterns should pass even if no files match
        let diagnostics = validate(r#"{"instructions": ["**/*.md"]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert!(oc_002.is_empty());
    }

    #[test]
    fn test_oc_002_invalid_glob_pattern() {
        let diagnostics = validate(r#"{"instructions": ["[unclosed"]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert_eq!(oc_002.len(), 1);
    }

    #[test]
    fn test_oc_002_absent_instructions() {
        // No instructions field should not trigger OC-002
        let diagnostics = validate(r#"{}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert!(oc_002.is_empty());
    }

    #[test]
    fn test_oc_002_empty_instructions_array() {
        let diagnostics = validate(r#"{"instructions": []}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert!(oc_002.is_empty());
    }

    #[test]
    fn test_oc_002_multiple_invalid_paths() {
        let diagnostics = validate(r#"{"instructions": ["nonexistent1.md", "nonexistent2.md"]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert_eq!(oc_002.len(), 2);
    }

    #[test]
    fn test_oc_002_mixed_valid_invalid() {
        // Glob patterns pass, nonexistent literal paths fail
        let diagnostics = validate(r#"{"instructions": ["**/*.md", "nonexistent.md"]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert_eq!(oc_002.len(), 1);
        assert!(oc_002[0].message.contains("nonexistent.md"));
    }

    #[test]
    fn test_oc_002_empty_path_skipped() {
        let diagnostics = validate(r#"{"instructions": [""]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert!(oc_002.is_empty());
    }

    // ===== Config Integration =====

    #[test]
    fn test_config_disabled_opencode_category() {
        let mut config = LintConfig::default();
        config.rules.opencode = false;

        let diagnostics = validate_with_config(r#"{"share": "invalid"}"#, &config);
        let oc_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("OC-"))
            .collect();
        assert!(oc_rules.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["OC-001".to_string()];

        let diagnostics = validate_with_config(r#"{"share": "invalid"}"#, &config);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert!(oc_001.is_empty());
    }

    #[test]
    fn test_all_oc_rules_can_be_disabled() {
        let rules = ["OC-001", "OC-002", "OC-003"];

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules.disabled_rules = vec![rule.to_string()];

            let content = match rule {
                "OC-001" => r#"{"share": "invalid"}"#,
                "OC-002" => r#"{"instructions": ["nonexistent.md"]}"#,
                "OC-003" => "{ invalid }",
                _ => unreachable!(),
            };

            let diagnostics = validate_with_config(content, &config);
            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }

    // ===== Valid Config =====

    #[test]
    fn test_valid_config_no_issues() {
        let content = r#"{
  "share": "manual",
  "instructions": ["**/*.md"]
}"#;
        let diagnostics = validate(content);
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics, got: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_empty_object_no_issues() {
        let diagnostics = validate("{}");
        assert!(diagnostics.is_empty());
    }

    // ===== Path Traversal Prevention =====

    #[test]
    fn test_oc_002_absolute_path_rejected() {
        let diagnostics = validate(r#"{"instructions": ["/etc/passwd"]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert_eq!(oc_002.len(), 1);
    }

    #[test]
    fn test_oc_002_parent_dir_traversal_rejected() {
        let diagnostics = validate(r#"{"instructions": ["../../etc/shadow"]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert_eq!(oc_002.len(), 1);
    }

    // ===== Type Mismatch Handling =====

    #[test]
    fn test_type_mismatch_share_not_string() {
        // "share": true is valid JSON but wrong type; should not be OC-003
        let diagnostics = validate(r#"{"share": true}"#);
        let oc_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-003").collect();
        assert!(
            oc_003.is_empty(),
            "Type mismatch should not be a parse error"
        );
        // Should emit OC-001 for wrong type
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1, "Wrong type share should trigger OC-001");
        assert!(oc_001[0].message.contains("string"));
    }

    #[test]
    fn test_type_mismatch_share_number() {
        let diagnostics = validate(r#"{"share": 123}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1, "Numeric share should trigger OC-001");
    }

    #[test]
    fn test_type_mismatch_instructions_not_array() {
        // "instructions": "README.md" is valid JSON but wrong type
        let diagnostics = validate(r#"{"instructions": "README.md"}"#);
        let oc_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-003").collect();
        assert!(
            oc_003.is_empty(),
            "Type mismatch should not be a parse error"
        );
        // Should emit OC-002 for wrong type
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert_eq!(
            oc_002.len(),
            1,
            "Non-array instructions should trigger OC-002"
        );
        assert!(oc_002[0].message.contains("array"));
    }

    #[test]
    fn test_type_mismatch_instructions_with_non_string_elements() {
        let diagnostics = validate(r#"{"instructions": [123, true]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert!(
            !oc_002.is_empty(),
            "Non-string array elements should trigger OC-002"
        );
    }

    // ===== find_key_line =====

    #[test]
    fn test_find_key_line() {
        let content = "{\n  \"share\": \"manual\",\n  \"instructions\": []\n}";
        assert_eq!(find_key_line(content, "share"), Some(2));
        assert_eq!(find_key_line(content, "instructions"), Some(3));
        assert_eq!(find_key_line(content, "nonexistent"), None);
    }

    #[test]
    fn test_find_key_line_ignores_value_match() {
        // "share" appears as a value, not as a key
        let content = r#"{"comment": "the share key is important", "share": "manual"}"#;
        // Should still find "share" as a key (second occurrence)
        assert_eq!(find_key_line(content, "share"), Some(1));
    }

    #[test]
    fn test_find_key_line_no_false_positive_on_value() {
        // "share" only appears as a value, never as a key
        let content = "{\n  \"comment\": \"share\"\n}";
        assert_eq!(find_key_line(content, "share"), None);
    }
}
