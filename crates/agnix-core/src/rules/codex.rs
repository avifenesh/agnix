//! Codex CLI configuration validation rules (CDX-001 to CDX-003)
//!
//! Validates:
//! - CDX-001: Invalid approvalMode (HIGH) - must be "suggest", "auto-edit", or "full-auto"
//! - CDX-002: Invalid fullAutoErrorMode (HIGH) - must be "ask-user" or "ignore-and-continue"
//! - CDX-003: AGENTS.override.md in version control (MEDIUM) - should be in .gitignore

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::Validator,
    schemas::codex::{VALID_APPROVAL_MODES, VALID_FULL_AUTO_ERROR_MODES, parse_codex_toml},
};
use rust_i18n::t;
use std::collections::HashMap;
use std::path::Path;

pub struct CodexValidator;

impl Validator for CodexValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Determine whether this is a .md file (ClaudeMd) or a .toml file (CodexConfig)
        // using a direct filename check instead of the full detect_file_type() call.
        // This runs on every ClaudeMd file but the cost is negligible: a single
        // OsStr comparison before early return.
        let is_markdown = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|name| name.ends_with(".md"));

        if is_markdown {
            if config.is_rule_enabled("CDX-003") {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename == "AGENTS.override.md" {
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                1,
                                0,
                                "CDX-003",
                                t!("rules.cdx_003.message"),
                            )
                            .with_suggestion(t!("rules.cdx_003.suggestion")),
                        );
                    }
                }
            }
            return diagnostics;
        }

        // For CodexConfig files, check CDX-001 and CDX-002
        // Skip TOML parsing entirely when both rules are disabled (performance)
        let cdx_001_enabled = config.is_rule_enabled("CDX-001");
        let cdx_002_enabled = config.is_rule_enabled("CDX-002");
        if !cdx_001_enabled && !cdx_002_enabled {
            return diagnostics;
        }

        let parsed = parse_codex_toml(content);

        // If TOML is broken, we cannot validate further
        if parsed.parse_error.is_some() {
            return diagnostics;
        }

        let schema = match parsed.schema {
            Some(s) => s,
            None => return diagnostics,
        };

        // Build key-to-line mappings in a single pass for O(1) lookups
        let key_lines = build_key_line_map(content);

        // CDX-001: Invalid approvalMode (ERROR)
        if cdx_001_enabled {
            if parsed.approval_mode_wrong_type {
                let line = key_lines.get("approvalMode").copied().unwrap_or(1);
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        0,
                        "CDX-001",
                        t!("rules.cdx_001.type_error"),
                    )
                    .with_suggestion(t!("rules.cdx_001.suggestion")),
                );
            } else if let Some(ref approval_value) = schema.approval_mode {
                if !VALID_APPROVAL_MODES.contains(&approval_value.as_str()) {
                    let line = key_lines.get("approvalMode").copied().unwrap_or(1);
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            0,
                            "CDX-001",
                            t!("rules.cdx_001.message", value = approval_value.as_str()),
                        )
                        .with_suggestion(t!("rules.cdx_001.suggestion")),
                    );
                }
            }
        }

        // CDX-002: Invalid fullAutoErrorMode (ERROR)
        if cdx_002_enabled {
            if parsed.full_auto_error_mode_wrong_type {
                let line = key_lines.get("fullAutoErrorMode").copied().unwrap_or(1);
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        0,
                        "CDX-002",
                        t!("rules.cdx_002.type_error"),
                    )
                    .with_suggestion(t!("rules.cdx_002.suggestion")),
                );
            } else if let Some(ref error_mode_value) = schema.full_auto_error_mode {
                if !VALID_FULL_AUTO_ERROR_MODES.contains(&error_mode_value.as_str()) {
                    let line = key_lines.get("fullAutoErrorMode").copied().unwrap_or(1);
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            0,
                            "CDX-002",
                            t!("rules.cdx_002.message", value = error_mode_value.as_str()),
                        )
                        .with_suggestion(t!("rules.cdx_002.suggestion")),
                    );
                }
            }
        }

        diagnostics
    }
}

/// Build a map of TOML key names to their 1-indexed line numbers in a single pass.
///
/// Scans each line for a bare key followed by `=` (the TOML key-value separator).
/// Extracts keys by finding '=' positions; indexing is safe because find() returns
/// char-boundary positions in valid UTF-8. Prevents partial matches by extracting
/// only up to `=` (e.g., `approvalMode` will not match `approvalModeExtra`).
///
/// Returns only the first occurrence of each key, which matches TOML semantics.
fn build_key_line_map(content: &str) -> HashMap<&str, usize> {
    let mut map = HashMap::new();
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        // Extract the key portion: everything up to `=` or whitespace before `=`
        if let Some(eq_pos) = trimmed.find('=') {
            let key = trimmed[..eq_pos].trim_end();
            // Only record the first occurrence (TOML spec: duplicate keys are errors)
            if !key.is_empty() && !map.contains_key(key) {
                map.insert(key, i + 1);
            }
        }
    }
    map
}

