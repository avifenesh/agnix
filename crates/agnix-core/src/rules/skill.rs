//! Skill file validation

use crate::{
    config::LintConfig, diagnostics::Diagnostic, parsers::parse_frontmatter, rules::Validator,
    schemas::SkillSchema,
};
use std::path::Path;

/// Valid model values for CC-SK-001
const VALID_MODELS: &[&str] = &["sonnet", "opus", "haiku", "inherit"];

/// Valid agent types for CC-SK-005
const VALID_AGENTS: &[&str] = &["Explore", "Plan", "general-purpose"];

/// Known Claude Code tools for CC-SK-008
const KNOWN_TOOLS: &[&str] = &[
    "Bash",
    "Read",
    "Write",
    "Edit",
    "Grep",
    "Glob",
    "Task",
    "WebFetch",
    "AskUserQuestion",
    "TodoRead",
    "TodoWrite",
    "MultiTool",
];

/// Maximum dynamic injections for CC-SK-009
const MAX_INJECTIONS: usize = 3;

pub struct SkillValidator;

impl Validator for SkillValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if !config.rules.frontmatter_validation {
            return diagnostics;
        }

        // Parse frontmatter
        let result: Result<(SkillSchema, String), _> = parse_frontmatter(content);

        match result {
            Ok((schema, body)) => {
                // Run schema validations
                let errors = schema.validate();
                for error in errors {
                    diagnostics.push(Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "skill::schema",
                        error,
                    ));
                }

                // AS-005: Name cannot start or end with hyphen
                if config.is_rule_enabled("AS-005")
                    && (schema.name.starts_with('-') || schema.name.ends_with('-'))
                {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "AS-005",
                            format!("Name '{}' cannot start or end with hyphen", schema.name),
                        )
                        .with_suggestion(
                            "Remove leading/trailing hyphens from the name".to_string(),
                        ),
                    );
                }

                // AS-006: Name cannot contain consecutive hyphens
                if config.is_rule_enabled("AS-006") && schema.name.contains("--") {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "AS-006",
                            format!("Name '{}' cannot contain consecutive hyphens", schema.name),
                        )
                        .with_suggestion("Replace '--' with '-' in the name".to_string()),
                    );
                }

                // AS-010: Description should include trigger phrase
                if config.is_rule_enabled("AS-010") {
                    let desc_lower = schema.description.to_lowercase();
                    if !desc_lower.contains("use when") && !desc_lower.contains("use this") {
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                1,
                                0,
                                "AS-010",
                                "Description should include a 'Use when...' trigger phrase"
                                    .to_string(),
                            )
                            .with_suggestion(
                                "Add 'Use when [condition]' to help Claude understand when to invoke this skill".to_string(),
                            ),
                        );
                    }
                }

                // CC-SK-006: Dangerous auto-invocation check
                if config.is_rule_enabled("CC-SK-006") {
                    const DANGEROUS_NAMES: &[&str] =
                        &["deploy", "ship", "publish", "delete", "release", "push"];
                    let name_lower = schema.name.to_lowercase();
                    if DANGEROUS_NAMES.iter().any(|d| name_lower.contains(d))
                        && !schema.disable_model_invocation.unwrap_or(false)
                    {
                        diagnostics.push(Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
                                "CC-SK-006",
                                format!(
                                    "Dangerous skill '{}' must set 'disable-model-invocation: true' to prevent accidental invocation",
                                    schema.name
                                ),
                            ).with_suggestion("Add 'disable-model-invocation: true' to the frontmatter".to_string()));
                    }
                }

                // CC-SK-007: Unrestricted Bash warning
                if config.is_rule_enabled("CC-SK-007") {
                    if let Some(tools) = &schema.allowed_tools {
                        // Parse space-delimited tool list
                        let tool_list: Vec<&str> = tools.split_whitespace().collect();
                        for tool in tool_list {
                            if tool == "Bash" {
                                diagnostics.push(Diagnostic::warning(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "CC-SK-007",
                                    "Unrestricted Bash access detected. Consider using scoped version for better security.".to_string(),
                                ).with_suggestion("Use scoped Bash like 'Bash(git:*)' or 'Bash(npm:*)' instead of plain 'Bash'".to_string()));
                            }
                        }
                    }
                }

                // CC-SK-001: Invalid model value
                if config.is_rule_enabled("CC-SK-001") {
                    if let Some(model) = &schema.model {
                        if !VALID_MODELS.contains(&model.as_str()) {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "CC-SK-001",
                                    format!(
                                        "Invalid model '{}'. Must be one of: {}",
                                        model,
                                        VALID_MODELS.join(", ")
                                    ),
                                )
                                .with_suggestion(format!(
                                    "Use one of the valid model values: {}",
                                    VALID_MODELS.join(", ")
                                )),
                            );
                        }
                    }
                }

                // CC-SK-002: Invalid context value
                if config.is_rule_enabled("CC-SK-002") {
                    if let Some(context) = &schema.context {
                        if context != "fork" {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "CC-SK-002",
                                    format!(
                                        "Invalid context '{}'. Must be 'fork' or omitted",
                                        context
                                    ),
                                )
                                .with_suggestion(
                                    "Set context to 'fork' or remove the field entirely"
                                        .to_string(),
                                ),
                            );
                        }
                    }
                }

                // CC-SK-003: Context without agent
                if config.is_rule_enabled("CC-SK-003")
                    && schema.context.as_deref() == Some("fork")
                    && schema.agent.is_none()
                {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CC-SK-003",
                            "Context 'fork' requires an 'agent' field".to_string(),
                        )
                        .with_suggestion(
                            "Add 'agent: general-purpose' or another valid agent type".to_string(),
                        ),
                    );
                }

                // CC-SK-004: Agent without context
                if config.is_rule_enabled("CC-SK-004")
                    && schema.agent.is_some()
                    && schema.context.as_deref() != Some("fork")
                {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CC-SK-004",
                            "Agent field requires 'context: fork'".to_string(),
                        )
                        .with_suggestion("Add 'context: fork' to the frontmatter".to_string()),
                    );
                }

                // CC-SK-005: Invalid agent type
                if config.is_rule_enabled("CC-SK-005") {
                    if let Some(agent) = &schema.agent {
                        if !VALID_AGENTS.contains(&agent.as_str()) {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "CC-SK-005",
                                    format!(
                                        "Invalid agent type '{}'. Must be one of: {}",
                                        agent,
                                        VALID_AGENTS.join(", ")
                                    ),
                                )
                                .with_suggestion(format!(
                                    "Use one of the valid agent types: {}",
                                    VALID_AGENTS.join(", ")
                                )),
                            );
                        }
                    }
                }

                // CC-SK-008: Unknown tool name
                if config.is_rule_enabled("CC-SK-008") {
                    if let Some(tools) = &schema.allowed_tools {
                        let tool_list: Vec<&str> = tools.split_whitespace().collect();
                        for tool in tool_list {
                            // Extract base tool name (before parentheses for scoped tools)
                            let base_name = tool.split('(').next().unwrap_or(tool);
                            if !KNOWN_TOOLS.contains(&base_name) {
                                diagnostics.push(
                                    Diagnostic::error(
                                        path.to_path_buf(),
                                        1,
                                        0,
                                        "CC-SK-008",
                                        format!(
                                            "Unknown tool '{}'. Known tools: {}",
                                            base_name,
                                            KNOWN_TOOLS.join(", ")
                                        ),
                                    )
                                    .with_suggestion(format!(
                                        "Use one of the known Claude Code tools: {}",
                                        KNOWN_TOOLS.join(", ")
                                    )),
                                );
                            }
                        }
                    }
                }

                // CC-SK-009: Too many injections (warning)
                if config.is_rule_enabled("CC-SK-009") {
                    let injection_count = body.matches("!`").count();
                    if injection_count > MAX_INJECTIONS {
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                1,
                                0,
                                "CC-SK-009",
                                format!(
                                    "Too many dynamic injections ({}). Limit to {} for better performance",
                                    injection_count, MAX_INJECTIONS
                                ),
                            )
                            .with_suggestion(
                                "Consider moving complex logic to a scripts/ directory or reducing injections".to_string(),
                            ),
                        );
                    }
                }
            }
            Err(e) => {
                diagnostics.push(Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "skill::parse",
                    format!("Failed to parse SKILL.md: {}", e),
                ));
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;

    #[test]
    fn test_valid_skill() {
        let content = r#"---
name: test-skill
description: Use when testing skill validation
---
Skill body content"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_invalid_skill_name() {
        let content = r#"---
name: Test-Skill
description: A test skill
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn test_cc_sk_006_dangerous_name_without_safety() {
        let content = r#"---
name: deploy-prod
description: Deploys to production
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        // Should have an error for CC-SK-006
        let cc_sk_006_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-006")
            .collect();

        assert_eq!(cc_sk_006_errors.len(), 1);
        assert_eq!(
            cc_sk_006_errors[0].level,
            crate::diagnostics::DiagnosticLevel::Error
        );
    }

    #[test]
    fn test_cc_sk_006_dangerous_name_with_safety() {
        let content = r#"---
name: deploy-prod
description: Deploys to production
disable-model-invocation: true
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        // Should NOT have an error for CC-SK-006
        let cc_sk_006_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-006")
            .collect();

        assert_eq!(cc_sk_006_errors.len(), 0);
    }

    #[test]
    fn test_cc_sk_006_covers_all_dangerous_names() {
        let dangerous_names = vec!["deploy", "ship", "publish", "delete", "release", "push"];

        for name in dangerous_names {
            let content = format!(
                r#"---
name: {}-prod
description: A dangerous skill
---
Body"#,
                name
            );

            let validator = SkillValidator;
            let diagnostics =
                validator.validate(Path::new("test.md"), &content, &LintConfig::default());

            // Should have an error for CC-SK-006
            let cc_sk_006_errors: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-SK-006")
                .collect();

            assert_eq!(
                cc_sk_006_errors.len(),
                1,
                "Expected CC-SK-006 error for name: {}",
                name
            );
        }
    }

    #[test]
    fn test_cc_sk_007_unrestricted_bash() {
        let content = r#"---
name: git-helper
description: Git operations helper
allowed-tools: Bash Read Write
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        // Should have a warning for CC-SK-007
        let cc_sk_007_warnings: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-007")
            .collect();

        assert_eq!(cc_sk_007_warnings.len(), 1);
        assert_eq!(
            cc_sk_007_warnings[0].level,
            crate::diagnostics::DiagnosticLevel::Warning
        );
    }

    #[test]
    fn test_cc_sk_007_scoped_bash_ok() {
        let content = r#"---
name: git-helper
description: Git operations helper
allowed-tools: Bash(git:*) Read Write
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        // Should NOT have a warning for CC-SK-007 (scoped Bash is ok)
        let cc_sk_007_warnings: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-007")
            .collect();

        assert_eq!(cc_sk_007_warnings.len(), 0);
    }

    #[test]
    fn test_cc_sk_007_no_bash() {
        let content = r#"---
name: reader
description: File reader
allowed-tools: Read Write
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        // Should NOT have a warning for CC-SK-007 (no Bash at all)
        let cc_sk_007_warnings: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-007")
            .collect();

        assert_eq!(cc_sk_007_warnings.len(), 0);
    }

    #[test]
    fn test_as_005_leading_hyphen() {
        let content = r#"---
name: -bad-name
description: Use when testing validation
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_005_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-005").collect();

        assert_eq!(as_005_errors.len(), 1);
        assert_eq!(
            as_005_errors[0].level,
            crate::diagnostics::DiagnosticLevel::Error
        );
    }

    #[test]
    fn test_as_005_trailing_hyphen() {
        let content = r#"---
name: bad-name-
description: Use when testing validation
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_005_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-005").collect();

        assert_eq!(as_005_errors.len(), 1);
        assert_eq!(
            as_005_errors[0].level,
            crate::diagnostics::DiagnosticLevel::Error
        );
    }

    #[test]
    fn test_as_006_consecutive_hyphens() {
        let content = r#"---
name: bad--name
description: Use when testing validation
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_006_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-006").collect();

        assert_eq!(as_006_errors.len(), 1);
        assert_eq!(
            as_006_errors[0].level,
            crate::diagnostics::DiagnosticLevel::Error
        );
    }

    #[test]
    fn test_as_010_missing_trigger() {
        let content = r#"---
name: code-review
description: Reviews code for quality
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_010_warnings: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();

        assert_eq!(as_010_warnings.len(), 1);
        assert_eq!(
            as_010_warnings[0].level,
            crate::diagnostics::DiagnosticLevel::Warning
        );
    }

    #[test]
    fn test_as_010_has_use_when_trigger() {
        let content = r#"---
name: code-review
description: Use when user asks for code review
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_010_warnings: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();

        assert_eq!(as_010_warnings.len(), 0);
    }

    #[test]
    fn test_as_010_has_use_this_trigger() {
        let content = r#"---
name: code-review
description: Use this skill to review code
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_010_warnings: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();

        assert_eq!(as_010_warnings.len(), 0);
    }

    // ===== CC-SK-001: Invalid Model Value =====

    #[test]
    fn test_cc_sk_001_invalid_model() {
        let content = r#"---
name: test-skill
description: Use when testing
model: gpt-4
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-001")
            .collect();

        assert_eq!(cc_sk_001.len(), 1);
        assert_eq!(
            cc_sk_001[0].level,
            crate::diagnostics::DiagnosticLevel::Error
        );
        assert!(cc_sk_001[0].message.contains("gpt-4"));
    }

    #[test]
    fn test_cc_sk_001_valid_models() {
        for model in &["sonnet", "opus", "haiku", "inherit"] {
            let content = format!(
                r#"---
name: test-skill
description: Use when testing
model: {}
---
Body"#,
                model
            );

            let validator = SkillValidator;
            let diagnostics =
                validator.validate(Path::new("test.md"), &content, &LintConfig::default());

            let cc_sk_001: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-SK-001")
                .collect();

            assert_eq!(cc_sk_001.len(), 0, "Model '{}' should be valid", model);
        }
    }

    #[test]
    fn test_cc_sk_001_no_model_ok() {
        let content = r#"---
name: test-skill
description: Use when testing
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-001")
            .collect();

        assert_eq!(cc_sk_001.len(), 0);
    }

    // ===== CC-SK-002: Invalid Context Value =====

    #[test]
    fn test_cc_sk_002_invalid_context() {
        let content = r#"---
name: test-skill
description: Use when testing
context: split
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-002")
            .collect();

        assert_eq!(cc_sk_002.len(), 1);
        assert_eq!(
            cc_sk_002[0].level,
            crate::diagnostics::DiagnosticLevel::Error
        );
        assert!(cc_sk_002[0].message.contains("split"));
    }

    #[test]
    fn test_cc_sk_002_valid_context_fork() {
        let content = r#"---
name: test-skill
description: Use when testing
context: fork
agent: general-purpose
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-002")
            .collect();

        assert_eq!(cc_sk_002.len(), 0);
    }

    #[test]
    fn test_cc_sk_002_no_context_ok() {
        let content = r#"---
name: test-skill
description: Use when testing
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-002")
            .collect();

        assert_eq!(cc_sk_002.len(), 0);
    }

    // ===== CC-SK-003: Context Without Agent =====

    #[test]
    fn test_cc_sk_003_context_fork_without_agent() {
        let content = r#"---
name: test-skill
description: Use when testing
context: fork
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-003")
            .collect();

        assert_eq!(cc_sk_003.len(), 1);
        assert_eq!(
            cc_sk_003[0].level,
            crate::diagnostics::DiagnosticLevel::Error
        );
    }

    #[test]
    fn test_cc_sk_003_context_fork_with_agent_ok() {
        let content = r#"---
name: test-skill
description: Use when testing
context: fork
agent: Explore
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-003")
            .collect();

        assert_eq!(cc_sk_003.len(), 0);
    }

    // ===== CC-SK-004: Agent Without Context =====

    #[test]
    fn test_cc_sk_004_agent_without_context() {
        let content = r#"---
name: test-skill
description: Use when testing
agent: Explore
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-004")
            .collect();

        assert_eq!(cc_sk_004.len(), 1);
        assert_eq!(
            cc_sk_004[0].level,
            crate::diagnostics::DiagnosticLevel::Error
        );
    }

    #[test]
    fn test_cc_sk_004_agent_with_context_ok() {
        let content = r#"---
name: test-skill
description: Use when testing
context: fork
agent: Explore
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-004")
            .collect();

        assert_eq!(cc_sk_004.len(), 0);
    }

    #[test]
    fn test_cc_sk_004_no_agent_no_context_ok() {
        let content = r#"---
name: test-skill
description: Use when testing
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-004")
            .collect();

        assert_eq!(cc_sk_004.len(), 0);
    }

    // ===== CC-SK-005: Invalid Agent Type =====

    #[test]
    fn test_cc_sk_005_invalid_agent() {
        let content = r#"---
name: test-skill
description: Use when testing
context: fork
agent: CustomAgent
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-005")
            .collect();

        assert_eq!(cc_sk_005.len(), 1);
        assert_eq!(
            cc_sk_005[0].level,
            crate::diagnostics::DiagnosticLevel::Error
        );
        assert!(cc_sk_005[0].message.contains("CustomAgent"));
    }

    #[test]
    fn test_cc_sk_005_valid_agents() {
        for agent in &["Explore", "Plan", "general-purpose"] {
            let content = format!(
                r#"---
name: test-skill
description: Use when testing
context: fork
agent: {}
---
Body"#,
                agent
            );

            let validator = SkillValidator;
            let diagnostics =
                validator.validate(Path::new("test.md"), &content, &LintConfig::default());

            let cc_sk_005: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-SK-005")
                .collect();

            assert_eq!(cc_sk_005.len(), 0, "Agent '{}' should be valid", agent);
        }
    }

    // ===== CC-SK-008: Unknown Tool Name =====

    #[test]
    fn test_cc_sk_008_unknown_tool() {
        let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Read Write UnknownTool
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-008")
            .collect();

        assert_eq!(cc_sk_008.len(), 1);
        assert_eq!(
            cc_sk_008[0].level,
            crate::diagnostics::DiagnosticLevel::Error
        );
        assert!(cc_sk_008[0].message.contains("UnknownTool"));
    }

    #[test]
    fn test_cc_sk_008_all_known_tools_ok() {
        let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Bash Read Write Edit Grep Glob Task
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-008")
            .collect();

        assert_eq!(cc_sk_008.len(), 0);
    }

    #[test]
    fn test_cc_sk_008_scoped_tool_extracts_base_name() {
        let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Bash(git:*) Read Write
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-008")
            .collect();

        assert_eq!(cc_sk_008.len(), 0);
    }

    #[test]
    fn test_cc_sk_008_multiple_unknown_tools() {
        let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: FakeTool1 Read FakeTool2
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-008")
            .collect();

        assert_eq!(cc_sk_008.len(), 2);
    }

    #[test]
    fn test_cc_sk_008_scoped_unknown_tool() {
        let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: FakeTool(scope:*) Read
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-008")
            .collect();

        assert_eq!(
            cc_sk_008.len(),
            1,
            "Should detect FakeTool as unknown even when scoped"
        );
        assert!(cc_sk_008[0].message.contains("FakeTool"));
    }

    // ===== CC-SK-009: Too Many Injections =====

    #[test]
    fn test_cc_sk_009_too_many_injections() {
        let content = r#"---
name: test-skill
description: Use when testing
---
Current date: !`date`
Git status: !`git status`
Branch: !`git branch`
User: !`whoami`
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-009")
            .collect();

        assert_eq!(cc_sk_009.len(), 1);
        assert_eq!(
            cc_sk_009[0].level,
            crate::diagnostics::DiagnosticLevel::Warning
        );
        assert!(cc_sk_009[0].message.contains("4"));
    }

    #[test]
    fn test_cc_sk_009_exactly_three_injections_ok() {
        let content = r#"---
name: test-skill
description: Use when testing
---
Date: !`date`
Status: !`git status`
Branch: !`git branch`
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-009")
            .collect();

        assert_eq!(cc_sk_009.len(), 0);
    }

    #[test]
    fn test_cc_sk_009_no_injections_ok() {
        let content = r#"---
name: test-skill
description: Use when testing
---
No dynamic injections here.
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-009")
            .collect();

        assert_eq!(cc_sk_009.len(), 0);
    }

    // ===== Config Wiring Tests =====

    #[test]
    fn test_config_disabled_skills_category() {
        let mut config = LintConfig::default();
        config.rules.skills = false;

        let content = r#"---
name: -bad-name
description: Missing trigger phrase
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &config);

        // AS-005 and AS-010 should not fire when skills category is disabled
        let skill_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("AS-") || d.rule.starts_with("CC-SK-"))
            .collect();
        assert_eq!(skill_rules.len(), 0);
    }

    #[test]
    fn test_config_disabled_specific_skill_rule() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["AS-005".to_string()];

        let content = r#"---
