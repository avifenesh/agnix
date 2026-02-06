//! GitHub Copilot instruction file validation rules (COP-001 to COP-004)
//!
//! Validates:
//! - COP-001: Empty instruction file (HIGH) - files must have content
//! - COP-002: Invalid frontmatter (HIGH) - scoped files require valid YAML with applyTo
//! - COP-003: Invalid glob pattern (HIGH) - applyTo must contain valid globs
//! - COP-004: Unknown frontmatter keys (MEDIUM) - warn about unrecognized keys

use crate::{
    FileType,
    config::LintConfig,
    diagnostics::{Diagnostic, Fix},
    rules::Validator,
    schemas::copilot::{is_body_empty, is_content_empty, parse_frontmatter, validate_glob_pattern},
};
use rust_i18n::t;
use std::path::Path;

pub struct CopilotValidator;

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

impl Validator for CopilotValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Determine if this is global or scoped instruction file
        let file_type = crate::detect_file_type(path);
        let is_scoped = file_type == FileType::CopilotScoped;

        // COP-001: Empty instruction file (ERROR)
        if config.is_rule_enabled("COP-001") {
            if is_scoped {
                // For scoped files, check body after frontmatter
                if let Some(parsed) = parse_frontmatter(content) {
                    if is_body_empty(&parsed.body) {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                parsed.end_line + 1,
                                0,
                                "COP-001",
                                t!("rules.cop_001.message_no_content"),
                            )
                            .with_suggestion(t!("rules.cop_001.suggestion_empty")),
                        );
                    }
                } else if is_content_empty(content) {
                    // Scoped file with no frontmatter and no content
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "COP-001",
                            t!("rules.cop_001.message_empty"),
                        )
                        .with_suggestion(t!("rules.cop_001.suggestion_scoped_empty")),
                    );
                }
            } else {
                // For global files, check entire content
                if is_content_empty(content) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "COP-001",
                            t!("rules.cop_001.message_empty"),
                        )
                        .with_suggestion(t!("rules.cop_001.suggestion_empty")),
                    );
                }
            }
        }

        // Rules COP-002, COP-003, COP-004 only apply to scoped instruction files
        if !is_scoped {
            return diagnostics;
        }

        // Parse frontmatter for scoped files
        let parsed = match parse_frontmatter(content) {
            Some(p) => p,
            None => {
                // COP-002: Missing frontmatter in scoped file
                if config.is_rule_enabled("COP-002") && !is_content_empty(content) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "COP-002",
                            t!("rules.cop_002.message_missing"),
                        )
                        .with_suggestion(t!("rules.cop_002.suggestion_add_frontmatter")),
                    );
                }
                return diagnostics;
            }
        };

        // COP-002: Invalid frontmatter (YAML parse error)
        if config.is_rule_enabled("COP-002") {
            if let Some(ref error) = parsed.parse_error {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        parsed.start_line,
                        0,
                        "COP-002",
                        t!("rules.cop_002.message_invalid_yaml", error = error.as_str()),
                    )
                    .with_suggestion(t!("rules.cop_002.suggestion_fix_yaml")),
                );
                // Can't continue validating if YAML is broken
                return diagnostics;
            }

            // Check for missing applyTo field
            if let Some(ref schema) = parsed.schema {
                if schema.apply_to.is_none() {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            parsed.start_line,
                            0,
                            "COP-002",
                            t!("rules.cop_002.message_missing_apply_to"),
                        )
                        .with_suggestion(t!("rules.cop_002.suggestion_add_apply_to")),
                    );
                }
            }
        }

        // COP-003: Invalid glob pattern
        if config.is_rule_enabled("COP-003") {
            if let Some(ref schema) = parsed.schema {
                if let Some(ref apply_to) = schema.apply_to {
                    let validation = validate_glob_pattern(apply_to);
                    if !validation.valid {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                parsed.start_line + 1, // applyTo is typically on line 2
                                0,
                                "COP-003",
                                t!(
                                    "rules.cop_003.message",
                                    pattern = apply_to.as_str(),
                                    error = validation.error.unwrap_or_default()
                                ),
                            )
                            .with_suggestion(t!("rules.cop_003.suggestion")),
                        );
                    }
                }
            }
        }

        // COP-004: Unknown frontmatter keys (WARNING)
        if config.is_rule_enabled("COP-004") {
            for unknown in &parsed.unknown_keys {
                let mut diagnostic = Diagnostic::warning(
                    path.to_path_buf(),
                    unknown.line,
                    unknown.column,
                    "COP-004",
                    t!("rules.cop_004.message", key = unknown.key.as_str()),
                )
                .with_suggestion(t!("rules.cop_004.suggestion", key = unknown.key.as_str()));

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

    fn validate_global(content: &str) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/copilot-instructions.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_scoped(content: &str) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/instructions/typescript.instructions.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_scoped_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/instructions/typescript.instructions.md"),
            content,
            config,
        )
    }

    // ===== COP-001: Empty Instruction File =====

    #[test]
    fn test_cop_001_empty_global_file() {
        let diagnostics = validate_global("");
        let cop_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-001").collect();
        assert_eq!(cop_001.len(), 1);
        assert_eq!(cop_001[0].level, DiagnosticLevel::Error);
        assert!(cop_001[0].message.contains("empty"));
    }

    #[test]
    fn test_cop_001_whitespace_only_global() {
        let diagnostics = validate_global("   \n\n\t  ");
        let cop_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-001").collect();
        assert_eq!(cop_001.len(), 1);
    }

    #[test]
    fn test_cop_001_valid_global_file() {
        let content = "# Copilot Instructions\n\nFollow the coding style guide.";
        let diagnostics = validate_global(content);
        let cop_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-001").collect();
        assert!(cop_001.is_empty());
    }

    #[test]
    fn test_cop_001_empty_scoped_body() {
        let content = r#"---
applyTo: "**/*.ts"
---
"#;
        let diagnostics = validate_scoped(content);
        let cop_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-001").collect();
        assert_eq!(cop_001.len(), 1);
        assert!(cop_001[0].message.contains("no content after frontmatter"));
    }

    #[test]
    fn test_cop_001_valid_scoped_file() {
        let content = r#"---
applyTo: "**/*.ts"
---
# TypeScript Instructions

Use strict mode.
"#;
        let diagnostics = validate_scoped(content);
        let cop_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-001").collect();
        assert!(cop_001.is_empty());
    }

    // ===== COP-002: Invalid Frontmatter =====

    #[test]
    fn test_cop_002_missing_frontmatter() {
        let content = "# Instructions without frontmatter";
        let diagnostics = validate_scoped(content);
        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        assert_eq!(cop_002.len(), 1);
        assert!(cop_002[0].message.contains("missing required frontmatter"));
    }

    #[test]
    fn test_cop_002_invalid_yaml() {
        let content = r#"---
applyTo: [unclosed
---
# Body
"#;
        let diagnostics = validate_scoped(content);
        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        assert_eq!(cop_002.len(), 1);
        assert!(cop_002[0].message.contains("Invalid YAML"));
    }

    #[test]
    fn test_cop_002_missing_apply_to() {
        let content = r#"---
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        assert_eq!(cop_002.len(), 1);
        assert!(cop_002[0].message.contains("missing required 'applyTo'"));
    }

    #[test]
    fn test_cop_002_valid_frontmatter() {
        let content = r#"---
applyTo: "**/*.ts"
---
# TypeScript Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        assert!(cop_002.is_empty());
    }

    // ===== COP-003: Invalid Glob Pattern =====

    #[test]
    fn test_cop_003_invalid_glob() {
        let content = r#"---
applyTo: "[unclosed"
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-003").collect();
        assert_eq!(cop_003.len(), 1);
        assert!(cop_003[0].message.contains("Invalid glob pattern"));
    }

    #[test]
    fn test_cop_003_valid_glob_patterns() {
        let patterns = vec!["**/*.ts", "*.rs", "src/**/*.js", "tests/**/*.test.ts"];

        for pattern in patterns {
            let content = format!(
                r#"---
applyTo: "{}"
---
# Instructions
"#,
                pattern
            );
            let diagnostics = validate_scoped(&content);
            let cop_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-003").collect();
            assert!(cop_003.is_empty(), "Pattern '{}' should be valid", pattern);
        }
    }

    // ===== COP-004: Unknown Frontmatter Keys =====

    #[test]
    fn test_cop_004_unknown_keys() {
        let content = r#"---
applyTo: "**/*.ts"
unknownKey: value
anotherBadKey: 123
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-004").collect();
        assert_eq!(cop_004.len(), 2);
        assert_eq!(cop_004[0].level, DiagnosticLevel::Warning);
        assert!(cop_004.iter().any(|d| d.message.contains("unknownKey")));
        assert!(cop_004.iter().any(|d| d.message.contains("anotherBadKey")));
        assert!(
            cop_004.iter().all(|d| d.has_fixes()),
            "All unknown key diagnostics should include safe deletion fixes"
        );
        assert!(cop_004.iter().all(|d| d.fixes[0].safe));
    }

    #[test]
    fn test_cop_004_no_unknown_keys() {
        let content = r#"---
applyTo: "**/*.rs"
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-004").collect();
        assert!(cop_004.is_empty());
    }

    // ===== Global vs Scoped Behavior =====

    #[test]
    fn test_global_file_no_frontmatter_rules() {
        // Global files should not trigger COP-002/003/004
        let content = "# Instructions without frontmatter";
        let diagnostics = validate_global(content);

        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        let cop_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-003").collect();
        let cop_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-004").collect();

        assert!(cop_002.is_empty());
        assert!(cop_003.is_empty());
        assert!(cop_004.is_empty());
    }

    // ===== Config Integration =====

    #[test]
    fn test_config_disabled_copilot_category() {
        let mut config = LintConfig::default();
        config.rules.copilot = false;

        let content = "";
        let diagnostics = validate_scoped_with_config(content, &config);

        let cop_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("COP-"))
            .collect();
        assert!(cop_rules.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["COP-001".to_string()];

        let content = "";
        let diagnostics = validate_scoped_with_config(content, &config);

        let cop_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-001").collect();
        assert!(cop_001.is_empty());
    }

    // ===== Combined Issues =====

    #[test]
    fn test_multiple_issues() {
        let content = r#"---
unknownKey: value
---
"#;
        let diagnostics = validate_scoped(content);

        // Should have:
        // - COP-001 for empty body
        // - COP-002 for missing applyTo
        // - COP-004 for unknown key
        assert!(
            diagnostics.iter().any(|d| d.rule == "COP-001"),
            "Expected COP-001"
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "COP-002"),
            "Expected COP-002"
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "COP-004"),
            "Expected COP-004"
        );
    }

    #[test]
    fn test_valid_scoped_no_issues() {
        let content = r#"---
applyTo: "**/*.ts"
---
# TypeScript Guidelines

Always use strict mode and explicit types.
"#;
        let diagnostics = validate_scoped(content);
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .collect();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    // ===== Additional COP rule tests =====

    #[test]
    fn test_cop_001_newlines_only() {
        let content = "\n\n\n";
        let diagnostics = validate_global(content);
        assert!(diagnostics.iter().any(|d| d.rule == "COP-001"));
    }

    #[test]
    fn test_cop_001_spaces_and_tabs() {
        let content = "   \t\t   ";
        let diagnostics = validate_global(content);
        assert!(diagnostics.iter().any(|d| d.rule == "COP-001"));
    }

    #[test]
    fn test_cop_002_yaml_with_tabs() {
        // YAML doesn't allow tabs for indentation
        let content = "---\n\tapplyTo: \"**/*.ts\"\n---\nBody";
        let diagnostics = validate_scoped(content);
        assert!(diagnostics.iter().any(|d| d.rule == "COP-002"));
    }

    #[test]
    fn test_cop_002_valid_frontmatter_no_error() {
        // Test that valid frontmatter doesn't trigger COP-002
        let content = r#"---
applyTo: "**/*.ts"
---
Body content"#;
        let diagnostics = validate_scoped(content);
        // Valid frontmatter should not trigger COP-002
        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        assert!(
            cop_002.is_empty(),
            "Valid frontmatter should not trigger COP-002"
        );
    }

    #[test]
    fn test_cop_003_all_valid_patterns() {
        let valid_patterns = [
            "**/*.ts",
            "*.rs",
            "src/**/*.py",
            "tests/*.test.js",
            "{src,lib}/**/*.ts",
        ];

        for pattern in valid_patterns {
            let content = format!("---\napplyTo: \"{}\"\n---\nBody", pattern);
            let diagnostics = validate_scoped(&content);
            let cop_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-003").collect();
            assert!(cop_003.is_empty(), "Pattern '{}' should be valid", pattern);
        }
    }

    #[test]
    fn test_cop_003_invalid_patterns() {
        let invalid_patterns = ["[invalid", "***", "**["];

        for pattern in invalid_patterns {
            let content = format!("---\napplyTo: \"{}\"\n---\nBody", pattern);
            let diagnostics = validate_scoped(&content);
            let cop_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-003").collect();
            assert!(
                !cop_003.is_empty(),
                "Pattern '{}' should be invalid",
                pattern
            );
        }
    }

    #[test]
    fn test_cop_004_all_known_keys() {
        let content = r#"---
applyTo: "**/*.ts"
---
Body"#;
        let diagnostics = validate_scoped(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "COP-004"));
    }

    #[test]
    fn test_cop_004_multiple_unknown_keys() {
        let content = r#"---
applyTo: "**/*.ts"
unknownKey1: value1
unknownKey2: value2
---
Body"#;
        let diagnostics = validate_scoped(content);
        let cop_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-004").collect();
        // Should report at least one unknown key warning
        assert!(!cop_004.is_empty());
    }

    #[test]
    fn test_all_cop_rules_can_be_disabled() {
        let rules = ["COP-001", "COP-002", "COP-003", "COP-004"];

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules.disabled_rules = vec![rule.to_string()];

            // Content that could trigger each rule
            let content = match rule {
                "COP-001" => "",
                _ => "---\nunknown: value\n---\n",
            };

            let validator = CopilotValidator;
            let diagnostics = validator.validate(
                Path::new(".github/copilot-instructions.md"),
                content,
                &config,
            );

            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }
}
