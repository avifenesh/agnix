//! File type detection based on path patterns.
//!
//! The detection logic is path-based only (no I/O) and is used by the
//! validation pipeline to dispatch files to the correct validators.

use std::path::Path;

use super::types::FileType;

// ============================================================================
// Named constants for hardcoded pattern sets
// ============================================================================

/// Directory names that indicate project documentation rather than agent
/// configuration. Markdown files under these directories are classified as
/// [`FileType::Unknown`] to avoid false positives from HTML tags, @mentions,
/// and cross-platform references.
///
/// Matching is case-insensitive.
pub const DOCUMENTATION_DIRECTORIES: &[&str] = &[
    "docs",
    "doc",
    "documentation",
    "wiki",
    "licenses",
    "examples",
    "api-docs",
    "api_docs",
];

/// Filenames (lowercase) of common project files that are not agent
/// configurations. Files matching these names are classified as
/// [`FileType::Unknown`] to avoid false positives.
pub const EXCLUDED_FILENAMES: &[&str] = &[
    "changelog.md",
    "history.md",
    "releases.md",
    "readme.md",
    "contributing.md",
    "license.md",
    "code_of_conduct.md",
    "security.md",
    "pull_request_template.md",
    "issue_template.md",
    "bug_report.md",
    "feature_request.md",
    "developer.md",
    "developers.md",
    "development.md",
    "hacking.md",
    "maintainers.md",
    "governance.md",
    "support.md",
    "authors.md",
    "credits.md",
    "thanks.md",
    "migration.md",
    "upgrading.md",
];

/// Parent directory names (case-insensitive) that cause a `.md` file to be
/// classified as [`FileType::Unknown`] rather than [`FileType::GenericMarkdown`].
pub const EXCLUDED_PARENT_DIRECTORIES: &[&str] =
    &[".github", "issue_template", "pull_request_template"];

// ============================================================================
// Detection helpers
// ============================================================================

