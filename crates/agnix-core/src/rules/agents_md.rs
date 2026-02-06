//! AGENTS.md validation rules (AGM-001 to AGM-006)
//!
//! Validates:
//! - AGM-001: Valid Markdown Structure (HIGH) - unclosed code blocks, malformed links
//! - AGM-002: Missing Section Headers (MEDIUM) - no # or ## headers
//! - AGM-003: Character Limit (HIGH) - over 12000 chars (Windsurf compatibility)
//! - AGM-004: Missing Project Context (MEDIUM) - no project description
//! - AGM-005: Platform-Specific Features Without Guard (HIGH) - missing guard comments
//! - AGM-006: Nested AGENTS.md Hierarchy (MEDIUM) - project-level check

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::Validator,
    schemas::agents_md::{
        MarkdownIssueType, WINDSURF_CHAR_LIMIT, check_character_limit, check_markdown_validity,
        check_project_context, check_section_headers, find_unguarded_platform_features,
    },
};
use rust_i18n::t;
use std::path::Path;

pub struct AgentsMdValidator;

impl Validator for AgentsMdValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Only validate AGENTS.md variants (not CLAUDE.md files)
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !matches!(
            filename,
            "AGENTS.md" | "AGENTS.local.md" | "AGENTS.override.md"
        ) {
            return diagnostics;
        }

        // AGM-001: Valid Markdown Structure (ERROR)
        if config.is_rule_enabled("AGM-001") {
            let validity_issues = check_markdown_validity(content);
            for issue in validity_issues {
                let level_fn = match issue.issue_type {
                    MarkdownIssueType::UnclosedCodeBlock => Diagnostic::error,
                    MarkdownIssueType::MalformedLink => Diagnostic::error,
                };
                diagnostics.push(
                    level_fn(
                        path.to_path_buf(),
                        issue.line,
                        issue.column,
                        "AGM-001",
                        t!(
                            "rules.agm_001.message",
                            description = issue.description.as_str()
                        ),
                    )
                    .with_suggestion(t!("rules.agm_001.suggestion")),
                );
            }
        }

        // AGM-002: Missing Section Headers (WARNING)
        if config.is_rule_enabled("AGM-002") {
            if let Some(issue) = check_section_headers(content) {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        issue.line,
                        issue.column,
                        "AGM-002",
                        issue.description,
                    )
                    .with_suggestion(issue.suggestion),
                );
            }
        }

        // AGM-003: Character Limit (WARNING)
        if config.is_rule_enabled("AGM-003") {
            if let Some(exceeded) = check_character_limit(content, WINDSURF_CHAR_LIMIT) {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        1,
                        0,
                        "AGM-003",
                        t!(
                            "rules.agm_003.message",
                            filename = filename,
                            chars = exceeded.char_count,
                            limit = exceeded.limit
                        ),
                    )
                    .with_suggestion(t!("rules.agm_003.suggestion")),
                );
            }
        }

        // AGM-004: Missing Project Context (WARNING)
        if config.is_rule_enabled("AGM-004") {
            if let Some(issue) = check_project_context(content) {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        issue.line,
                        issue.column,
                        "AGM-004",
                        issue.description,
                    )
                    .with_suggestion(issue.suggestion),
                );
            }
        }

        // AGM-005: Platform-Specific Features Without Guard (WARNING)
        if config.is_rule_enabled("AGM-005") {
            let unguarded = find_unguarded_platform_features(content);
            for feature in unguarded {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        feature.line,
                        feature.column,
                        "AGM-005",
                        feature.description,
                    )
                    .with_suggestion(t!(
                        "rules.agm_005.suggestion",
                        platform = feature.platform.as_str()
                    )),
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

    fn validate(content: &str) -> Vec<Diagnostic> {
        let validator = AgentsMdValidator;
        validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default())
    }

    fn validate_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = AgentsMdValidator;
        validator.validate(Path::new("AGENTS.md"), content, config)
    }

    // ===== Skip non-AGENTS.md files =====

    #[test]
    fn test_skip_claude_md() {
        let content = r#"```unclosed
Some content"#;
        let validator = AgentsMdValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());
        // Should return empty for CLAUDE.md
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_skip_other_md() {
        let content = r#"```unclosed"#;
        let validator = AgentsMdValidator;
        let diagnostics =
            validator.validate(Path::new("README.md"), content, &LintConfig::default());
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_skip_claude_local_md() {
        // CLAUDE.local.md should NOT get AGM rules (only AGENTS.* files do)
        let content = r#"```unclosed
Some content"#;
        let validator = AgentsMdValidator;
        let diagnostics = validator.validate(
            Path::new("CLAUDE.local.md"),
            content,
            &LintConfig::default(),
        );
        assert!(
            diagnostics.is_empty(),
            "CLAUDE.local.md should not get AGM rules"
        );
    }

    // ===== AGENTS.* Variant Files =====

    #[test]
    fn test_agents_variants_get_agm_rules() {
        // Both AGENTS.local.md and AGENTS.override.md should get AGM rules
        let content = r#"```unclosed
Some content"#;
        let variants = ["AGENTS.local.md", "AGENTS.override.md"];
        let validator = AgentsMdValidator;

        for variant in variants {
            let diagnostics =
                validator.validate(Path::new(variant), content, &LintConfig::default());
            let agm_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-001").collect();
            assert_eq!(
                agm_001.len(),
                1,
                "{} should get AGM-001 for unclosed code block",
                variant
            );
        }
    }

    #[test]
    fn test_agents_local_md_char_limit() {
        let content = format!("# Project\n\n{}", "x".repeat(13000));
        let validator = AgentsMdValidator;
        let diagnostics = validator.validate(
            Path::new("AGENTS.local.md"),
            &content,
            &LintConfig::default(),
        );
        let agm_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-003").collect();
        assert_eq!(
            agm_003.len(),
            1,
            "AGENTS.local.md should get AGM-003 for char limit"
        );
    }

    #[test]
    fn test_agents_override_md_unguarded_features() {
        let content = r#"# Project

- type: PreToolExecution
  command: echo "test"
"#;
        let validator = AgentsMdValidator;
        let diagnostics = validator.validate(
            Path::new("AGENTS.override.md"),
            content,
            &LintConfig::default(),
        );
        let agm_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-005").collect();
        assert_eq!(
            agm_005.len(),
            1,
            "AGENTS.override.md should get AGM-005 for unguarded hooks"
        );
    }

    // ===== AGM-001: Valid Markdown Structure =====

    #[test]
    fn test_agm_001_unclosed_code_block() {
        let content = r#"# Project
```rust
fn main() {}
"#;
        let diagnostics = validate(content);
        let agm_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-001").collect();
        assert_eq!(agm_001.len(), 1);
        assert_eq!(agm_001[0].level, DiagnosticLevel::Error);
        assert!(agm_001[0].message.contains("Unclosed code block"));
    }

    #[test]
    fn test_agm_001_valid_markdown() {
        let content = r#"# Project
```rust
fn main() {}
```

Check [this link](http://example.com) for more.
"#;
        let diagnostics = validate(content);
        let agm_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-001").collect();
        assert!(agm_001.is_empty());
    }

    // ===== AGM-002: Missing Section Headers =====

    #[test]
    fn test_agm_002_no_headers() {
        let content = "Just plain text without any headers.";
        let diagnostics = validate(content);
        let agm_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-002").collect();
        assert_eq!(agm_002.len(), 1);
        assert_eq!(agm_002[0].level, DiagnosticLevel::Warning);
        assert!(agm_002[0].message.contains("No markdown headers"));
    }

    #[test]
    fn test_agm_002_has_headers() {
        let content = r#"# Main Title

Some content here.

## Section

More content.
"#;
        let diagnostics = validate(content);
        let agm_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-002").collect();
        assert!(agm_002.is_empty());
    }

    // ===== AGM-003: Character Limit =====

    #[test]
    fn test_agm_003_over_limit() {
        let content = format!("# Project\n\n{}", "x".repeat(13000));
        let diagnostics = validate(&content);
        let agm_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-003").collect();
        assert_eq!(agm_003.len(), 1);
        assert_eq!(agm_003[0].level, DiagnosticLevel::Warning);
        assert!(agm_003[0].message.contains("exceeds character limit"));
    }

    #[test]
    fn test_agm_003_under_limit() {
        let content = format!("# Project\n\n{}", "x".repeat(10000));
        let diagnostics = validate(&content);
        let agm_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-003").collect();
        assert!(agm_003.is_empty());
    }

    // ===== AGM-004: Missing Project Context =====

    #[test]
    fn test_agm_004_missing_context() {
        let content = r#"# Build Commands

Run npm install and npm build.

## Testing

Use npm test.
"#;
        let diagnostics = validate(content);
        let agm_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-004").collect();
        assert_eq!(agm_004.len(), 1);
        assert_eq!(agm_004[0].level, DiagnosticLevel::Warning);
        assert!(agm_004[0].message.contains("Missing project context"));
    }

    #[test]
    fn test_agm_004_has_project_section() {
        let content = r#"# Project

This is a linter for agent configurations.

## Build Commands

Run npm install.
"#;
        let diagnostics = validate(content);
        let agm_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-004").collect();
        assert!(agm_004.is_empty());
    }

    #[test]
    fn test_agm_004_has_overview_section() {
        let content = r#"# Overview

A comprehensive validation tool.

## Usage

Run the CLI.
"#;
        let diagnostics = validate(content);
        let agm_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-004").collect();
        assert!(agm_004.is_empty());
    }

    // ===== AGM-005: Unguarded Platform Features =====

    #[test]
    fn test_agm_005_unguarded_hooks() {
        let content = r#"# Project

This project uses hooks.

- type: PreToolExecution
  command: echo "test"
"#;
        let diagnostics = validate(content);
        let agm_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-005").collect();
        assert_eq!(agm_005.len(), 1);
        assert_eq!(agm_005[0].level, DiagnosticLevel::Warning);
        assert!(agm_005[0].message.contains("hooks"));
    }

    #[test]
    fn test_agm_005_guarded_hooks() {
        let content = r#"# Project

This project uses hooks.

## Claude Code Specific

- type: PreToolExecution
  command: echo "test"
"#;
        let diagnostics = validate(content);
        let agm_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-005").collect();
        assert!(agm_005.is_empty());
    }

    #[test]
    fn test_agm_005_unguarded_context_fork() {
        let content = r#"# Project

---
context: fork
---

Some content.
"#;
        let diagnostics = validate(content);
        let agm_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-005").collect();
        assert!(agm_005.iter().any(|d| d.message.contains("context:fork")));
    }

    #[test]
    fn test_agm_005_multiple_unguarded() {
        let content = r#"# Project

context: fork
agent: reviewer
allowed-tools: Read Write
"#;
        let diagnostics = validate(content);
        let agm_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-005").collect();
        // Should detect all three unguarded features
        assert!(agm_005.len() >= 3);
    }

    // ===== Config Integration Tests =====

    #[test]
    fn test_config_disabled_agents_md_category() {
        let mut config = LintConfig::default();
        config.rules.agents_md = false;

        let content = r#"```unclosed
Just text without headers."#;
        let diagnostics = validate_with_config(content, &config);

        // All AGM-* rules should be disabled
        let agm_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("AGM-"))
            .collect();
        assert!(agm_rules.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["AGM-001".to_string()];

        let content = r#"# Project
```unclosed"#;
        let diagnostics = validate_with_config(content, &config);

        // AGM-001 should not fire when specifically disabled
        let agm_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-001").collect();
        assert!(agm_001.is_empty());

        // Other rules should still work
        assert!(config.is_rule_enabled("AGM-002"));
        assert!(config.is_rule_enabled("AGM-003"));
    }

    #[test]
    fn test_valid_agents_md_no_errors() {
        let content = r#"# Project

This project validates agent configurations.

## Build Commands

Run npm install and npm build.

## Claude Code Specific

- type: PreToolExecution
  command: echo "test"
"#;
        let diagnostics = validate(content);

        // Should have no errors (warnings are OK)
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .collect();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    #[test]
    fn test_combined_issues() {
        let content = r#"```unclosed
context: fork
Plain text only."#;
        let diagnostics = validate(content);

        // Should detect multiple issues
        assert!(
            diagnostics.iter().any(|d| d.rule == "AGM-001"),
            "Should detect unclosed code block"
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "AGM-002"),
            "Should detect missing headers"
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "AGM-005"),
            "Should detect unguarded platform feature"
        );
    }

    // ===== Additional AGM rule tests =====

    #[test]
    fn test_agm_001_balanced_code_blocks() {
        let content = r#"# Project

```python
def hello():
    print("world")
```

More text here."#;

        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "AGM-001"));
    }

    #[test]
    fn test_agm_001_single_unclosed_block() {
        // The parser detects unclosed blocks differently - test with single unclosed
        let content = r#"# Project

```python
code here without closing"#;

        let diagnostics = validate(content);
        let agm_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-001").collect();
        assert!(!agm_001.is_empty(), "Should detect unclosed code block");
    }

    #[test]
    fn test_agm_002_multiple_header_levels() {
        let content = r#"# Main Title

## Subsection

### Details

Content here."#;

        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "AGM-002"));
    }

    #[test]
    fn test_agm_003_exact_12000_chars() {
        // AGM-003 checks for content exceeding WINDSURF_CHAR_LIMIT
        let content = "a".repeat(WINDSURF_CHAR_LIMIT);

        let validator = AgentsMdValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), &content, &LintConfig::default());
        // At exactly WINDSURF_CHAR_LIMIT chars, should not trigger (limit is >12000)
        let agm_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-003").collect();
        assert!(agm_003.is_empty());
    }

    #[test]
    fn test_agm_003_over_12001_chars() {
        let content = "a".repeat(WINDSURF_CHAR_LIMIT + 1);

        let validator = AgentsMdValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), &content, &LintConfig::default());
        let agm_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-003").collect();
        // Over WINDSURF_CHAR_LIMIT should exceed the Windsurf compatibility limit
        assert!(!agm_003.is_empty());
    }

    #[test]
    fn test_agm_004_has_tech_stack_section() {
        let content = r#"# Project

## Tech Stack

- Rust
- TypeScript"#;

        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "AGM-004"));
    }

    #[test]
    fn test_agm_005_hooks_yaml_unguarded() {
        // Test platform feature detection with YAML hook format
        let content = r#"# Project

- type: PreToolExecution
  command: echo "test""#;

        let diagnostics = validate(content);
        let agm_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-005").collect();
        // Should detect unguarded hooks
        assert!(!agm_005.is_empty(), "Should detect unguarded hooks");
    }

    #[test]
    fn test_agm_005_guarded_with_tool_section() {
        let content = r#"# Project

## Claude Code

Use context: fork for subagents.
Configure hooks for automation."#;

        let diagnostics = validate(content);
        let agm_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-005").collect();
        // Platform features under "Claude Code" section should be guarded
        assert!(agm_005.is_empty());
    }

    #[test]
    fn test_all_agm_rules_can_be_disabled() {
        let rules = ["AGM-001", "AGM-002", "AGM-003", "AGM-004", "AGM-005"];

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules.disabled_rules = vec![rule.to_string()];

            // Content that could trigger each rule
            let content = r#"```unclosed
context: fork"#;

            let validator = AgentsMdValidator;
            let diagnostics = validator.validate(Path::new("AGENTS.md"), content, &config);

            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }
}
