//! `.claude/rules/*.md` frontmatter validation rules (CC-MEM-011, CC-MEM-012)
//!
//! Validates:
//! - CC-MEM-011: Invalid paths glob in rules (HIGH) - glob patterns must be valid
//! - CC-MEM-012: Rules file unknown frontmatter key (MEDIUM) - only `paths` is known

use crate::{
    config::LintConfig,
    diagnostics::{Diagnostic, Fix},
    rules::Validator,
    schemas::claude_rules::{parse_frontmatter, validate_glob_pattern},
};
use rust_i18n::t;
use std::path::Path;

pub struct ClaudeRulesValidator;

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

impl Validator for ClaudeRulesValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Only validate .claude/rules/*.md files
        let parent = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str());
        let grandparent = path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str());

        if parent != Some("rules") || grandparent != Some(".claude") {
            return diagnostics;
        }

        // Parse frontmatter - if no frontmatter, nothing to validate
        let parsed = match parse_frontmatter(content) {
            Some(p) => p,
            None => return diagnostics,
        };

        // If there's a parse error, emit a diagnostic and return early
        if let Some(ref parse_error) = parsed.parse_error {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    parsed.start_line,
                    0,
                    "CC-MEM-011",
                    format!("Invalid frontmatter: {}", parse_error),
                )
                .with_suggestion("Close frontmatter with a line containing only `---`."),
            );
            return diagnostics;
        }

        // CC-MEM-011: Invalid paths glob in rules (ERROR)
        if config.is_rule_enabled("CC-MEM-011") {
            if let Some(ref schema) = parsed.schema {
                for (i, pattern) in schema.paths.iter().enumerate() {
                    let validation = validate_glob_pattern(pattern);
                    if !validation.valid {
                        // Try to find the line number of this pattern in the frontmatter
                        let line = find_pattern_line(&parsed.raw, pattern, parsed.start_line)
                            .unwrap_or(parsed.start_line + 1);

                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                line,
                                0,
                                "CC-MEM-011",
                                t!(
                                    "rules.cc_mem_011.message",
                                    pattern = pattern.as_str(),
                                    error = validation.error.unwrap_or_default(),
                                    index = i + 1
                                ),
                            )
                            .with_suggestion(t!("rules.cc_mem_011.suggestion")),
                        );
                    }
                }
            }
        }

        // CC-MEM-012: Unknown frontmatter keys (WARNING)
        if config.is_rule_enabled("CC-MEM-012") {
            for unknown in &parsed.unknown_keys {
                let mut diagnostic = Diagnostic::warning(
                    path.to_path_buf(),
                    unknown.line,
                    unknown.column,
                    "CC-MEM-012",
                    t!("rules.cc_mem_012.message", key = unknown.key.as_str()),
                )
                .with_suggestion(t!(
                    "rules.cc_mem_012.suggestion",
                    key = unknown.key.as_str()
                ));

                // Auto-fix: remove unknown top-level frontmatter key line.
                // Marked unsafe because the key's value may span multiple lines
                // (e.g. block scalars, nested maps) and we only delete the key line.
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

/// Find the line number of a pattern in the raw frontmatter
///
/// Matches YAML list items precisely by stripping the leading `- ` prefix
/// and surrounding quotes, avoiding false matches when one pattern is a
/// substring of another.
fn find_pattern_line(raw: &str, pattern: &str, start_line: usize) -> Option<usize> {
    for (i, line) in raw.lines().enumerate() {
        if let Some(value_part) = line.trim().strip_prefix('-') {
            let value = value_part.trim();
            let value_unquoted = value.trim_matches(|c| c == '\'' || c == '"');
            if value_unquoted == pattern {
                return Some(start_line + 1 + i);
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

    fn validate_rule(content: &str) -> Vec<Diagnostic> {
        let validator = ClaudeRulesValidator;
        validator.validate(
            Path::new(".claude/rules/my-rule.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_rule_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = ClaudeRulesValidator;
        validator.validate(Path::new(".claude/rules/my-rule.md"), content, config)
    }

    // ===== CC-MEM-011: Invalid Paths Glob =====

    #[test]
    fn test_cc_mem_011_invalid_glob() {
        let content = r#"---
paths:
  - "[unclosed"
---
# Rule content
"#;
        let diagnostics = validate_rule(content);
        let mem_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-011")
            .collect();
        assert_eq!(mem_011.len(), 1);
        assert_eq!(mem_011[0].level, DiagnosticLevel::Error);
        assert!(mem_011[0].message.contains("[unclosed"));
    }

    #[test]
    fn test_cc_mem_011_multiple_invalid_globs() {
        let content = r#"---
paths:
  - "[bad1"
  - "**/*.ts"
  - "[bad2"