name: -bad-name
description: Missing trigger phrase
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &config);

        // AS-005 should not fire when specifically disabled
        let as_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-005").collect();
        assert_eq!(as_005.len(), 0);

        // But AS-010 should still fire
        let as_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();
        assert_eq!(as_010.len(), 1);
    }

    #[test]
    fn test_config_cursor_target_disables_cc_sk_rules() {
        use crate::config::TargetTool;

        let mut config = LintConfig::default();
        config.target = TargetTool::Cursor;

        let content = r#"---
name: deploy-prod
description: Deploys to production
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &config);

        // CC-SK-006 should not fire for Cursor target
        let cc_sk_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-006")
            .collect();
        assert_eq!(cc_sk_006.len(), 0);

        // But AS-010 should still fire (it's not CC- prefix)
        let as_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();
        assert_eq!(as_010.len(), 1);
    }

    #[test]
    fn test_config_claude_code_target_enables_cc_sk_rules() {
        use crate::config::TargetTool;

        let mut config = LintConfig::default();
        config.target = TargetTool::ClaudeCode;

        let content = r#"---
name: deploy-prod
description: Use when deploying to production
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &config);

        // CC-SK-006 should fire for ClaudeCode target
        let cc_sk_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-006")
            .collect();
        assert_eq!(cc_sk_006.len(), 1);
    }
}
