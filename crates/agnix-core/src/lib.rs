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
pub mod eval;
mod file_utils;
pub mod fixes;
mod parsers;
mod rules;
mod schemas;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

pub use config::LintConfig;
pub use diagnostics::{Diagnostic, DiagnosticLevel, Fix, LintError, LintResult};
pub use fixes::{apply_fixes, FixResult};
pub use rules::Validator;

/// Result of validating a project, including diagnostics and metadata.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Diagnostics found during validation.
    pub diagnostics: Vec<Diagnostic>,
    /// Number of files that were checked (excludes Unknown file types).
    pub files_checked: usize,
}

/// Detected file type for validator dispatch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// GitHub Copilot global instructions (.github/copilot-instructions.md)
    Copilot,
    /// GitHub Copilot scoped instructions (.github/instructions/*.instructions.md)
    CopilotScoped,
    /// Cursor project rules (.cursor/rules/*.mdc)
    CursorRule,
    /// Legacy Cursor rules file (.cursorrules)
    CursorRulesLegacy,
    /// Other .md files (for XML/import checks)
    GenericMarkdown,
    /// Skip validation
    Unknown,
}

/// Factory function type that creates validator instances.
pub type ValidatorFactory = fn() -> Box<dyn Validator>;

/// Registry that maps [`FileType`] values to validator factories.
///
/// This is the extension point for the validation engine. A
/// `ValidatorRegistry` owns a set of [`ValidatorFactory`] functions for each
/// supported [`FileType`], and constructs concrete [`Validator`] instances on
/// demand.
///
/// Most callers should use [`ValidatorRegistry::with_defaults`] to obtain a
/// registry pre-populated with all built-in validators.
pub struct ValidatorRegistry {
    validators: HashMap<FileType, Vec<ValidatorFactory>>,
}

impl ValidatorRegistry {
    /// Create an empty registry with no registered validators.
    pub fn new() -> Self {
        Self {
            validators: HashMap::new(),
        }
    }

    /// Create a registry pre-populated with built-in validators.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register_defaults();
        registry
    }

    /// Register a validator factory for a given file type.
    pub fn register(&mut self, file_type: FileType, factory: ValidatorFactory) {
        self.validators.entry(file_type).or_default().push(factory);
    }

    /// Build a fresh validator instance list for the given file type.
    pub fn validators_for(&self, file_type: FileType) -> Vec<Box<dyn Validator>> {
        self.validators
            .get(&file_type)
            .into_iter()
            .flatten()
            .map(|factory| factory())
            .collect()
    }

    fn register_defaults(&mut self) {
        const DEFAULTS: &[(FileType, ValidatorFactory)] = &[
            (FileType::Skill, skill_validator),
            (FileType::Skill, xml_validator),
            (FileType::Skill, imports_validator),
            (FileType::ClaudeMd, claude_md_validator),
            (FileType::ClaudeMd, cross_platform_validator),
            (FileType::ClaudeMd, agents_md_validator),
            (FileType::ClaudeMd, xml_validator),
            (FileType::ClaudeMd, imports_validator),
            (FileType::ClaudeMd, prompt_validator),
            (FileType::Agent, agent_validator),
            (FileType::Agent, xml_validator),
            (FileType::Hooks, hooks_validator),
            (FileType::Plugin, plugin_validator),
            (FileType::Mcp, mcp_validator),
            (FileType::Copilot, copilot_validator),
            (FileType::Copilot, xml_validator),
            (FileType::CopilotScoped, copilot_validator),
            (FileType::CopilotScoped, xml_validator),
            (FileType::CursorRule, cursor_validator),
            (FileType::CursorRulesLegacy, cursor_validator),
            (FileType::GenericMarkdown, cross_platform_validator),
            (FileType::GenericMarkdown, xml_validator),
            (FileType::GenericMarkdown, imports_validator),
        ];

        for &(file_type, factory) in DEFAULTS {
            self.register(file_type, factory);
        }
    }
}

impl Default for ValidatorRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

fn skill_validator() -> Box<dyn Validator> {
    Box::new(rules::skill::SkillValidator)
}

fn claude_md_validator() -> Box<dyn Validator> {
    Box::new(rules::claude_md::ClaudeMdValidator)
}

fn agents_md_validator() -> Box<dyn Validator> {
    Box::new(rules::agents_md::AgentsMdValidator)
}

fn agent_validator() -> Box<dyn Validator> {
    Box::new(rules::agent::AgentValidator)
}

fn hooks_validator() -> Box<dyn Validator> {
    Box::new(rules::hooks::HooksValidator)
}

fn plugin_validator() -> Box<dyn Validator> {
    Box::new(rules::plugin::PluginValidator)
}

fn mcp_validator() -> Box<dyn Validator> {
    Box::new(rules::mcp::McpValidator)
}

fn xml_validator() -> Box<dyn Validator> {
    Box::new(rules::xml::XmlValidator)
}

fn imports_validator() -> Box<dyn Validator> {
    Box::new(rules::imports::ImportsValidator)
}

fn cross_platform_validator() -> Box<dyn Validator> {
    Box::new(rules::cross_platform::CrossPlatformValidator)
}

fn prompt_validator() -> Box<dyn Validator> {
    Box::new(rules::prompt::PromptValidator)
}

fn copilot_validator() -> Box<dyn Validator> {
    Box::new(rules::copilot::CopilotValidator)
}

