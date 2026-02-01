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
pub mod fixes;
pub mod parsers;
pub mod rules;
pub mod schemas;

use std::path::{Path, PathBuf};

use rayon::prelude::*;

pub use config::LintConfig;
pub use diagnostics::{Diagnostic, DiagnosticLevel, Fix, LintError, LintResult};
pub use fixes::{apply_fixes, FixResult};
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
    /// plugin.json (validator checks .claude-plugin/ location)
    Plugin,
    /// MCP configuration files (*.mcp.json, mcp.json, mcp-*.json)
    Mcp,
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
        // Classify any plugin.json as Plugin - validator checks location constraint (CC-PL-001)
        "plugin.json" => FileType::Plugin,
        // MCP configuration files
        "mcp.json" => FileType::Mcp,
        name if name.ends_with(".mcp.json") => FileType::Mcp,
        name if name.starts_with("mcp-") && name.ends_with(".json") => FileType::Mcp,
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
            Box::new(rules::cross_platform::CrossPlatformValidator),
            Box::new(rules::xml::XmlValidator),
            Box::new(rules::imports::ImportsValidator),
        ],
        FileType::Agent => vec![
            Box::new(rules::agent::AgentValidator),
            Box::new(rules::xml::XmlValidator),
        ],
        FileType::Hooks => vec![Box::new(rules::hooks::HooksValidator)],
        FileType::Plugin => vec![Box::new(rules::plugin::PluginValidator)],
        FileType::Mcp => vec![Box::new(rules::mcp::McpValidator)],
        FileType::GenericMarkdown => vec![
            Box::new(rules::cross_platform::CrossPlatformValidator),
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
            let normalized = p.replace('\\', "/");
            glob::Pattern::new(&normalized)
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
            let mut path_str = entry.path().to_string_lossy().replace('\\', "/");
            if let Some(stripped) = path_str.strip_prefix("./") {
                path_str = stripped.to_string();
            }
            !exclude_patterns.iter().any(|p| p.matches(&path_str))
        })
        .map(|entry| entry.path().to_path_buf())
        .collect();

    // Validate files in parallel
    let mut diagnostics: Vec<Diagnostic> = paths
        .par_iter()
        .flat_map(|file_path| match validate_file(file_path, config) {
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
        assert_eq!(detect_file_type(Path::new("SKILL.md")), FileType::Skill);
        assert_eq!(
            detect_file_type(Path::new(".claude/skills/my-skill/SKILL.md")),
            FileType::Skill
        );
    }

    #[test]
    fn test_detect_claude_md() {
        assert_eq!(detect_file_type(Path::new("CLAUDE.md")), FileType::ClaudeMd);
        assert_eq!(detect_file_type(Path::new("AGENTS.md")), FileType::ClaudeMd);
        assert_eq!(
            detect_file_type(Path::new("project/CLAUDE.md")),
            FileType::ClaudeMd
        );
    }

    #[test]
    fn test_repo_agents_md_matches_claude_md() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir
            .ancestors()
            .nth(2)
            .expect("Failed to locate repo root from CARGO_MANIFEST_DIR");

        let claude = std::fs::read_to_string(repo_root.join("CLAUDE.md")).unwrap();
        let agents = std::fs::read_to_string(repo_root.join("AGENTS.md")).unwrap();

        assert_eq!(agents, claude, "AGENTS.md must match CLAUDE.md");
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
        // plugin.json in .claude-plugin/ directory
        assert_eq!(
            detect_file_type(Path::new("my-plugin.claude-plugin/plugin.json")),
            FileType::Plugin
        );
        // plugin.json outside .claude-plugin/ is still classified as Plugin
        // (validator checks location constraint CC-PL-001)
        assert_eq!(
            detect_file_type(Path::new("some/plugin.json")),
            FileType::Plugin
        );
        assert_eq!(detect_file_type(Path::new("plugin.json")), FileType::Plugin);
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
    fn test_detect_mcp() {
        assert_eq!(detect_file_type(Path::new("mcp.json")), FileType::Mcp);
        assert_eq!(detect_file_type(Path::new("tools.mcp.json")), FileType::Mcp);
        assert_eq!(
            detect_file_type(Path::new("my-server.mcp.json")),
            FileType::Mcp
        );
        assert_eq!(detect_file_type(Path::new("mcp-tools.json")), FileType::Mcp);
        assert_eq!(
            detect_file_type(Path::new("mcp-servers.json")),
            FileType::Mcp
        );
        assert_eq!(
            detect_file_type(Path::new(".claude/mcp.json")),
            FileType::Mcp
        );
    }

    #[test]
    fn test_detect_unknown() {
        assert_eq!(detect_file_type(Path::new("main.rs")), FileType::Unknown);
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
        assert_eq!(validators.len(), 4);
    }

    #[test]
    fn test_validators_for_mcp() {
        let validators = get_validators_for_type(FileType::Mcp);
        assert_eq!(validators.len(), 1);
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
            error_diagnostics
                .iter()
                .all(|d| d.file.to_string_lossy().contains("invalid")),
            "Errors should only come from the invalid skill"
        );
    }

    #[test]
    fn test_validate_project_plugin_detection() {
        let temp = tempfile::TempDir::new().unwrap();
        let plugin_dir = temp.path().join("my-plugin.claude-plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();

        // Create plugin.json with a validation issue (missing description - CC-PL-004)
        std::fs::write(
            plugin_dir.join("plugin.json"),
            r#"{"name": "test-plugin", "version": "1.0.0"}"#,
        )
        .unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_project(temp.path(), &config).unwrap();

        // Should detect the plugin.json and report CC-PL-004 for missing description
        let plugin_diagnostics: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("CC-PL-"))
            .collect();

        assert!(
            !plugin_diagnostics.is_empty(),
            "validate_project() should detect and validate plugin.json files"
        );

        assert!(
            plugin_diagnostics.iter().any(|d| d.rule == "CC-PL-004"),
            "Should report CC-PL-004 for missing description field"
        );
    }

    // ===== MCP Validation Integration Tests =====

    #[test]
    fn test_validate_file_mcp() {
        let temp = tempfile::TempDir::new().unwrap();
        let mcp_path = temp.path().join("tools.mcp.json");
        std::fs::write(
            &mcp_path,
            r#"{"name": "test-tool", "description": "A test tool for testing purposes", "inputSchema": {"type": "object"}}"#,
        )
        .unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_file(&mcp_path, &config).unwrap();

        // Tool without consent field should trigger MCP-005 warning
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-005"));
    }

    #[test]
    fn test_validate_file_mcp_invalid_schema() {
        let temp = tempfile::TempDir::new().unwrap();
        let mcp_path = temp.path().join("mcp.json");
        std::fs::write(
            &mcp_path,
            r#"{"name": "test-tool", "description": "A test tool for testing purposes", "inputSchema": "not an object"}"#,
        )
        .unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_file(&mcp_path, &config).unwrap();

        // Invalid schema should trigger MCP-003
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-003"));
    }

    #[test]
    fn test_validate_project_mcp_detection() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create an MCP file with issues
        std::fs::write(
            temp.path().join("tools.mcp.json"),
            r#"{"name": "", "description": "Short", "inputSchema": {"type": "object"}}"#,
        )
        .unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_project(temp.path(), &config).unwrap();

        // Should detect the MCP file and report issues
        let mcp_diagnostics: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("MCP-"))
            .collect();

        assert!(
            !mcp_diagnostics.is_empty(),
            "validate_project() should detect and validate MCP files"
        );

        // Empty name should trigger MCP-002
        assert!(
            mcp_diagnostics.iter().any(|d| d.rule == "MCP-002"),
            "Should report MCP-002 for empty name"
        );
    }

    // ===== Cross-Platform Validation Integration Tests =====

    #[test]
    fn test_validate_agents_md_with_claude_features() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create AGENTS.md with Claude-specific features
        std::fs::write(
            temp.path().join("AGENTS.md"),
            r#"# Agent Config
- type: PreToolExecution
  command: echo "test"
"#,
        )
        .unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_project(temp.path(), &config).unwrap();

        // Should detect XP-001 error for Claude-specific hooks in AGENTS.md
        let xp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-001").collect();
        assert!(
            !xp_001.is_empty(),
            "Expected XP-001 error for hooks in AGENTS.md"
        );
    }

    #[test]
    fn test_validate_agents_md_with_context_fork() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create AGENTS.md with context: fork
        std::fs::write(
            temp.path().join("AGENTS.md"),
            r#"---
name: test
context: fork
agent: Explore
---
# Test Agent
"#,
        )
        .unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_project(temp.path(), &config).unwrap();

        // Should detect XP-001 errors for Claude-specific features
        let xp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-001").collect();
        assert!(
            !xp_001.is_empty(),
            "Expected XP-001 errors for context:fork and agent in AGENTS.md"
        );
    }

    #[test]
    fn test_validate_agents_md_no_headers() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create AGENTS.md with no headers
        std::fs::write(
            temp.path().join("AGENTS.md"),
            "Just plain text without any markdown headers.",
        )
        .unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_project(temp.path(), &config).unwrap();

        // Should detect XP-002 warning for missing headers
        let xp_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-002").collect();
        assert!(
            !xp_002.is_empty(),
            "Expected XP-002 warning for missing headers in AGENTS.md"
        );
    }

    #[test]
    fn test_validate_agents_md_hard_coded_paths() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create AGENTS.md with hard-coded platform paths
        std::fs::write(
            temp.path().join("AGENTS.md"),
            r#"# Config
Check .claude/settings.json and .cursor/rules/ for configuration.
"#,
        )
        .unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_project(temp.path(), &config).unwrap();

        // Should detect XP-003 warnings for hard-coded paths
        let xp_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-003").collect();
        assert_eq!(
            xp_003.len(),
            2,
            "Expected 2 XP-003 warnings for hard-coded paths"
        );
    }

    #[test]
    fn test_validate_valid_agents_md() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create valid AGENTS.md without any issues
        std::fs::write(
            temp.path().join("AGENTS.md"),
            r#"# Project Guidelines

Follow the coding style guide.

## Commands
- npm run build
- npm run test
"#,
        )
        .unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_project(temp.path(), &config).unwrap();

        // Should have no XP-* diagnostics
        let xp_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("XP-"))
            .collect();
        assert!(
            xp_rules.is_empty(),
            "Valid AGENTS.md should have no XP-* diagnostics"
        );
    }

    #[test]
    fn test_validate_claude_md_allows_claude_features() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create CLAUDE.md with Claude-specific features (allowed)
        std::fs::write(
            temp.path().join("CLAUDE.md"),
            r#"---
name: test
context: fork
agent: Explore
allowed-tools: Read Write
---
# Claude Agent
"#,
        )
        .unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_project(temp.path(), &config).unwrap();

        // XP-001 should NOT fire for CLAUDE.md (Claude features are allowed there)
        let xp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-001").collect();
        assert!(
            xp_001.is_empty(),
            "CLAUDE.md should be allowed to have Claude-specific features"
        );
    }
}