---
# Rule content
"#;
        let diagnostics = validate_rule(content);
        let mem_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-011")
            .collect();
        assert_eq!(mem_011.len(), 2);
    }

    #[test]
    fn test_cc_mem_011_valid_globs() {
        let patterns = vec![
            "**/*.ts",
            "*.rs",
            "src/**/*.js",
            "tests/**/*.test.ts",
            "{src,lib}/**/*.ts",
        ];

        for pattern in patterns {
            let content = format!(
                r#"---
paths:
  - "{}"
---
# Rule content
"#,
                pattern
            );
            let diagnostics = validate_rule(&content);
            let mem_011: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-MEM-011")
                .collect();
            assert!(mem_011.is_empty(), "Pattern '{}' should be valid", pattern);
        }
    }

    #[test]
    fn test_cc_mem_011_no_frontmatter() {
        let content = "# Rule without frontmatter\n\nSome instructions.";
        let diagnostics = validate_rule(content);
        let mem_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-011")
            .collect();
        assert!(mem_011.is_empty());
    }

    #[test]
    fn test_cc_mem_011_empty_paths() {
        let content = "---\npaths: []\n---\n# Content";
        let diagnostics = validate_rule(content);
        let mem_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-011")
            .collect();
        assert!(mem_011.is_empty());
    }

    #[test]
    fn test_cc_mem_011_unclosed_frontmatter() {
        let content = "---\npaths:\n  - \"src/**/*.ts\"";
        let diagnostics = validate_rule(content);
        let mem_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-011")
            .collect();
        assert_eq!(mem_011.len(), 1);
        assert!(mem_011[0].message.contains("missing closing ---"));
    }

    // ===== CC-MEM-012: Unknown Frontmatter Keys =====

    #[test]
    fn test_cc_mem_012_unknown_keys() {
        let content = r#"---
paths:
  - "src/**/*.ts"
description: "some rule"
alwaysApply: true
---
# Content
"#;
        let diagnostics = validate_rule(content);
        let mem_012: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-012")
            .collect();
        assert_eq!(mem_012.len(), 2);
        assert_eq!(mem_012[0].level, DiagnosticLevel::Warning);
        assert!(mem_012.iter().any(|d| d.message.contains("description")));
        assert!(mem_012.iter().any(|d| d.message.contains("alwaysApply")));
    }

    #[test]
    fn test_cc_mem_012_has_autofix() {
        let content = r#"---
paths:
  - "src/**/*.ts"
description: "some rule"
---
# Content
"#;
        let diagnostics = validate_rule(content);
        let mem_012: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-012")
            .collect();
        assert_eq!(mem_012.len(), 1);
        assert!(mem_012[0].has_fixes());
        assert!(!mem_012[0].fixes[0].safe); // Unsafe: single-line delete may miss multi-line values
        assert!(mem_012[0].fixes[0].is_deletion());
    }

    #[test]
    fn test_cc_mem_012_no_unknown_keys() {
        let content = r#"---
paths:
  - "src/**/*.ts"
---
# Content
"#;
        let diagnostics = validate_rule(content);
        let mem_012: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-012")
            .collect();
        assert!(mem_012.is_empty());
    }

    #[test]
    fn test_cc_mem_012_no_frontmatter() {
        let content = "# Rule without frontmatter";
        let diagnostics = validate_rule(content);
        let mem_012: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-012")
            .collect();
        assert!(mem_012.is_empty());
    }

    // ===== Path Guard =====

    #[test]
    fn test_wrong_path_no_diagnostics() {
        let validator = ClaudeRulesValidator;
        let content = r#"---
unknownKey: value
---
# Content
"#;
        // Not in .claude/rules/ path
        let diagnostics = validator.validate(
            Path::new("some/other/path.md"),
            content,
            &LintConfig::default(),
        );
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_claude_rules_path() {
        let validator = ClaudeRulesValidator;
        let content = r#"---
unknownKey: value
---
# Content
"#;
        let diagnostics = validator.validate(
            Path::new(".claude/rules/my-rule.md"),
            content,
            &LintConfig::default(),
        );
        assert!(!diagnostics.is_empty());
    }

    // ===== Config Integration =====

    #[test]
    fn test_config_disabled_memory_category() {
        let mut config = LintConfig::default();
        config.rules_mut().memory = false;

        let content = r#"---
unknownKey: value
paths:
  - "[invalid"
---
# Content
"#;
        let diagnostics = validate_rule_with_config(content, &config);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rules() {
        let rules = ["CC-MEM-011", "CC-MEM-012"];

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules_mut().disabled_rules = vec![rule.to_string()];

            let content = r#"---
unknownKey: value
paths:
  - "[invalid"
---
# Content
"#;
            let diagnostics = validate_rule_with_config(content, &config);

            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }

    // ===== Combined Issues =====

    #[test]
    fn test_both_rules_trigger() {
        let content = r#"---
paths:
  - "[invalid"
unknownKey: value
---
# Content
"#;
        let diagnostics = validate_rule(content);
        assert!(
            diagnostics.iter().any(|d| d.rule == "CC-MEM-011"),
            "Expected CC-MEM-011"
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "CC-MEM-012"),
            "Expected CC-MEM-012"
        );
    }

    #[test]
    fn test_valid_rule_file_no_issues() {
        let content = r#"---
paths:
  - "src/**/*.ts"
  - "lib/**/*.js"
---
# TypeScript Guidelines

Always use strict mode and explicit types.
"#;
        let diagnostics = validate_rule(content);
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics, got: {:?}",
            diagnostics
        );
    }
}
