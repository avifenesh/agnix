//! Prompt engineering validation rules
//!
//! Validates:
//! - PE-001: Critical content in middle 40-60% zone ("lost in the middle")
//! - PE-002: Chain-of-thought phrases in simple tasks
//! - PE-003: Weak language (should/try/consider) in critical sections
//! - PE-004: Ambiguous terms (usually/sometimes/if possible)

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::Validator,
    schemas::prompt::{
        find_ambiguous_instructions, find_cot_on_simple_tasks, find_critical_in_middle_pe,
        find_weak_imperative_language,
    },
};
use rust_i18n::t;
use std::path::Path;

pub struct PromptValidator;

impl Validator for PromptValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // PE-001: Critical content in middle ("lost in the middle")
        if config.is_rule_enabled("PE-001") {
            let critical_in_middle = find_critical_in_middle_pe(content);
            for issue in critical_in_middle {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        issue.line,
                        issue.column,
                        "PE-001",
                        t!(
                            "rules.pe_001.message",
                            keyword = issue.keyword.as_str(),
                            percent = format!("{:.0}", issue.position_percent)
                        ),
                    )
                    .with_suggestion(t!("rules.pe_001.suggestion")),
                );
            }
        }

        // PE-002: Chain-of-thought on simple tasks
        if config.is_rule_enabled("PE-002") {
            let cot_issues = find_cot_on_simple_tasks(content);
            for issue in cot_issues {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        issue.line,
                        issue.column,
                        "PE-002",
                        t!(
                            "rules.pe_002.message",
                            phrase = issue.phrase.as_str(),
                            task = issue.task_indicator.as_str()
                        ),
                    )
                    .with_suggestion(t!("rules.pe_002.suggestion")),
                );
            }
        }

        // PE-003: Weak imperative language in critical sections
        if config.is_rule_enabled("PE-003") {
            let weak_language = find_weak_imperative_language(content);
            for issue in weak_language {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        issue.line,
                        issue.column,
                        "PE-003",
                        t!(
                            "rules.pe_003.message",
                            term = issue.weak_term.as_str(),
                            section = issue.section_name.as_str()
                        ),
                    )
                    .with_suggestion(t!("rules.pe_003.suggestion")),
                );
            }
        }

        // PE-004: Ambiguous instructions
        if config.is_rule_enabled("PE-004") {
            let ambiguous = find_ambiguous_instructions(content);
            for issue in ambiguous {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        issue.line,
                        issue.column,
                        "PE-004",
                        t!("rules.pe_004.message", term = issue.term.as_str()),
                    )
                    .with_suggestion(t!("rules.pe_004.suggestion")),
                );
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

    // ===== PE-001: Critical Content in Middle =====

    #[test]
    fn test_pe_001_critical_in_middle() {
        // Create 20 lines with "critical" at line 10 (50%)
        let mut lines: Vec<String> = (0..20).map(|i| format!("Line {}", i)).collect();
        lines[10] = "This is critical information.".to_string();
        let content = lines.join("\n");

        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), &content, &LintConfig::default());

        let pe_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-001").collect();
        assert_eq!(pe_001.len(), 1);
        assert_eq!(pe_001[0].level, DiagnosticLevel::Warning);
        assert!(pe_001[0].message.contains("critical"));
    }

    #[test]
    fn test_pe_001_critical_at_start_ok() {
        let mut lines: Vec<String> = (0..20).map(|i| format!("Line {}", i)).collect();
        lines[1] = "This is critical information.".to_string();
        let content = lines.join("\n");

        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), &content, &LintConfig::default());

        let pe_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-001").collect();
        assert!(pe_001.is_empty());
    }

    #[test]
    fn test_pe_001_critical_at_end_ok() {
        let mut lines: Vec<String> = (0..20).map(|i| format!("Line {}", i)).collect();
        lines[18] = "This is critical information.".to_string();
        let content = lines.join("\n");

        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), &content, &LintConfig::default());

        let pe_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-001").collect();
        assert!(pe_001.is_empty());
    }

    #[test]
    fn test_pe_001_short_document_skipped() {
        let content = "Critical info.\nShort doc.";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-001").collect();
        assert!(pe_001.is_empty());
    }

    // ===== PE-002: Chain-of-Thought on Simple Tasks =====

    #[test]
    fn test_pe_002_cot_on_read_file() {
        let content = r#"# Read File Skill

When asked to read the file, think step by step.
"#;
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-002").collect();
        assert_eq!(pe_002.len(), 1);
        assert_eq!(pe_002[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_pe_002_no_cot_on_complex_task() {
        let content = r#"# Code Review Skill

Think step by step when reviewing.
"#;
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-002").collect();
        assert!(pe_002.is_empty());
    }

    #[test]
    fn test_pe_002_simple_task_without_cot_ok() {
        let content = r#"# Read File Skill

Read the file and return contents.
"#;
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-002").collect();
        assert!(pe_002.is_empty());
    }

    // ===== PE-003: Weak Imperative Language =====

    #[test]
    fn test_pe_003_weak_language_in_critical() {
        let content = r#"# Critical Rules

You should follow the style guide.
"#;
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());

        let pe_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-003").collect();
        assert_eq!(pe_003.len(), 1);
        assert_eq!(pe_003[0].level, DiagnosticLevel::Warning);
        assert!(pe_003[0].message.contains("should"));
    }

    #[test]
    fn test_pe_003_strong_language_in_critical_ok() {
        let content = r#"# Critical Rules

You must follow the style guide.
"#;
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());

        let pe_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-003").collect();
        assert!(pe_003.is_empty());
    }

    #[test]
    fn test_pe_003_weak_language_outside_critical_ok() {
        let content = r#"# General Guidelines

You should follow the style guide.
"#;
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());

        let pe_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-003").collect();
        assert!(pe_003.is_empty());
    }

    // ===== PE-004: Ambiguous Instructions =====

    #[test]
    fn test_pe_004_usually() {
        let content = "Usually format output as JSON.";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-004").collect();
        assert_eq!(pe_004.len(), 1);
        assert_eq!(pe_004[0].level, DiagnosticLevel::Warning);
        assert!(pe_004[0].message.to_lowercase().contains("usually"));
    }

    #[test]
    fn test_pe_004_if_possible() {
        let content = "Include tests if possible.";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-004").collect();
        assert_eq!(pe_004.len(), 1);
    }

    #[test]
    fn test_pe_004_clear_instruction_ok() {
        let content = "Always format output as JSON.";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-004").collect();
        assert!(pe_004.is_empty());
    }

    #[test]
    fn test_pe_004_skips_code_blocks() {
        let content = r#"```
// Usually this comment is fine
```"#;
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-004").collect();
        assert!(pe_004.is_empty());
    }

    // ===== Config Integration Tests =====

    #[test]
    fn test_config_disabled_prompt_engineering_category() {
        let mut config = LintConfig::default();
        config.rules.prompt_engineering = false;

        let content = r#"# Critical Rules
You should do X.
Usually do Y.
"#;

        let validator = PromptValidator;
        let diagnostics = validator.validate(Path::new("SKILL.md"), content, &config);

        // All PE-* rules should be disabled
        let pe_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("PE-"))
            .collect();
        assert!(pe_rules.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["PE-003".to_string()];

        let content = r#"# Critical Rules
You should do X.
"#;

        let validator = PromptValidator;
        let diagnostics = validator.validate(Path::new("SKILL.md"), content, &config);

        // PE-003 should not fire when specifically disabled
        let pe_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-003").collect();
        assert!(pe_003.is_empty());

        // PE-004 should still work
        assert!(config.is_rule_enabled("PE-004"));
    }

    #[test]
    fn test_combined_issues() {
        // Create content that triggers multiple PE rules
        let mut lines: Vec<String> = (0..20).map(|i| format!("Line {}", i)).collect();
        lines[0] = "# Critical Rules".to_string();
        lines[1] = "You should follow the style.".to_string();
        lines[2] = "Usually do X.".to_string();
        lines[10] = "This is critical information.".to_string();
        let content = lines.join("\n");

        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), &content, &LintConfig::default());

        // Should have PE-001, PE-003, and PE-004 issues
        let pe_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-001").collect();
        let pe_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-003").collect();
        let pe_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-004").collect();

        assert!(!pe_001.is_empty(), "Expected PE-001 for critical in middle");
        assert!(!pe_003.is_empty(), "Expected PE-003 for weak language");
        assert!(!pe_004.is_empty(), "Expected PE-004 for ambiguous term");
    }

    // ===== Edge Case Tests =====

    #[test]
    fn test_empty_content_string() {
        let content = "";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        // Should not panic, return empty diagnostics
        assert!(
            diagnostics.is_empty(),
            "Empty content should produce no diagnostics"
        );
    }

    #[test]
    fn test_pe_001_exactly_ten_lines() {
        // PE-001 requires at least 10 lines to check
        let lines: Vec<&str> = (0..10).map(|_| "Line").collect();
        let content = lines.join("\n");

        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), &content, &LintConfig::default());

        let pe_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-001").collect();
        // 10 lines is the boundary; at 50% no critical word, so empty
        assert!(
            pe_001.is_empty(),
            "10 exact lines without critical keyword should be ok"
        );
    }

    #[test]
    fn test_pe_001_nine_lines_skipped() {
        // Fewer than 10 lines should skip PE-001 check
        let lines: Vec<&str> = (0..9).map(|_| "Line").collect();
        let content = lines.join("\n");

        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), &content, &LintConfig::default());

        let pe_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-001").collect();
        assert!(pe_001.is_empty(), "9 lines should skip PE-001 check");
    }

    #[test]
    fn test_pe_003_word_boundary_hypercritical() {
        let content = r#"# Hypercritical Guide

This is not a critical section.
"#;
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-003").collect();
        // "Hypercritical" should NOT be recognized as a critical section
        // because the pattern should match word boundaries
        assert!(
            pe_003.is_empty(),
            "Hypercritical should not trigger critical section"
        );
    }

    #[test]
    fn test_pe_004_inline_code_not_flagged() {
        let content = "Format output with `usually` backticks for clarity.";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-004").collect();
        // Content inside inline code should still be checked (current behavior)
        // This test documents the current behavior
        assert!(
            !pe_004.is_empty(),
            "Inline code with ambiguous terms is currently flagged"
        );
    }

    #[test]
    fn test_config_disabled_pe_001_only() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["PE-001".to_string()];

        let mut lines: Vec<String> = (0..20).map(|i| format!("Line {}", i)).collect();
        lines[10] = "This is critical information.".to_string();
        lines[1] = "# Critical Rules".to_string();
        lines[2] = "You should follow style.".to_string();
        lines[3] = "Usually do X.".to_string();
        let content = lines.join("\n");

        let validator = PromptValidator;
        let diagnostics = validator.validate(Path::new("SKILL.md"), &content, &config);

        // PE-001 should be disabled
        let pe_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-001").collect();
        assert!(pe_001.is_empty(), "PE-001 should be disabled");

        // PE-003 and PE-004 should still work
        let pe_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-003").collect();
        let pe_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-004").collect();
        assert!(!pe_003.is_empty(), "PE-003 should still be enabled");
        assert!(!pe_004.is_empty(), "PE-004 should still be enabled");
    }

    #[test]
    fn test_config_disabled_multiple_pe_rules() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["PE-001".to_string(), "PE-004".to_string()];

        let mut lines: Vec<String> = (0..20).map(|i| format!("Line {}", i)).collect();
        lines[10] = "This is critical information.".to_string();
        lines[1] = "# Critical Rules".to_string();
        lines[2] = "You should follow style.".to_string();
        lines[3] = "Usually do X.".to_string();
        let content = lines.join("\n");

        let validator = PromptValidator;
        let diagnostics = validator.validate(Path::new("SKILL.md"), &content, &config);

        // PE-001 and PE-004 should be disabled
        let pe_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-001").collect();
        let pe_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-004").collect();
        assert!(pe_001.is_empty(), "PE-001 should be disabled");
        assert!(pe_004.is_empty(), "PE-004 should be disabled");

        // PE-003 should still work
        let pe_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-003").collect();
        assert!(!pe_003.is_empty(), "PE-003 should still be enabled");
    }

    // ===== Additional PE rule tests =====

    #[test]
    fn test_pe_001_critical_at_very_end() {
        let mut lines: Vec<String> = (0..20).map(|i| format!("Line {}", i)).collect();
        lines[19] = "This is critical information.".to_string();
        let content = lines.join("\n");

        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), &content, &LintConfig::default());

        let pe_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-001").collect();
        assert!(
            pe_001.is_empty(),
            "Critical at end should not trigger PE-001"
        );
    }

    #[test]
    fn test_pe_001_critical_at_very_start() {
        let mut lines: Vec<String> = (0..20).map(|i| format!("Line {}", i)).collect();
        lines[0] = "This is critical information.".to_string();
        let content = lines.join("\n");

        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), &content, &LintConfig::default());

        let pe_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-001").collect();
        assert!(
            pe_001.is_empty(),
            "Critical at start should not trigger PE-001"
        );
    }

    #[test]
    fn test_pe_002_cot_on_file_read() {
        // Test PE-002: CoT markers on simple tasks
        // Content must have both: CoT marker + simple task indicator within 5 lines
        let content = "# Rules\n\nLet's think step by step.\nYou need to read the file carefully.";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-002").collect();
        // PE-002 triggers when CoT marker is within 5 lines of a simple task indicator
        assert!(
            !pe_002.is_empty(),
            "Chain-of-thought near 'read the file' should trigger PE-002"
        );
    }

    #[test]
    fn test_pe_003_should_weak_word() {
        // Test with known weak word
        let content = "# Critical Rules\n\nYou should do this.";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-003").collect();
        assert!(
            !pe_003.is_empty(),
            "Weak word 'should' in critical section should trigger PE-003"
        );
    }

    #[test]
    fn test_pe_003_consider_weak_word() {
        let content = "# Critical Rules\n\nYou consider doing this.";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-003").collect();
        assert!(
            !pe_003.is_empty(),
            "Weak word 'consider' in critical section should trigger PE-003"
        );
    }

    #[test]
    fn test_pe_003_strong_words_ok() {
        let strong_words = ["must", "always", "never", "shall"];

        for word in strong_words {
            let content = format!("# Critical Rules\n\nYou {} do this.", word);
            let validator = PromptValidator;
            let diagnostics =
                validator.validate(Path::new("SKILL.md"), &content, &LintConfig::default());

            let pe_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-003").collect();
            assert!(
                pe_003.is_empty(),
                "Strong word '{}' should not trigger PE-003",
                word
            );
        }
    }

    #[test]
    fn test_pe_004_all_ambiguous_phrases() {
        let ambiguous = ["usually", "if possible", "when appropriate", "sometimes"];

        for phrase in ambiguous {
            let content = format!("# Rules\n\n{} do this task.", phrase);
            let validator = PromptValidator;
            let diagnostics =
                validator.validate(Path::new("SKILL.md"), &content, &LintConfig::default());

            let pe_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-004").collect();
            assert!(
                !pe_004.is_empty(),
                "Ambiguous phrase '{}' should trigger PE-004",
                phrase
            );
        }
    }

    #[test]
    fn test_pe_004_clear_instructions_ok() {
        let content = "# Rules\n\nAlways run tests before committing.";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-004").collect();
        assert!(
            pe_004.is_empty(),
            "Clear instructions should not trigger PE-004"
        );
    }

    #[test]
    fn test_all_pe_rules_can_be_disabled() {
        let rules = ["PE-001", "PE-002", "PE-003", "PE-004"];

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules.disabled_rules = vec![rule.to_string()];

            // Content that could trigger each rule
            let mut lines: Vec<String> = (0..20).map(|i| format!("Line {}", i)).collect();
            lines[10] = "This is critical information.".to_string();
            lines[1] = "# Critical Rules".to_string();
            lines[2] = "You should step by step read the file. Usually do it.".to_string();
            let content = lines.join("\n");

            let validator = PromptValidator;
            let diagnostics = validator.validate(Path::new("SKILL.md"), &content, &config);

            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }
}
