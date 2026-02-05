//! Agent file validation (CC-AG-001 to CC-AG-006)
//!
//! Validates Claude Code subagent definitions in `.claude/agents/*.md`

use crate::{
    config::LintConfig, diagnostics::Diagnostic, fs::FileSystem,
    parsers::frontmatter::parse_frontmatter, rules::Validator, schemas::agent::AgentSchema,
};
use std::collections::HashSet;
use std::path::Path;

/// Valid model values per CC-AG-003
const VALID_MODELS: &[&str] = &["sonnet", "opus", "haiku", "inherit"];

/// Valid permission modes per CC-AG-004
const VALID_PERMISSION_MODES: &[&str] = &[
    "default",
    "acceptEdits",
    "dontAsk",
    "bypassPermissions",
    "plan",
];

pub struct AgentValidator;

/// Maximum directory traversal depth to prevent unbounded filesystem walking
const MAX_TRAVERSAL_DEPTH: usize = 10;

impl AgentValidator {
    /// Find the project root by looking for .claude directory.
    /// Limited to MAX_TRAVERSAL_DEPTH levels to prevent unbounded traversal.
    fn find_project_root<'a>(path: &'a Path, fs: &dyn FileSystem) -> Option<&'a Path> {
        let mut current = path.parent();
        let mut depth = 0;
        while let Some(dir) = current {
            if depth >= MAX_TRAVERSAL_DEPTH {
                break;
            }
            if fs.exists(&dir.join(".claude")) {
                return Some(dir);
            }
            // Also check if we're inside .claude
            if dir.file_name().map(|n| n == ".claude").unwrap_or(false) {
                return dir.parent();
            }
            current = dir.parent();
            depth += 1;
        }
        // No fallback - return None if .claude directory not found
        None
    }

    /// Validate skill name to prevent path traversal attacks.
    /// Returns true if the name is safe (alphanumeric, hyphens, underscores only).
    fn is_safe_skill_name(name: &str) -> bool {
        !name.is_empty()
            && !name.contains('/')
            && !name.contains('\\')
            && !name.contains("..")
            && !name.starts_with('.')
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    }

    /// Check if a skill exists at the expected location.
    /// Returns false for invalid skill names (path traversal attempts).
    fn skill_exists(project_root: &Path, skill_name: &str, fs: &dyn FileSystem) -> bool {
        if !Self::is_safe_skill_name(skill_name) {
            return false;
        }
        let skill_path = project_root
            .join(".claude")
            .join("skills")
            .join(skill_name)
            .join("SKILL.md");
        fs.exists(&skill_path)
    }
}

