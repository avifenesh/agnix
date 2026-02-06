//! Cursor project rules validation rules (CUR-001 to CUR-006)
//!
//! Validates:
//! - CUR-001: Empty .mdc rule file (HIGH) - files must have content
//! - CUR-002: Missing frontmatter (MEDIUM) - .mdc files should have frontmatter
//! - CUR-003: Invalid YAML frontmatter (HIGH) - frontmatter must be valid YAML
//! - CUR-004: Invalid glob pattern (HIGH) - globs field must contain valid patterns
//! - CUR-005: Unknown frontmatter keys (MEDIUM) - warn about unrecognized keys
//! - CUR-006: Legacy .cursorrules detected (MEDIUM) - migration warning

use crate::{
    FileType,
    config::LintConfig,
    diagnostics::{Diagnostic, Fix},
    rules::Validator,
    schemas::cursor::{
        is_body_empty, is_content_empty, parse_mdc_frontmatter, validate_glob_pattern,
    },
};
use rust_i18n::t;
use std::path::Path;

pub struct CursorValidator;

fn line_byte_range(content: &str, line_number: usize) -> Option<(usize, usize)> {
    if line_number == 0 {
        return None;
    }

    let mut current_line = 1usize;
    let mut line_start = 0usize;

    for (idx, ch) in content.char_indices() {
        if current_line == line_number && ch == '\n' {
            return Some((line_start, idx + 1));
        }
        if ch == '\n' {
            current_line += 1;
            line_start = idx + 1;
        }
    }

    if current_line == line_number {
        Some((line_start, content.len()))
    } else {
        None
    }
}

