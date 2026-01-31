//! # agnix-core
//!
//! Core validation engine for agent configurations.
//!
//! Validates:
//! - Agent Skills (SKILL.md)
//! - Agent definitions (.md files with frontmatter)
//! - MCP tool configurations
//! - Claude Code hooks
//! - CLAUDE.md memory files
//! - Plugin manifests

pub mod config;
pub mod diagnostics;
pub mod parsers;
pub mod rules;
pub mod schemas;

use std::path::{Path, PathBuf};

use rayon::prelude::*;

pub use config::LintConfig;
pub use diagnostics::{Diagnostic, DiagnosticLevel, LintError, LintResult};
use rules::Validator;

/// Detected file type for validator dispatch
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// SKILL.md files
    Skill,
    /// CLAUDE.md, AGENTS.md files
    ClaudeMd,
    /// .claude/agents/*.md or agents/*.md
    Agent,
    /// settings.json, settings.local.json
    Hooks,
    /// plugin.json in .claude-plugin/
    Plugin,
    /// Other .md files (for XML/import checks)
    GenericMarkdown,
    /// Skip validation
    Unknown,
}

/// Detect file type based on path patterns
pub fn detect_file_type(path: &Path) -> FileType {
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let parent = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str());
    let grandparent = path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str());

    match filename {
        "SKILL.md" => FileType::Skill,
        "CLAUDE.md" | "AGENTS.md" => FileType::ClaudeMd,
        "settings.json" | "settings.local.json" => FileType::Hooks,
        "plugin.json" if parent.map_or(false, |p| p.ends_with(".claude-plugin")) => {
            FileType::Plugin
        }
        name if name.ends_with(".md") => {
            if parent == Some("agents") || grandparent == Some("agents") {
                FileType::Agent
            } else {
                FileType::GenericMarkdown
            }
        }
        _ => FileType::Unknown,
    }
}

/// Get validators for a file type
fn get_validators_for_type(file_type: FileType) -> Vec<Box<dyn Validator>> {
    match file_type {
        FileType::Skill => vec![
            Box::new(rules::skill::SkillValidator),
            Box::new(rules::xml::XmlValidator),
            Box::new(rules::imports::ImportsValidator),
        ],
        FileType::ClaudeMd => vec![
            Box::new(rules::claude_md::ClaudeMdValidator),
            Box::new(rules::xml::XmlValidator),
            Box::new(rules::imports::ImportsValidator),
        ],
        FileType::Agent => vec![
            Box::new(rules::agent::AgentValidator),
            Box::new(rules::xml::XmlValidator),
        ],
        FileType::Hooks => vec![Box::new(rules::hooks::HooksValidator)],
        FileType::Plugin => vec![Box::new(rules::plugin::PluginValidator)],
        FileType::GenericMarkdown => vec![
            Box::new(rules::xml::XmlValidator),
            Box::new(rules::imports::ImportsValidator),
        ],
        FileType::Unknown => vec![],
    }
}

