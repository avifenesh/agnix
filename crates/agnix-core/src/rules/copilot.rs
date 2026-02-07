//! GitHub Copilot instruction file validation rules (COP-001 to COP-006)
//!
//! Validates:
//! - COP-001: Empty instruction file (HIGH) - files must have content
//! - COP-002: Invalid frontmatter (HIGH) - scoped files require valid YAML with applyTo
//! - COP-003: Invalid glob pattern (HIGH) - applyTo must contain valid globs
//! - COP-004: Unknown frontmatter keys (MEDIUM) - warn about unrecognized keys
//! - COP-005: Invalid excludeAgent value (HIGH) - must be "code-review" or "coding-agent"
//! - COP-006: File length limit (MEDIUM) - global files should not exceed ~4000 characters

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

/// Find the byte range of a YAML value for a given key in parsed frontmatter.
/// Returns the value range (including quotes if present).
/// Find the byte range of a YAML value (without quotes) for a given key.
/// Wrapper around the shared helper for backward compatibility.
fn find_yaml_value_range(
    content: &str,
    parsed: &crate::schemas::copilot::ParsedFrontmatter,
    key: &str,
) -> Option<(usize, usize)> {
    crate::rules::find_yaml_value_range(content, parsed, key, false)
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

        // COP-006: File length limit for global files (WARNING)
        const COPILOT_GLOBAL_LENGTH_LIMIT: usize = 4000;
        let char_count = content.chars().count();
        if config.is_rule_enabled("COP-006")
            && !is_scoped
            && char_count > COPILOT_GLOBAL_LENGTH_LIMIT
        {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "COP-006",
                    t!("rules.cop_006.message", len = char_count),
                )
                .with_suggestion(t!("rules.cop_006.suggestion")),
            );
        }

        // Rules COP-002, COP-003, COP-004, COP-005 only apply to scoped instruction files
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

        // COP-005: Invalid excludeAgent value (ERROR)
        if config.is_rule_enabled("COP-005") {
            if let Some(ref schema) = parsed.schema {
                if let Some(ref agent_value) = schema.exclude_agent {
                    const VALID_AGENTS: &[&str] = &["code-review", "coding-agent"];
                    if !VALID_AGENTS.contains(&agent_value.as_str()) {
                        // Find the line number of excludeAgent in raw frontmatter
                        let line = parsed
                            .raw
                            .lines()
                            .enumerate()
                            .find(|(_, l)| l.trim_start().starts_with("excludeAgent:"))
                            .map(|(i, _)| parsed.start_line + 1 + i)
                            .unwrap_or(parsed.start_line + 1);

                        let mut diagnostic = Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            0,
                            "COP-005",
                            t!("rules.cop_005.message", value = agent_value.as_str()),
                        )
                        .with_suggestion(t!("rules.cop_005.suggestion"));

                        // Unsafe auto-fix: replace with closest valid agent value
                        if let Some(closest) =
                            super::find_closest_value(agent_value.as_str(), VALID_AGENTS)
                        {
                            if let Some((start, end)) =
                                find_yaml_value_range(content, &parsed, "excludeAgent")
                            {
                                let slice = content.get(start..end).unwrap_or("");
                                let replacement = if slice.starts_with('"') {
                                    format!("\"{}\"", closest)
                                } else if slice.starts_with('\'') {
                                    format!("'{}'", closest)
                                } else {
                                    closest.to_string()
                                };
                                diagnostic = diagnostic.with_fix(Fix::replace(
                                    start,
                                    end,
                                    replacement,
                                    t!("rules.cop_005.fix", fixed = closest),
                                    false,
                                ));
                            }
                        }

                        diagnostics.push(diagnostic);
                    }
                }
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
        let rules = [
            "COP-001", "COP-002", "COP-003", "COP-004", "COP-005", "COP-006",
        ];
        let long_content = make_long_content();

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules.disabled_rules = vec![rule.to_string()];

            // Content and path that could trigger each rule
            let (content, path): (&str, &str) = match rule {
                "COP-001" => ("", ".github/copilot-instructions.md"),
                "COP-002" => (
                    "Content without frontmatter",
                    ".github/instructions/test.instructions.md",
                ),
                "COP-003" => (
                    "---\napplyTo: \"[invalid\"\n---\nBody",
                    ".github/instructions/test.instructions.md",
                ),
                "COP-004" => (
                    "---\nunknown: value\n---\nBody",
                    ".github/instructions/test.instructions.md",
                ),
                "COP-005" => (
                    "---\napplyTo: \"**/*.ts\"\nexcludeAgent: \"invalid\"\n---\nBody",
                    ".github/instructions/test.instructions.md",
                ),
                "COP-006" => (&long_content, ".github/copilot-instructions.md"),
                _ => unreachable!("Unknown rule: {rule}"),
            };

            let validator = CopilotValidator;
            let diagnostics = validator.validate(Path::new(path), content, &config);

            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }

    /// Generate long content for COP-006 tests (>4000 chars)
    fn make_long_content() -> String {
        let mut s = String::from("# Copilot Instructions\n\n");
        while s.len() <= 4001 {
            s.push_str("Follow consistent naming conventions for variables and functions.\n");
        }
        s
    }

    fn validate_global_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/copilot-instructions.md"),
            content,
            config,
        )
    }

    // ===== COP-005: Invalid excludeAgent Value =====

    #[test]
    fn test_cop_005_invalid_exclude_agent() {
        let content = r#"---
applyTo: "**/*.ts"
excludeAgent: "invalid-agent"
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert_eq!(cop_005.len(), 1);
        assert_eq!(cop_005[0].level, DiagnosticLevel::Error);
        assert!(cop_005[0].message.contains("invalid-agent"));
    }

    #[test]
    fn test_cop_005_valid_code_review() {
        let content = r#"---
applyTo: "**/*.ts"
excludeAgent: "code-review"
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert!(cop_005.is_empty());
    }

    #[test]
    fn test_cop_005_valid_coding_agent() {
        let content = r#"---
applyTo: "**/*.ts"
excludeAgent: "coding-agent"
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert!(cop_005.is_empty());
    }

    #[test]
    fn test_cop_005_absent_exclude_agent() {
        let content = r#"---
applyTo: "**/*.ts"
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert!(cop_005.is_empty());
    }

    #[test]
    fn test_cop_005_global_file_no_trigger() {
        let content = r#"---
applyTo: "**/*.ts"
excludeAgent: "invalid-agent"
---
# Instructions
"#;
        // Global files should not trigger COP-005 (scoped-only rule)
        let diagnostics = validate_global(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert!(cop_005.is_empty());
    }

    #[test]
    fn test_cop_005_case_sensitive() {
        let content =
            "---\napplyTo: \"**/*.ts\"\nexcludeAgent: \"Code-Review\"\n---\n# Instructions\n";
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert_eq!(cop_005.len(), 1, "Mixed-case value should trigger COP-005");
        // Case-insensitive match should produce an auto-fix
        assert!(
            cop_005[0].has_fixes(),
            "COP-005 should have auto-fix for case mismatch"
        );
        let fix = &cop_005[0].fixes[0];
        assert!(!fix.safe, "COP-005 fix should be unsafe");
        assert!(
            fix.replacement.contains("code-review"),
            "Fix should suggest 'code-review', got: {}",
            fix.replacement
        );
    }

    #[test]
    fn test_cop_005_empty_string() {
        let content = "---\napplyTo: \"**/*.ts\"\nexcludeAgent: \"\"\n---\n# Instructions\n";
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert_eq!(
            cop_005.len(),
            1,
            "Empty excludeAgent should trigger COP-005"
        );
        // Empty string should NOT get a fix (no close match)
        assert!(
            !cop_005[0].has_fixes(),
            "COP-005 should not auto-fix empty string"
        );
    }

    #[test]
    fn test_cop_005_autofix_nonsense() {
        let content =
            "---\napplyTo: \"**/*.ts\"\nexcludeAgent: \"nonsense\"\n---\n# Instructions\n";
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert_eq!(cop_005.len(), 1);
        // "nonsense" has no close match - should NOT get a fix
        assert!(
            !cop_005[0].has_fixes(),
            "COP-005 should not auto-fix nonsense values"
        );
    }

    // ===== COP-006: File Length Limit =====

    #[test]
    fn test_cop_006_short_file() {
        let content = "# Short copilot instructions\n\nFollow the coding standards.";
        let diagnostics = validate_global(content);
        let cop_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-006").collect();
        assert!(cop_006.is_empty());
    }

    #[test]
    fn test_cop_006_long_file() {
        let long_content = make_long_content();
        let expected_len = long_content.len().to_string();
        let diagnostics = validate_global(&long_content);
        let cop_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-006").collect();
        assert_eq!(cop_006.len(), 1);
        assert_eq!(cop_006[0].level, DiagnosticLevel::Warning);
        assert!(
            cop_006[0].message.contains(&expected_len),
            "Diagnostic message should contain the file length"
        );
    }

    #[test]
    fn test_cop_006_exact_boundary() {
        // 4000 chars should pass
        let content_4000 = "x".repeat(4000);
        let diagnostics = validate_global(&content_4000);
        let cop_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-006").collect();
        assert!(cop_006.is_empty(), "4000 chars should not trigger COP-006");

        // 4001 chars should warn
        let content_4001 = "x".repeat(4001);
        let diagnostics = validate_global(&content_4001);
        let cop_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-006").collect();
        assert_eq!(cop_006.len(), 1, "4001 chars should trigger COP-006");
    }

    #[test]
    fn test_cop_006_scoped_file_no_trigger() {
        // Scoped files should not trigger COP-006
        let mut content = String::from("---\napplyTo: \"**/*.ts\"\n---\n# Instructions\n\n");
        while content.len() <= 5000 {
            content.push_str("Follow consistent naming conventions for all variables.\n");
        }
        let diagnostics = validate_scoped(&content);
        let cop_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-006").collect();
        assert!(cop_006.is_empty());
    }

    #[test]
    fn test_cop_006_disabled() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["COP-006".to_string()];

        let diagnostics = validate_global_with_config(&make_long_content(), &config);
        let cop_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-006").collect();
        assert!(cop_006.is_empty());
    }
}