/// Find the 1-indexed line number of a TOML key in the content.
///
/// Uses `strip_prefix` for UTF-8 safety and verifies the next non-whitespace
/// character is `=` to prevent partial key matches (e.g., `approvalMode`
/// does not match `approvalModeExtra`).
///
/// Production code uses `build_key_line_map` for single-pass efficiency;
/// this function is retained for targeted lookups in tests.
#[cfg(test)]
fn find_key_line(content: &str, key: &str) -> Option<usize> {
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(after) = trimmed.strip_prefix(key) {
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
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;

    fn validate_config(content: &str) -> Vec<Diagnostic> {
        let validator = CodexValidator;
        validator.validate(
            Path::new(".codex/config.toml"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_config_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = CodexValidator;
        validator.validate(Path::new(".codex/config.toml"), content, config)
    }

    fn validate_claude_md(path: &str, content: &str) -> Vec<Diagnostic> {
        let validator = CodexValidator;
        validator.validate(Path::new(path), content, &LintConfig::default())
    }

    // ===== CDX-001: Invalid approvalMode =====

    #[test]
    fn test_cdx_001_invalid_approval_mode() {
        let diagnostics = validate_config("approvalMode = \"yolo\"");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1);
        assert_eq!(cdx_001[0].level, DiagnosticLevel::Error);
        assert!(cdx_001[0].message.contains("yolo"));
    }

    #[test]
    fn test_cdx_001_valid_suggest() {
        let diagnostics = validate_config("approvalMode = \"suggest\"");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert!(cdx_001.is_empty());
    }

    #[test]
    fn test_cdx_001_valid_auto_edit() {
        let diagnostics = validate_config("approvalMode = \"auto-edit\"");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert!(cdx_001.is_empty());
    }

    #[test]
    fn test_cdx_001_valid_full_auto() {
        let diagnostics = validate_config("approvalMode = \"full-auto\"");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert!(cdx_001.is_empty());
    }

    #[test]
    fn test_cdx_001_all_valid_modes() {
        for mode in VALID_APPROVAL_MODES {
            let content = format!("approvalMode = \"{}\"", mode);
            let diagnostics = validate_config(&content);
            let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
            assert!(cdx_001.is_empty(), "Mode '{}' should be valid", mode);
        }
    }

    #[test]
    fn test_cdx_001_absent_approval_mode() {
        let diagnostics = validate_config("model = \"o4-mini\"");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert!(cdx_001.is_empty());
    }

    #[test]
    fn test_cdx_001_empty_string() {
        let diagnostics = validate_config("approvalMode = \"\"");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1);
    }

    #[test]
    fn test_cdx_001_case_sensitive() {
        let diagnostics = validate_config("approvalMode = \"Suggest\"");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1, "approvalMode should be case-sensitive");
    }

    #[test]
    fn test_cdx_001_type_mismatch() {
        let diagnostics = validate_config("approvalMode = true");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1);
        assert!(cdx_001[0].message.contains("string"));
    }

    #[test]
    fn test_cdx_001_type_mismatch_number() {
        let diagnostics = validate_config("approvalMode = 42");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1);
    }

    #[test]
    fn test_cdx_001_type_mismatch_float() {
        let diagnostics = validate_config("approvalMode = 1.5");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1);
        assert!(
            cdx_001[0].message.contains("string"),
            "Expected type error message for float value"
        );
    }

    #[test]
    fn test_cdx_001_type_mismatch_array() {
        let diagnostics = validate_config("approvalMode = [\"suggest\"]");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1);
        assert!(
            cdx_001[0].message.contains("string"),
            "Expected type error message for array value"
        );
    }

    #[test]
    fn test_cdx_001_line_number() {
        let content = "model = \"o4-mini\"\napprovalMode = \"invalid\"";
        let diagnostics = validate_config(content);
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1);
        assert_eq!(cdx_001[0].line, 2);
    }

    // ===== CDX-002: Invalid fullAutoErrorMode =====

    #[test]
    fn test_cdx_002_invalid_error_mode() {
        let diagnostics = validate_config("fullAutoErrorMode = \"crash\"");
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert_eq!(cdx_002.len(), 1);
        assert_eq!(cdx_002[0].level, DiagnosticLevel::Error);
        assert!(cdx_002[0].message.contains("crash"));
    }

    #[test]
    fn test_cdx_002_valid_ask_user() {
        let diagnostics = validate_config("fullAutoErrorMode = \"ask-user\"");
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert!(cdx_002.is_empty());
    }

    #[test]
    fn test_cdx_002_valid_ignore_and_continue() {
        let diagnostics = validate_config("fullAutoErrorMode = \"ignore-and-continue\"");
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert!(cdx_002.is_empty());
    }

    #[test]
    fn test_cdx_002_all_valid_modes() {
        for mode in VALID_FULL_AUTO_ERROR_MODES {
            let content = format!("fullAutoErrorMode = \"{}\"", mode);
            let diagnostics = validate_config(&content);
            let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
            assert!(cdx_002.is_empty(), "Mode '{}' should be valid", mode);
        }
    }

    #[test]
    fn test_cdx_002_absent_error_mode() {
        let diagnostics = validate_config("model = \"o4-mini\"");
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert!(cdx_002.is_empty());
    }

    #[test]
    fn test_cdx_002_empty_string() {
        let diagnostics = validate_config("fullAutoErrorMode = \"\"");
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert_eq!(cdx_002.len(), 1);
    }

    #[test]
    fn test_cdx_002_type_mismatch() {
        let diagnostics = validate_config("fullAutoErrorMode = false");
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert_eq!(cdx_002.len(), 1);
        assert!(cdx_002[0].message.contains("string"));
    }

    #[test]
    fn test_cdx_002_case_sensitive() {
        let diagnostics = validate_config("fullAutoErrorMode = \"Ask-User\"");
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert_eq!(
            cdx_002.len(),
            1,
            "fullAutoErrorMode should be case-sensitive"
        );
    }

    #[test]
    fn test_cdx_002_line_number() {
        let content = "model = \"o4-mini\"\nfullAutoErrorMode = \"crash\"";
        let diagnostics = validate_config(content);
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert_eq!(cdx_002.len(), 1);
        assert_eq!(cdx_002[0].line, 2);
    }

    // ===== CDX-003: AGENTS.override.md in version control =====

    #[test]
    fn test_cdx_003_agents_override_md() {
        let diagnostics = validate_claude_md("AGENTS.override.md", "# Override");
        let cdx_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-003").collect();
        assert_eq!(cdx_003.len(), 1);
        assert_eq!(cdx_003[0].level, DiagnosticLevel::Warning);
        assert!(cdx_003[0].message.contains("AGENTS.override.md"));
    }

    #[test]
    fn test_cdx_003_not_triggered_on_claude_md() {
        let diagnostics = validate_claude_md("CLAUDE.md", "# My project");
        let cdx_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-003").collect();
        assert!(cdx_003.is_empty());
    }

    #[test]
    fn test_cdx_003_not_triggered_on_agents_md() {
        let diagnostics = validate_claude_md("AGENTS.md", "# My project");
        let cdx_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-003").collect();
        assert!(cdx_003.is_empty());
    }

    #[test]
    fn test_cdx_003_case_sensitive_extension() {
        // AGENTS.override.MD (wrong extension case) should NOT trigger CDX-003
        let diagnostics = validate_claude_md("AGENTS.override.MD", "# test");
        let cdx_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-003").collect();
        assert!(
            cdx_003.is_empty(),
            "CDX-003 should not fire for AGENTS.override.MD"
        );
    }

    #[test]
    fn test_cdx_003_not_triggered_on_config() {
        let diagnostics = validate_config("approvalMode = \"suggest\"");
        let cdx_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-003").collect();
        assert!(cdx_003.is_empty());
    }

    // ===== Config Integration =====

    #[test]
    fn test_config_disabled_codex_category() {
        let mut config = LintConfig::default();
        config.rules.codex = false;

        let diagnostics = validate_config_with_config("approvalMode = \"invalid\"", &config);
        let cdx_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("CDX-"))
            .collect();
        assert!(cdx_rules.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["CDX-001".to_string()];

        let diagnostics = validate_config_with_config("approvalMode = \"invalid\"", &config);
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert!(cdx_001.is_empty());
    }

    #[test]
    fn test_all_cdx_rules_can_be_disabled() {
        let rules = ["CDX-001", "CDX-002", "CDX-003"];

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules.disabled_rules = vec![rule.to_string()];

            let (content, path): (&str, &str) = match rule {
                "CDX-001" => ("approvalMode = \"invalid\"", ".codex/config.toml"),
                "CDX-002" => ("fullAutoErrorMode = \"crash\"", ".codex/config.toml"),
                "CDX-003" => ("# Override", "AGENTS.override.md"),
                _ => unreachable!(),
            };

            let validator = CodexValidator;
            let diagnostics = validator.validate(Path::new(path), content, &config);

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
        let content = r#"
model = "o4-mini"
approvalMode = "suggest"
fullAutoErrorMode = "ask-user"
notify = true
"#;
        let diagnostics = validate_config(content);
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics, got: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_empty_config_no_issues() {
        let diagnostics = validate_config("");
        assert!(diagnostics.is_empty());
    }

    // ===== Multiple Issues =====

    #[test]
    fn test_multiple_issues() {
        let content = "approvalMode = \"yolo\"\nfullAutoErrorMode = \"crash\"";
        let diagnostics = validate_config(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-001"));
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-002"));
    }

    #[test]
    fn test_cdx_002_empty_with_cdx_001_invalid() {
        // Both CDX-001 (invalid value) and CDX-002 (empty string) should fire together
        let content = "approvalMode = \"invalid\"\nfullAutoErrorMode = \"\"";
        let diagnostics = validate_config(content);
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert_eq!(
            cdx_001.len(),
            1,
            "CDX-001 should fire for invalid approvalMode"
        );
        assert_eq!(
            cdx_002.len(),
            1,
            "CDX-002 should fire for empty fullAutoErrorMode"
        );
    }

    #[test]
    fn test_both_fields_wrong_type() {
        let content = "approvalMode = true\nfullAutoErrorMode = 123";
        let diagnostics = validate_config(content);
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert_eq!(
            cdx_001.len(),
            1,
            "CDX-001 should fire for wrong-type approvalMode"
        );
        assert_eq!(
            cdx_002.len(),
            1,
            "CDX-002 should fire for wrong-type fullAutoErrorMode"
        );
        assert!(cdx_001[0].message.contains("string"));
        assert!(cdx_002[0].message.contains("string"));
    }

    // ===== Fixture Integration =====

    #[test]
    fn test_valid_codex_fixture_no_diagnostics() {
        let fixture = include_str!("../../../../tests/fixtures/codex/.codex/config.toml");
        let diagnostics = validate_config(fixture);
        assert!(
            diagnostics.is_empty(),
            "Valid codex fixture should produce 0 diagnostics, got: {:?}",
            diagnostics
        );
    }

    // ===== find_key_line =====

    #[test]
    fn test_find_key_line() {
        let content =
            "model = \"o4-mini\"\napprovalMode = \"suggest\"\nfullAutoErrorMode = \"ask-user\"";
        assert_eq!(find_key_line(content, "model"), Some(1));
        assert_eq!(find_key_line(content, "approvalMode"), Some(2));
        assert_eq!(find_key_line(content, "fullAutoErrorMode"), Some(3));
        assert_eq!(find_key_line(content, "nonexistent"), None);
    }

    #[test]
    fn test_find_key_line_ignores_value_match() {
        // "approvalMode" appears as part of a string value, not as a key
        let content = "comment = \"the approvalMode field\"\napprovalMode = \"suggest\"";
        assert_eq!(find_key_line(content, "approvalMode"), Some(2));
    }

    #[test]
    fn test_find_key_line_at_start_of_content() {
        // Key on the very first line with no preceding content
        let content = "approvalMode = \"suggest\"";
        assert_eq!(find_key_line(content, "approvalMode"), Some(1));
    }

    #[test]
    fn test_find_key_line_with_leading_whitespace() {
        // Key with leading whitespace (indented)
        let content = "  approvalMode = \"suggest\"";
        assert_eq!(find_key_line(content, "approvalMode"), Some(1));
    }

    #[test]
    fn test_find_key_line_no_partial_match() {
        // "approvalMode" must not match "approvalModeExtra"
        let content = "approvalModeExtra = \"value\"\napprovalMode = \"suggest\"";
        assert_eq!(find_key_line(content, "approvalMode"), Some(2));
    }

    // ===== build_key_line_map =====

    #[test]
    fn test_build_key_line_map() {
        let content =
            "model = \"o4-mini\"\napprovalMode = \"suggest\"\nfullAutoErrorMode = \"ask-user\"";
        let map = build_key_line_map(content);
        assert_eq!(map.get("model"), Some(&1));
        assert_eq!(map.get("approvalMode"), Some(&2));
        assert_eq!(map.get("fullAutoErrorMode"), Some(&3));
        assert_eq!(map.get("nonexistent"), None);
    }

    #[test]
    fn test_build_key_line_map_no_partial_match() {
        let content = "approvalModeExtra = \"value\"\napprovalMode = \"suggest\"";
        let map = build_key_line_map(content);
        assert_eq!(map.get("approvalModeExtra"), Some(&1));
        assert_eq!(map.get("approvalMode"), Some(&2));
    }
}

