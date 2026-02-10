//! File type detection for validator dispatch.

use std::path::Path;

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
    /// Claude Code rules (.claude/rules/*.md)
    ClaudeRule,
    /// Cursor project rules (.cursor/rules/*.mdc)
    CursorRule,
    /// Legacy Cursor rules file (.cursorrules)
    CursorRulesLegacy,
    /// Cline rules single file (.clinerules)
    ClineRules,
    /// Cline rules folder files (.clinerules/*.md)
    ClineRulesFolder,
    /// OpenCode configuration (opencode.json)
    OpenCodeConfig,
    /// Gemini CLI instruction files (GEMINI.md, GEMINI.local.md)
    GeminiMd,
    /// Codex CLI configuration (.codex/config.toml)
    CodexConfig,
    /// Other .md files (for XML/import checks)
    GenericMarkdown,
    /// Skip validation
    Unknown,
}

/// Returns true if the file is inside a documentation directory that
/// is unlikely to contain agent configuration files. This prevents
/// false positives from XML tags, broken links, and cross-platform
/// references in project documentation.
fn is_documentation_directory(path: &Path) -> bool {
    // Check if any ancestor directory is a documentation directory
    for component in path.components() {
        if let std::path::Component::Normal(name) = component {
            if let Some(name_str) = name.to_str() {
                if name_str.eq_ignore_ascii_case("docs")
                    || name_str.eq_ignore_ascii_case("doc")
                    || name_str.eq_ignore_ascii_case("documentation")
                    || name_str.eq_ignore_ascii_case("wiki")
                    || name_str.eq_ignore_ascii_case("licenses")
                    || name_str.eq_ignore_ascii_case("examples")
                    || name_str.eq_ignore_ascii_case("api-docs")
                    || name_str.eq_ignore_ascii_case("api_docs")
                {
                    return true;
                }
            }
        }
    }
    false
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
                if name.eq_ignore_ascii_case("changelog.md")
                    || name.eq_ignore_ascii_case("history.md")
                    || name.eq_ignore_ascii_case("releases.md")
                    || name.eq_ignore_ascii_case("readme.md")
                    || name.eq_ignore_ascii_case("contributing.md")
                    || name.eq_ignore_ascii_case("license.md")
                    || name.eq_ignore_ascii_case("code_of_conduct.md")
                    || name.eq_ignore_ascii_case("security.md")
                    || name.eq_ignore_ascii_case("pull_request_template.md")
                    || name.eq_ignore_ascii_case("issue_template.md")
                    || name.eq_ignore_ascii_case("bug_report.md")
                    || name.eq_ignore_ascii_case("feature_request.md")
                    // Developer-focused docs, not agent instructions
                    || name.eq_ignore_ascii_case("developer.md")
                    || name.eq_ignore_ascii_case("developers.md")
                    || name.eq_ignore_ascii_case("development.md")
                    || name.eq_ignore_ascii_case("hacking.md")
                    || name.eq_ignore_ascii_case("maintainers.md")
                    || name.eq_ignore_ascii_case("governance.md")
                    || name.eq_ignore_ascii_case("support.md")
                    || name.eq_ignore_ascii_case("authors.md")
                    || name.eq_ignore_ascii_case("credits.md")
                    || name.eq_ignore_ascii_case("thanks.md")
                    || name.eq_ignore_ascii_case("migration.md")
                    || name.eq_ignore_ascii_case("upgrading.md")
                {
                    FileType::Unknown
                } else if is_documentation_directory(path) {
                    // Markdown files in documentation directories are not agent configs
                    FileType::Unknown
                } else if parent.is_some_and(|p| p.eq_ignore_ascii_case(".github"))
                    || parent.is_some_and(|p| p.eq_ignore_ascii_case("issue_template"))
                    || parent.is_some_and(|p| p.eq_ignore_ascii_case("pull_request_template"))
                {
                    FileType::Unknown
                } else {
                    FileType::GenericMarkdown
                }
            }
        }
        _ => FileType::Unknown,
    }
}
