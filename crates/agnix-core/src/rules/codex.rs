//! Codex CLI configuration validation rules (CDX-000 to CDX-005)
//!
//! Validates:
//! - CDX-000: TOML Parse Error (HIGH) - invalid TOML syntax in config.toml
//! - CDX-001: Invalid approvalMode (HIGH) - must be "suggest", "auto-edit", or "full-auto"
//! - CDX-002: Invalid fullAutoErrorMode (HIGH) - must be "ask-user" or "ignore-and-continue"
//! - CDX-003: AGENTS.override.md in version control (MEDIUM) - should be in .gitignore
//! - CDX-004: Unknown config key (MEDIUM) - unrecognized key in .codex/config.toml
//! - CDX-005: project_doc_max_bytes exceeds limit (HIGH) - must be <= 65536

use crate::{
    config::LintConfig,
    diagnostics::{Diagnostic, Fix},
    rules::{Validator, ValidatorMetadata},
    schemas::codex::{VALID_APPROVAL_MODES, VALID_FULL_AUTO_ERROR_MODES, parse_codex_toml},
};
use rust_i18n::t;
use std::collections::HashMap;
use std::path::Path;

use crate::rules::find_closest_value;

/// Find the byte span of a TOML string value for the given key.
/// Returns byte positions of the inner string (without quotes).
/// Returns None if the key is not found or appears more than once (uniqueness guard).
fn find_toml_string_value_span(
    content: &str,
    key: &str,
    current_value: &str,
) -> Option<(usize, usize)> {
    crate::span_utils::find_unique_toml_string_value(content, key, current_value)
}

const RULE_IDS: &[&str] = &[
    "CDX-000", "CDX-001", "CDX-002", "CDX-003", "CDX-004", "CDX-005",
];

pub struct CodexValidator;

impl Validator for CodexValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

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

        // For CodexConfig files, check CDX-001 through CDX-005
        // Skip TOML parsing entirely when all TOML-dependent rules are disabled
        let cdx_001_enabled = config.is_rule_enabled("CDX-001");
        let cdx_002_enabled = config.is_rule_enabled("CDX-002");
        let cdx_004_enabled = config.is_rule_enabled("CDX-004");
        let cdx_005_enabled = config.is_rule_enabled("CDX-005");
        if !cdx_001_enabled && !cdx_002_enabled && !cdx_004_enabled && !cdx_005_enabled {
            return diagnostics;
        }

        let parsed = parse_codex_toml(content);

        // If TOML is broken, emit a diagnostic so users can fix invalid syntax
        if config.is_rule_enabled("CDX-000")
            && let Some(parse_error) = &parsed.parse_error
        {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    parse_error.line,
                    parse_error.column,
                    "CDX-000",
                    t!("rules.cdx_000.message", error = parse_error.message),
                )
                .with_suggestion(t!("rules.cdx_000.suggestion")),
            );
            return diagnostics;
        }

        // CDX-004: Unknown config keys (WARNING)
        // Runs on unknown_keys which are populated whenever TOML parses successfully,
        // even when schema extraction fails.
        if cdx_004_enabled {
            for unknown in &parsed.unknown_keys {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        unknown.line,
                        unknown.column,
                        "CDX-004",
                        t!("rules.cdx_004.message", key = unknown.key.as_str()),
                    )
                    .with_suggestion(t!("rules.cdx_004.suggestion")),
                );
            }
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
                    let mut diagnostic = Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        0,
                        "CDX-001",
                        t!("rules.cdx_001.message", value = approval_value.as_str()),
                    )
                    .with_suggestion(t!("rules.cdx_001.suggestion"));

                    // Unsafe auto-fix: replace with closest valid approval mode.
                    if let Some(suggested) =
                        find_closest_value(approval_value, VALID_APPROVAL_MODES)
                    {
                        if let Some((start, end)) =
                            find_toml_string_value_span(content, "approvalMode", approval_value)
                        {
                            diagnostic = diagnostic.with_fix(Fix::replace(
                                start,
                                end,
                                suggested,
                                t!("rules.cdx_001.fix", fixed = suggested),
                                false,
                            ));
                        }
                    }

                    diagnostics.push(diagnostic);
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
                    let mut diagnostic = Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        0,
                        "CDX-002",
                        t!("rules.cdx_002.message", value = error_mode_value.as_str()),
                    )
                    .with_suggestion(t!("rules.cdx_002.suggestion"));

                    // Unsafe auto-fix: replace with closest valid error mode.
                    if let Some(suggested) =
                        find_closest_value(error_mode_value, VALID_FULL_AUTO_ERROR_MODES)
                    {
                        if let Some((start, end)) = find_toml_string_value_span(
                            content,
                            "fullAutoErrorMode",
                            error_mode_value,
                        ) {
                            diagnostic = diagnostic.with_fix(Fix::replace(
                                start,
                                end,
                                suggested,
                                t!("rules.cdx_002.fix", fixed = suggested),
                                false,
                            ));
                        }
                    }

                    diagnostics.push(diagnostic);
                }
            }
        }

        // CDX-005: project_doc_max_bytes exceeds limit (ERROR)
        if cdx_005_enabled {
            if parsed.project_doc_max_bytes_wrong_type {
                let line = key_lines.get("project_doc_max_bytes").copied().unwrap_or(1);
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        0,
                        "CDX-005",
                        t!("rules.cdx_005.type_error"),
                    )
                    .with_suggestion(t!("rules.cdx_005.suggestion")),
                );
            } else if let Some(value) = schema.project_doc_max_bytes {
                if value > 65536 {
                    let line = key_lines.get("project_doc_max_bytes").copied().unwrap_or(1);
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            0,
                            "CDX-005",
                            t!("rules.cdx_005.message", value = &value.to_string()),
                        )
                        .with_suggestion(t!("rules.cdx_005.suggestion")),
                    );
                }
            }
        }

        diagnostics
    }
}