fn cursor_validator() -> Box<dyn Validator> {
    Box::new(rules::cursor::CursorValidator)
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
        "CLAUDE.md" | "CLAUDE.local.md" | "AGENTS.md" | "AGENTS.local.md"
        | "AGENTS.override.md" => FileType::ClaudeMd,
        "settings.json" | "settings.local.json" => FileType::Hooks,
        // Classify any plugin.json as Plugin - validator checks location constraint (CC-PL-001)
        "plugin.json" => FileType::Plugin,
        // MCP configuration files
        "mcp.json" => FileType::Mcp,
        name if name.ends_with(".mcp.json") => FileType::Mcp,
        name if name.starts_with("mcp-") && name.ends_with(".json") => FileType::Mcp,
        // GitHub Copilot global instructions (.github/copilot-instructions.md)
        "copilot-instructions.md" if parent == Some(".github") => FileType::Copilot,
        // GitHub Copilot scoped instructions (.github/instructions/*.instructions.md)
        name if name.ends_with(".instructions.md")
            && parent == Some("instructions")
            && grandparent == Some(".github") =>
        {
            FileType::CopilotScoped
        }
        // Cursor project rules (.cursor/rules/*.mdc)
        name if name.ends_with(".mdc")
            && parent == Some("rules")
            && grandparent == Some(".cursor") =>
        {
            FileType::CursorRule
        }
        // Legacy Cursor rules file (.cursorrules)
        ".cursorrules" => FileType::CursorRulesLegacy,
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

/// Validate a single file
pub fn validate_file(path: &Path, config: &LintConfig) -> LintResult<Vec<Diagnostic>> {
    let registry = ValidatorRegistry::with_defaults();
    validate_file_with_registry(path, config, &registry)
}

/// Validate a single file with a custom validator registry
pub fn validate_file_with_registry(
    path: &Path,
    config: &LintConfig,
    registry: &ValidatorRegistry,
) -> LintResult<Vec<Diagnostic>> {
    let file_type = detect_file_type(path);

    if file_type == FileType::Unknown {
        return Ok(vec![]);
    }

    let content = file_utils::safe_read_file(path)?;

    let validators = registry.validators_for(file_type);
    let mut diagnostics = Vec::new();

    for validator in validators {
        diagnostics.extend(validator.validate(path, &content, config));
    }

    Ok(diagnostics)
}

/// Main entry point for validating a project
pub fn validate_project(path: &Path, config: &LintConfig) -> LintResult<ValidationResult> {
    let registry = ValidatorRegistry::with_defaults();
    validate_project_with_registry(path, config, &registry)
}

struct ExcludePattern {
    pattern: glob::Pattern,
    dir_only_prefix: Option<String>,
    allow_probe: bool,
}

fn normalize_rel_path(entry_path: &Path, root: &Path) -> String {
    let rel_path = entry_path.strip_prefix(root).unwrap_or(entry_path);
    let mut path_str = rel_path.to_string_lossy().replace('\\', "/");
    if let Some(stripped) = path_str.strip_prefix("./") {
        path_str = stripped.to_string();
    }
    path_str
}

fn compile_exclude_patterns(excludes: &[String]) -> Vec<ExcludePattern> {
    excludes
        .iter()
        .map(|pattern| {
            let normalized = pattern.replace('\\', "/");
            let (glob_str, dir_only_prefix) = if let Some(prefix) = normalized.strip_suffix('/') {
                (format!("{}/**", prefix), Some(prefix.to_string()))
            } else {
                (normalized.clone(), None)
            };
            let allow_probe = dir_only_prefix.is_some() || glob_str.contains("**");
            ExcludePattern {
                pattern: glob::Pattern::new(&glob_str)
                    .unwrap_or_else(|_| panic!("Invalid exclude pattern in config: {}", pattern)),
                dir_only_prefix,
                allow_probe,
            }
        })
        .collect()
}

fn should_prune_dir(rel_dir: &str, exclude_patterns: &[ExcludePattern]) -> bool {
    if rel_dir.is_empty() {
        return false;
    }
    // Probe path used to detect patterns that match files inside a directory.
    // Only apply it for recursive patterns (e.g. ** or dir-only prefix).
    let probe = format!("{}/__agnix_probe__", rel_dir.trim_end_matches('/'));
    exclude_patterns
        .iter()
        .any(|p| p.pattern.matches(rel_dir) || (p.allow_probe && p.pattern.matches(&probe)))
}

fn is_excluded_file(path_str: &str, exclude_patterns: &[ExcludePattern]) -> bool {
    exclude_patterns
        .iter()
        .any(|p| p.pattern.matches(path_str) && p.dir_only_prefix.as_deref() != Some(path_str))
}

/// Main entry point for validating a project with a custom validator registry
pub fn validate_project_with_registry(
    path: &Path,
    config: &LintConfig,
    registry: &ValidatorRegistry,
) -> LintResult<ValidationResult> {
    use ignore::WalkBuilder;
    use std::sync::Arc;

    let root_dir = resolve_validation_root(path);
    let mut config = config.clone();
    config.set_root_dir(root_dir.clone());

    // Pre-compile exclude patterns once (avoids N+1 pattern compilation)
    // Panic on invalid patterns to catch config errors early
    let exclude_patterns = compile_exclude_patterns(&config.exclude);
    let exclude_patterns = Arc::new(exclude_patterns);
    let root_path = root_dir.clone();

    let walk_root = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    // Collect file paths (sequential walk, parallel validation)
    // Note: hidden(false) includes .github directory for Copilot instruction files
    let paths: Vec<PathBuf> = WalkBuilder::new(&walk_root)
        .hidden(false)
        .git_ignore(true)
        .filter_entry({
            let exclude_patterns = Arc::clone(&exclude_patterns);
            let root_path = root_path.clone();
            move |entry| {
                let entry_path = entry.path();
                if entry_path == root_path {
                    return true;
                }
                if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                    let rel_path = normalize_rel_path(entry_path, &root_path);
                    return !should_prune_dir(&rel_path, exclude_patterns.as_slice());
                }
                true
            }
        })
        .build()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .filter(|entry| {
            let entry_path = entry.path();
            let path_str = normalize_rel_path(entry_path, &root_path);
            !is_excluded_file(&path_str, exclude_patterns.as_slice())
        })
        .map(|entry| entry.path().to_path_buf())
        .collect();

    // Count recognized files (detect_file_type is string-only, no I/O)
    let files_checked = paths
        .iter()
        .filter(|p| detect_file_type(p) != FileType::Unknown)
        .count();

    // Validate files in parallel
    let mut diagnostics: Vec<Diagnostic> = paths
        .par_iter()
        .flat_map(
            |file_path| match validate_file_with_registry(file_path, &config, registry) {
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
            },
        )
        .collect();

    // AGM-006: Check for multiple AGENTS.md files in the directory tree (project-level check)
    if config.is_rule_enabled("AGM-006") {
        let agents_md_paths: Vec<_> = paths
            .iter()
            .filter(|p| p.file_name().and_then(|n| n.to_str()) == Some("AGENTS.md"))
            .collect();

        if agents_md_paths.len() > 1 {
            for agents_file in &agents_md_paths {
                let parent_files =
                    schemas::agents_md::check_agents_md_hierarchy(agents_file, &paths);
                let description = if !parent_files.is_empty() {
                    let parent_paths: Vec<String> = parent_files
                        .iter()
                        .map(|p| p.to_string_lossy().to_string())
                        .collect();
                    format!(
                        "Nested AGENTS.md detected - parent AGENTS.md files exist at: {}",
                        parent_paths.join(", ")
                    )
                } else {
                    let other_paths: Vec<String> = agents_md_paths
                        .iter()
                        .filter(|p| p.as_path() != agents_file.as_path())
                        .map(|p| p.to_string_lossy().to_string())
                        .collect();
                    format!(
                        "Multiple AGENTS.md files detected - other AGENTS.md files exist at: {}",
                        other_paths.join(", ")
                    )
                };

                diagnostics.push(
                    Diagnostic::warning(
                        (*agents_file).clone(),
                        1,
                        0,
                        "AGM-006",
                        description,
                    )
                    .with_suggestion(
                        "Some tools load AGENTS.md hierarchically. Document inheritance behavior or consolidate files.".to_string(),
                    ),
                );
            }
        }
    }

    // XP-004, XP-005, XP-006: Cross-layer contradiction detection (project-level checks)
    // These rules analyze relationships between multiple instruction files
    let xp004_enabled = config.is_rule_enabled("XP-004");
    let xp005_enabled = config.is_rule_enabled("XP-005");
    let xp006_enabled = config.is_rule_enabled("XP-006");

    if xp004_enabled || xp005_enabled || xp006_enabled {
        // Collect instruction files (CLAUDE.md, AGENTS.md, etc.)
        let instruction_files: Vec<_> = paths
            .iter()
            .filter(|p| schemas::cross_platform::is_instruction_file(p))
            .collect();

        if instruction_files.len() > 1 {
            // Read content of all instruction files
            let mut file_contents: Vec<(PathBuf, String)> = Vec::new();
            for file_path in &instruction_files {
                match file_utils::safe_read_file(file_path) {
                    Ok(content) => {
                        file_contents.push(((*file_path).clone(), content));
                    }
                    Err(e) => {
                        diagnostics.push(Diagnostic::error(
                            (*file_path).clone(),
                            0,
                            0,
                            "XP-004",
                            format!("Failed to read instruction file: {}", e),
                        ));
                    }
                }
            }

            // XP-004: Detect conflicting build/test commands
            if xp004_enabled {
                let file_commands: Vec<_> = file_contents
                    .iter()
                    .map(|(path, content)| {
                        (
                            path.clone(),
                            schemas::cross_platform::extract_build_commands(content),
                        )
                    })
                    .filter(|(_, cmds)| !cmds.is_empty())
                    .collect();

                let build_conflicts =
                    schemas::cross_platform::detect_build_conflicts(&file_commands);
                for conflict in build_conflicts {
                    diagnostics.push(
                        Diagnostic::warning(
                            conflict.file1.clone(),
                            conflict.file1_line,
                            0,
                            "XP-004",
                            format!(
                                "Conflicting package managers: {} uses {} but {} uses {} for {} commands",
                                conflict.file1.display(),
                                conflict.file1_manager.as_str(),
                                conflict.file2.display(),
                                conflict.file2_manager.as_str(),
                                match conflict.command_type {
                                    schemas::cross_platform::CommandType::Install => "install",
                                    schemas::cross_platform::CommandType::Build => "build",
                                    schemas::cross_platform::CommandType::Test => "test",
                                    schemas::cross_platform::CommandType::Run => "run",
                                    schemas::cross_platform::CommandType::Other => "other",
                                }
                            ),
                        )
                        .with_suggestion(
                            "Standardize on a single package manager across all instruction files".to_string(),
                        ),
                    );
                }
            }

            // XP-005: Detect conflicting tool constraints
            if xp005_enabled {
                let file_constraints: Vec<_> = file_contents
                    .iter()
                    .map(|(path, content)| {
                        (
                            path.clone(),
                            schemas::cross_platform::extract_tool_constraints(content),
                        )
                    })
                    .filter(|(_, constraints)| !constraints.is_empty())
                    .collect();

                let tool_conflicts =
                    schemas::cross_platform::detect_tool_conflicts(&file_constraints);
                for conflict in tool_conflicts {
                    diagnostics.push(
                        Diagnostic::error(
                            conflict.allow_file.clone(),
                            conflict.allow_line,
                            0,
                            "XP-005",
                            format!(
                                "Conflicting tool constraints: '{}' is allowed in {} but disallowed in {}",
                                conflict.tool_name,
                                conflict.allow_file.display(),
                                conflict.disallow_file.display()
                            ),
                        )
                        .with_suggestion(
                            "Resolve the conflict by consistently allowing or disallowing the tool".to_string(),
                        ),
                    );
                }
            }

            // XP-006: Detect multiple layers without documented precedence
            if xp006_enabled {
                let layers: Vec<_> = file_contents
                    .iter()
                    .map(|(path, content)| schemas::cross_platform::categorize_layer(path, content))
                    .collect();

                if let Some(issue) = schemas::cross_platform::detect_precedence_issues(&layers) {
                    // Report on the first layer file
                    if let Some(first_layer) = issue.layers.first() {
                        diagnostics.push(
                            Diagnostic::warning(
                                first_layer.path.clone(),
                                1,
                                0,
                                "XP-006",
                                issue.description,
                            )
                            .with_suggestion(
                                "Document which file takes precedence (e.g., 'CLAUDE.md takes precedence over AGENTS.md')".to_string(),
                            ),
                        );
                    }
                }
            }
        }
    }

    // Sort by severity (errors first), then by file path, then by line/rule for full determinism
    diagnostics.sort_by(|a, b| {
        a.level
            .cmp(&b.level)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.rule.cmp(&b.rule))
    });

    Ok(ValidationResult {
        diagnostics,
        files_checked,
    })
}