/// Validate a single file
pub fn validate_file(path: &Path, config: &LintConfig) -> LintResult<Vec<Diagnostic>> {
    let file_type = detect_file_type(path);

    if file_type == FileType::Unknown {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(path).map_err(|e| LintError::FileRead {
        path: path.to_path_buf(),
        source: e,
    })?;

    let validators = get_validators_for_type(file_type);
    let mut diagnostics = Vec::new();

    for validator in validators {
        diagnostics.extend(validator.validate(path, &content, config));
    }

    Ok(diagnostics)
}

/// Main entry point for validating a project
pub fn validate_project(path: &Path, config: &LintConfig) -> LintResult<Vec<Diagnostic>> {
    use ignore::WalkBuilder;

    // Pre-compile exclude patterns once (avoids N+1 pattern compilation)
    // Panic on invalid patterns to catch config errors early
    let exclude_patterns: Vec<glob::Pattern> = config
        .exclude
        .iter()
        .map(|p| {
            glob::Pattern::new(p)
                .unwrap_or_else(|_| panic!("Invalid exclude pattern in config: {}", p))
        })
        .collect();

    // Collect all file paths to validate (sequential walk, parallel validation)
    let paths: Vec<PathBuf> = WalkBuilder::new(path)
        .standard_filters(true)
        .build()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .filter(|entry| {
            let path_str = entry.path().to_string_lossy();
            !exclude_patterns.iter().any(|p| p.matches(&path_str))
        })
        .map(|entry| entry.path().to_path_buf())
        .collect();

    // Validate files in parallel
    let mut diagnostics: Vec<Diagnostic> = paths
        .par_iter()
        .flat_map(|file_path| {
            match validate_file(file_path, config) {
                Ok(file_diagnostics) => file_diagnostics,
                Err(e) => {
                    vec![Diagnostic::error(
                        file_path.clone(),
                        0,
                        0,
                        "file::read",
                        format!("Failed to validate file: {}", e),
                    )]
                }
            }
        })
        .collect();

    // Sort by severity (errors first), then by file path, then by line/rule for full determinism
    diagnostics.sort_by(|a, b| {
        a.level
            .cmp(&b.level)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.rule.cmp(&b.rule))
    });

    Ok(diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_skill_file() {
        assert_eq!(
            detect_file_type(Path::new("SKILL.md")),
            FileType::Skill
        );
        assert_eq!(
            detect_file_type(Path::new(".claude/skills/my-skill/SKILL.md")),
            FileType::Skill
        );
    }

    #[test]
    fn test_detect_claude_md() {
        assert_eq!(
            detect_file_type(Path::new("CLAUDE.md")),
            FileType::ClaudeMd
        );
        assert_eq!(
            detect_file_type(Path::new("AGENTS.md")),
            FileType::ClaudeMd
        );
        assert_eq!(
            detect_file_type(Path::new("project/CLAUDE.md")),
            FileType::ClaudeMd
        );
    }

    #[test]
    fn test_detect_agents() {
        assert_eq!(
            detect_file_type(Path::new("agents/my-agent.md")),
            FileType::Agent
        );
        assert_eq!(
            detect_file_type(Path::new(".claude/agents/helper.md")),
            FileType::Agent
        );
    }

    #[test]
    fn test_detect_hooks() {
        assert_eq!(
            detect_file_type(Path::new("settings.json")),
            FileType::Hooks
        );
        assert_eq!(
            detect_file_type(Path::new(".claude/settings.local.json")),
            FileType::Hooks
        );
    }

    #[test]
    fn test_detect_plugin() {
        assert_eq!(
            detect_file_type(Path::new("my-plugin.claude-plugin/plugin.json")),
            FileType::Plugin
        );
        assert_ne!(
            detect_file_type(Path::new("some/plugin.json")),
            FileType::Plugin
        );
    }

    #[test]
    fn test_detect_generic_markdown() {
        assert_eq!(
            detect_file_type(Path::new("README.md")),
            FileType::GenericMarkdown
        );
        assert_eq!(
            detect_file_type(Path::new("docs/guide.md")),
            FileType::GenericMarkdown
        );
    }

    #[test]
    fn test_detect_unknown() {
        assert_eq!(
            detect_file_type(Path::new("main.rs")),
            FileType::Unknown
        );
        assert_eq!(
            detect_file_type(Path::new("package.json")),
            FileType::Unknown
        );
    }

    #[test]
    fn test_validators_for_skill() {
        let validators = get_validators_for_type(FileType::Skill);
        assert_eq!(validators.len(), 3);
    }

    #[test]
    fn test_validators_for_claude_md() {
        let validators = get_validators_for_type(FileType::ClaudeMd);
        assert_eq!(validators.len(), 3);
    }

    #[test]
    fn test_validators_for_unknown() {
        let validators = get_validators_for_type(FileType::Unknown);
        assert_eq!(validators.len(), 0);
    }

    #[test]
    fn test_validate_file_unknown_type() {
        let temp = tempfile::TempDir::new().unwrap();
        let unknown_path = temp.path().join("test.rs");
        std::fs::write(&unknown_path, "fn main() {}").unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_file(&unknown_path, &config).unwrap();

        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_validate_file_skill() {
        let temp = tempfile::TempDir::new().unwrap();
        let skill_path = temp.path().join("SKILL.md");
        std::fs::write(
            &skill_path,
            "---\nname: test-skill\ndescription: Use when testing\n---\nBody",
        )
        .unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_file(&skill_path, &config).unwrap();

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_validate_file_invalid_skill() {
        let temp = tempfile::TempDir::new().unwrap();
        let skill_path = temp.path().join("SKILL.md");
        std::fs::write(
            &skill_path,
            "---\nname: deploy-prod\ndescription: Deploys\n---\nBody",
        )
        .unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_file(&skill_path, &config).unwrap();

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SK-006"));
    }

    #[test]
    fn test_validate_project_finds_issues() {
        let temp = tempfile::TempDir::new().unwrap();
        let skill_dir = temp.path().join("skills").join("deploy");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: deploy-prod\ndescription: Deploys\n---\nBody",
        )
        .unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_project(temp.path(), &config).unwrap();

        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn test_validate_project_empty_dir() {
        let temp = tempfile::TempDir::new().unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_project(temp.path(), &config).unwrap();

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_validate_project_sorts_by_severity() {
        let temp = tempfile::TempDir::new().unwrap();

        let skill_dir = temp.path().join("skill1");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: deploy-prod\ndescription: Deploys\n---\nBody",
        )
        .unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_project(temp.path(), &config).unwrap();

        for i in 1..diagnostics.len() {
            assert!(diagnostics[i - 1].level <= diagnostics[i].level);
        }
    }

    #[test]
    fn test_validate_invalid_skill_triggers_both_rules() {
        let temp = tempfile::TempDir::new().unwrap();
        let skill_path = temp.path().join("SKILL.md");
        std::fs::write(
            &skill_path,
            "---\nname: deploy-prod\ndescription: Deploys\nallowed-tools: Bash Read Write\n---\nBody",
        )
        .unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_file(&skill_path, &config).unwrap();

        assert!(diagnostics.iter().any(|d| d.rule == "CC-SK-006"));
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SK-007"));
    }

    #[test]
    fn test_validate_valid_skill_produces_no_errors() {
        let temp = tempfile::TempDir::new().unwrap();
        let skill_path = temp.path().join("SKILL.md");
        std::fs::write(
            &skill_path,
            "---\nname: code-review\ndescription: Use when reviewing code\n---\nBody",
        )
        .unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_file(&skill_path, &config).unwrap();

        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .collect();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_parallel_validation_deterministic_output() {
        // Create a project structure with multiple files that will generate diagnostics
        let temp = tempfile::TempDir::new().unwrap();

        // Create multiple skill files with issues to ensure non-trivial parallel work
        for i in 0..5 {
            let skill_dir = temp.path().join(format!("skill-{}", i));
            std::fs::create_dir_all(&skill_dir).unwrap();
            std::fs::write(
                skill_dir.join("SKILL.md"),
                format!(
                    "---\nname: deploy-prod-{}\ndescription: Deploys things\n---\nBody",
                    i
                ),
            )
            .unwrap();
        }

        // Create some CLAUDE.md files too
        for i in 0..3 {
            let dir = temp.path().join(format!("project-{}", i));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("CLAUDE.md"),
                "# Project\n\nBe helpful and concise.\n",
            )
            .unwrap();
        }

        let config = LintConfig::default();

        // Run validation multiple times and verify identical output
        let first_result = validate_project(temp.path(), &config).unwrap();

        for run in 1..=10 {
            let result = validate_project(temp.path(), &config).unwrap();

            assert_eq!(
                first_result.len(),
                result.len(),
                "Run {} produced different number of diagnostics",
                run
            );

            for (i, (a, b)) in first_result.iter().zip(result.iter()).enumerate() {
                assert_eq!(
                    a.file, b.file,
                    "Run {} diagnostic {} has different file",
                    run, i
                );
                assert_eq!(
                    a.rule, b.rule,
                    "Run {} diagnostic {} has different rule",
                    run, i
                );
                assert_eq!(
                    a.level, b.level,
                    "Run {} diagnostic {} has different level",
                    run, i
                );
            }
        }

        // Verify we actually got some diagnostics (the dangerous name rule should fire)
        assert!(
            !first_result.is_empty(),
            "Expected diagnostics for deploy-prod-* skill names"
        );
    }

    #[test]
    fn test_parallel_validation_single_file() {
        // Edge case: verify parallel code works correctly with just one file
        let temp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            temp.path().join("SKILL.md"),
            "---\nname: deploy-prod\ndescription: Deploys\n---\nBody",
        )
        .unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_project(temp.path(), &config).unwrap();

        // Should have at least one diagnostic for the dangerous name (CC-SK-006)
        assert!(
            diagnostics.iter().any(|d| d.rule == "CC-SK-006"),
            "Expected CC-SK-006 diagnostic for dangerous deploy-prod name"
        );
    }

    #[test]
    fn test_parallel_validation_mixed_results() {
        // Test mix of valid and invalid files processed in parallel
        let temp = tempfile::TempDir::new().unwrap();

        // Valid skill (no diagnostics expected)
        let valid_dir = temp.path().join("valid");
        std::fs::create_dir_all(&valid_dir).unwrap();
        std::fs::write(
            valid_dir.join("SKILL.md"),
            "---\nname: code-review\ndescription: Use when reviewing code\n---\nBody",
        )
        .unwrap();

        // Invalid skill (diagnostics expected)
        let invalid_dir = temp.path().join("invalid");
        std::fs::create_dir_all(&invalid_dir).unwrap();
        std::fs::write(
            invalid_dir.join("SKILL.md"),
            "---\nname: deploy-prod\ndescription: Deploys\n---\nBody",
        )
        .unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_project(temp.path(), &config).unwrap();

        // Should have diagnostics only from the invalid skill
        let error_diagnostics: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .collect();

        assert!(
            error_diagnostics.iter().all(|d| d.file.to_string_lossy().contains("invalid")),
            "Errors should only come from the invalid skill"
        );
    }
}