/// Build a map of TOML key names to their 1-indexed line numbers in a single pass.
///
/// Scans each line for a key followed by `=` (the TOML key-value separator).
/// Extracts keys by finding '=' positions; indexing is safe because find() returns
/// char-boundary positions in valid UTF-8. Handles both bare keys and simple quoted keys
/// (e.g., `"approvalMode"`), stripping quotes to normalize lookups. Prevents partial
/// matches by extracting only up to `=` (e.g., `approvalMode` will not match
/// `approvalModeExtra`).
///
/// Returns only the first occurrence of each key, which matches TOML semantics.
fn build_key_line_map(content: &str) -> HashMap<String, usize> {
    let mut map = HashMap::new();
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        // Extract the key portion: everything up to `=` or whitespace before `=`
        if let Some(eq_pos) = trimmed.find('=') {
            let key_part = trimmed[..eq_pos].trim_end();

            // Handle both bare keys and simple quoted keys.
            let key = if key_part.starts_with('"') && key_part.ends_with('"') && key_part.len() >= 2
            {
                key_part[1..key_part.len() - 1].to_string()
            } else {
                key_part.to_string()
            };

            // Only record the first occurrence (TOML spec: duplicate keys are errors)
            if !key.is_empty() && !map.contains_key(&key) {
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
    fn test_cdx_001_autofix_case_insensitive() {
        // "Suggest" is a case-insensitive match to "suggest"
        let diagnostics = validate_config("approvalMode = \"Suggest\"");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1);
        assert!(
            cdx_001[0].has_fixes(),
            "CDX-001 should have auto-fix for case mismatch"
        );
        let fix = &cdx_001[0].fixes[0];
        assert!(!fix.safe, "CDX-001 fix should be unsafe");
        assert_eq!(fix.replacement, "suggest", "Fix should suggest 'suggest'");
    }

    #[test]
    fn test_cdx_001_no_autofix_when_duplicate() {
        // The regex pattern `approvalMode\s*=\s*"Suggest"` appears twice in this
        // valid TOML because a [section] table also uses the same key name.
        // The uniqueness guard should prevent autofix when there are multiple matches.
        let content = "approvalMode = \"Suggest\"\n\n[overrides]\napprovalMode = \"Suggest\"";
        let diagnostics = validate_config(content);
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1);
        assert!(
            !cdx_001[0].has_fixes(),
            "CDX-001 should not have auto-fix when value pattern appears multiple times"
        );
    }

    #[test]
    fn test_cdx_001_no_autofix_nonsense() {
        let diagnostics = validate_config("approvalMode = \"yolo\"");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1);
        // "yolo" has no close match - should NOT get a fix
        assert!(
            !cdx_001[0].has_fixes(),
            "CDX-001 should not auto-fix nonsense values"
        );
    }

    #[test]
    fn test_cdx_001_autofix_targets_correct_bytes() {
        let content = "approvalMode = \"Suggest\"";
        let diagnostics = validate_config(content);
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1);
        assert!(cdx_001[0].has_fixes());
        let fix = &cdx_001[0].fixes[0];
        let target = &content[fix.start_byte..fix.end_byte];
        assert_eq!(target, "Suggest", "Fix should target the inner value");
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
    fn test_cdx_002_autofix_case_insensitive() {
        // "Ask-User" is a case-insensitive match to "ask-user"
        let diagnostics = validate_config("fullAutoErrorMode = \"Ask-User\"");
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert_eq!(cdx_002.len(), 1);
        assert!(
            cdx_002[0].has_fixes(),
            "CDX-002 should have auto-fix for case mismatch"
        );
        let fix = &cdx_002[0].fixes[0];
        assert!(!fix.safe, "CDX-002 fix should be unsafe");
        assert_eq!(fix.replacement, "ask-user", "Fix should suggest 'ask-user'");
    }

    #[test]
    fn test_cdx_002_no_autofix_nonsense() {
        let diagnostics = validate_config("fullAutoErrorMode = \"crash\"");
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert_eq!(cdx_002.len(), 1);
        // "crash" has no close match - should NOT get a fix
        assert!(
            !cdx_002[0].has_fixes(),
            "CDX-002 should not auto-fix nonsense values"
        );
    }

    #[test]
    fn test_cdx_002_autofix_targets_correct_bytes() {
        let content = "fullAutoErrorMode = \"Ask-User\"";
        let diagnostics = validate_config(content);
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert_eq!(cdx_002.len(), 1);
        assert!(cdx_002[0].has_fixes());
        let fix = &cdx_002[0].fixes[0];
        let target = &content[fix.start_byte..fix.end_byte];
        assert_eq!(target, "Ask-User", "Fix should target the inner value");
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
        config.rules_mut().codex = false;

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
        config.rules_mut().disabled_rules = vec!["CDX-001".to_string()];

        let diagnostics = validate_config_with_config("approvalMode = \"invalid\"", &config);
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert!(cdx_001.is_empty());
    }

    #[test]
    fn test_all_cdx_rules_can_be_disabled() {
        let rules = ["CDX-001", "CDX-002", "CDX-003", "CDX-004", "CDX-005"];

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules_mut().disabled_rules = vec![rule.to_string()];

            let (content, path): (&str, &str) = match rule {
                "CDX-001" => ("approvalMode = \"invalid\"", ".codex/config.toml"),
                "CDX-002" => ("fullAutoErrorMode = \"crash\"", ".codex/config.toml"),
                "CDX-003" => ("# Override", "AGENTS.override.md"),
                "CDX-004" => ("totally_unknown_key = true", ".codex/config.toml"),
                "CDX-005" => ("project_doc_max_bytes = 999999", ".codex/config.toml"),
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

    // ===== CDX-004: Unknown config keys =====

    #[test]
    fn test_cdx_004_unknown_key() {
        let diagnostics = validate_config("totally_unknown_key = true");
        let cdx_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-004").collect();
        assert_eq!(cdx_004.len(), 1);
        assert_eq!(cdx_004[0].level, DiagnosticLevel::Warning);
        assert!(cdx_004[0].message.contains("totally_unknown_key"));
    }

    #[test]
    fn test_cdx_004_known_keys_no_warning() {
        let content = r#"
model = "o4-mini"
approvalMode = "suggest"
fullAutoErrorMode = "ask-user"
notify = true
project_doc_max_bytes = 32768
"#;
        let diagnostics = validate_config(content);
        let cdx_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-004").collect();
        assert!(cdx_004.is_empty(), "Known keys should not trigger CDX-004");
    }

    #[test]
    fn test_cdx_004_multiple_unknown_keys() {
        let content = "unknown_a = true\nunknown_b = false\nmodel = \"o4-mini\"";
        let diagnostics = validate_config(content);
        let cdx_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-004").collect();
        assert_eq!(cdx_004.len(), 2);
    }

    #[test]
    fn test_cdx_004_line_number() {
        let content = "model = \"o4-mini\"\nmy_custom_key = true";
        let diagnostics = validate_config(content);
        let cdx_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-004").collect();
        assert_eq!(cdx_004.len(), 1);
        assert_eq!(cdx_004[0].line, 2);
    }

    #[test]
    fn test_cdx_004_table_keys_not_flagged() {
        let content = r#"
model = "o4-mini"

[mcp_servers]
name = "test"
"#;
        let diagnostics = validate_config(content);
        let cdx_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-004").collect();
        assert!(
            cdx_004.is_empty(),
            "Known table keys should not trigger CDX-004"
        );
    }

    #[test]
    fn test_cdx_004_has_suggestion() {
        let diagnostics = validate_config("bogus_setting = 42");
        let cdx_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-004").collect();
        assert_eq!(cdx_004.len(), 1);
        assert!(
            cdx_004[0].suggestion.is_some(),
            "CDX-004 should have a suggestion"
        );
    }

    // ===== CDX-005: project_doc_max_bytes exceeds limit =====

    #[test]
    fn test_cdx_005_exceeds_limit() {
        let diagnostics = validate_config("project_doc_max_bytes = 100000");
        let cdx_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-005").collect();
        assert_eq!(cdx_005.len(), 1);
        assert_eq!(cdx_005[0].level, DiagnosticLevel::Error);
        assert!(cdx_005[0].message.contains("100000"));
    }

    #[test]
    fn test_cdx_005_at_limit() {
        let diagnostics = validate_config("project_doc_max_bytes = 65536");
        let cdx_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-005").collect();
        assert!(cdx_005.is_empty(), "65536 is at the limit, should be valid");
    }

    #[test]
    fn test_cdx_005_below_limit() {
        let diagnostics = validate_config("project_doc_max_bytes = 32768");
        let cdx_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-005").collect();
        assert!(cdx_005.is_empty(), "32768 is below limit, should be valid");
    }

    #[test]
    fn test_cdx_005_just_over_limit() {
        let diagnostics = validate_config("project_doc_max_bytes = 65537");
        let cdx_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-005").collect();
        assert_eq!(cdx_005.len(), 1, "65537 exceeds 65536 limit");
    }

    #[test]
    fn test_cdx_005_wrong_type() {
        let diagnostics = validate_config("project_doc_max_bytes = \"not a number\"");
        let cdx_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-005").collect();
        assert_eq!(cdx_005.len(), 1);
        assert!(cdx_005[0].message.contains("integer"));
    }

    #[test]
    fn test_cdx_005_absent() {
        let diagnostics = validate_config("model = \"o4-mini\"");
        let cdx_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-005").collect();
        assert!(
            cdx_005.is_empty(),
            "Absent field should not trigger CDX-005"
        );
    }

    #[test]
    fn test_cdx_005_line_number() {
        let content = "model = \"o4-mini\"\nproject_doc_max_bytes = 999999";
        let diagnostics = validate_config(content);
        let cdx_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-005").collect();
        assert_eq!(cdx_005.len(), 1);
        assert_eq!(cdx_005[0].line, 2);
    }

    #[test]
    fn test_cdx_005_has_suggestion() {
        let diagnostics = validate_config("project_doc_max_bytes = 100000");
        let cdx_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-005").collect();
        assert_eq!(cdx_005.len(), 1);
        assert!(
            cdx_005[0].suggestion.is_some(),
            "CDX-005 should have a suggestion"
        );
    }

    #[test]
    fn test_cdx_005_negative_value() {
        // Negative values are semantically invalid but CDX-005 only checks the upper bound.
        // Codex itself rejects negative values, but since this is an unusual edge case
        // and not the primary concern (the issue is about the 65536 cap), we document
        // that negative values pass CDX-005 validation.
        let diagnostics = validate_config("project_doc_max_bytes = -1");
        let cdx_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-005").collect();
        assert!(
            cdx_005.is_empty(),
            "Negative values do not exceed the upper limit"
        );
    }

    #[test]
    fn test_cdx_004_and_cdx_005_combined() {
        // Both an unknown key and exceeding limit in the same file
        let content = "unknown_key = true\nproject_doc_max_bytes = 999999";
        let diagnostics = validate_config(content);
        let cdx_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-004").collect();
        let cdx_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-005").collect();
        assert_eq!(cdx_004.len(), 1, "CDX-004 should fire for unknown_key");
        assert_eq!(cdx_005.len(), 1, "CDX-005 should fire for exceeding limit");
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

    // ===== CDX-000 suggestion test =====

    #[test]
    fn test_cdx_000_has_suggestion() {
        let content = "this is not valid toml {{{}}}";
        let diagnostics = validate_config(content);

        let cdx_000: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-000").collect();
        assert_eq!(cdx_000.len(), 1);
        assert!(
            cdx_000[0].suggestion.is_some(),
            "CDX-000 should have a suggestion"
        );
        assert!(
            cdx_000[0]
                .suggestion
                .as_ref()
                .unwrap()
                .contains("TOML syntax"),
            "CDX-000 suggestion should mention TOML syntax"
        );
    }

    #[test]
    fn test_cdx_000_uses_localized_message() {
        let content = "this is not valid toml {{{}}}";
        let diagnostics = validate_config(content);

        let cdx_000: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-000").collect();
        assert_eq!(cdx_000.len(), 1);
        assert!(
            cdx_000[0]
                .message
                .contains("Failed to parse .codex/config.toml as TOML"),
            "CDX-000 message should use localized text, got: {}",
            cdx_000[0].message
        );
    }
}