fn resolve_validation_root(path: &Path) -> PathBuf {
    let candidate = if path.is_file() {
        path.parent().unwrap_or(Path::new("."))
    } else {
        path
    };
    std::fs::canonicalize(candidate).unwrap_or_else(|_| candidate.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_root() -> &'static Path {
        use std::sync::OnceLock;

        static ROOT: OnceLock<PathBuf> = OnceLock::new();
        ROOT.get_or_init(|| {
            let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            for ancestor in manifest_dir.ancestors() {
                let cargo_toml = ancestor.join("Cargo.toml");
                if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                    if content.contains("[workspace]") || content.contains("[workspace.") {
                        return ancestor.to_path_buf();
                    }
                }
            }
            panic!(
                "Failed to locate workspace root from CARGO_MANIFEST_DIR={}",
                manifest_dir.display()
            );
        })
        .as_path()
    }

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
    fn test_detect_instruction_variants() {
        // CLAUDE.local.md variant
        assert_eq!(
            detect_file_type(Path::new("CLAUDE.local.md")),
            FileType::ClaudeMd
        );
        assert_eq!(
            detect_file_type(Path::new("project/CLAUDE.local.md")),
            FileType::ClaudeMd
        );

        // AGENTS.local.md variant
        assert_eq!(
            detect_file_type(Path::new("AGENTS.local.md")),
            FileType::ClaudeMd
        );
        assert_eq!(
            detect_file_type(Path::new("subdir/AGENTS.local.md")),
            FileType::ClaudeMd
        );

        // AGENTS.override.md variant
        assert_eq!(
            detect_file_type(Path::new("AGENTS.override.md")),
            FileType::ClaudeMd
        );
        assert_eq!(
            detect_file_type(Path::new("deep/nested/AGENTS.override.md")),
            FileType::ClaudeMd
        );
    }

    #[test]
    fn test_repo_agents_md_matches_claude_md() {
        let repo_root = workspace_root();

        let claude_path = repo_root.join("CLAUDE.md");
        let claude = std::fs::read_to_string(&claude_path).unwrap_or_else(|e| {
            panic!("Failed to read CLAUDE.md at {}: {e}", claude_path.display());
        });
        let agents_path = repo_root.join("AGENTS.md");
        let agents = std::fs::read_to_string(&agents_path).unwrap_or_else(|e| {
            panic!("Failed to read AGENTS.md at {}: {e}", agents_path.display());
        });

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
        let registry = ValidatorRegistry::with_defaults();
        let validators = registry.validators_for(FileType::Skill);
        assert_eq!(validators.len(), 3);
    }

    #[test]
    fn test_validators_for_claude_md() {
        let registry = ValidatorRegistry::with_defaults();
        let validators = registry.validators_for(FileType::ClaudeMd);
        assert_eq!(validators.len(), 6);
    }

    #[test]
    fn test_validators_for_mcp() {
        let registry = ValidatorRegistry::with_defaults();
        let validators = registry.validators_for(FileType::Mcp);
        assert_eq!(validators.len(), 1);
    }

    #[test]
    fn test_validators_for_unknown() {
        let registry = ValidatorRegistry::with_defaults();
        let validators = registry.validators_for(FileType::Unknown);
        assert_eq!(validators.len(), 0);
    }

    #[test]
    fn test_validate_file_with_custom_registry() {
        struct DummyValidator;

        impl Validator for DummyValidator {
            fn validate(
                &self,
                path: &Path,
                _content: &str,
                _config: &LintConfig,
            ) -> Vec<Diagnostic> {
                vec![Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    1,
                    "TEST-001",
                    "Registry override".to_string(),
                )]
            }
        }

        let temp = tempfile::TempDir::new().unwrap();
        let skill_path = temp.path().join("SKILL.md");
        std::fs::write(&skill_path, "---\nname: test\n---\nBody").unwrap();

        let mut registry = ValidatorRegistry::new();
        registry.register(FileType::Skill, || Box::new(DummyValidator));

        let diagnostics =
            validate_file_with_registry(&skill_path, &LintConfig::default(), &registry).unwrap();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, "TEST-001");
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
        let result = validate_project(temp.path(), &config).unwrap();

        assert!(!result.diagnostics.is_empty());
    }

    #[test]
    fn test_validate_project_empty_dir() {
        let temp = tempfile::TempDir::new().unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        assert!(result.diagnostics.is_empty());
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
        let result = validate_project(temp.path(), &config).unwrap();

        for i in 1..result.diagnostics.len() {
            assert!(result.diagnostics[i - 1].level <= result.diagnostics[i].level);
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
                first_result.diagnostics.len(),
                result.diagnostics.len(),
                "Run {} produced different number of diagnostics",
                run
            );

            for (i, (a, b)) in first_result
                .diagnostics
                .iter()
                .zip(result.diagnostics.iter())
                .enumerate()
            {
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
            !first_result.diagnostics.is_empty(),
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
        let result = validate_project(temp.path(), &config).unwrap();

        // Should have at least one diagnostic for the dangerous name (CC-SK-006)
        assert!(
            result.diagnostics.iter().any(|d| d.rule == "CC-SK-006"),
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
        let result = validate_project(temp.path(), &config).unwrap();

        // Should have diagnostics only from the invalid skill
        let error_diagnostics: Vec<_> = result
            .diagnostics
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
    fn test_validate_project_agents_md_collection() {
        // Verify that validation correctly collects AGENTS.md paths for AGM-006
        let temp = tempfile::TempDir::new().unwrap();

        // Create multiple AGENTS.md files in different directories
        std::fs::write(temp.path().join("AGENTS.md"), "# Root agents").unwrap();

        let subdir = temp.path().join("subproject");
        std::fs::create_dir_all(&subdir).unwrap();
        std::fs::write(subdir.join("AGENTS.md"), "# Subproject agents").unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        // Should have AGM-006 warnings for both AGENTS.md files
        let agm006_diagnostics: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "AGM-006")
            .collect();

        assert_eq!(
            agm006_diagnostics.len(),
            2,
            "Expected AGM-006 diagnostic for each AGENTS.md file, got: {:?}",
            agm006_diagnostics
        );
    }

    #[test]
    fn test_validate_project_files_checked_count() {
        // Verify that validation correctly counts recognized file types
        let temp = tempfile::TempDir::new().unwrap();

        // Create recognized file types
        std::fs::write(
            temp.path().join("SKILL.md"),
            "---\nname: test-skill\ndescription: Test skill\n---\nBody",
        )
        .unwrap();
        std::fs::write(temp.path().join("CLAUDE.md"), "# Project memory").unwrap();

        // Create unrecognized file types (should not be counted)
        // Note: .md files are GenericMarkdown (recognized), so use non-markdown extensions
        std::fs::write(temp.path().join("notes.txt"), "Some notes").unwrap();
        std::fs::write(temp.path().join("data.json"), "{}").unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        // files_checked should only count recognized types (SKILL.md + CLAUDE.md = 2)
        // .txt and .json (not matching MCP patterns) are FileType::Unknown
        assert_eq!(
            result.files_checked, 2,
            "files_checked should count only recognized file types, got {}",
            result.files_checked
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
        let result = validate_project(temp.path(), &config).unwrap();

        // Should detect the plugin.json and report CC-PL-004 for missing description
        let plugin_diagnostics: Vec<_> = result
            .diagnostics
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
        let result = validate_project(temp.path(), &config).unwrap();

        // Should detect the MCP file and report issues
        let mcp_diagnostics: Vec<_> = result
            .diagnostics
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
        let result = validate_project(temp.path(), &config).unwrap();

        // Should detect XP-001 error for Claude-specific hooks in AGENTS.md
        let xp_001: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "XP-001")
            .collect();
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
        let result = validate_project(temp.path(), &config).unwrap();

        // Should detect XP-001 errors for Claude-specific features
        let xp_001: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "XP-001")
            .collect();
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
        let result = validate_project(temp.path(), &config).unwrap();

        // Should detect XP-002 warning for missing headers
        let xp_002: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "XP-002")
            .collect();
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
        let result = validate_project(temp.path(), &config).unwrap();

        // Should detect XP-003 warnings for hard-coded paths
        let xp_003: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "XP-003")
            .collect();
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
        let result = validate_project(temp.path(), &config).unwrap();

        // Should have no XP-* diagnostics
        let xp_rules: Vec<_> = result
            .diagnostics
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
        let result = validate_project(temp.path(), &config).unwrap();

        // XP-001 should NOT fire for CLAUDE.md (Claude features are allowed there)
        let xp_001: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "XP-001")
            .collect();
        assert!(
            xp_001.is_empty(),
            "CLAUDE.md should be allowed to have Claude-specific features"
        );
    }

    // ===== AGM-006: Multiple AGENTS.md Tests =====

    #[test]
    fn test_agm_006_nested_agents_md() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create nested AGENTS.md files
        std::fs::write(
            temp.path().join("AGENTS.md"),
            "# Project\n\nThis project does something.",
        )
        .unwrap();

        let subdir = temp.path().join("subdir");
        std::fs::create_dir_all(&subdir).unwrap();
        std::fs::write(
            subdir.join("AGENTS.md"),
            "# Subproject\n\nThis is a nested AGENTS.md.",
        )
        .unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        // Should detect AGM-006 for both AGENTS.md files
        let agm_006: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "AGM-006")
            .collect();
        assert_eq!(
            agm_006.len(),
            2,
            "Should detect both AGENTS.md files, got {:?}",
            agm_006
        );
        assert!(agm_006
            .iter()
            .any(|d| d.file.to_string_lossy().contains("subdir")));
        assert!(agm_006
            .iter()
            .any(|d| d.message.contains("Nested AGENTS.md")));
        assert!(agm_006
            .iter()
            .any(|d| d.message.contains("Multiple AGENTS.md files")));
    }

    #[test]
    fn test_agm_006_no_nesting() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create single AGENTS.md file
        std::fs::write(
            temp.path().join("AGENTS.md"),
            "# Project\n\nThis project does something.",
        )
        .unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        // Should not detect AGM-006 for a single AGENTS.md
        let agm_006: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "AGM-006")
            .collect();
        assert!(
            agm_006.is_empty(),
            "Single AGENTS.md should not trigger AGM-006"
        );
    }

    #[test]
    fn test_agm_006_multiple_agents_md() {
        let temp = tempfile::TempDir::new().unwrap();

        let app_a = temp.path().join("app-a");
        let app_b = temp.path().join("app-b");
        std::fs::create_dir_all(&app_a).unwrap();
        std::fs::create_dir_all(&app_b).unwrap();

        std::fs::write(
            app_a.join("AGENTS.md"),
            "# App A\n\nThis project does something.",
        )
        .unwrap();
        std::fs::write(
            app_b.join("AGENTS.md"),
            "# App B\n\nThis project does something.",
        )
        .unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        let agm_006: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "AGM-006")
            .collect();
        assert_eq!(
            agm_006.len(),
            2,
            "Should detect both AGENTS.md files, got {:?}",
            agm_006
        );
        assert!(agm_006
            .iter()
            .all(|d| d.message.contains("Multiple AGENTS.md files")));
    }

    #[test]
    fn test_agm_006_disabled() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create nested AGENTS.md files
        std::fs::write(
            temp.path().join("AGENTS.md"),
            "# Project\n\nThis project does something.",
        )
        .unwrap();

        let subdir = temp.path().join("subdir");
        std::fs::create_dir_all(&subdir).unwrap();
        std::fs::write(
            subdir.join("AGENTS.md"),
            "# Subproject\n\nThis is a nested AGENTS.md.",
        )
        .unwrap();

        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["AGM-006".to_string()];
        let result = validate_project(temp.path(), &config).unwrap();

        // Should not detect AGM-006 when disabled
        let agm_006: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "AGM-006")
            .collect();
        assert!(agm_006.is_empty(), "AGM-006 should not fire when disabled");
    }

    // ===== XP-004: Conflicting Build Commands =====

    #[test]
    fn test_xp_004_conflicting_package_managers() {
        let temp = tempfile::TempDir::new().unwrap();

        // CLAUDE.md uses npm
        std::fs::write(
            temp.path().join("CLAUDE.md"),
            "# Project\n\nUse `npm install` for dependencies.",
        )
        .unwrap();

        // AGENTS.md uses pnpm
        std::fs::write(
            temp.path().join("AGENTS.md"),
            "# Project\n\nUse `pnpm install` for dependencies.",
        )
        .unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        let xp_004: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "XP-004")
            .collect();
        assert!(
            !xp_004.is_empty(),
            "Should detect conflicting package managers"
        );
        assert!(xp_004.iter().any(|d| d.message.contains("npm")));
        assert!(xp_004.iter().any(|d| d.message.contains("pnpm")));
    }

    #[test]
    fn test_xp_004_no_conflict_same_manager() {
        let temp = tempfile::TempDir::new().unwrap();

        // Both files use npm
        std::fs::write(
            temp.path().join("CLAUDE.md"),
            "# Project\n\nUse `npm install` for dependencies.",
        )
        .unwrap();

        std::fs::write(
            temp.path().join("AGENTS.md"),
            "# Project\n\nUse `npm run build` for building.",
        )
        .unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        let xp_004: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "XP-004")
            .collect();
        assert!(
            xp_004.is_empty(),
            "Should not detect conflict when same package manager is used"
        );
    }

    // ===== XP-005: Conflicting Tool Constraints =====

    #[test]
    fn test_xp_005_conflicting_tool_constraints() {
        let temp = tempfile::TempDir::new().unwrap();

        // CLAUDE.md allows Bash
        std::fs::write(
            temp.path().join("CLAUDE.md"),
            "# Project\n\nallowed-tools: Read Write Bash",
        )
        .unwrap();

        // AGENTS.md disallows Bash
        std::fs::write(
            temp.path().join("AGENTS.md"),
            "# Project\n\nNever use Bash for operations.",
        )
        .unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        let xp_005: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "XP-005")
            .collect();
        assert!(
            !xp_005.is_empty(),
            "Should detect conflicting tool constraints"
        );
        assert!(xp_005.iter().any(|d| d.message.contains("Bash")));
    }

    #[test]
    fn test_xp_005_no_conflict_consistent_constraints() {
        let temp = tempfile::TempDir::new().unwrap();

        // Both files allow Read
        std::fs::write(
            temp.path().join("CLAUDE.md"),
            "# Project\n\nallowed-tools: Read Write",
        )
        .unwrap();

        std::fs::write(
            temp.path().join("AGENTS.md"),
            "# Project\n\nYou can use Read for file access.",
        )
        .unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        let xp_005: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "XP-005")
            .collect();
        assert!(
            xp_005.is_empty(),
            "Should not detect conflict when constraints are consistent"
        );
    }

    // ===== XP-006: Layer Precedence =====

    #[test]
    fn test_xp_006_no_precedence_documentation() {
        let temp = tempfile::TempDir::new().unwrap();

        // Both files exist but neither documents precedence
        std::fs::write(
            temp.path().join("CLAUDE.md"),
            "# Project\n\nThis is Claude.md.",
        )
        .unwrap();

        std::fs::write(
            temp.path().join("AGENTS.md"),
            "# Project\n\nThis is Agents.md.",
        )
        .unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        let xp_006: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "XP-006")
            .collect();
        assert!(
            !xp_006.is_empty(),
            "Should detect missing precedence documentation"
        );
    }

    #[test]
    fn test_xp_006_with_precedence_documentation() {
        let temp = tempfile::TempDir::new().unwrap();

        // CLAUDE.md documents precedence
        std::fs::write(
            temp.path().join("CLAUDE.md"),
            "# Project\n\nCLAUDE.md takes precedence over AGENTS.md.",
        )
        .unwrap();

        std::fs::write(
            temp.path().join("AGENTS.md"),
            "# Project\n\nThis is Agents.md.",
        )
        .unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        let xp_006: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "XP-006")
            .collect();
        assert!(
            xp_006.is_empty(),
            "Should not trigger XP-006 when precedence is documented"
        );
    }

    #[test]
    fn test_xp_006_single_layer_no_issue() {
        let temp = tempfile::TempDir::new().unwrap();

        // Only CLAUDE.md exists
        std::fs::write(
            temp.path().join("CLAUDE.md"),
            "# Project\n\nThis is Claude.md.",
        )
        .unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        let xp_006: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "XP-006")
            .collect();
        assert!(
            xp_006.is_empty(),
            "Should not trigger XP-006 with single instruction layer"
        );
    }

    // ===== XP-004/005/006 Edge Case Tests (review findings) =====

    #[test]
    fn test_xp_004_three_files_conflicting_managers() {
        let temp = tempfile::TempDir::new().unwrap();

        // CLAUDE.md uses npm
        std::fs::write(
            temp.path().join("CLAUDE.md"),
            "# Project\n\nUse `npm install` for dependencies.",
        )
        .unwrap();

        // AGENTS.md uses pnpm
        std::fs::write(
            temp.path().join("AGENTS.md"),
            "# Project\n\nUse `pnpm install` for dependencies.",
        )
        .unwrap();

        // Add .cursor rules directory with yarn
        let cursor_dir = temp.path().join(".cursor").join("rules");
        std::fs::create_dir_all(&cursor_dir).unwrap();
        std::fs::write(
            cursor_dir.join("dev.mdc"),
            "# Rules\n\nUse `yarn install` for dependencies.",
        )
        .unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        let xp_004: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "XP-004")
            .collect();

        // Should detect conflicts between all three different package managers
        assert!(
            xp_004.len() >= 2,
            "Should detect multiple conflicts with 3 different package managers, got {}",
            xp_004.len()
        );
    }

    #[test]
    fn test_xp_004_disabled_rule() {
        let temp = tempfile::TempDir::new().unwrap();

        // CLAUDE.md uses npm
        std::fs::write(
            temp.path().join("CLAUDE.md"),
            "# Project\n\nUse `npm install` for dependencies.",
        )
        .unwrap();

        // AGENTS.md uses pnpm
        std::fs::write(
            temp.path().join("AGENTS.md"),
            "# Project\n\nUse `pnpm install` for dependencies.",
        )
        .unwrap();

        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["XP-004".to_string()];
        let result = validate_project(temp.path(), &config).unwrap();

        let xp_004: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "XP-004")
            .collect();
        assert!(xp_004.is_empty(), "XP-004 should not fire when disabled");
    }

    #[test]
    fn test_xp_005_disabled_rule() {
        let temp = tempfile::TempDir::new().unwrap();

        // CLAUDE.md allows Bash
        std::fs::write(
            temp.path().join("CLAUDE.md"),
            "# Project\n\nallowed-tools: Read Write Bash",
        )
        .unwrap();

        // AGENTS.md disallows Bash
        std::fs::write(
            temp.path().join("AGENTS.md"),
            "# Project\n\nNever use Bash for operations.",
        )
        .unwrap();

        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["XP-005".to_string()];
        let result = validate_project(temp.path(), &config).unwrap();

        let xp_005: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "XP-005")
            .collect();
        assert!(xp_005.is_empty(), "XP-005 should not fire when disabled");
    }

    #[test]
    fn test_xp_006_disabled_rule() {
        let temp = tempfile::TempDir::new().unwrap();

        // Both files exist but neither documents precedence
        std::fs::write(
            temp.path().join("CLAUDE.md"),
            "# Project\n\nThis is Claude.md.",
        )
        .unwrap();

        std::fs::write(
            temp.path().join("AGENTS.md"),
            "# Project\n\nThis is Agents.md.",
        )
        .unwrap();

        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["XP-006".to_string()];
        let result = validate_project(temp.path(), &config).unwrap();

        let xp_006: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "XP-006")
            .collect();
        assert!(xp_006.is_empty(), "XP-006 should not fire when disabled");
    }

    #[test]
    fn test_xp_empty_instruction_files() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create empty CLAUDE.md and AGENTS.md
        std::fs::write(temp.path().join("CLAUDE.md"), "").unwrap();
        std::fs::write(temp.path().join("AGENTS.md"), "").unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        // XP-004 should not fire for empty files (no commands)
        let xp_004: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "XP-004")
            .collect();
        assert!(xp_004.is_empty(), "Empty files should not trigger XP-004");

        // XP-005 should not fire for empty files (no constraints)
        let xp_005: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "XP-005")
            .collect();
        assert!(xp_005.is_empty(), "Empty files should not trigger XP-005");
    }

    #[test]
    fn test_xp_005_case_insensitive_tool_matching() {
        let temp = tempfile::TempDir::new().unwrap();

        // CLAUDE.md allows BASH (uppercase)
        std::fs::write(
            temp.path().join("CLAUDE.md"),
            "# Project\n\nallowed-tools: Read Write BASH",
        )
        .unwrap();

        // AGENTS.md disallows bash (lowercase)
        std::fs::write(
            temp.path().join("AGENTS.md"),
            "# Project\n\nNever use bash for operations.",
        )
        .unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        let xp_005: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "XP-005")
            .collect();
        assert!(
            !xp_005.is_empty(),
            "Should detect conflict between BASH and bash (case-insensitive)"
        );
    }

    #[test]
    fn test_xp_005_word_boundary_no_false_positive() {
        let temp = tempfile::TempDir::new().unwrap();

        // CLAUDE.md allows Bash
        std::fs::write(
            temp.path().join("CLAUDE.md"),
            "# Project\n\nallowed-tools: Read Write Bash",
        )
        .unwrap();

        // AGENTS.md mentions "subash" (not "Bash")
        std::fs::write(
            temp.path().join("AGENTS.md"),
            "# Project\n\nNever use subash command.",
        )
        .unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        let xp_005: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "XP-005")
            .collect();
        assert!(
            xp_005.is_empty(),
            "Should NOT detect conflict - 'subash' is not 'Bash'"
        );
    }

    // ===== AGM Validation Integration Tests =====

    #[test]
    fn test_agm_001_unclosed_code_block() {
        let temp = tempfile::TempDir::new().unwrap();

        std::fs::write(
            temp.path().join("AGENTS.md"),
            "# Project\n\n```rust\nfn main() {}",
        )
        .unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        let agm_001: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "AGM-001")
            .collect();
        assert!(!agm_001.is_empty(), "Should detect unclosed code block");
    }

    #[test]
    fn test_agm_003_over_char_limit() {
        let temp = tempfile::TempDir::new().unwrap();

        let content = format!("# Project\n\n{}", "x".repeat(13000));
        std::fs::write(temp.path().join("AGENTS.md"), content).unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        let agm_003: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "AGM-003")
            .collect();
        assert!(
            !agm_003.is_empty(),
            "Should detect character limit exceeded"
        );
    }

    #[test]
    fn test_agm_005_unguarded_platform_features() {
        let temp = tempfile::TempDir::new().unwrap();

        std::fs::write(
            temp.path().join("AGENTS.md"),
            "# Project\n\n- type: PreToolExecution\n  command: echo test",
        )
        .unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        let agm_005: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "AGM-005")
            .collect();
        assert!(
            !agm_005.is_empty(),
            "Should detect unguarded platform features"
        );
    }

    #[test]
    fn test_valid_agents_md_no_agm_errors() {
        let temp = tempfile::TempDir::new().unwrap();

        std::fs::write(
            temp.path().join("AGENTS.md"),
            r#"# Project

This project is a linter for agent configurations.

## Build Commands

Run npm install and npm build.

## Claude Code Specific

- type: PreToolExecution
  command: echo "test"
"#,
        )
        .unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        let agm_errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("AGM-") && d.level == DiagnosticLevel::Error)
            .collect();
        assert!(
            agm_errors.is_empty(),
            "Valid AGENTS.md should have no AGM-* errors, got: {:?}",
            agm_errors
        );
    }
    // ===== Fixture Directory Regression Tests =====

    /// Helper to locate the fixtures directory for testing
    fn get_fixtures_dir() -> PathBuf {
        workspace_root().join("tests").join("fixtures")
    }

    #[test]
    fn test_validate_fixtures_directory() {
        // Run validate_project() over tests/fixtures/ to verify detect_file_type() works
        // This is a regression guard for fixture layout (issue #74)
        let fixtures_dir = get_fixtures_dir();

        let config = LintConfig::default();
        let result = validate_project(&fixtures_dir, &config).unwrap();

        // Verify skill fixtures trigger expected AS-* rules
        let skill_diagnostics: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("AS-"))
            .collect();

        // deep-reference/SKILL.md should trigger AS-013 (reference too deep)
        assert!(
            skill_diagnostics
                .iter()
                .any(|d| d.rule == "AS-013" && d.file.to_string_lossy().contains("deep-reference")),
            "Expected AS-013 from deep-reference/SKILL.md fixture"
        );

        // missing-frontmatter/SKILL.md should trigger AS-001 (missing frontmatter)
        assert!(
            skill_diagnostics
                .iter()
                .any(|d| d.rule == "AS-001"
                    && d.file.to_string_lossy().contains("missing-frontmatter")),
            "Expected AS-001 from missing-frontmatter/SKILL.md fixture"
        );

        // windows-path/SKILL.md should trigger AS-014 (windows path separator)
        assert!(
            skill_diagnostics
                .iter()
                .any(|d| d.rule == "AS-014" && d.file.to_string_lossy().contains("windows-path")),
            "Expected AS-014 from windows-path/SKILL.md fixture"
        );

        // Verify MCP fixtures trigger expected MCP-* rules
        let mcp_diagnostics: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("MCP-"))
            .collect();

        // At least some MCP diagnostics should be present
        assert!(
            !mcp_diagnostics.is_empty(),
            "Expected MCP diagnostics from tests/fixtures/mcp/*.mcp.json files"
        );

        // missing-required-fields.mcp.json should trigger MCP-002 (missing description)
        assert!(
            mcp_diagnostics.iter().any(|d| d.rule == "MCP-002"
                && d.file.to_string_lossy().contains("missing-required-fields")),
            "Expected MCP-002 from missing-required-fields.mcp.json fixture"
        );

        // empty-description.mcp.json should trigger MCP-004 (short description)
        assert!(
            mcp_diagnostics
                .iter()
                .any(|d| d.rule == "MCP-004"
                    && d.file.to_string_lossy().contains("empty-description")),
            "Expected MCP-004 from empty-description.mcp.json fixture"
        );

        // invalid-input-schema.mcp.json should trigger MCP-003 (invalid schema)
        assert!(
            mcp_diagnostics.iter().any(|d| d.rule == "MCP-003"
                && d.file.to_string_lossy().contains("invalid-input-schema")),
            "Expected MCP-003 from invalid-input-schema.mcp.json fixture"
        );

        // invalid-jsonrpc-version.mcp.json should trigger MCP-001 (invalid jsonrpc)
        assert!(
            mcp_diagnostics.iter().any(|d| d.rule == "MCP-001"
                && d.file.to_string_lossy().contains("invalid-jsonrpc-version")),
            "Expected MCP-001 from invalid-jsonrpc-version.mcp.json fixture"
        );

        // missing-consent.mcp.json should trigger MCP-005 (missing consent)
        assert!(
            mcp_diagnostics.iter().any(
                |d| d.rule == "MCP-005" && d.file.to_string_lossy().contains("missing-consent")
            ),
            "Expected MCP-005 from missing-consent.mcp.json fixture"
        );

        // untrusted-annotations.mcp.json should trigger MCP-006 (untrusted annotations)
        assert!(
            mcp_diagnostics.iter().any(|d| d.rule == "MCP-006"
                && d.file.to_string_lossy().contains("untrusted-annotations")),
            "Expected MCP-006 from untrusted-annotations.mcp.json fixture"
        );

        // Verify AGM, XP, REF, and XML fixtures trigger expected rules
        let expectations = [
            (
                "AGM-002",
                "no-headers",
                "Expected AGM-002 from agents_md/no-headers/AGENTS.md fixture",
            ),
            (
                "XP-003",
                "hard-coded",
                "Expected XP-003 from cross_platform/hard-coded/AGENTS.md fixture",
            ),
            (
                "REF-001",
                "missing-import",
                "Expected REF-001 from refs/missing-import.md fixture",
            ),
            (
                "REF-002",
                "broken-link",
                "Expected REF-002 from refs/broken-link.md fixture",
            ),
            (
                "XML-001",
                "xml-001-unclosed",
                "Expected XML-001 from xml/xml-001-unclosed.md fixture",
            ),
            (
                "XML-002",
                "xml-002-mismatch",
                "Expected XML-002 from xml/xml-002-mismatch.md fixture",
            ),
            (
                "XML-003",
                "xml-003-unmatched",
                "Expected XML-003 from xml/xml-003-unmatched.md fixture",
            ),
        ];

        for (rule, file_part, message) in expectations {
            assert!(
                result
                    .diagnostics
                    .iter()
                    .any(|d| { d.rule == rule && d.file.to_string_lossy().contains(file_part) }),
                "{}",
                message
            );
        }
    }

    #[test]
    fn test_fixture_positive_cases_by_family() {
        let fixtures_dir = get_fixtures_dir();
        let config = LintConfig::default();

        let temp = tempfile::TempDir::new().unwrap();
        let pe_source = fixtures_dir.join("valid/pe/prompt-complete-valid.md");
        let pe_content = std::fs::read_to_string(&pe_source)
            .unwrap_or_else(|_| panic!("Failed to read {}", pe_source.display()));
        let pe_path = temp.path().join("CLAUDE.md");
        std::fs::write(&pe_path, pe_content).unwrap();

        let mut cases = vec![
            ("AGM-", fixtures_dir.join("agents_md/valid/AGENTS.md")),
            ("XP-", fixtures_dir.join("cross_platform/valid/AGENTS.md")),
            ("MCP-", fixtures_dir.join("mcp/valid-tool.mcp.json")),
            ("REF-", fixtures_dir.join("refs/valid-links.md")),
            ("XML-", fixtures_dir.join("xml/xml-valid.md")),
        ];
        cases.push(("PE-", pe_path));

        for (prefix, path) in cases {
            let diagnostics = validate_file(&path, &config).unwrap();
            let family_diagnostics: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule.starts_with(prefix))
                .collect();

            assert!(
                family_diagnostics.is_empty(),
                "Expected no {} diagnostics for fixture {}",
                prefix,
                path.display()
            );
        }
    }

    #[test]
    fn test_fixture_file_type_detection() {
        // Verify that fixture files are detected as correct FileType
        let fixtures_dir = get_fixtures_dir();

        // Skill fixtures should be detected as FileType::Skill
        assert_eq!(
            detect_file_type(&fixtures_dir.join("skills/deep-reference/SKILL.md")),
            FileType::Skill,
            "deep-reference/SKILL.md should be detected as Skill"
        );
        assert_eq!(
            detect_file_type(&fixtures_dir.join("skills/missing-frontmatter/SKILL.md")),
            FileType::Skill,
            "missing-frontmatter/SKILL.md should be detected as Skill"
        );
        assert_eq!(
            detect_file_type(&fixtures_dir.join("skills/windows-path/SKILL.md")),
            FileType::Skill,
            "windows-path/SKILL.md should be detected as Skill"
        );

        // MCP fixtures should be detected as FileType::Mcp
        assert_eq!(
            detect_file_type(&fixtures_dir.join("mcp/valid-tool.mcp.json")),
            FileType::Mcp,
            "valid-tool.mcp.json should be detected as Mcp"
        );
        assert_eq!(
            detect_file_type(&fixtures_dir.join("mcp/empty-description.mcp.json")),
            FileType::Mcp,
            "empty-description.mcp.json should be detected as Mcp"
        );

        // Copilot fixtures should be detected as FileType::Copilot or CopilotScoped
        assert_eq!(
            detect_file_type(&fixtures_dir.join("copilot/.github/copilot-instructions.md")),
            FileType::Copilot,
            "copilot-instructions.md should be detected as Copilot"
        );
        assert_eq!(
            detect_file_type(
                &fixtures_dir.join("copilot/.github/instructions/typescript.instructions.md")
            ),
            FileType::CopilotScoped,
            "typescript.instructions.md should be detected as CopilotScoped"
        );
    }

    // ===== GitHub Copilot Validation Integration Tests =====

    #[test]
    fn test_detect_copilot_global() {
        assert_eq!(
            detect_file_type(Path::new(".github/copilot-instructions.md")),
            FileType::Copilot
        );
        assert_eq!(
            detect_file_type(Path::new("project/.github/copilot-instructions.md")),
            FileType::Copilot
        );
    }

    #[test]
    fn test_detect_copilot_scoped() {
        assert_eq!(
            detect_file_type(Path::new(".github/instructions/typescript.instructions.md")),
            FileType::CopilotScoped
        );
        assert_eq!(
            detect_file_type(Path::new(
                "project/.github/instructions/rust.instructions.md"
            )),
            FileType::CopilotScoped
        );
    }

    #[test]
    fn test_copilot_not_detected_outside_github() {
        // Files outside .github/ should not be detected as Copilot
        assert_ne!(
            detect_file_type(Path::new("copilot-instructions.md")),
            FileType::Copilot
        );
        assert_ne!(
            detect_file_type(Path::new("instructions/typescript.instructions.md")),
            FileType::CopilotScoped
        );
    }

    #[test]
    fn test_validators_for_copilot() {
        let registry = ValidatorRegistry::with_defaults();

        let copilot_validators = registry.validators_for(FileType::Copilot);
        assert_eq!(copilot_validators.len(), 2); // copilot + xml

        let scoped_validators = registry.validators_for(FileType::CopilotScoped);
        assert_eq!(scoped_validators.len(), 2); // copilot + xml
    }

    #[test]
    fn test_validate_copilot_fixtures() {
        // Use validate_file directly since .github is a hidden directory
        // that ignore::WalkBuilder skips by default
        let fixtures_dir = get_fixtures_dir();
        let copilot_dir = fixtures_dir.join("copilot");

        let config = LintConfig::default();

        // Validate global instructions
        let global_path = copilot_dir.join(".github/copilot-instructions.md");
        let diagnostics = validate_file(&global_path, &config).unwrap();
        let cop_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("COP-") && d.level == DiagnosticLevel::Error)
            .collect();
        assert!(
            cop_errors.is_empty(),
            "Valid global file should have no COP errors, got: {:?}",
            cop_errors
        );

        // Validate scoped instructions
        let scoped_path = copilot_dir.join(".github/instructions/typescript.instructions.md");
        let diagnostics = validate_file(&scoped_path, &config).unwrap();
        let cop_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("COP-") && d.level == DiagnosticLevel::Error)
            .collect();
        assert!(
            cop_errors.is_empty(),
            "Valid scoped file should have no COP errors, got: {:?}",
            cop_errors
        );
    }

    #[test]
    fn test_validate_copilot_invalid_fixtures() {
        // Use validate_file directly since .github is a hidden directory
        let fixtures_dir = get_fixtures_dir();
        let copilot_invalid_dir = fixtures_dir.join("copilot-invalid");
        let config = LintConfig::default();

        // COP-001: Empty global file
        let empty_global = copilot_invalid_dir.join(".github/copilot-instructions.md");
        let diagnostics = validate_file(&empty_global, &config).unwrap();
        assert!(
            diagnostics.iter().any(|d| d.rule == "COP-001"),
            "Expected COP-001 from empty copilot-instructions.md fixture"
        );

        // COP-002: Invalid YAML in bad-frontmatter
        let bad_frontmatter =
            copilot_invalid_dir.join(".github/instructions/bad-frontmatter.instructions.md");
        let diagnostics = validate_file(&bad_frontmatter, &config).unwrap();
        assert!(
            diagnostics.iter().any(|d| d.rule == "COP-002"),
            "Expected COP-002 from bad-frontmatter.instructions.md fixture"
        );

        // COP-003: Invalid glob in bad-glob
        let bad_glob = copilot_invalid_dir.join(".github/instructions/bad-glob.instructions.md");
        let diagnostics = validate_file(&bad_glob, &config).unwrap();
        assert!(
            diagnostics.iter().any(|d| d.rule == "COP-003"),
            "Expected COP-003 from bad-glob.instructions.md fixture"
        );

        // COP-004: Unknown keys in unknown-keys
        let unknown_keys =
            copilot_invalid_dir.join(".github/instructions/unknown-keys.instructions.md");
        let diagnostics = validate_file(&unknown_keys, &config).unwrap();
        assert!(
            diagnostics.iter().any(|d| d.rule == "COP-004"),
            "Expected COP-004 from unknown-keys.instructions.md fixture"
        );
    }

    #[test]
    fn test_validate_copilot_file_empty() {
        // Test validate_file directly (not validate_project which skips hidden dirs)
        let temp = tempfile::TempDir::new().unwrap();
        let github_dir = temp.path().join(".github");
        std::fs::create_dir_all(&github_dir).unwrap();
        let file_path = github_dir.join("copilot-instructions.md");
        std::fs::write(&file_path, "").unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_file(&file_path, &config).unwrap();

        let cop_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-001").collect();
        assert_eq!(cop_001.len(), 1, "Expected COP-001 for empty file");
    }

    #[test]
    fn test_validate_copilot_scoped_missing_frontmatter() {
        // Test validate_file directly
        let temp = tempfile::TempDir::new().unwrap();
        let instructions_dir = temp.path().join(".github").join("instructions");
        std::fs::create_dir_all(&instructions_dir).unwrap();
        let file_path = instructions_dir.join("test.instructions.md");
        std::fs::write(&file_path, "# Instructions without frontmatter").unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_file(&file_path, &config).unwrap();

        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        assert_eq!(cop_002.len(), 1, "Expected COP-002 for missing frontmatter");
    }

    #[test]
    fn test_validate_copilot_valid_scoped() {
        // Test validate_file directly
        let temp = tempfile::TempDir::new().unwrap();
        let instructions_dir = temp.path().join(".github").join("instructions");
        std::fs::create_dir_all(&instructions_dir).unwrap();
        let file_path = instructions_dir.join("rust.instructions.md");
        std::fs::write(
            &file_path,
            r#"---
applyTo: "**/*.rs"
---
# Rust Instructions

Use idiomatic Rust patterns.
"#,
        )
        .unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_file(&file_path, &config).unwrap();

        let cop_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("COP-") && d.level == DiagnosticLevel::Error)
            .collect();
        assert!(
            cop_errors.is_empty(),
            "Valid scoped file should have no COP errors"
        );
    }

    #[test]
    fn test_validate_project_finds_github_hidden_dir() {
        // Test validate_project walks .github directory (not just validate_file)
        let temp = tempfile::TempDir::new().unwrap();
        let github_dir = temp.path().join(".github");
        std::fs::create_dir_all(&github_dir).unwrap();

        // Create an empty copilot-instructions.md file (should trigger COP-001)
        let file_path = github_dir.join("copilot-instructions.md");
        std::fs::write(&file_path, "").unwrap();

        let config = LintConfig::default();
        // Use validate_project (directory walk) instead of validate_file
        let result = validate_project(temp.path(), &config).unwrap();

        assert!(
            result.diagnostics.iter().any(|d| d.rule == "COP-001"),
            "validate_project should find .github/copilot-instructions.md and report COP-001. Found: {:?}",
            result.diagnostics.iter().map(|d| &d.rule).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_validate_project_finds_copilot_invalid_fixtures() {
        // Test validate_project on the actual fixture directory
        let fixtures_dir = get_fixtures_dir();
        let copilot_invalid_dir = fixtures_dir.join("copilot-invalid");

        let config = LintConfig::default();
        let result = validate_project(&copilot_invalid_dir, &config).unwrap();

        // Should find COP-001 from empty copilot-instructions.md
        assert!(
            result.diagnostics.iter().any(|d| d.rule == "COP-001"),
            "validate_project should find COP-001 in copilot-invalid fixtures. Found rules: {:?}",
            result
                .diagnostics
                .iter()
                .map(|d| &d.rule)
                .collect::<Vec<_>>()
        );

        // Should find COP-002 from bad-frontmatter.instructions.md
        assert!(
            result.diagnostics.iter().any(|d| d.rule == "COP-002"),
            "validate_project should find COP-002 in copilot-invalid fixtures. Found rules: {:?}",
            result
                .diagnostics
                .iter()
                .map(|d| &d.rule)
                .collect::<Vec<_>>()
        );
    }

    // ===== Cursor Project Rules Validation Integration Tests =====

    #[test]
    fn test_detect_cursor_rule() {
        assert_eq!(
            detect_file_type(Path::new(".cursor/rules/typescript.mdc")),
            FileType::CursorRule
        );
        assert_eq!(
            detect_file_type(Path::new("project/.cursor/rules/rust.mdc")),
            FileType::CursorRule
        );
    }

    #[test]
    fn test_detect_cursor_legacy() {
        assert_eq!(
            detect_file_type(Path::new(".cursorrules")),
            FileType::CursorRulesLegacy
        );
        assert_eq!(
            detect_file_type(Path::new("project/.cursorrules")),
            FileType::CursorRulesLegacy
        );
    }

    #[test]
    fn test_cursor_not_detected_outside_cursor_dir() {
        // .mdc files outside .cursor/rules/ should not be detected as CursorRule
        assert_ne!(
            detect_file_type(Path::new("rules/typescript.mdc")),
            FileType::CursorRule
        );
        assert_ne!(
            detect_file_type(Path::new(".cursor/typescript.mdc")),
            FileType::CursorRule
        );
    }

    #[test]
    fn test_validators_for_cursor() {
        let registry = ValidatorRegistry::with_defaults();

        let cursor_validators = registry.validators_for(FileType::CursorRule);
        assert_eq!(cursor_validators.len(), 1); // cursor only

        let legacy_validators = registry.validators_for(FileType::CursorRulesLegacy);
        assert_eq!(legacy_validators.len(), 1); // cursor only
    }

    #[test]
    fn test_validate_cursor_fixtures() {
        // Use validate_file directly since .cursor is a hidden directory
        let fixtures_dir = get_fixtures_dir();
        let cursor_dir = fixtures_dir.join("cursor");

        let config = LintConfig::default();

        // Validate valid .mdc file
        let valid_path = cursor_dir.join(".cursor/rules/valid.mdc");
        let diagnostics = validate_file(&valid_path, &config).unwrap();
        let cur_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("CUR-") && d.level == DiagnosticLevel::Error)
            .collect();
        assert!(
            cur_errors.is_empty(),
            "Valid .mdc file should have no CUR errors, got: {:?}",
            cur_errors
        );

        // Validate .mdc file with multiple globs
        let multiple_globs_path = cursor_dir.join(".cursor/rules/multiple-globs.mdc");
        let diagnostics = validate_file(&multiple_globs_path, &config).unwrap();
        let cur_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("CUR-") && d.level == DiagnosticLevel::Error)
            .collect();
        assert!(
            cur_errors.is_empty(),
            "Valid .mdc file with multiple globs should have no CUR errors, got: {:?}",
            cur_errors
        );
    }

    #[test]
    fn test_validate_cursor_invalid_fixtures() {
        // Use validate_file directly since .cursor is a hidden directory
        let fixtures_dir = get_fixtures_dir();
        let cursor_invalid_dir = fixtures_dir.join("cursor-invalid");
        let config = LintConfig::default();

        // CUR-001: Empty .mdc file
        let empty_mdc = cursor_invalid_dir.join(".cursor/rules/empty.mdc");
        let diagnostics = validate_file(&empty_mdc, &config).unwrap();
        assert!(
            diagnostics.iter().any(|d| d.rule == "CUR-001"),
            "Expected CUR-001 from empty.mdc fixture"
        );

        // CUR-002: Missing frontmatter
        let no_frontmatter = cursor_invalid_dir.join(".cursor/rules/no-frontmatter.mdc");
        let diagnostics = validate_file(&no_frontmatter, &config).unwrap();
        assert!(
            diagnostics.iter().any(|d| d.rule == "CUR-002"),
            "Expected CUR-002 from no-frontmatter.mdc fixture"
        );

        // CUR-003: Invalid YAML
        let bad_yaml = cursor_invalid_dir.join(".cursor/rules/bad-yaml.mdc");
        let diagnostics = validate_file(&bad_yaml, &config).unwrap();
        assert!(
            diagnostics.iter().any(|d| d.rule == "CUR-003"),
            "Expected CUR-003 from bad-yaml.mdc fixture"
        );

        // CUR-004: Invalid glob pattern
        let bad_glob = cursor_invalid_dir.join(".cursor/rules/bad-glob.mdc");
        let diagnostics = validate_file(&bad_glob, &config).unwrap();
        assert!(
            diagnostics.iter().any(|d| d.rule == "CUR-004"),
            "Expected CUR-004 from bad-glob.mdc fixture"
        );

        // CUR-005: Unknown keys
        let unknown_keys = cursor_invalid_dir.join(".cursor/rules/unknown-keys.mdc");
        let diagnostics = validate_file(&unknown_keys, &config).unwrap();
        assert!(
            diagnostics.iter().any(|d| d.rule == "CUR-005"),
            "Expected CUR-005 from unknown-keys.mdc fixture"
        );
    }

    #[test]
    fn test_validate_cursor_legacy_fixture() {
        let fixtures_dir = get_fixtures_dir();
        let legacy_path = fixtures_dir.join("cursor-legacy/.cursorrules");
        let config = LintConfig::default();

        let diagnostics = validate_file(&legacy_path, &config).unwrap();
        assert!(
            diagnostics.iter().any(|d| d.rule == "CUR-006"),
            "Expected CUR-006 from .cursorrules fixture"
        );
    }

    #[test]
    fn test_validate_cursor_file_empty() {
        let temp = tempfile::TempDir::new().unwrap();
        let cursor_dir = temp.path().join(".cursor").join("rules");
        std::fs::create_dir_all(&cursor_dir).unwrap();
        let file_path = cursor_dir.join("empty.mdc");
        std::fs::write(&file_path, "").unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_file(&file_path, &config).unwrap();

        let cur_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-001").collect();
        assert_eq!(cur_001.len(), 1, "Expected CUR-001 for empty file");
    }

    #[test]
    fn test_validate_cursor_mdc_missing_frontmatter() {
        let temp = tempfile::TempDir::new().unwrap();
        let cursor_dir = temp.path().join(".cursor").join("rules");
        std::fs::create_dir_all(&cursor_dir).unwrap();
        let file_path = cursor_dir.join("test.mdc");
        std::fs::write(&file_path, "# Rules without frontmatter").unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_file(&file_path, &config).unwrap();

        let cur_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-002").collect();
        assert_eq!(cur_002.len(), 1, "Expected CUR-002 for missing frontmatter");
    }

    #[test]
    fn test_validate_cursor_valid_mdc() {
        let temp = tempfile::TempDir::new().unwrap();
        let cursor_dir = temp.path().join(".cursor").join("rules");
        std::fs::create_dir_all(&cursor_dir).unwrap();
        let file_path = cursor_dir.join("rust.mdc");
        std::fs::write(
            &file_path,
            r#"---
description: Rust rules
globs: "**/*.rs"
---
# Rust Rules

Use idiomatic Rust patterns.
"#,
        )
        .unwrap();

        let config = LintConfig::default();
        let diagnostics = validate_file(&file_path, &config).unwrap();

        let cur_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("CUR-") && d.level == DiagnosticLevel::Error)
            .collect();
        assert!(
            cur_errors.is_empty(),
            "Valid .mdc file should have no CUR errors"
        );
    }

    #[test]
    fn test_validate_project_finds_cursor_hidden_dir() {
        // Test validate_project walks .cursor directory
        let temp = tempfile::TempDir::new().unwrap();
        let cursor_dir = temp.path().join(".cursor").join("rules");
        std::fs::create_dir_all(&cursor_dir).unwrap();

        // Create an empty .mdc file (should trigger CUR-001)
        let file_path = cursor_dir.join("empty.mdc");
        std::fs::write(&file_path, "").unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        assert!(
            result.diagnostics.iter().any(|d| d.rule == "CUR-001"),
            "validate_project should find .cursor/rules/empty.mdc and report CUR-001. Found: {:?}",
            result
                .diagnostics
                .iter()
                .map(|d| &d.rule)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_validate_project_finds_cursor_invalid_fixtures() {
        // Test validate_project on the actual fixture directory
        let fixtures_dir = get_fixtures_dir();
        let cursor_invalid_dir = fixtures_dir.join("cursor-invalid");

        let config = LintConfig::default();
        let result = validate_project(&cursor_invalid_dir, &config).unwrap();

        // Should find CUR-001 from empty.mdc
        assert!(
            result.diagnostics.iter().any(|d| d.rule == "CUR-001"),
            "validate_project should find CUR-001 in cursor-invalid fixtures. Found rules: {:?}",
            result
                .diagnostics
                .iter()
                .map(|d| &d.rule)
                .collect::<Vec<_>>()
        );

        // Should find CUR-002 from no-frontmatter.mdc
        assert!(
            result.diagnostics.iter().any(|d| d.rule == "CUR-002"),
            "validate_project should find CUR-002 in cursor-invalid fixtures. Found rules: {:?}",
            result
                .diagnostics
                .iter()
                .map(|d| &d.rule)
                .collect::<Vec<_>>()
        );
    }

    // ===== PE Rules Dispatch Integration Tests =====

    #[test]
    fn test_pe_rules_dispatched() {
        // Verify PE-* rules are dispatched when validating ClaudeMd file type.
        // Per SPEC.md, PE rules apply to CLAUDE.md and AGENTS.md only (not SKILL.md).
        let fixtures_dir = get_fixtures_dir().join("prompt");
        let config = LintConfig::default();
        let registry = ValidatorRegistry::with_defaults();
        let temp = tempfile::TempDir::new().unwrap();
        let claude_path = temp.path().join("CLAUDE.md");

        // Test cases: (fixture_file, expected_rule)
        let test_cases = [
            ("pe-001-critical-in-middle.md", "PE-001"),
            ("pe-002-cot-on-simple.md", "PE-002"),
            ("pe-003-weak-language.md", "PE-003"),
            ("pe-004-ambiguous.md", "PE-004"),
        ];

        for (fixture, expected_rule) in test_cases {
            let content = std::fs::read_to_string(fixtures_dir.join(fixture))
                .unwrap_or_else(|_| panic!("Failed to read fixture: {}", fixture));
            std::fs::write(&claude_path, &content).unwrap();
            let diagnostics =
                validate_file_with_registry(&claude_path, &config, &registry).unwrap();
            assert!(
                diagnostics.iter().any(|d| d.rule == expected_rule),
                "Expected {} from {} content",
                expected_rule,
                fixture
            );
        }

        // Also verify PE rules dispatch on AGENTS.md file type
        let agents_path = temp.path().join("AGENTS.md");
        let pe_003_content =
            std::fs::read_to_string(fixtures_dir.join("pe-003-weak-language.md")).unwrap();
        std::fs::write(&agents_path, &pe_003_content).unwrap();
        let diagnostics = validate_file_with_registry(&agents_path, &config, &registry).unwrap();
        assert!(
            diagnostics.iter().any(|d| d.rule == "PE-003"),
            "Expected PE-003 from AGENTS.md with weak language content"
        );
    }

    #[test]
    fn test_exclude_patterns_with_absolute_path() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create a structure that should be partially excluded
        let target_dir = temp.path().join("target");
        std::fs::create_dir_all(&target_dir).unwrap();
        std::fs::write(
            target_dir.join("SKILL.md"),
            "---\nname: build-artifact\ndescription: Should be excluded\n---\nBody",
        )
        .unwrap();

        // Create a file that should NOT be excluded
        std::fs::write(
            temp.path().join("SKILL.md"),
            "---\nname: valid-skill\ndescription: Should be validated\n---\nBody",
        )
        .unwrap();

        let mut config = LintConfig::default();
        config.exclude = vec!["target/**".to_string()];

        // Use absolute path (canonicalize returns absolute path)
        let abs_path = std::fs::canonicalize(temp.path()).unwrap();
        let result = validate_project(&abs_path, &config).unwrap();

        // Should NOT have diagnostics from target/SKILL.md (excluded)
        let target_diags: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.file.to_string_lossy().contains("target"))
            .collect();
        assert!(
            target_diags.is_empty(),
            "Files in target/ should be excluded when using absolute path, got: {:?}",
            target_diags
        );
    }

    #[test]
    fn test_exclude_patterns_with_relative_path() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create a structure that should be partially excluded
        let node_modules = temp.path().join("node_modules");
        std::fs::create_dir_all(&node_modules).unwrap();
        std::fs::write(
            node_modules.join("SKILL.md"),
            "---\nname: npm-artifact\ndescription: Should be excluded\n---\nBody",
        )
        .unwrap();

        // Create a file that should NOT be excluded
        std::fs::write(
            temp.path().join("AGENTS.md"),
            "# Project\n\nThis should be validated.",
        )
        .unwrap();

        let mut config = LintConfig::default();
        config.exclude = vec!["node_modules/**".to_string()];

        // Use temp.path() directly to validate exclude pattern handling
        let result = validate_project(temp.path(), &config).unwrap();

        // Should NOT have diagnostics from node_modules/
        let nm_diags: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.file.to_string_lossy().contains("node_modules"))
            .collect();
        assert!(
            nm_diags.is_empty(),
            "Files in node_modules/ should be excluded, got: {:?}",
            nm_diags
        );
    }

    #[test]
    fn test_exclude_patterns_nested_directories() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create deeply nested target directory
        let deep_target = temp.path().join("subproject").join("target").join("debug");
        std::fs::create_dir_all(&deep_target).unwrap();
        std::fs::write(
            deep_target.join("SKILL.md"),
            "---\nname: deep-artifact\ndescription: Deep exclude test\n---\nBody",
        )
        .unwrap();

        let mut config = LintConfig::default();
        // Use ** prefix to match at any level
        config.exclude = vec!["**/target/**".to_string()];

        let abs_path = std::fs::canonicalize(temp.path()).unwrap();
        let result = validate_project(&abs_path, &config).unwrap();

        let target_diags: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.file.to_string_lossy().contains("target"))
            .collect();
        assert!(
            target_diags.is_empty(),
            "Deeply nested target/ files should be excluded, got: {:?}",
            target_diags
        );
    }

    #[test]
    fn test_should_prune_dir_with_globbed_patterns() {
        let patterns =
            compile_exclude_patterns(&vec!["target/**".to_string(), "**/target/**".to_string()]);
        assert!(
            should_prune_dir("target", &patterns),
            "Expected target/** to prune target directory"
        );
        assert!(
            should_prune_dir("sub/target", &patterns),
            "Expected **/target/** to prune nested target directory"
        );
    }

    #[test]
    fn test_should_prune_dir_for_bare_pattern() {
        let patterns = compile_exclude_patterns(&vec!["target".to_string()]);
        assert!(
            should_prune_dir("target", &patterns),
            "Bare pattern should prune directory"
        );
        assert!(
            !should_prune_dir("sub/target", &patterns),
            "Bare pattern should not prune nested directories"
        );
    }

    #[test]
    fn test_should_prune_dir_for_trailing_slash_pattern() {
        let patterns = compile_exclude_patterns(&vec!["target/".to_string()]);
        assert!(
            should_prune_dir("target", &patterns),
            "Trailing slash pattern should prune directory"
        );
    }

    #[test]
    fn test_should_not_prune_root_dir() {
        let patterns = compile_exclude_patterns(&vec!["target/**".to_string()]);
        assert!(
            !should_prune_dir("", &patterns),
            "Root directory should never be pruned"
        );
    }

    #[test]
    fn test_should_not_prune_dir_for_single_level_glob() {
        let patterns = compile_exclude_patterns(&vec!["target/*".to_string()]);
        assert!(
            !should_prune_dir("target", &patterns),
            "Single-level glob should not prune directory"
        );
    }

    #[test]
    fn test_dir_only_pattern_does_not_exclude_file_named_dir() {
        let patterns = compile_exclude_patterns(&vec!["target/".to_string()]);
        assert!(
            !is_excluded_file("target", &patterns),
            "Directory-only pattern should not exclude a file named target"
        );
    }

    #[test]
    fn test_dir_only_pattern_excludes_files_under_dir() {
        let patterns = compile_exclude_patterns(&vec!["target/".to_string()]);
        assert!(
            is_excluded_file("target/file.txt", &patterns),
            "Directory-only pattern should exclude files under target/"
        );
    }

    // ===== ValidationResult files_checked Tests =====

    #[test]
    fn test_files_checked_with_no_diagnostics() {
        // Test that files_checked is accurate even when there are no diagnostics
        let temp = tempfile::TempDir::new().unwrap();

        // Create valid skill files that produce no diagnostics
        let skill_dir = temp.path().join("skills").join("code-review");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: code-review\ndescription: Use when reviewing code\n---\nBody",
        )
        .unwrap();

        // Create another valid skill
        let skill_dir2 = temp.path().join("skills").join("test-runner");
        std::fs::create_dir_all(&skill_dir2).unwrap();
        std::fs::write(
            skill_dir2.join("SKILL.md"),
            "---\nname: test-runner\ndescription: Use when running tests\n---\nBody",
        )
        .unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        // Should have counted exactly the two valid skill files
        assert_eq!(
            result.files_checked, 2,
            "files_checked should count exactly the validated skill files, got {}",
            result.files_checked
        );
        assert!(
            result.diagnostics.is_empty(),
            "Valid skill files should have no diagnostics"
        );
    }

    #[test]
    fn test_files_checked_excludes_unknown_file_types() {
        // Test that files_checked only counts recognized file types
        let temp = tempfile::TempDir::new().unwrap();

        // Create files of unknown type
        std::fs::write(temp.path().join("main.rs"), "fn main() {}").unwrap();
        std::fs::write(temp.path().join("package.json"), "{}").unwrap();

        // Create one recognized file
        std::fs::write(
            temp.path().join("SKILL.md"),
            "---\nname: code-review\ndescription: Use when reviewing code\n---\nBody",
        )
        .unwrap();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config).unwrap();

        // Should only count the SKILL.md file, not .rs or package.json
        assert_eq!(
            result.files_checked, 1,
            "files_checked should only count recognized file types"
        );
    }
}
