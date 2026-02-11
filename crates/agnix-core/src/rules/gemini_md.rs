//! Gemini CLI instruction file validation rules (GM-001 to GM-003)
//!
//! Validates:
//! - GM-001: Valid markdown structure (HIGH) - unclosed code blocks, malformed links
//! - GM-002: Missing section headers (MEDIUM) - no # or ## headers
//! - GM-003: Missing project context (MEDIUM) - no project description

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::Validator,
    schemas::agents_md::{
        MarkdownIssueType, check_markdown_validity, check_project_context, check_section_headers,
    },
};
use rust_i18n::t;
use std::path::Path;

pub struct GeminiMdValidator;

impl Validator for GeminiMdValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Only validate GEMINI.md variants
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !matches!(filename, "GEMINI.md" | "GEMINI.local.md") {
            return diagnostics;
        }

        let path_buf = path.to_path_buf();

        // GM-001: Valid Markdown Structure (ERROR)
        if config.is_rule_enabled("GM-001") {
            let validity_issues = check_markdown_validity(content);
            for issue in validity_issues {
                let level_fn = match issue.issue_type {
                    MarkdownIssueType::UnclosedCodeBlock => Diagnostic::error,
                    MarkdownIssueType::MalformedLink => Diagnostic::error,
                };
                diagnostics.push(
                    level_fn(
                        path_buf.clone(),
                        issue.line,
                        issue.column,
                        "GM-001",
                        t!(
                            "rules.gm_001.message",
                            description = issue.description.as_str()
                        ),
                    )
                    .with_suggestion(t!("rules.gm_001.suggestion")),
                );
            }
        }

        // GM-002: Missing Section Headers (WARNING)
        if config.is_rule_enabled("GM-002") {
            if let Some(issue) = check_section_headers(content) {
                diagnostics.push(
                    Diagnostic::warning(
                        path_buf.clone(),
                        issue.line,
                        issue.column,
                        "GM-002",
                        t!("rules.gm_002.message"),
                    )
                    .with_suggestion(t!("rules.gm_002.suggestion")),
                );
            }
        }

        // GM-003: Missing Project Context (WARNING)
        if config.is_rule_enabled("GM-003") {
            if let Some(issue) = check_project_context(content) {
                diagnostics.push(
                    Diagnostic::warning(
                        path_buf,
                        issue.line,
                        issue.column,
                        "GM-003",
                        t!("rules.gm_003.message"),
                    )
                    .with_suggestion(t!("rules.gm_003.suggestion")),
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
        let validator = GeminiMdValidator;
        validator.validate(Path::new("GEMINI.md"), content, &LintConfig::default())
    }

    fn validate_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = GeminiMdValidator;
        validator.validate(Path::new("GEMINI.md"), content, config)
    }

    // ===== Skip non-GEMINI.md files =====

    #[test]
    fn test_skip_claude_md() {
        let content = r#"```unclosed
Some content"#;
        let validator = GeminiMdValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_skip_other_md() {
        let content = "```unclosed";
        let validator = GeminiMdValidator;
        let diagnostics =
            validator.validate(Path::new("README.md"), content, &LintConfig::default());
        assert!(diagnostics.is_empty());
    }

    // ===== GEMINI.md variant files =====

    #[test]
    fn test_gemini_local_md_gets_gm_rules() {
        let content = r#"```unclosed
Some content"#;
        let validator = GeminiMdValidator;
        let diagnostics = validator.validate(
            Path::new("GEMINI.local.md"),
            content,
            &LintConfig::default(),
        );
        let gm_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-001").collect();
        assert_eq!(
            gm_001.len(),
            1,
            "GEMINI.local.md should get GM-001 for unclosed code block"
        );
    }

    // ===== GM-001: Valid Markdown Structure =====

    #[test]
    fn test_gm_001_unclosed_code_block() {
        let content = r#"# Project
```rust
fn main() {}
"#;
        let diagnostics = validate(content);
        let gm_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-001").collect();
        assert_eq!(gm_001.len(), 1);
        assert_eq!(gm_001[0].level, DiagnosticLevel::Error);
        assert!(gm_001[0].message.contains("Unclosed code block"));
    }

    #[test]
    fn test_gm_001_valid_markdown() {
        let content = r#"# Project
```rust
fn main() {}
```

Check [this link](http://example.com) for more.
"#;
        let diagnostics = validate(content);
        let gm_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-001").collect();
        assert!(gm_001.is_empty());
    }

    #[test]
    fn test_gm_001_malformed_link() {
        let content = r#"# Project

Check [this link](http://example.com for more info.
"#;
        let diagnostics = validate(content);
        let gm_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-001").collect();
        assert_eq!(gm_001.len(), 1);
        assert_eq!(gm_001[0].level, DiagnosticLevel::Error);
        assert!(gm_001[0].message.contains("Malformed markdown link"));
    }

    #[test]
    fn test_gm_001_balanced_code_blocks() {
        let content = r#"# Project

```python
def hello():
    print("world")
```

More text here."#;

        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "GM-001"));
    }

    // ===== GM-002: Missing Section Headers =====

    #[test]
    fn test_gm_002_no_headers() {
        let content = "Just plain text without any headers.";
        let diagnostics = validate(content);
        let gm_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-002").collect();
        assert_eq!(gm_002.len(), 1);
        assert_eq!(gm_002[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_gm_002_has_headers() {
        let content = r#"# Main Title

Some content here.

## Section

More content.
"#;
        let diagnostics = validate(content);
        let gm_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-002").collect();
        assert!(gm_002.is_empty());
    }

    #[test]
    fn test_gm_002_multiple_header_levels() {
        let content = r#"# Main Title

## Subsection

### Details

Content here."#;

        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "GM-002"));
    }

    // ===== GM-003: Missing Project Context =====

    #[test]
    fn test_gm_003_missing_context() {
        let content = r#"# Build Commands

Run npm install and npm build.

## Testing

Use npm test.
"#;
        let diagnostics = validate(content);
        let gm_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-003").collect();
        assert_eq!(gm_003.len(), 1);
        assert_eq!(gm_003[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_gm_003_has_project_section() {
        let content = r#"# Project

This is a linter for agent configurations.

## Build Commands

Run npm install.
"#;
        let diagnostics = validate(content);
        let gm_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-003").collect();
        assert!(gm_003.is_empty());
    }

    #[test]
    fn test_gm_003_has_overview_section() {
        let content = r#"# Overview

A comprehensive validation tool.

## Usage

Run the CLI.
"#;
        let diagnostics = validate(content);
        let gm_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-003").collect();
        assert!(gm_003.is_empty());
    }

    // ===== Config Integration Tests =====

    #[test]
    fn test_config_disabled_gemini_md_category() {
        let mut config = LintConfig::default();
        config.rules_mut().gemini_md = false;

        let content = r#"```unclosed
Just text without headers."#;
        let diagnostics = validate_with_config(content, &config);

        let gm_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("GM-"))
            .collect();
        assert!(gm_rules.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["GM-001".to_string()];

        let content = r#"# Project
```unclosed"#;
        let diagnostics = validate_with_config(content, &config);

        let gm_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-001").collect();
        assert!(gm_001.is_empty());

        // Other rules should still work
        assert!(config.is_rule_enabled("GM-002"));
        assert!(config.is_rule_enabled("GM-003"));
    }

    #[test]
    fn test_all_gm_rules_can_be_disabled() {
        let rules = ["GM-001", "GM-002", "GM-003"];

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules_mut().disabled_rules = vec![rule.to_string()];

            // Content that could trigger each rule
            let content = r#"```unclosed
plain text only"#;

            let validator = GeminiMdValidator;
            let diagnostics = validator.validate(Path::new("GEMINI.md"), content, &config);

            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }

    // ===== Combined Issues =====

    #[test]
    fn test_combined_issues() {
        let content = r#"```unclosed
plain text only"#;
        let diagnostics = validate(content);

        assert!(
            diagnostics.iter().any(|d| d.rule == "GM-001"),
            "Should detect unclosed code block"
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "GM-002"),
            "Should detect missing headers"
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "GM-003"),
            "Should detect missing project context"
        );
    }

    #[test]
    fn test_valid_gemini_md_no_errors() {
        let content = r#"# Project

This project validates agent configurations.

## Build Commands

```bash
npm install
npm build
```

## Testing

Run npm test.
"#;
        let diagnostics = validate(content);

        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .collect();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    // ===== File Type Detection =====

    #[test]
    fn test_file_type_detection() {
        assert_eq!(
            crate::detect_file_type(Path::new("GEMINI.md")),
            crate::FileType::GeminiMd
        );
        assert_eq!(
            crate::detect_file_type(Path::new("GEMINI.local.md")),
            crate::FileType::GeminiMd
        );
    }

    // ===== GM-001 improved suggestion test =====

    #[test]
    fn test_gm_001_suggestion_mentions_unclosed_tags() {
        let content = r#"```unclosed
Some content"#;
        let diagnostics = validate(content);

        let gm_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-001").collect();
        assert!(
            !gm_001.is_empty(),
            "GM-001 should fire for unclosed code block"
        );
        assert!(
            gm_001[0].suggestion.is_some(),
            "GM-001 should have a suggestion"
        );
        let suggestion = gm_001[0].suggestion.as_ref().unwrap();
        assert!(
            suggestion.contains("unclosed tags"),
            "GM-001 suggestion should mention 'unclosed tags', got: {}",
            suggestion
        );
    }
}