impl Validator for AgentValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Check if content has frontmatter
        if !content.trim_start().starts_with("---") {
            if config.is_rule_enabled("CC-AG-007") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-AG-007",
                        "Agent file must have YAML frontmatter".to_string(),
                    )
                    .with_suggestion("Add frontmatter between --- markers".to_string()),
                );
            }
            return diagnostics;
        }

        // Parse frontmatter
        let schema: AgentSchema = match parse_frontmatter(content) {
            Ok((s, _body)) => s,
            Err(e) => {
                if config.is_rule_enabled("CC-AG-007") {
                    diagnostics.push(Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-AG-007",
                        format!("Failed to parse agent frontmatter: {}", e),
                    ));
                }
                return diagnostics;
            }
        };

        // CC-AG-001: Missing name field
        if config.is_rule_enabled("CC-AG-001")
            && schema.name.as_deref().unwrap_or("").trim().is_empty()
        {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "CC-AG-001",
                    "Agent frontmatter is missing required 'name' field".to_string(),
                )
                .with_suggestion("Add 'name: your-agent-name' to frontmatter".to_string()),
            );
        }

        // CC-AG-002: Missing description field
        if config.is_rule_enabled("CC-AG-002")
            && schema
                .description
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
        {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "CC-AG-002",
                    "Agent frontmatter is missing required 'description' field".to_string(),
                )
                .with_suggestion(
                    "Add 'description: Describe what this agent does' to frontmatter".to_string(),
                ),
            );
        }

        // CC-AG-003: Invalid model value
        if config.is_rule_enabled("CC-AG-003") {
            if let Some(model) = &schema.model {
                if !VALID_MODELS.contains(&model.as_str()) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CC-AG-003",
                            format!(
                                "Invalid model '{}'. Valid values: {}",
                                model,
                                VALID_MODELS.join(", ")
                            ),
                        )
                        .with_suggestion(format!(
                            "Change model to one of: {}",
                            VALID_MODELS.join(", ")
                        )),
                    );
                }
            }
        }

        // CC-AG-004: Invalid permission mode
        if config.is_rule_enabled("CC-AG-004") {
            if let Some(mode) = &schema.permission_mode {
                if !VALID_PERMISSION_MODES.contains(&mode.as_str()) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CC-AG-004",
                            format!(
                                "Invalid permissionMode '{}'. Valid values: {}",
                                mode,
                                VALID_PERMISSION_MODES.join(", ")
                            ),
                        )
                        .with_suggestion(format!(
                            "Change permissionMode to one of: {}",
                            VALID_PERMISSION_MODES.join(", ")
                        )),
                    );
                }
            }
        }

        // CC-AG-005: Referenced skill not found
        if config.is_rule_enabled("CC-AG-005") {
            if let Some(skills) = &schema.skills {
                let fs = config.fs();
                if let Some(project_root) = Self::find_project_root(path, fs.as_ref()) {
                    for skill_name in skills {
                        if !Self::skill_exists(project_root, skill_name, fs.as_ref()) {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "CC-AG-005",
                                    format!(
                                        "Referenced skill '{}' not found at .claude/skills/{}/SKILL.md",
                                        skill_name, skill_name
                                    ),
                                )
                                .with_suggestion(format!(
                                    "Create the skill at .claude/skills/{}/SKILL.md or remove the reference",
                                    skill_name
                                )),
                            );
                        }
                    }
                }
            }
        }

        // CC-AG-006: Tool/disallowed conflict
        if config.is_rule_enabled("CC-AG-006") {
            if let (Some(tools), Some(disallowed)) = (&schema.tools, &schema.disallowed_tools) {
                let tools_set: HashSet<&str> = tools.iter().map(|s| s.as_str()).collect();
                let disallowed_set: HashSet<&str> = disallowed.iter().map(|s| s.as_str()).collect();

                let conflicts: Vec<&str> =
                    tools_set.intersection(&disallowed_set).copied().collect();

                if !conflicts.is_empty() {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CC-AG-006",
                            format!(
                                "Tool(s) appear in both 'tools' and 'disallowedTools': {}",
                                conflicts.join(", ")
                            ),
                        )
                        .with_suggestion(
                            "Remove conflicting tool(s) from either 'tools' or 'disallowedTools'"
                                .to_string(),
                        ),
                    );
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;
    use tempfile::TempDir;

    fn validate(content: &str) -> Vec<Diagnostic> {
        let validator = AgentValidator;
        validator.validate(
            Path::new("agents/test-agent.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_with_path(path: &Path, content: &str) -> Vec<Diagnostic> {
        let validator = AgentValidator;
        validator.validate(path, content, &LintConfig::default())
    }

    // ===== CC-AG-001 Tests: Missing Name Field =====

    #[test]
    fn test_cc_ag_001_missing_name() {
        let content = r#"---
description: A test agent
---
Agent instructions here"#;

        let diagnostics = validate(content);
        let cc_ag_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-001")
            .collect();

        assert_eq!(cc_ag_001.len(), 1);
        assert_eq!(cc_ag_001[0].level, DiagnosticLevel::Error);
        assert!(cc_ag_001[0].message.contains("missing required 'name'"));
    }

    #[test]
    fn test_cc_ag_001_empty_name() {
        let content = r#"---
name: ""
description: A test agent
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-001")
            .collect();

        assert_eq!(cc_ag_001.len(), 1);
    }

    #[test]
    fn test_cc_ag_001_whitespace_name() {
        let content = r#"---
name: "   "
description: A test agent
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-001")
            .collect();

        assert_eq!(cc_ag_001.len(), 1);
    }

    #[test]
    fn test_cc_ag_001_valid_name() {
        let content = r#"---
name: my-agent
description: A test agent
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-001")
            .collect();

        assert_eq!(cc_ag_001.len(), 0);
    }

    // ===== CC-AG-002 Tests: Missing Description Field =====

    #[test]
    fn test_cc_ag_002_missing_description() {
        let content = r#"---
name: my-agent
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-002")
            .collect();

        assert_eq!(cc_ag_002.len(), 1);
        assert_eq!(cc_ag_002[0].level, DiagnosticLevel::Error);
        assert!(cc_ag_002[0]
            .message
            .contains("missing required 'description'"));
    }

    #[test]
    fn test_cc_ag_002_empty_description() {
        let content = r#"---
name: my-agent
description: ""
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-002")
            .collect();

        assert_eq!(cc_ag_002.len(), 1);
    }

    #[test]
    fn test_cc_ag_002_valid_description() {
        let content = r#"---
name: my-agent
description: This agent helps with testing
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-002")
            .collect();

        assert_eq!(cc_ag_002.len(), 0);
    }

    // ===== CC-AG-003 Tests: Invalid Model Value =====

    #[test]
    fn test_cc_ag_003_invalid_model() {
        let content = r#"---
name: my-agent
description: A test agent
model: gpt-4
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-003")
            .collect();

        assert_eq!(cc_ag_003.len(), 1);
        assert_eq!(cc_ag_003[0].level, DiagnosticLevel::Error);
        assert!(cc_ag_003[0].message.contains("Invalid model"));
        assert!(cc_ag_003[0].message.contains("gpt-4"));
    }

    #[test]
    fn test_cc_ag_003_valid_model_sonnet() {
        let content = r#"---
name: my-agent
description: A test agent
model: sonnet
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-003")
            .collect();

        assert_eq!(cc_ag_003.len(), 0);
    }

    #[test]
    fn test_cc_ag_003_valid_model_opus() {
        let content = r#"---
name: my-agent
description: A test agent
model: opus
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-003")
            .collect();

        assert_eq!(cc_ag_003.len(), 0);
    }

    #[test]
    fn test_cc_ag_003_valid_model_haiku() {
        let content = r#"---
name: my-agent
description: A test agent
model: haiku
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-003")
            .collect();

        assert_eq!(cc_ag_003.len(), 0);
    }

    #[test]
    fn test_cc_ag_003_valid_model_inherit() {
        let content = r#"---
name: my-agent
description: A test agent
model: inherit
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-003")
            .collect();

        assert_eq!(cc_ag_003.len(), 0);
    }

    #[test]
    fn test_cc_ag_003_no_model_ok() {
        let content = r#"---
name: my-agent
description: A test agent
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-003")
            .collect();

        assert_eq!(cc_ag_003.len(), 0);
    }

    // ===== CC-AG-004 Tests: Invalid Permission Mode =====

    #[test]
    fn test_cc_ag_004_invalid_permission_mode() {
        let content = r#"---
name: my-agent
description: A test agent
permissionMode: admin
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-004")
            .collect();

        assert_eq!(cc_ag_004.len(), 1);
        assert_eq!(cc_ag_004[0].level, DiagnosticLevel::Error);
        assert!(cc_ag_004[0].message.contains("Invalid permissionMode"));
        assert!(cc_ag_004[0].message.contains("admin"));
    }

    #[test]
    fn test_cc_ag_004_valid_permission_mode_default() {
        let content = r#"---
name: my-agent
description: A test agent
permissionMode: default
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-004")
            .collect();

        assert_eq!(cc_ag_004.len(), 0);
    }

    #[test]
    fn test_cc_ag_004_valid_permission_mode_accept_edits() {
        let content = r#"---
name: my-agent
description: A test agent
permissionMode: acceptEdits
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-004")
            .collect();

        assert_eq!(cc_ag_004.len(), 0);
    }

    #[test]
    fn test_cc_ag_004_valid_permission_mode_dont_ask() {
        let content = r#"---
name: my-agent
description: A test agent
permissionMode: dontAsk
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-004")
            .collect();

        assert_eq!(cc_ag_004.len(), 0);
    }

    #[test]
    fn test_cc_ag_004_valid_permission_mode_bypass() {
        let content = r#"---
name: my-agent
description: A test agent
permissionMode: bypassPermissions
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-004")
            .collect();

        assert_eq!(cc_ag_004.len(), 0);
    }

    #[test]
    fn test_cc_ag_004_valid_permission_mode_plan() {
        let content = r#"---
name: my-agent
description: A test agent
permissionMode: plan
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-004")
            .collect();

        assert_eq!(cc_ag_004.len(), 0);
    }

    #[test]
    fn test_cc_ag_004_no_permission_mode_ok() {
        let content = r#"---
name: my-agent
description: A test agent
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-004")
            .collect();

        assert_eq!(cc_ag_004.len(), 0);
    }

    // ===== CC-AG-005 Tests: Referenced Skill Not Found =====

    #[test]
    fn test_cc_ag_005_missing_skill() {
        let temp = TempDir::new().unwrap();
        let claude_dir = temp.path().join(".claude");
        let agents_dir = claude_dir.join("agents");
        std::fs::create_dir_all(&agents_dir).unwrap();

        let agent_path = agents_dir.join("test-agent.md");

        let content = r#"---
name: my-agent
description: A test agent
skills:
  - nonexistent-skill
---
Agent instructions"#;

        let diagnostics = validate_with_path(&agent_path, content);
        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();

        assert_eq!(cc_ag_005.len(), 1);
        assert_eq!(cc_ag_005[0].level, DiagnosticLevel::Error);
        assert!(cc_ag_005[0].message.contains("nonexistent-skill"));
        assert!(cc_ag_005[0].message.contains("not found"));
    }

    #[test]
    fn test_cc_ag_005_existing_skill() {
        let temp = TempDir::new().unwrap();
        let claude_dir = temp.path().join(".claude");
        let agents_dir = claude_dir.join("agents");
        let skills_dir = claude_dir.join("skills").join("my-skill");
        std::fs::create_dir_all(&agents_dir).unwrap();
        std::fs::create_dir_all(&skills_dir).unwrap();
        std::fs::write(
            skills_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: A skill\n---\nBody",
        )
        .unwrap();

        let agent_path = agents_dir.join("test-agent.md");

        let content = r#"---
name: my-agent
description: A test agent
skills:
  - my-skill
---
Agent instructions"#;

        let diagnostics = validate_with_path(&agent_path, content);
        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();

        assert_eq!(cc_ag_005.len(), 0);
    }

    #[test]
    fn test_cc_ag_005_multiple_missing_skills() {
        let temp = TempDir::new().unwrap();
        let claude_dir = temp.path().join(".claude");
        let agents_dir = claude_dir.join("agents");
        std::fs::create_dir_all(&agents_dir).unwrap();

        let agent_path = agents_dir.join("test-agent.md");

        let content = r#"---
name: my-agent
description: A test agent
skills:
  - missing-one
  - missing-two
  - missing-three
---
Agent instructions"#;

        let diagnostics = validate_with_path(&agent_path, content);
        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();

        assert_eq!(cc_ag_005.len(), 3);
    }

    #[test]
    fn test_cc_ag_005_no_skills_ok() {
        let content = r#"---
name: my-agent
description: A test agent
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();

        assert_eq!(cc_ag_005.len(), 0);
    }

    // ===== CC-AG-006 Tests: Tool/Disallowed Conflict =====

    #[test]
    fn test_cc_ag_006_tool_conflict() {
        let content = r#"---
name: my-agent
description: A test agent
tools:
  - Bash
  - Read
  - Write
disallowedTools:
  - Bash
  - Edit
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-006")
            .collect();

        assert_eq!(cc_ag_006.len(), 1);
        assert_eq!(cc_ag_006[0].level, DiagnosticLevel::Error);
        assert!(cc_ag_006[0].message.contains("Bash"));
        assert!(cc_ag_006[0].message.contains("both"));
    }

    #[test]
    fn test_cc_ag_006_multiple_conflicts() {
        let content = r#"---
name: my-agent
description: A test agent
tools:
  - Bash
  - Read
  - Write
disallowedTools:
  - Bash
  - Read
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-006")
            .collect();

        assert_eq!(cc_ag_006.len(), 1);
        // Should mention both conflicting tools
        assert!(cc_ag_006[0].message.contains("Bash") && cc_ag_006[0].message.contains("Read"));
    }

    #[test]
    fn test_cc_ag_006_no_conflict() {
        let content = r#"---
name: my-agent
description: A test agent
tools:
  - Bash
  - Read
disallowedTools:
  - Write
  - Edit
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-006")
            .collect();

        assert_eq!(cc_ag_006.len(), 0);
    }

    #[test]
    fn test_cc_ag_006_only_tools_ok() {
        let content = r#"---
name: my-agent
description: A test agent
tools:
  - Bash
  - Read
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-006")
            .collect();

        assert_eq!(cc_ag_006.len(), 0);
    }

    #[test]
    fn test_cc_ag_006_only_disallowed_ok() {
        let content = r#"---
name: my-agent
description: A test agent
disallowedTools:
  - Bash
  - Read
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-006")
            .collect();

        assert_eq!(cc_ag_006.len(), 0);
    }

    // ===== Parse Error Tests =====

    #[test]
    fn test_no_frontmatter() {
        let content = "Just agent instructions without frontmatter";

        let diagnostics = validate(content);
        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-007")
            .collect();

        assert_eq!(parse_errors.len(), 1);
        assert!(parse_errors[0]
            .message
            .contains("must have YAML frontmatter"));
    }

    #[test]
    fn test_invalid_yaml() {
        let content = r#"---
name: [invalid yaml
description: test
---
Body"#;

        let diagnostics = validate(content);
        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-007")
            .collect();

        assert_eq!(parse_errors.len(), 1);
        assert!(parse_errors[0].message.contains("Failed to parse"));
    }

    // ===== Valid Agent Tests =====

    #[test]
    fn test_valid_agent_minimal() {
        let content = r#"---
name: my-agent
description: A helpful agent for testing
---
Agent instructions here"#;

        let diagnostics = validate(content);
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .collect();

        assert!(errors.is_empty());
    }

    #[test]
    fn test_valid_agent_full() {
        let content = r#"---
name: full-agent
description: A fully configured agent
model: opus
permissionMode: acceptEdits
tools:
  - Bash
  - Read
  - Write
disallowedTools:
  - Edit
---
Agent instructions with full configuration"#;

        let diagnostics = validate(content);
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .collect();

        assert!(errors.is_empty());
    }

    // ===== Fixture Tests =====

    #[test]
    fn test_fixture_missing_name() {
        let content = include_str!("../../../../tests/fixtures/invalid/agents/missing-name.md");
        let diagnostics = validate(content);
        let cc_ag_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-001")
            .collect();
        assert!(!cc_ag_001.is_empty());
    }

    #[test]
    fn test_fixture_missing_description() {
        let content =
            include_str!("../../../../tests/fixtures/invalid/agents/missing-description.md");
        let diagnostics = validate(content);
        let cc_ag_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-002")
            .collect();
        assert!(!cc_ag_002.is_empty());
    }

    #[test]
    fn test_fixture_invalid_model() {
        let content = include_str!("../../../../tests/fixtures/invalid/agents/invalid-model.md");
        let diagnostics = validate(content);
        let cc_ag_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-003")
            .collect();
        assert!(!cc_ag_003.is_empty());
    }

    #[test]
    fn test_fixture_invalid_permission() {
        let content =
            include_str!("../../../../tests/fixtures/invalid/agents/invalid-permission.md");
        let diagnostics = validate(content);
        let cc_ag_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-004")
            .collect();
        assert!(!cc_ag_004.is_empty());
    }

    #[test]
    fn test_fixture_tool_conflict() {
        let content = include_str!("../../../../tests/fixtures/invalid/agents/tool-conflict.md");
        let diagnostics = validate(content);
        let cc_ag_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-006")
            .collect();
        assert!(!cc_ag_006.is_empty());
    }

    #[test]
    fn test_fixture_valid_agent() {
        let content = include_str!("../../../../tests/fixtures/valid/agents/valid-agent.md");
        let diagnostics = validate(content);
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .collect();
        assert!(errors.is_empty());
    }

    // ===== Edge Case Tests =====

    #[test]
    fn test_cc_ag_005_empty_skills_array() {
        let content = r#"---
name: my-agent
description: A test agent
skills: []
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();
        assert_eq!(cc_ag_005.len(), 0);
    }

    #[test]
    fn test_cc_ag_006_empty_tools_array() {
        let content = r#"---
name: my-agent
description: A test agent
tools: []
disallowedTools:
  - Bash
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-006")
            .collect();
        assert_eq!(cc_ag_006.len(), 0);
    }

    #[test]
    fn test_cc_ag_006_empty_disallowed_array() {
        let content = r#"---
name: my-agent
description: A test agent
tools:
  - Bash
disallowedTools: []
---
Agent instructions"#;

        let diagnostics = validate(content);
        let cc_ag_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-006")
            .collect();
        assert_eq!(cc_ag_006.len(), 0);
    }

    #[test]
    fn test_skill_name_path_traversal_rejected() {
        let temp = TempDir::new().unwrap();
        let claude_dir = temp.path().join(".claude");
        let agents_dir = claude_dir.join("agents");
        std::fs::create_dir_all(&agents_dir).unwrap();

        let agent_path = agents_dir.join("test-agent.md");

        // Try path traversal attack
        let content = r#"---
name: my-agent
description: A test agent
skills:
  - ../../../etc/passwd
---
Agent instructions"#;

        let diagnostics = validate_with_path(&agent_path, content);
        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();
        // Should report as not found (rejected), not as a security breach
        assert_eq!(cc_ag_005.len(), 1);
    }

    #[test]
    fn test_is_safe_skill_name() {
        assert!(AgentValidator::is_safe_skill_name("my-skill"));
        assert!(AgentValidator::is_safe_skill_name("skill_name"));
        assert!(AgentValidator::is_safe_skill_name("skill123"));
        assert!(!AgentValidator::is_safe_skill_name("../parent"));
        assert!(!AgentValidator::is_safe_skill_name("path/to/skill"));
        assert!(!AgentValidator::is_safe_skill_name(".hidden"));
        assert!(!AgentValidator::is_safe_skill_name(""));
    }

    // ===== Config Wiring Tests =====

    #[test]
    fn test_config_disabled_agents_category_returns_empty() {
        let mut config = LintConfig::default();
        config.rules.agents = false;

        let content = r#"---
description: A test agent without name
---
Agent instructions"#;

        let validator = AgentValidator;
        let diagnostics = validator.validate(Path::new("test-agent.md"), content, &config);

        // CC-AG-001 should not fire when agents category is disabled
        let cc_ag_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-001")
            .collect();
        assert_eq!(cc_ag_001.len(), 0);
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["CC-AG-001".to_string()];

        // Agent missing both name and description
        let content = r#"---
model: sonnet
---
Agent instructions"#;

        let validator = AgentValidator;
        let diagnostics = validator.validate(Path::new("test-agent.md"), content, &config);

        // CC-AG-001 should not fire when specifically disabled
        let cc_ag_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-001")
            .collect();
        assert_eq!(cc_ag_001.len(), 0);

        // But CC-AG-002 should still fire (description is missing)
        let cc_ag_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-002")
            .collect();
        assert_eq!(cc_ag_002.len(), 1);
    }

    #[test]
    fn test_config_cursor_target_disables_agent_rules() {
        use crate::config::TargetTool;

        let mut config = LintConfig::default();
        config.target = TargetTool::Cursor;

        let content = r#"---
description: Agent without name
---
Agent instructions"#;

        let validator = AgentValidator;
        let diagnostics = validator.validate(Path::new("test-agent.md"), content, &config);

        // CC-AG-* rules should not fire for Cursor target
        let agent_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("CC-AG-"))
            .collect();
        assert_eq!(agent_rules.len(), 0);
    }

    #[test]
    fn test_config_claude_code_target_enables_agent_rules() {
        use crate::config::TargetTool;

        let mut config = LintConfig::default();
        config.target = TargetTool::ClaudeCode;

        let content = r#"---
description: Agent without name
---
Agent instructions"#;

        let validator = AgentValidator;
        let diagnostics = validator.validate(Path::new("test-agent.md"), content, &config);

        // CC-AG-001 should fire for ClaudeCode target
        let cc_ag_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-001")
            .collect();
        assert_eq!(cc_ag_001.len(), 1);
    }

    // ===== MockFileSystem Integration Tests for CC-AG-005 =====

    #[test]
    fn test_cc_ag_005_with_mock_fs_missing_skill() {
        use crate::fs::MockFileSystem;
        use std::sync::Arc;

        let mock_fs = Arc::new(MockFileSystem::new());
        // Set up directory structure: .claude/agents exists but skill doesn't
        mock_fs.add_dir("/project/.claude");
        mock_fs.add_dir("/project/.claude/agents");
        // No skill directory created

        let mut config = LintConfig::default();
        config.set_fs(mock_fs);

        let content = r#"---
name: my-agent
description: A test agent
skills:
  - nonexistent-skill
---
Agent instructions"#;

        let validator = AgentValidator;
        let diagnostics = validator.validate(
            Path::new("/project/.claude/agents/test-agent.md"),
            content,
            &config,
        );

        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();

        assert_eq!(cc_ag_005.len(), 1);
        assert!(cc_ag_005[0].message.contains("nonexistent-skill"));
        assert!(cc_ag_005[0].message.contains("not found"));
    }

    #[test]
    fn test_cc_ag_005_with_mock_fs_existing_skill() {
        use crate::fs::MockFileSystem;
        use std::sync::Arc;

        let mock_fs = Arc::new(MockFileSystem::new());
        // Set up complete directory structure with skill
        mock_fs.add_dir("/project/.claude");
        mock_fs.add_dir("/project/.claude/agents");
        mock_fs.add_dir("/project/.claude/skills");
        mock_fs.add_dir("/project/.claude/skills/my-skill");
        mock_fs.add_file(
            "/project/.claude/skills/my-skill/SKILL.md",
            "---\nname: my-skill\ndescription: A skill\n---\nBody",
        );

        let mut config = LintConfig::default();
        config.set_fs(mock_fs);

        let content = r#"---
name: my-agent
description: A test agent
skills:
  - my-skill
---
Agent instructions"#;

        let validator = AgentValidator;
        let diagnostics = validator.validate(
            Path::new("/project/.claude/agents/test-agent.md"),
            content,
            &config,
        );

        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();

        // No errors - skill exists
        assert_eq!(cc_ag_005.len(), 0);
    }

    #[test]
    fn test_cc_ag_005_with_mock_fs_multiple_skills_mixed() {
        use crate::fs::MockFileSystem;
        use std::sync::Arc;

        let mock_fs = Arc::new(MockFileSystem::new());
        // Set up structure with one skill present, two missing
        mock_fs.add_dir("/project/.claude");
        mock_fs.add_dir("/project/.claude/agents");
        mock_fs.add_dir("/project/.claude/skills");
        mock_fs.add_dir("/project/.claude/skills/existing-skill");
        mock_fs.add_file(
            "/project/.claude/skills/existing-skill/SKILL.md",
            "---\nname: existing-skill\ndescription: Exists\n---\nBody",
        );
        // missing-skill-1 and missing-skill-2 are not created

        let mut config = LintConfig::default();
        config.set_fs(mock_fs);

        let content = r#"---
name: my-agent
description: A test agent
skills:
  - existing-skill
  - missing-skill-1
  - missing-skill-2
---
Agent instructions"#;

        let validator = AgentValidator;
        let diagnostics = validator.validate(
            Path::new("/project/.claude/agents/test-agent.md"),
            content,
            &config,
        );

        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();

        // Should report 2 missing skills
        assert_eq!(cc_ag_005.len(), 2);
        let messages: Vec<&str> = cc_ag_005.iter().map(|d| d.message.as_str()).collect();
        assert!(messages.iter().any(|m| m.contains("missing-skill-1")));
        assert!(messages.iter().any(|m| m.contains("missing-skill-2")));
    }

    #[test]
    fn test_cc_ag_005_with_mock_fs_path_traversal_rejected() {
        use crate::fs::MockFileSystem;
        use std::sync::Arc;

        let mock_fs = Arc::new(MockFileSystem::new());
        mock_fs.add_dir("/project/.claude");
        mock_fs.add_dir("/project/.claude/agents");
        // Even if we create a file at the traversal target, it should be rejected
        mock_fs.add_file("/etc/passwd", "root:x:0:0:root:/root:/bin/bash");

        let mut config = LintConfig::default();
        config.set_fs(mock_fs);

        let content = r#"---
name: my-agent
description: A test agent
skills:
  - ../../../etc/passwd
---
Agent instructions"#;

        let validator = AgentValidator;
        let diagnostics = validator.validate(
            Path::new("/project/.claude/agents/test-agent.md"),
            content,
            &config,
        );

        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();

        // Path traversal attempts should be rejected as "not found"
        assert_eq!(cc_ag_005.len(), 1);
        assert!(cc_ag_005[0].message.contains("not found"));
    }

    #[test]
    fn test_cc_ag_005_with_mock_fs_no_claude_directory() {
        use crate::fs::MockFileSystem;
        use std::sync::Arc;

        let mock_fs = Arc::new(MockFileSystem::new());
        // No .claude directory at all
        mock_fs.add_dir("/project");

        let mut config = LintConfig::default();
        config.set_fs(mock_fs);

        let content = r#"---
name: my-agent
description: A test agent
skills:
  - some-skill
---
Agent instructions"#;

        let validator = AgentValidator;
        let diagnostics =
            validator.validate(Path::new("/project/random/test-agent.md"), content, &config);

        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();

        // Without .claude directory, no project root found, so no CC-AG-005 errors
        // (can't validate skill references without knowing where to look)
        assert_eq!(cc_ag_005.len(), 0);
    }

    #[test]
    fn test_cc_ag_005_with_mock_fs_empty_skills_array() {
        use crate::fs::MockFileSystem;
        use std::sync::Arc;

        let mock_fs = Arc::new(MockFileSystem::new());
        mock_fs.add_dir("/project/.claude");
        mock_fs.add_dir("/project/.claude/agents");

        let mut config = LintConfig::default();
        config.set_fs(mock_fs);

        let content = r#"---
name: my-agent
description: A test agent
skills: []
---
Agent instructions"#;

        let validator = AgentValidator;
        let diagnostics = validator.validate(
            Path::new("/project/.claude/agents/test-agent.md"),
            content,
            &config,
        );

        let cc_ag_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-AG-005")
            .collect();

        // Empty skills array = no errors
        assert_eq!(cc_ag_005.len(), 0);
    }
}