/// Returns true if the file is inside a documentation directory that
/// is unlikely to contain agent configuration files. This prevents
/// false positives from XML tags, broken links, and cross-platform
/// references in project documentation.
fn is_documentation_directory(path: &Path) -> bool {
    for component in path.components() {
        if let std::path::Component::Normal(name) = component {
            if let Some(name_str) = name.to_str() {
                if DOCUMENTATION_DIRECTORIES
                    .iter()
                    .any(|d| d.eq_ignore_ascii_case(name_str))
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Returns true if the path contains `.github/instructions` as consecutive
/// components anywhere in the path. This allows scoped Copilot instruction
/// files to live in subdirectories under `.github/instructions/`.
fn is_under_github_instructions(path: &Path) -> bool {
    path.components()
        .zip(path.components().skip(1))
        .any(|(a, b)| {
            matches!(
                (a, b),
                (std::path::Component::Normal(a_os), std::path::Component::Normal(b_os))
                if a_os == ".github" && b_os == "instructions"
            )
        })
}

fn is_excluded_filename(name: &str) -> bool {
    EXCLUDED_FILENAMES
        .iter()
        .any(|&excl| excl.eq_ignore_ascii_case(name))
}

fn is_excluded_parent(parent: Option<&str>) -> bool {
    parent.is_some_and(|p| {
        EXCLUDED_PARENT_DIRECTORIES
            .iter()
            .any(|&excl| p.eq_ignore_ascii_case(excl))
    })
}

// ============================================================================
// Primary detection function
// ============================================================================

/// Detect file type based on path patterns.
///
/// Classification is purely path-based (no file I/O). The returned
/// [`FileType`] determines which validators the pipeline dispatches for
/// the file.
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
        // GitHub Copilot scoped instructions (.github/instructions/**/*.instructions.md)
        name if name.ends_with(".instructions.md") && is_under_github_instructions(path) => {
            FileType::CopilotScoped
        }
        // Claude Code rules (.claude/rules/*.md)
        name if name.ends_with(".md")
            && parent == Some("rules")
            && grandparent == Some(".claude") =>
        {
            FileType::ClaudeRule
        }
        // Cursor project rules (.cursor/rules/*.mdc)
        name if name.ends_with(".mdc")
            && parent == Some("rules")
            && grandparent == Some(".cursor") =>
        {
            FileType::CursorRule
        }
        // Legacy Cursor rules file (.cursorrules or .cursorrules.md)
        ".cursorrules" | ".cursorrules.md" => FileType::CursorRulesLegacy,
        // Cline rules single file (.clinerules without extension)
        ".clinerules" => FileType::ClineRules,
        // Cline rules folder (.clinerules/*.md)
        name if name.ends_with(".md") && parent == Some(".clinerules") => {
            FileType::ClineRulesFolder
        }
        // OpenCode configuration (opencode.json)
        "opencode.json" => FileType::OpenCodeConfig,
        // Gemini CLI instruction files (GEMINI.md, GEMINI.local.md)
        "GEMINI.md" | "GEMINI.local.md" => FileType::GeminiMd,
        // Codex CLI configuration (.codex/config.toml)
        // Path safety: symlink rejection and size limits are enforced upstream
        // by file_utils::safe_read_file before content reaches any validator.
        "config.toml" if parent == Some(".codex") => FileType::CodexConfig,
        name if name.ends_with(".md") => {
            // Agent directories take precedence over filename exclusions.
            // Files like agents/README.md should be validated as agent configs.
            if parent == Some("agents") || grandparent == Some("agents") {
                FileType::Agent
            } else {
                // Exclude common project files that are not agent configurations.
                // These files commonly contain HTML, @mentions, and cross-platform
                // references that would produce false positives if validated.
                if is_excluded_filename(name) {
                    FileType::Unknown
                } else if is_documentation_directory(path) {
                    // Markdown files in documentation directories are not agent configs
                    FileType::Unknown
                } else if is_excluded_parent(parent) {
                    FileType::Unknown
                } else {
                    FileType::GenericMarkdown
                }
            }
        }
        _ => FileType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Named constant completeness ----

    /// DOCUMENTATION_DIRECTORIES has the expected number of entries.
    #[test]
    fn documentation_directories_count() {
        assert_eq!(
            DOCUMENTATION_DIRECTORIES.len(),
            8,
            "Expected 8 documentation directory entries"
        );
    }

    /// EXCLUDED_FILENAMES has the expected number of entries.
    #[test]
    fn excluded_filenames_count() {
        assert_eq!(
            EXCLUDED_FILENAMES.len(),
            24,
            "Expected 24 excluded filename entries"
        );
    }

    /// EXCLUDED_PARENT_DIRECTORIES has the expected number of entries.
    #[test]
    fn excluded_parent_directories_count() {
        assert_eq!(
            EXCLUDED_PARENT_DIRECTORIES.len(),
            3,
            "Expected 3 excluded parent directory entries"
        );
    }

    /// All EXCLUDED_FILENAMES must be lowercase.
    #[test]
    fn excluded_filenames_are_lowercase() {
        for name in EXCLUDED_FILENAMES {
            assert_eq!(
                *name,
                name.to_ascii_lowercase(),
                "EXCLUDED_FILENAMES entry '{}' must be lowercase",
                name
            );
        }
    }

    /// No duplicates in DOCUMENTATION_DIRECTORIES.
    #[test]
    fn no_duplicate_documentation_directories() {
        let mut seen = std::collections::HashSet::new();
        for dir in DOCUMENTATION_DIRECTORIES {
            assert!(
                seen.insert(dir.to_ascii_lowercase()),
                "Duplicate entry in DOCUMENTATION_DIRECTORIES: {}",
                dir
            );
        }
    }

    /// No duplicates in EXCLUDED_FILENAMES.
    #[test]
    fn no_duplicate_excluded_filenames() {
        let mut seen = std::collections::HashSet::new();
        for name in EXCLUDED_FILENAMES {
            assert!(
                seen.insert(*name),
                "Duplicate entry in EXCLUDED_FILENAMES: {}",
                name
            );
        }
    }

    /// No duplicates in EXCLUDED_PARENT_DIRECTORIES.
    #[test]
    fn no_duplicate_excluded_parent_directories() {
        let mut seen = std::collections::HashSet::new();
        for dir in EXCLUDED_PARENT_DIRECTORIES {
            assert!(
                seen.insert(dir.to_ascii_lowercase()),
                "Duplicate entry in EXCLUDED_PARENT_DIRECTORIES: {}",
                dir
            );
        }
    }

    // ---- Detection function tests ----

    #[test]
    fn detect_skill_md() {
        assert_eq!(
            detect_file_type(Path::new("project/SKILL.md")),
            FileType::Skill
        );
    }

    #[test]
    fn detect_claude_md_variants() {
        for name in &[
            "CLAUDE.md",
            "CLAUDE.local.md",
            "AGENTS.md",
            "AGENTS.local.md",
            "AGENTS.override.md",
        ] {
            assert_eq!(
                detect_file_type(Path::new(name)),
                FileType::ClaudeMd,
                "Expected ClaudeMd for {}",
                name
            );
        }
    }

    #[test]
    fn detect_mcp_variants() {
        assert_eq!(detect_file_type(Path::new("mcp.json")), FileType::Mcp);
        assert_eq!(
            detect_file_type(Path::new("server.mcp.json")),
            FileType::Mcp
        );
        assert_eq!(
            detect_file_type(Path::new("mcp-server.json")),
            FileType::Mcp
        );
    }

    #[test]
    fn detect_copilot_global() {
        assert_eq!(
            detect_file_type(Path::new(".github/copilot-instructions.md")),
            FileType::Copilot
        );
    }

    #[test]
    fn detect_copilot_scoped() {
        assert_eq!(
            detect_file_type(Path::new(".github/instructions/rust.instructions.md")),
            FileType::CopilotScoped
        );
    }

    #[test]
    fn detect_excluded_filenames() {
        for name in EXCLUDED_FILENAMES {
            let lowercase_path = format!("project/{}", name);
            let path = Path::new(&lowercase_path);
            assert_eq!(
                detect_file_type(path),
                FileType::Unknown,
                "Expected Unknown for excluded filename: {}",
                name
            );
        }
    }

    #[test]
    fn detect_documentation_directories() {
        for dir in DOCUMENTATION_DIRECTORIES {
            let path = Path::new(dir).join("guide.md");
            assert_eq!(
                detect_file_type(&path),
                FileType::Unknown,
                "Expected Unknown for file in documentation directory: {}",
                dir
            );
        }
    }

    #[test]
    fn detect_excluded_parent_directories() {
        for dir in EXCLUDED_PARENT_DIRECTORIES {
            let path = Path::new(dir).join("template.md");
            assert_eq!(
                detect_file_type(&path),
                FileType::Unknown,
                "Expected Unknown for file in excluded parent: {}",
                dir
            );
        }
    }

    #[test]
    fn detect_agents_directory_takes_precedence() {
        // Even README.md in agents/ should be Agent, not excluded
        assert_eq!(
            detect_file_type(Path::new("agents/README.md")),
            FileType::Agent
        );
        assert_eq!(
            detect_file_type(Path::new("agents/sub/file.md")),
            FileType::Agent
        );
    }

    #[test]
    fn detect_generic_markdown() {
        assert_eq!(
            detect_file_type(Path::new("project/custom.md")),
            FileType::GenericMarkdown
        );
    }

    #[test]
    fn detect_hooks() {
        assert_eq!(
            detect_file_type(Path::new("settings.json")),
            FileType::Hooks
        );
        assert_eq!(
            detect_file_type(Path::new("settings.local.json")),
            FileType::Hooks
        );
    }

    #[test]
    fn detect_plugin() {
        assert_eq!(detect_file_type(Path::new("plugin.json")), FileType::Plugin);
    }

    #[test]
    fn detect_claude_rule() {
        assert_eq!(
            detect_file_type(Path::new(".claude/rules/custom.md")),
            FileType::ClaudeRule
        );
    }

    #[test]
    fn detect_cursor_rule() {
        assert_eq!(
            detect_file_type(Path::new(".cursor/rules/custom.mdc")),
            FileType::CursorRule
        );
    }

    #[test]
    fn detect_cursor_rules_legacy() {
        assert_eq!(
            detect_file_type(Path::new(".cursorrules")),
            FileType::CursorRulesLegacy
        );
        assert_eq!(
            detect_file_type(Path::new(".cursorrules.md")),
            FileType::CursorRulesLegacy
        );
    }

    #[test]
    fn detect_cline_rules() {
        assert_eq!(
            detect_file_type(Path::new(".clinerules")),
            FileType::ClineRules
        );
    }

    #[test]
    fn detect_cline_rules_folder() {
        assert_eq!(
            detect_file_type(Path::new(".clinerules/custom.md")),
            FileType::ClineRulesFolder
        );
    }

    #[test]
    fn detect_opencode_config() {
        assert_eq!(
            detect_file_type(Path::new("opencode.json")),
            FileType::OpenCodeConfig
        );
    }

    #[test]
    fn detect_gemini_md() {
        assert_eq!(detect_file_type(Path::new("GEMINI.md")), FileType::GeminiMd);
        assert_eq!(
            detect_file_type(Path::new("GEMINI.local.md")),
            FileType::GeminiMd
        );
    }

    #[test]
    fn detect_codex_config() {
        assert_eq!(
            detect_file_type(Path::new(".codex/config.toml")),
            FileType::CodexConfig
        );
    }

    #[test]
    fn detect_excluded_filename_case_insensitive() {
        assert_eq!(
            detect_file_type(Path::new("project/README.md")),
            FileType::Unknown
        );
        assert_eq!(
            detect_file_type(Path::new("project/Readme.md")),
            FileType::Unknown
        );
    }

    #[test]
    fn detect_unknown_for_non_config_files() {
        assert_eq!(
            detect_file_type(Path::new("src/main.rs")),
            FileType::Unknown
        );
        assert_eq!(
            detect_file_type(Path::new("package.json")),
            FileType::Unknown
        );
    }

    #[test]
    fn is_documentation_directory_case_insensitive() {
        assert!(is_documentation_directory(Path::new("DOCS/guide.md")));
        assert!(is_documentation_directory(Path::new("Docs/guide.md")));
        assert!(is_documentation_directory(Path::new("docs/guide.md")));
    }

    #[test]
    fn is_documentation_directory_negative() {
        assert!(!is_documentation_directory(Path::new("src/lib.rs")));
        assert!(!is_documentation_directory(Path::new("agents/task.md")));
    }

    // ---- CopilotScoped subdirectory detection ----

    #[test]
    fn detect_copilot_scoped_subdirectory() {
        assert_eq!(
            detect_file_type(Path::new(
                ".github/instructions/frontend/react.instructions.md"
            )),
            FileType::CopilotScoped
        );
    }

    #[test]
    fn detect_copilot_scoped_deep_nesting() {
        assert_eq!(
            detect_file_type(Path::new(
                ".github/instructions/frontend/components/dialog.instructions.md"
            )),
            FileType::CopilotScoped
        );
    }

    #[test]
    fn detect_copilot_scoped_not_under_github() {
        // .instructions.md under a different parent should NOT be CopilotScoped
        assert_ne!(
            detect_file_type(Path::new("other/instructions/react.instructions.md")),
            FileType::CopilotScoped
        );
    }

    #[test]
    fn detect_copilot_scoped_wrong_order() {
        // instructions/.github is the wrong order - should NOT be CopilotScoped
        assert_ne!(
            detect_file_type(Path::new("instructions/.github/foo.instructions.md")),
            FileType::CopilotScoped
        );
    }
}
