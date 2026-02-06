//! Cross-platform validation rules
//!
//! Validates:
//! - XP-001: Claude-specific features in AGENTS.md (error)
//! - XP-002: AGENTS.md markdown structure (warning)
//! - XP-003: Hard-coded platform paths in configs (warning)

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::Validator,
    schemas::cross_platform::{
        check_markdown_structure, find_claude_specific_features, find_hard_coded_paths,
    },
};
use rust_i18n::t;
use std::path::Path;

pub struct CrossPlatformValidator;

impl Validator for CrossPlatformValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let is_agents_md = matches!(
            filename,
            "AGENTS.md" | "AGENTS.local.md" | "AGENTS.override.md"
        );

        // XP-001: Claude-specific features in AGENTS.md (ERROR)
        // Only check AGENTS.md files - CLAUDE.md is allowed to have these features
        if config.is_rule_enabled("XP-001") && is_agents_md {
            let claude_features = find_claude_specific_features(content);
            for feature in claude_features {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        feature.line,
                        feature.column,
                        "XP-001",
                        t!(
                            "rules.xp_001.message",
                            feature = feature.feature.as_str(),
                            filename = filename,
                            description = feature.description.as_str()
                        ),
                    )
                    .with_suggestion(t!("rules.xp_001.suggestion")),
                );
            }
        }

        // XP-002: AGENTS.md markdown structure (WARNING)
        // Validate that AGENTS.md has proper markdown structure
        if config.is_rule_enabled("XP-002") && is_agents_md {
            let structure_issues = check_markdown_structure(content);
            for issue in structure_issues {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        issue.line,
                        issue.column,
                        "XP-002",
                        t!(
                            "rules.xp_002.message",
                            filename = filename,
                            issue = issue.issue.as_str()
                        ),
                    )
                    .with_suggestion(issue.suggestion),
                );
            }
        }

        // XP-003: Hard-coded platform paths (WARNING)
        // Check all config files for hard-coded platform-specific paths
        if config.is_rule_enabled("XP-003") {
            let hard_coded = find_hard_coded_paths(content);
            for path_issue in hard_coded {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        path_issue.line,
                        path_issue.column,
                        "XP-003",
                        t!(
                            "rules.xp_003.message",
                            platform = path_issue.platform.as_str(),
                            path = path_issue.path.as_str()
                        ),
                    )
                    .with_suggestion(t!("rules.xp_003.suggestion")),
                );
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LintConfig, TargetTool};
    use crate::diagnostics::DiagnosticLevel;

    // ===== XP-001: Claude-Specific Features in AGENTS.md =====

    #[test]
    fn test_xp_001_hooks_in_agents_md() {
        let content = r#"# Agent Config

- type: PreToolExecution
  command: echo "test"
"#;
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());

        let xp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-001").collect();
        assert_eq!(xp_001.len(), 1);
        assert_eq!(xp_001[0].level, DiagnosticLevel::Error);
        assert!(xp_001[0].message.contains("hooks"));
    }

    #[test]
    fn test_xp_001_context_fork_in_agents_md() {
        let content = r#"---
name: test
context: fork
---
Body"#;
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());

        let xp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-001").collect();
        assert!(xp_001.iter().any(|d| d.message.contains("context:fork")));
    }

    #[test]
    fn test_xp_001_allowed_in_claude_md() {
        // Same content but in CLAUDE.md should NOT trigger XP-001
        let content = r#"---
name: test
context: fork
agent: Explore
allowed-tools: Read Write
---
Body"#;
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());

        let xp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-001").collect();
        assert!(
            xp_001.is_empty(),
            "XP-001 should not fire for CLAUDE.md files"
        );
    }

    #[test]
    fn test_xp_001_allowed_in_claude_local_md() {
        // CLAUDE.local.md should NOT trigger XP-001 (it's a Claude-specific file)
        let content = r#"---
name: test
context: fork
agent: Explore
---
Body"#;
        let validator = CrossPlatformValidator;
        let diagnostics = validator.validate(
            Path::new("CLAUDE.local.md"),
            content,
            &LintConfig::default(),
        );

        let xp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-001").collect();
        assert!(
            xp_001.is_empty(),
            "XP-001 should not fire for CLAUDE.local.md files"
        );
    }

    #[test]
    fn test_xp_001_agents_local_md() {
        // AGENTS.local.md SHOULD trigger XP-001 for Claude-specific features
        let content = r#"---
name: test
context: fork
agent: Explore
---
Body"#;
        let validator = CrossPlatformValidator;
        let diagnostics = validator.validate(
            Path::new("AGENTS.local.md"),
            content,
            &LintConfig::default(),
        );

        let xp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-001").collect();
        assert!(
            !xp_001.is_empty(),
            "XP-001 should fire for Claude-specific features in AGENTS.local.md"
        );
    }

    #[test]
    fn test_xp_001_agents_override_md() {
        // AGENTS.override.md SHOULD trigger XP-001 for Claude-specific features
        let content = r#"# Config
- type: PreToolExecution
  command: echo "test"
"#;
        let validator = CrossPlatformValidator;
        let diagnostics = validator.validate(
            Path::new("AGENTS.override.md"),
            content,
            &LintConfig::default(),
        );

        let xp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-001").collect();
        assert!(
            !xp_001.is_empty(),
            "XP-001 should fire for hooks in AGENTS.override.md"
        );
    }

    #[test]
    fn test_xp_002_agents_variants() {
        // AGENTS variants should get XP-002 for structure issues
        let content = "Just plain text without any markdown headers.";
        let validator = CrossPlatformValidator;
        let variants = ["AGENTS.local.md", "AGENTS.override.md"];

        for variant in variants {
            let diagnostics =
                validator.validate(Path::new(variant), content, &LintConfig::default());

            let xp_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-002").collect();
            assert_eq!(
                xp_002.len(),
                1,
                "XP-002 should fire for {} without headers",
                variant
            );
        }
    }

    #[test]
    fn test_xp_001_clean_agents_md() {
        let content = r#"# Project Guidelines

Follow the coding style guide.

## Commands
- npm run build
- npm run test
"#;
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());

        let xp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-001").collect();
        assert!(xp_001.is_empty());
    }

    #[test]
    fn test_xp_001_multiple_features() {
        let content = r#"---
name: test
context: fork
agent: Plan
allowed-tools: Read Write
---

# Config
- type: Stop
  command: echo
"#;
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());

        let xp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-001").collect();
        // Should detect multiple Claude-specific features
        assert!(
            xp_001.len() >= 3,
            "Expected at least 3 XP-001 errors, got {}",
            xp_001.len()
        );
    }

    // ===== XP-002: AGENTS.md Markdown Structure =====

    #[test]
    fn test_xp_002_no_headers() {
        let content = "Just plain text without any markdown headers.";
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());

        let xp_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-002").collect();
        assert_eq!(xp_002.len(), 1);
        assert_eq!(xp_002[0].level, DiagnosticLevel::Warning);
        assert!(xp_002[0].message.contains("No markdown headers"));
    }

    #[test]
    fn test_xp_002_skipped_header_level() {
        let content = r#"# Main Title

#### Skipped to h4
"#;
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());

        let xp_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-002").collect();
        assert_eq!(xp_002.len(), 1);
        assert!(xp_002[0].message.contains("skipped"));
    }

    #[test]
    fn test_xp_002_valid_structure() {
        let content = r#"# Project Memory

## Build Commands

### Testing

Run tests with npm test.
"#;
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());

        let xp_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-002").collect();
        assert!(xp_002.is_empty());
    }

    #[test]
    fn test_xp_002_not_checked_for_claude_md() {
        // XP-002 is specifically for AGENTS.md
        let content = "Plain text without headers.";
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());

        let xp_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-002").collect();
        assert!(xp_002.is_empty(), "XP-002 should not fire for CLAUDE.md");
    }

    // ===== XP-003: Hard-Coded Platform Paths =====

    #[test]
    fn test_xp_003_claude_path() {
        let content = "Check the config at .claude/settings.json";
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());

        let xp_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-003").collect();
        assert_eq!(xp_003.len(), 1);
        assert_eq!(xp_003[0].level, DiagnosticLevel::Warning);
        assert!(xp_003[0].message.contains("Claude Code"));
    }

    #[test]
    fn test_xp_003_multiple_platforms() {
        let content = r#"
# Platform Configs
- Claude: .claude/settings.json
- Cursor: .cursor/rules/
- OpenCode: .opencode/config.yaml
"#;
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());

        let xp_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-003").collect();
        assert_eq!(xp_003.len(), 3);
    }

    #[test]
    fn test_xp_003_no_platform_paths() {
        let content = r#"# Configuration

Use environment variables for all platform-specific settings.
"#;
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());

        let xp_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-003").collect();
        assert!(xp_003.is_empty());
    }

    #[test]
    fn test_xp_003_applies_to_all_files() {
        // XP-003 should check all config files, not just AGENTS.md
        let content = "Config at .claude/settings.json";
        let validator = CrossPlatformValidator;

        // Test CLAUDE.md
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());
        let xp_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-003").collect();
        assert_eq!(xp_003.len(), 1, "XP-003 should fire for CLAUDE.md too");

        // Test generic markdown (non-excluded file)
        let diagnostics =
            validator.validate(Path::new("notes/setup.md"), content, &LintConfig::default());
        let xp_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-003").collect();
        assert_eq!(xp_003.len(), 1, "XP-003 should fire for generic markdown");
    }

    // ===== Config Integration Tests =====

    #[test]
    fn test_config_disabled_cross_platform_category() {
        let mut config = LintConfig::default();
        config.rules.cross_platform = false;

        let content = r#"---
context: fork
---
Check .claude/settings.json"#;

        let validator = CrossPlatformValidator;
        let diagnostics = validator.validate(Path::new("AGENTS.md"), content, &config);

        // All XP-* rules should be disabled
        let xp_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("XP-"))
            .collect();
        assert!(xp_rules.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["XP-001".to_string()];

        let content = r#"---
context: fork
agent: Explore
---
Body"#;

        let validator = CrossPlatformValidator;
        let diagnostics = validator.validate(Path::new("AGENTS.md"), content, &config);

        // XP-001 should not fire when specifically disabled
        let xp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-001").collect();
        assert!(xp_001.is_empty());

        // XP-002 and XP-003 should still work
        assert!(config.is_rule_enabled("XP-002"));
        assert!(config.is_rule_enabled("XP-003"));
    }

    #[test]
    fn test_xp_rules_not_target_specific() {
        // XP-* rules should apply to all targets (not just Claude Code)
        let mut config = LintConfig::default();
        config.target = TargetTool::Cursor;

        // Cursor target should still have XP-* rules enabled
        assert!(config.is_rule_enabled("XP-001"));
        assert!(config.is_rule_enabled("XP-002"));
        assert!(config.is_rule_enabled("XP-003"));
    }

    #[test]
    fn test_combined_issues() {
        // Test that all three rules can fire together
        let content = r#"context: fork
Check .claude/ and .cursor/ paths"#;

        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());

        // Should have:
        // - XP-001 for context:fork
        // - XP-002 for no headers
        // - XP-003 for .claude/ and .cursor/
        let xp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-001").collect();
        let xp_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-002").collect();
        let xp_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-003").collect();

        assert!(!xp_001.is_empty(), "Expected XP-001 errors");
        assert!(!xp_002.is_empty(), "Expected XP-002 warnings");
        assert_eq!(xp_003.len(), 2, "Expected 2 XP-003 warnings");
    }

    // ===== XP-001: Section Guard Integration Tests =====

    #[test]
    fn test_xp_001_guarded_section_no_errors() {
        let content = r#"# Project AGENTS.md

## Overview
This project uses various tools.

## Claude Code Specific
- type: PreToolExecution
  command: echo "lint"

context: fork
agent: security-reviewer
allowed-tools: Read Write Bash
"#;
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());

        let xp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-001").collect();
        assert!(
            xp_001.is_empty(),
            "XP-001 should not fire for features in Claude-specific section, got {} errors",
            xp_001.len()
        );
    }

    #[test]
    fn test_xp_001_mixed_guarded_unguarded() {
        let content = r#"# AGENTS.md

## Claude Code Specific
- type: Stop
  command: cleanup

## General Configuration
agent: some-agent
"#;
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());

        let xp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-001").collect();

        assert_eq!(
            xp_001.len(),
            1,
            "Expected 1 XP-001 error for unguarded agent field"
        );
        assert!(
            xp_001[0].message.contains("agent"),
            "Error should be for 'agent' feature"
        );
    }

    #[test]
    fn test_xp_001_guard_resets_at_new_section() {
        let content = r#"# Project

## Claude Only
- type: Notification
  command: notify

## Build Commands
- type: PostToolExecution
  command: build-check
"#;
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());

        let xp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-001").collect();

        assert_eq!(
            xp_001.len(),
            1,
            "Expected 1 XP-001 error for hooks outside Claude section"
        );
    }

    // ===== Additional XP rule tests =====

    #[test]
    fn test_xp_001_claude_code_features() {
        // Test all known Claude Code-specific features
        let features = [
            "context: fork",
            "agent: reviewer",
            "allowed-tools: Read Write",
            "- type: PreToolExecution",
        ];

        for feature in features {
            let content = format!("# Project\n\n{}", feature);
            let validator = CrossPlatformValidator;
            let diagnostics =
                validator.validate(Path::new("AGENTS.md"), &content, &LintConfig::default());

            let xp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-001").collect();
            assert!(
                !xp_001.is_empty(),
                "Feature '{}' should trigger XP-001 in AGENTS.md",
                feature
            );
        }
    }

    #[test]
    fn test_xp_001_allowed_in_claude_local() {
        // Claude-specific features are allowed in CLAUDE.local.md
        let content = "# Project\n\ncontext: fork\nagent: reviewer";
        let validator = CrossPlatformValidator;
        let diagnostics = validator.validate(
            Path::new("CLAUDE.local.md"),
            content,
            &LintConfig::default(),
        );

        let xp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-001").collect();
        assert!(
            xp_001.is_empty(),
            "CLAUDE.local.md should allow Claude features"
        );
    }

    #[test]
    fn test_xp_002_valid_markdown_structure() {
        let content = r#"# Project Name

## Overview

Description here.

## Tech Stack

- Rust
- TypeScript
"#;
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());

        let xp_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-002").collect();
        assert!(xp_002.is_empty());
    }

    #[test]
    fn test_xp_003_dot_claude_dir_path() {
        // Test with .claude directory path (without tilde)
        let content = "# Project\n\nCheck the .claude/settings.json file.";
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());

        let xp_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-003").collect();
        // .claude/ path should trigger XP-003
        assert!(!xp_003.is_empty());
    }

    #[test]
    fn test_xp_003_dot_cursor_dir_path() {
        let content = "# Project\n\nSee .cursor/rules for configuration.";
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());

        let xp_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-003").collect();
        // .cursor/ path should trigger XP-003
        assert!(!xp_003.is_empty());
    }

    #[test]
    fn test_xp_003_relative_paths_ok() {
        let content = "# Project\n\nSee ./src/main.rs and ../docs/README.md.";
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());

        let xp_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-003").collect();
        assert!(xp_003.is_empty(), "Relative paths should be OK");
    }

    #[test]
    fn test_xp_003_absolute_user_paths() {
        let content = "# Project\n\n- Path: /Users/lunelson/Code/project/src\n- Linux: /home/user/.config/app";
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());

        let xp_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-003").collect();
        assert_eq!(
            xp_003.len(),
            2,
            "Should detect /Users/ and /home/ absolute paths"
        );
    }

    #[test]
    fn test_xp_003_macos_library_path() {
        let content = "# Config\n\nChrome profile: ~/Library/Application Support/Google/Chrome";
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());

        let xp_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-003").collect();
        assert_eq!(
            xp_003.len(),
            1,
            "Should detect ~/Library/ as macOS-specific"
        );
    }

    #[test]
    fn test_xp_003_tilde_hidden_dir() {
        let content = "# Config\n\nSee ~/.config/my-tool/config.yaml for settings";
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());

        let xp_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-003").collect();
        assert_eq!(
            xp_003.len(),
            1,
            "Should detect ~/.config/ as user-specific path"
        );
    }

    #[test]
    fn test_xp_001_at_import_in_agents_md() {
        let content = "# Project\n\nSee @.config/agents/rules/coding.md for coding guidelines";
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());

        let xp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-001").collect();
        assert!(
            xp_001.iter().any(|d| d.message.contains("@import")),
            "Should detect @file import as Claude-specific. Got: {:?}",
            xp_001.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_xp_001_guarded_with_claude_code_section() {
        let content = r#"# Project

## Claude Code

Use context: fork for subagents.
"#;
        let validator = CrossPlatformValidator;
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());

        let xp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-001").collect();
        assert!(
            xp_001.is_empty(),
            "Guarded features should not trigger XP-001"
        );
    }

    #[test]
    fn test_all_xp_rules_can_be_disabled() {
        let rules = ["XP-001", "XP-002", "XP-003"];

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules.disabled_rules = vec![rule.to_string()];

            // Content that could trigger each rule
            let content = r#"# Project
context: fork
/etc/hosts"#;

            let validator = CrossPlatformValidator;
            let diagnostics = validator.validate(Path::new("AGENTS.md"), content, &config);

            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }
}