impl Validator for CursorValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Determine if this is a .mdc rule file or legacy .cursorrules
        let file_type = crate::detect_file_type(path);
        let is_legacy = file_type == FileType::CursorRulesLegacy;

        // CUR-006: Legacy .cursorrules detected (WARNING)
        if is_legacy && config.is_rule_enabled("CUR-006") {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "CUR-006",
                    t!("rules.cur_006.message"),
                )
                .with_suggestion(t!("rules.cur_006.suggestion")),
            );
            // For legacy files, just check if empty and return
            if config.is_rule_enabled("CUR-001") && is_content_empty(content) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CUR-001",
                        t!("rules.cur_006.legacy_empty"),
                    )
                    .with_suggestion(t!("rules.cur_001.suggestion_legacy_empty")),
                );
            }
            return diagnostics;
        }

        // CUR-001: Empty .mdc rule file (ERROR)
        if config.is_rule_enabled("CUR-001") {
            if let Some(parsed) = parse_mdc_frontmatter(content) {
                // Skip CUR-001 if there's a frontmatter parse error - CUR-003 will handle it
                if parsed.parse_error.is_none() && is_body_empty(&parsed.body) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            parsed.end_line + 1,
                            0,
                            "CUR-001",
                            t!("rules.cur_001.message_no_content"),
                        )
                        .with_suggestion(t!("rules.cur_001.suggestion_no_content")),
                    );
                }
            } else if is_content_empty(content) {
                // No frontmatter and no content
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CUR-001",
                        t!("rules.cur_001.message_empty"),
                    )
                    .with_suggestion(t!("rules.cur_001.suggestion_empty")),
                );
            }
        }

        // Parse frontmatter for further validation
        let parsed = match parse_mdc_frontmatter(content) {
            Some(p) => p,
            None => {
                // CUR-002: Missing frontmatter in .mdc file (WARNING)
                if config.is_rule_enabled("CUR-002") && !is_content_empty(content) {
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            1,
                            0,
                            "CUR-002",
                            t!("rules.cur_002.message"),
                        )
                        .with_suggestion(t!("rules.cur_002.suggestion")),
                    );
                }
                return diagnostics;
            }
        };

        // CUR-003: Invalid YAML frontmatter (ERROR)
        if config.is_rule_enabled("CUR-003") {
            if let Some(ref error) = parsed.parse_error {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        parsed.start_line,
                        0,
                        "CUR-003",
                        t!("rules.cur_003.message", error = error.as_str()),
                    )
                    .with_suggestion(t!("rules.cur_003.suggestion")),
                );
                // Can't continue validating if YAML is broken
                return diagnostics;
            }
        }

        // CUR-004: Invalid glob pattern (ERROR)
        if config.is_rule_enabled("CUR-004") {
            if let Some(ref schema) = parsed.schema {
                if let Some(ref globs) = schema.globs {
                    // Find the line number of the globs field for accurate diagnostics
                    // Note: parsed.raw doesn't include the opening --- line, so we need +1
                    let globs_line = parsed
                        .raw
                        .lines()
                        .enumerate()
                        .find(|(_, line)| line.trim_start().starts_with("globs:"))
                        .map(|(idx, _)| parsed.start_line + 1 + idx)
                        .unwrap_or(parsed.start_line);

                    for pattern in globs.patterns() {
                        let validation = validate_glob_pattern(pattern);
                        if !validation.valid {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    globs_line,
                                    0,
                                    "CUR-004",
                                    t!(
                                        "rules.cur_004.message",
                                        pattern = pattern,
                                        error = validation.error.unwrap_or_default()
                                    ),
                                )
                                .with_suggestion(t!("rules.cur_004.suggestion")),
                            );
                        }
                    }
                }
            }
        }

        // CUR-005: Unknown frontmatter keys (WARNING)
        if config.is_rule_enabled("CUR-005") {
            for unknown in &parsed.unknown_keys {
                let mut diagnostic = Diagnostic::warning(
                    path.to_path_buf(),
                    unknown.line,
                    unknown.column,
                    "CUR-005",
                    t!("rules.cur_005.message", key = unknown.key.as_str()),
                )
                .with_suggestion(t!("rules.cur_005.suggestion", key = unknown.key.as_str()));

                // Safe auto-fix: remove unknown top-level frontmatter key line.
                if let Some((start, end)) = line_byte_range(content, unknown.line) {
                    diagnostic = diagnostic.with_fix(Fix::delete(
                        start,
                        end,
                        format!("Remove unknown frontmatter key '{}'", unknown.key),
                        true,
                    ));
                }

                diagnostics.push(diagnostic);
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;

    fn validate_mdc(content: &str) -> Vec<Diagnostic> {
        let validator = CursorValidator;
        validator.validate(
            Path::new(".cursor/rules/typescript.mdc"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_legacy(content: &str) -> Vec<Diagnostic> {
        let validator = CursorValidator;
        validator.validate(Path::new(".cursorrules"), content, &LintConfig::default())
    }

    fn validate_mdc_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = CursorValidator;
        validator.validate(Path::new(".cursor/rules/typescript.mdc"), content, config)
    }

    // ===== CUR-001: Empty Rule File =====

    #[test]
    fn test_cur_001_empty_mdc_file() {
        let diagnostics = validate_mdc("");
        let cur_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-001").collect();
        assert_eq!(cur_001.len(), 1);
        assert_eq!(cur_001[0].level, DiagnosticLevel::Error);
        assert!(cur_001[0].message.contains("empty"));
    }

    #[test]
    fn test_cur_001_whitespace_only() {
        let diagnostics = validate_mdc("   \n\n\t  ");
        let cur_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-001").collect();
        assert_eq!(cur_001.len(), 1);
    }

    #[test]
    fn test_cur_001_valid_mdc_file() {
        let content = r#"---
description: TypeScript rules
globs: "**/*.ts"
---
# TypeScript Rules

Use strict mode.
"#;
        let diagnostics = validate_mdc(content);
        let cur_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-001").collect();
        assert!(cur_001.is_empty());
    }

    #[test]
    fn test_cur_001_empty_body_after_frontmatter() {
        let content = r#"---
description: Empty body
globs: "**/*.ts"
---
"#;
        let diagnostics = validate_mdc(content);
        let cur_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-001").collect();
        assert_eq!(cur_001.len(), 1);
        assert!(cur_001[0].message.contains("no content after frontmatter"));
    }

    #[test]
    fn test_cur_001_skips_when_parse_error() {
        // When frontmatter has parse error (missing closing ---),
        // CUR-001 should NOT trigger - CUR-003 handles it
        let content = r#"---
description: Unclosed frontmatter
# Missing closing ---
"#;
        let diagnostics = validate_mdc(content);
        let cur_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-001").collect();
        assert!(
            cur_001.is_empty(),
            "CUR-001 should not trigger when parse_error exists"
        );

        // Verify CUR-003 triggers instead
        let cur_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-003").collect();
        assert_eq!(cur_003.len(), 1);
        assert!(cur_003[0].message.contains("missing closing ---"));
    }

    // ===== CUR-002: Missing Frontmatter =====

    #[test]
    fn test_cur_002_missing_frontmatter() {
        let content = "# Rules without frontmatter";
        let diagnostics = validate_mdc(content);
        let cur_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-002").collect();
        assert_eq!(cur_002.len(), 1);
        assert_eq!(cur_002[0].level, DiagnosticLevel::Warning);
        assert!(
            cur_002[0]
                .message
                .contains("missing recommended frontmatter")
        );
    }

    #[test]
    fn test_cur_002_has_frontmatter() {
        let content = r#"---
description: Valid
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-002").collect();
        assert!(cur_002.is_empty());
    }

    // ===== CUR-003: Invalid YAML Frontmatter =====

    #[test]
    fn test_cur_003_invalid_yaml() {
        let content = r#"---
globs: [unclosed
---
# Body
"#;
        let diagnostics = validate_mdc(content);
        let cur_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-003").collect();
        assert_eq!(cur_003.len(), 1);
        assert_eq!(cur_003[0].level, DiagnosticLevel::Error);
        assert!(cur_003[0].message.contains("Invalid YAML"));
    }

    #[test]
    fn test_cur_003_unclosed_frontmatter() {
        let content = r#"---
description: Test
# Missing closing ---
"#;
        let diagnostics = validate_mdc(content);
        let cur_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-003").collect();
        assert_eq!(cur_003.len(), 1);
        assert!(cur_003[0].message.contains("missing closing ---"));
    }

    #[test]
    fn test_cur_003_valid_yaml() {
        let content = r#"---
description: Valid YAML
globs: "**/*.ts"
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-003").collect();
        assert!(cur_003.is_empty());
    }

    // ===== CUR-004: Invalid Glob Pattern =====

    #[test]
    fn test_cur_004_invalid_glob() {
        let content = r#"---
description: Bad glob
globs: "[unclosed"
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-004").collect();
        assert_eq!(cur_004.len(), 1);
        assert_eq!(cur_004[0].level, DiagnosticLevel::Error);
        assert!(cur_004[0].message.contains("Invalid glob pattern"));
    }

    #[test]
    fn test_cur_004_invalid_glob_in_array() {
        let content = r#"---
description: Some bad globs
globs:
  - "**/*.ts"
  - "[unclosed"
  - "**/*.js"
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-004").collect();
        assert_eq!(cur_004.len(), 1);
        assert!(cur_004[0].message.contains("[unclosed"));
    }

    #[test]
    fn test_cur_004_valid_glob_patterns() {
        let patterns = vec!["**/*.ts", "*.rs", "src/**/*.js", "tests/**/*.test.ts"];

        for pattern in patterns {
            let content = format!(
                r#"---
description: Test
globs: "{}"
---
# Rules
"#,
                pattern
            );
            let diagnostics = validate_mdc(&content);
            let cur_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-004").collect();
            assert!(cur_004.is_empty(), "Pattern '{}' should be valid", pattern);
        }
    }

    #[test]
    fn test_cur_004_line_number_accuracy() {
        // Test that CUR-004 reports the line number of the globs field, not frontmatter start
        let content = r#"---
description: Bad glob
globs: "[unclosed"
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-004").collect();
        assert_eq!(cur_004.len(), 1);
        // globs: is on line 3 (line 1 is ---, line 2 is description, line 3 is globs)
        assert_eq!(
            cur_004[0].line, 3,
            "CUR-004 should point to the globs field line"
        );
    }

    // ===== CUR-005: Unknown Frontmatter Keys =====

    #[test]
    fn test_cur_005_unknown_keys() {
        let content = r#"---
description: Valid key
unknownKey: value
anotherBadKey: 123
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-005").collect();
        assert_eq!(cur_005.len(), 2);
        assert_eq!(cur_005[0].level, DiagnosticLevel::Warning);
        assert!(cur_005.iter().any(|d| d.message.contains("unknownKey")));
        assert!(cur_005.iter().any(|d| d.message.contains("anotherBadKey")));
        assert!(
            cur_005.iter().all(|d| d.has_fixes()),
            "All unknown key diagnostics should include safe deletion fixes"
        );
        assert!(cur_005.iter().all(|d| d.fixes[0].safe));
    }

    #[test]
    fn test_cur_005_no_unknown_keys() {
        let content = r#"---
description: Valid
globs: "**/*.rs"
alwaysApply: true
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-005").collect();
        assert!(cur_005.is_empty());
    }

    // ===== CUR-006: Legacy .cursorrules =====

    #[test]
    fn test_cur_006_legacy_file() {
        let content = "# Legacy rules content";
        let diagnostics = validate_legacy(content);
        let cur_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-006").collect();
        assert_eq!(cur_006.len(), 1);
        assert_eq!(cur_006[0].level, DiagnosticLevel::Warning);
        assert!(cur_006[0].message.contains("Legacy .cursorrules"));
        assert!(cur_006[0].message.contains("migrating"));
    }

    #[test]
    fn test_cur_006_legacy_empty() {
        let content = "";
        let diagnostics = validate_legacy(content);
        // Should have both CUR-006 (legacy warning) and CUR-001 (empty file)
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-006"));
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-001"));
    }

    #[test]
    fn test_mdc_file_no_cur_006() {
        // .mdc files should NOT trigger CUR-006
        let content = r#"---
description: Modern format
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-006").collect();
        assert!(cur_006.is_empty());
    }

    // ===== Config Integration =====

    #[test]
    fn test_config_disabled_cursor_category() {
        let mut config = LintConfig::default();
        config.rules.cursor = false;

        let content = "";
        let diagnostics = validate_mdc_with_config(content, &config);

        let cur_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("CUR-"))
            .collect();
        assert!(cur_rules.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["CUR-001".to_string()];

        let content = "";
        let diagnostics = validate_mdc_with_config(content, &config);

        let cur_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-001").collect();
        assert!(cur_001.is_empty());
    }

    // ===== Combined Issues =====

    #[test]
    fn test_multiple_issues() {
        let content = r#"---
unknownKey: value
---
"#;
        let diagnostics = validate_mdc(content);

        // Should have:
        // - CUR-001 for empty body
        // - CUR-005 for unknown key
        assert!(
            diagnostics.iter().any(|d| d.rule == "CUR-001"),
            "Expected CUR-001"
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "CUR-005"),
            "Expected CUR-005"
        );
    }

    #[test]
    fn test_valid_mdc_no_issues() {
        let content = r#"---
description: TypeScript Guidelines
globs: "**/*.ts"
alwaysApply: false
---
# TypeScript Guidelines

Always use strict mode and explicit types.
"#;
        let diagnostics = validate_mdc(content);
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .collect();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    // ===== Additional CUR rule tests =====

    #[test]
    fn test_cur_001_newlines_only() {
        let content = "\n\n\n";
        let diagnostics = validate_mdc(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-001"));
    }

    #[test]
    fn test_cur_001_frontmatter_only() {
        // File with just frontmatter and no body
        let content = "---\ndescription: test\n---\n";
        let diagnostics = validate_mdc(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-001"));
    }

    #[test]
    fn test_cur_002_no_frontmatter_in_cursorrules() {
        // .cursorrules should NOT require frontmatter (CUR-002 is for .mdc files)
        let content = "Just plain text rules without frontmatter.";
        let validator = CursorValidator;
        let diagnostics =
            validator.validate(Path::new(".cursorrules"), content, &LintConfig::default());
        assert!(!diagnostics.iter().any(|d| d.rule == "CUR-002"));
    }

    #[test]
    fn test_cur_003_yaml_with_tabs() {
        // YAML doesn't allow tabs for indentation
        let content = "---\n\tdescription: test\n---\nBody";
        let diagnostics = validate_mdc(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-003"));
    }

    #[test]
    fn test_cur_004_all_valid_patterns() {
        let valid_patterns = ["**/*.ts", "*.rs", "src/**/*.py", "{src,lib}/**/*.tsx"];

        for pattern in valid_patterns {
            let content = format!("---\nglobs: \"{}\"\n---\nBody", pattern);
            let diagnostics = validate_mdc(&content);
            let cur_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-004").collect();
            assert!(cur_004.is_empty(), "Pattern '{}' should be valid", pattern);
        }
    }

    #[test]
    fn test_cur_004_invalid_patterns() {
        let invalid_patterns = ["[invalid", "***", "**["];

        for pattern in invalid_patterns {
            let content = format!("---\nglobs: \"{}\"\n---\nBody", pattern);
            let diagnostics = validate_mdc(&content);
            let cur_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-004").collect();
            assert!(
                !cur_004.is_empty(),
                "Pattern '{}' should be invalid",
                pattern
            );
        }
    }

    #[test]
    fn test_cur_004_globs_as_array() {
        let content = r#"---
globs:
  - "**/*.ts"
  - "**/*.tsx"
---
Body"#;
        let diagnostics = validate_mdc(content);
        let cur_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-004").collect();
        assert!(cur_004.is_empty(), "Array globs should be valid");
    }

    #[test]
    fn test_cur_005_all_known_keys() {
        let content = r#"---
description: Test rule
globs: "**/*.ts"
alwaysApply: false
---
Body"#;
        let diagnostics = validate_mdc(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "CUR-005"));
    }

    #[test]
    fn test_cur_005_multiple_unknown_keys() {
        let content = r#"---
description: test
unknownKey1: value1
unknownKey2: value2
---
Body"#;
        let diagnostics = validate_mdc(content);
        let cur_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-005").collect();
        assert!(!cur_005.is_empty());
    }

    #[test]
    fn test_cur_006_legacy_file_with_content() {
        let content = "Some legacy cursor rules.";
        let validator = CursorValidator;
        let diagnostics =
            validator.validate(Path::new(".cursorrules"), content, &LintConfig::default());
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-006"));
    }

    #[test]
    fn test_mdc_file_no_cur_006_warning() {
        // .mdc files should not trigger CUR-006 (legacy warning)
        let content = "---\ndescription: test\n---\nRules content";
        let diagnostics = validate_mdc(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "CUR-006"));
    }

    #[test]
    fn test_all_cur_rules_can_be_disabled() {
        let rules = [
            "CUR-001", "CUR-002", "CUR-003", "CUR-004", "CUR-005", "CUR-006",
        ];

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules.disabled_rules = vec![rule.to_string()];

            // Content that could trigger each rule
            let (content, path) = match rule {
                "CUR-001" => ("", ".cursor/rules/test.mdc"),
                "CUR-006" => ("content", ".cursorrules"),
                _ => ("---\nunknown: value\n---\n", ".cursor/rules/test.mdc"),
            };

            let validator = CursorValidator;
            let diagnostics = validator.validate(Path::new(path), content, &config);

            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }
}
