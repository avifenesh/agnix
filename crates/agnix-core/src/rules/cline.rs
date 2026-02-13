//! Cline rules validation rules (CLN-001 to CLN-003)
//!
//! Validates:
//! - CLN-001: Empty clinerules file (HIGH) - files must have content
//! - CLN-002: Invalid paths glob in clinerules (HIGH) - glob patterns must be valid
//! - CLN-003: Unknown frontmatter key in clinerules (MEDIUM) - only `paths` is recognized

use crate::{
    FileType,
    config::LintConfig,
    diagnostics::{Diagnostic, Fix},
    rules::{Validator, ValidatorMetadata},
    schemas::cline::{is_body_empty, is_content_empty, parse_frontmatter, validate_glob_pattern},
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &["CLN-001", "CLN-002", "CLN-003", "CLN-004"];

pub struct ClineValidator;

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

impl Validator for ClineValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let file_type = crate::detect_file_type(path);
        let is_folder = file_type == FileType::ClineRulesFolder;

        // CLN-001: Empty clinerules file (ERROR)
        if config.is_rule_enabled("CLN-001") {
            if is_folder {
                // For folder .md files, check body after frontmatter if present
                if let Some(parsed) = parse_frontmatter(content) {
                    // Only check body emptiness when frontmatter parsed successfully;
                    // parse errors (e.g. missing closing ---) produce empty body by default
                    if parsed.parse_error.is_none() && is_body_empty(&parsed.body) {
                        let total_lines = content.lines().count().max(1);
                        let report_line = (parsed.end_line + 1).min(total_lines);
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                report_line,
                                0,
                                "CLN-001",
                                t!("rules.cln_001.message_no_content"),
                            )
                            .with_suggestion(t!("rules.cln_001.suggestion_no_content")),
                        );
                    }
                } else if is_content_empty(content) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CLN-001",
                            t!("rules.cln_001.message_empty"),
                        )
                        .with_suggestion(t!("rules.cln_001.suggestion_empty")),
                    );
                }
            } else {
                // Single .clinerules file - just check entire content
                if is_content_empty(content) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CLN-001",
                            t!("rules.cln_001.message_empty"),
                        )
                        .with_suggestion(t!("rules.cln_001.suggestion_empty")),
                    );
                }
            }
        }

        // CLN-002 and CLN-003 only apply to folder .md files (they have frontmatter)
        if !is_folder {
            return diagnostics;
        }

        // Parse frontmatter for folder files
        let parsed = match parse_frontmatter(content) {
            Some(p) => p,
            None => {
                // No frontmatter in folder files is fine - paths field is optional
                return diagnostics;
            }
        };

        // If frontmatter has a parse error, skip CLN-002/003
        if parsed.parse_error.is_some() {
            return diagnostics;
        }

        // CLN-002: Invalid paths glob (ERROR)
        if config.is_rule_enabled("CLN-002") {
            if let Some(ref schema) = parsed.schema {
                if let Some(ref paths_field) = schema.paths {
                    for pattern in paths_field.patterns() {
                        let validation = validate_glob_pattern(pattern);
                        if !validation.valid {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    parsed.paths_line.unwrap_or(parsed.start_line + 1),
                                    0,
                                    "CLN-002",
                                    t!(
                                        "rules.cln_002.message",
                                        pattern = pattern,
                                        error = validation.error.unwrap_or_default()
                                    ),
                                )
                                .with_suggestion(t!("rules.cln_002.suggestion")),
                            );
                        }
                    }
                }
            }
        }

        // CLN-004: Scalar paths value (WARNING) - Cline ignores scalar strings
        if config.is_rule_enabled("CLN-004") {
            if let Some(ref schema) = parsed.schema {
                if let Some(ref paths_field) = schema.paths {
                    if paths_field.is_scalar() {
                        let pattern = paths_field.patterns()[0];
                        let line = parsed.paths_line.unwrap_or(parsed.start_line + 1);
                        let mut diagnostic = Diagnostic::warning(
                            path.to_path_buf(),
                            line,
                            0,
                            "CLN-004",
                            t!("rules.cln_004.message"),
                        )
                        .with_suggestion(t!(
                            "rules.cln_004.suggestion",
                            pattern = pattern
                        ));

                        // Auto-fix: convert scalar to array
                        if let Some((start, end)) = line_byte_range(content, line) {
                            let fix_text = format!("paths:\n  - \"{}\"\n", pattern);
                            diagnostic = diagnostic.with_fix(Fix::replace(
                                start,
                                end,
                                fix_text,
                                t!("rules.cln_004.fix"),
                                true,
                            ));
                        }

                        diagnostics.push(diagnostic);
                    }
                }
            }
        }

        // CLN-003: Unknown frontmatter keys (WARNING)
        if config.is_rule_enabled("CLN-003") {
            for unknown in &parsed.unknown_keys {
                let mut diagnostic = Diagnostic::warning(
                    path.to_path_buf(),
                    unknown.line,
                    unknown.column,
                    "CLN-003",
                    t!("rules.cln_003.message", key = unknown.key.as_str()),
                )
                .with_suggestion(t!("rules.cln_003.suggestion", key = unknown.key.as_str()));

                // Auto-fix: remove unknown top-level frontmatter key line.
                // Marked unsafe because multi-line YAML values would leave orphaned lines.
                if let Some((start, end)) = line_byte_range(content, unknown.line) {
                    diagnostic = diagnostic.with_fix(Fix::delete(
                        start,
                        end,
                        format!("Remove unknown frontmatter key '{}'", unknown.key),
                        false,
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

    fn validate_single(content: &str) -> Vec<Diagnostic> {
        let validator = ClineValidator;
        validator.validate(Path::new(".clinerules"), content, &LintConfig::default())
    }

    fn validate_folder(content: &str) -> Vec<Diagnostic> {
        let validator = ClineValidator;
        validator.validate(
            Path::new(".clinerules/typescript.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_folder_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = ClineValidator;
        validator.validate(Path::new(".clinerules/typescript.md"), content, config)
    }

    // ===== CLN-001: Empty Clinerules File =====

    #[test]
    fn test_cln_001_empty_single_file() {
        let diagnostics = validate_single("");
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert_eq!(cln_001.len(), 1);
        assert_eq!(cln_001[0].level, DiagnosticLevel::Error);
        assert!(cln_001[0].message.contains("empty"));
    }

    #[test]
    fn test_cln_001_whitespace_only_single() {
        let diagnostics = validate_single("   \n\n\t  ");
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert_eq!(cln_001.len(), 1);
    }

    #[test]
    fn test_cln_001_valid_single_file() {
        let content = "# Project Rules\n\nAlways follow the coding style guide.";
        let diagnostics = validate_single(content);
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert!(cln_001.is_empty());
    }

    #[test]
    fn test_cln_001_empty_folder_file() {
        let diagnostics = validate_folder("");
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert_eq!(cln_001.len(), 1);
        assert_eq!(cln_001[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_cln_001_empty_body_after_frontmatter() {
        let content = "---\npaths:\n  - \"**/*.ts\"\n---\n";
        let diagnostics = validate_folder(content);
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert_eq!(cln_001.len(), 1);
        assert!(cln_001[0].message.contains("no content after frontmatter"));
    }

    #[test]
    fn test_cln_001_valid_folder_file() {
        let content = "---\npaths:\n  - \"**/*.ts\"\n---\n# TypeScript Rules\n\nUse strict mode.\n";
        let diagnostics = validate_folder(content);
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert!(cln_001.is_empty());
    }

    #[test]
    fn test_cln_001_folder_no_frontmatter_with_content() {
        let content = "# Rules without frontmatter\n\nSome instructions.";
        let diagnostics = validate_folder(content);
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert!(cln_001.is_empty());
    }

    #[test]
    fn test_cln_001_newlines_only() {
        let content = "\n\n\n";
        let diagnostics = validate_single(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CLN-001"));
    }

    // ===== CLN-002: Invalid Paths Glob =====

    #[test]
    fn test_cln_002_invalid_glob() {
        let content = "---\npaths:\n  - \"[unclosed\"\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-002").collect();
        assert_eq!(cln_002.len(), 1);
        assert_eq!(cln_002[0].level, DiagnosticLevel::Error);
        assert!(cln_002[0].message.contains("Invalid glob pattern"));
    }

    #[test]
    fn test_cln_002_valid_glob_patterns() {
        let patterns = vec!["**/*.ts", "*.rs", "src/**/*.js", "tests/**/*.test.ts"];

        for pattern in patterns {
            let content = format!("---\npaths:\n  - \"{}\"\n---\n# Instructions\n", pattern);
            let diagnostics = validate_folder(&content);
            let cln_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-002").collect();
            assert!(cln_002.is_empty(), "Pattern '{}' should be valid", pattern);
        }
    }

    #[test]
    fn test_cln_002_invalid_patterns() {
        let invalid_patterns = ["[invalid", "***", "**["];

        for pattern in invalid_patterns {
            let content = format!("---\npaths:\n  - \"{}\"\n---\nBody", pattern);
            let diagnostics = validate_folder(&content);
            let cln_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-002").collect();
            assert!(
                !cln_002.is_empty(),
                "Pattern '{}' should be invalid",
                pattern
            );
        }
    }

    #[test]
    fn test_cln_002_multiple_patterns_mixed() {
        let content = "---\npaths:\n  - \"**/*.ts\"\n  - \"[invalid\"\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-002").collect();
        assert_eq!(cln_002.len(), 1, "Only the invalid pattern should trigger CLN-002");
        assert!(cln_002[0].message.contains("[invalid"));
    }

    #[test]
    fn test_cln_002_no_paths_field() {
        // No paths field should not trigger CLN-002
        let content = r#"---
---
# Instructions
"#;
        let diagnostics = validate_folder(content);
        let cln_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-002").collect();
        assert!(cln_002.is_empty());
    }

    #[test]
    fn test_cln_002_not_triggered_on_single_file() {
        // Single .clinerules file should not trigger CLN-002
        let content = "# Rules";
        let diagnostics = validate_single(content);
        let cln_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-002").collect();
        assert!(cln_002.is_empty());
    }

    // ===== CLN-003: Unknown Frontmatter Keys =====

    #[test]
    fn test_cln_003_unknown_keys() {
        let content = "---\npaths:\n  - \"**/*.ts\"\nunknownKey: value\nanotherBadKey: 123\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-003").collect();
        assert_eq!(cln_003.len(), 2);
        assert_eq!(cln_003[0].level, DiagnosticLevel::Warning);
        assert!(cln_003.iter().any(|d| d.message.contains("unknownKey")));
        assert!(cln_003.iter().any(|d| d.message.contains("anotherBadKey")));
        assert!(
            cln_003.iter().all(|d| d.has_fixes()),
            "All unknown key diagnostics should include deletion fixes"
        );
        // Fix is marked unsafe because multi-line YAML values would leave orphaned lines
        assert!(cln_003.iter().all(|d| !d.fixes[0].safe));
    }

    #[test]
    fn test_cln_003_no_unknown_keys() {
        let content = "---\npaths:\n  - \"**/*.rs\"\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-003").collect();
        assert!(cln_003.is_empty());
    }

    #[test]
    fn test_cln_003_not_triggered_on_single_file() {
        let content = "# Rules";
        let diagnostics = validate_single(content);
        let cln_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-003").collect();
        assert!(cln_003.is_empty());
    }

    // ===== Config Integration =====

    #[test]
    fn test_config_disabled_cline_category() {
        let mut config = LintConfig::default();
        config.rules_mut().cline = false;

        let content = "";
        let diagnostics = validate_folder_with_config(content, &config);

        let cln_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("CLN-"))
            .collect();
        assert!(cln_rules.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CLN-001".to_string()];

        let content = "";
        let diagnostics = validate_folder_with_config(content, &config);

        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert!(cln_001.is_empty());
    }

    // ===== Combined Issues =====

    #[test]
    fn test_multiple_issues() {
        let content = r#"---
unknownKey: value
---
"#;
        let diagnostics = validate_folder(content);

        // Should have CLN-001 (empty body) and CLN-003 (unknown key)
        assert!(
            diagnostics.iter().any(|d| d.rule == "CLN-001"),
            "Expected CLN-001"
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "CLN-003"),
            "Expected CLN-003"
        );
    }

    #[test]
    fn test_valid_folder_no_issues() {
        let content = "---\npaths:\n  - \"**/*.ts\"\n---\n# TypeScript Guidelines\n\nAlways use strict mode and explicit types.\n";
        let diagnostics = validate_folder(content);
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics, got: {:?}",
            diagnostics
        );
    }

    // ===== All Rules Can Be Disabled =====

    #[test]
    fn test_all_cln_rules_can_be_disabled() {
        let rules = ["CLN-001", "CLN-002", "CLN-003", "CLN-004"];

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules_mut().disabled_rules = vec![rule.to_string()];

            let (content, path): (&str, &str) = match rule {
                "CLN-001" => ("", ".clinerules"),
                "CLN-002" => (
                    "---\npaths:\n  - \"[invalid\"\n---\nBody",
                    ".clinerules/test.md",
                ),
                "CLN-003" => ("---\nunknown: value\n---\nBody", ".clinerules/test.md"),
                "CLN-004" => (
                    "---\npaths: \"**/*.ts\"\n---\nBody",
                    ".clinerules/test.md",
                ),
                _ => unreachable!("Unknown rule: {rule}"),
            };

            let validator = ClineValidator;
            let diagnostics = validator.validate(Path::new(path), content, &config);

            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }

    // ===== CLN-004: Scalar Paths Warning =====

    #[test]
    fn test_cln_004_scalar_paths_warns() {
        let content = "---\npaths: \"**/*.ts\"\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-004").collect();
        assert_eq!(cln_004.len(), 1);
        assert_eq!(cln_004[0].level, DiagnosticLevel::Warning);
        assert!(cln_004[0].message.contains("scalar"));
    }

    #[test]
    fn test_cln_004_array_paths_no_warning() {
        let content = "---\npaths:\n  - \"**/*.ts\"\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-004").collect();
        assert!(cln_004.is_empty());
    }

    #[test]
    fn test_cln_004_has_autofix() {
        let content = "---\npaths: \"**/*.ts\"\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-004").collect();
        assert_eq!(cln_004.len(), 1);
        assert!(cln_004[0].has_fixes(), "CLN-004 should have an auto-fix");
        assert!(cln_004[0].fixes[0].safe, "CLN-004 fix should be safe");
    }

    // ===== File Type Detection =====

    #[test]
    fn test_single_file_detection() {
        assert_eq!(
            crate::detect_file_type(Path::new(".clinerules")),
            FileType::ClineRules
        );
    }

    #[test]
    fn test_folder_file_detection() {
        assert_eq!(
            crate::detect_file_type(Path::new(".clinerules/typescript.md")),
            FileType::ClineRulesFolder
        );
    }

    #[test]
    fn test_folder_file_with_numeric_prefix() {
        assert_eq!(
            crate::detect_file_type(Path::new(".clinerules/01-coding.md")),
            FileType::ClineRulesFolder
        );
    }
}
