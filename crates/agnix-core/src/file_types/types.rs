//! FileType enum for validator dispatch.

use std::fmt;

/// Detected file type for validator dispatch.
///
/// Each variant maps to a class of agent configuration file that has
/// a dedicated set of validators registered in the
/// [`ValidatorRegistry`](crate::ValidatorRegistry).
///
/// The enum intentionally derives [`Hash`], [`Eq`], and [`Copy`] so that it
/// can be used as a key in [`HashMap`](std::collections::HashMap)-backed
/// registries without allocation.
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
    /// GitHub Copilot custom agents (.github/agents/*.agent.md)
    CopilotAgent,
    /// GitHub Copilot reusable prompts (.github/prompts/*.prompt.md)
    CopilotPrompt,
    /// GitHub Copilot coding agent hooks (.github/hooks/hooks.json)
    /// and setup workflow (.github/workflows/copilot-setup-steps.yml)
    CopilotHooks,
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

impl FileType {
    /// Returns `true` if this file type should be validated.
    ///
    /// This is the inverse of checking for [`FileType::Unknown`] and should
    /// be preferred over `file_type != FileType::Unknown` for clarity.
    #[must_use]
    pub fn is_validatable(self) -> bool {
        !matches!(self, FileType::Unknown)
    }
}

impl fmt::Display for FileType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            FileType::Skill => "Skill",
            FileType::ClaudeMd => "ClaudeMd",
            FileType::Agent => "Agent",
            FileType::Hooks => "Hooks",
            FileType::Plugin => "Plugin",
            FileType::Mcp => "Mcp",
            FileType::Copilot => "Copilot",
            FileType::CopilotScoped => "CopilotScoped",
            FileType::CopilotAgent => "CopilotAgent",
            FileType::CopilotPrompt => "CopilotPrompt",
            FileType::CopilotHooks => "CopilotHooks",
            FileType::ClaudeRule => "ClaudeRule",
            FileType::CursorRule => "CursorRule",
            FileType::CursorRulesLegacy => "CursorRulesLegacy",
            FileType::ClineRules => "ClineRules",
            FileType::ClineRulesFolder => "ClineRulesFolder",
            FileType::OpenCodeConfig => "OpenCodeConfig",
            FileType::GeminiMd => "GeminiMd",
            FileType::CodexConfig => "CodexConfig",
            FileType::GenericMarkdown => "GenericMarkdown",
            FileType::Unknown => "Unknown",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All variants must round-trip through Display.
    #[test]
    fn display_all_variants() {
        let variants = [
            (FileType::Skill, "Skill"),
            (FileType::ClaudeMd, "ClaudeMd"),
            (FileType::Agent, "Agent"),
            (FileType::Hooks, "Hooks"),
            (FileType::Plugin, "Plugin"),
            (FileType::Mcp, "Mcp"),
            (FileType::Copilot, "Copilot"),
            (FileType::CopilotScoped, "CopilotScoped"),
            (FileType::CopilotAgent, "CopilotAgent"),
            (FileType::CopilotPrompt, "CopilotPrompt"),
            (FileType::CopilotHooks, "CopilotHooks"),
            (FileType::ClaudeRule, "ClaudeRule"),
            (FileType::CursorRule, "CursorRule"),
            (FileType::CursorRulesLegacy, "CursorRulesLegacy"),
            (FileType::ClineRules, "ClineRules"),
            (FileType::ClineRulesFolder, "ClineRulesFolder"),
            (FileType::OpenCodeConfig, "OpenCodeConfig"),
            (FileType::GeminiMd, "GeminiMd"),
            (FileType::CodexConfig, "CodexConfig"),
            (FileType::GenericMarkdown, "GenericMarkdown"),
            (FileType::Unknown, "Unknown"),
        ];

        for (variant, expected) in &variants {
            assert_eq!(variant.to_string(), *expected);
        }
    }

    /// `is_validatable` returns true for all variants except Unknown.
    #[test]
    fn is_validatable_all_variants() {
        let validatable = [
            FileType::Skill,
            FileType::ClaudeMd,
            FileType::Agent,
            FileType::Hooks,
            FileType::Plugin,
            FileType::Mcp,
            FileType::Copilot,
            FileType::CopilotScoped,
            FileType::CopilotAgent,
            FileType::CopilotPrompt,
            FileType::CopilotHooks,
            FileType::ClaudeRule,
            FileType::CursorRule,
            FileType::CursorRulesLegacy,
            FileType::ClineRules,
            FileType::ClineRulesFolder,
            FileType::OpenCodeConfig,
            FileType::GeminiMd,
            FileType::CodexConfig,
            FileType::GenericMarkdown,
        ];

        for variant in &validatable {
            assert!(
                variant.is_validatable(),
                "{} should be validatable",
                variant
            );
        }

        assert!(
            !FileType::Unknown.is_validatable(),
            "Unknown should not be validatable"
        );
    }

    /// FileType must be usable as a HashMap key (requires Hash + Eq).
    #[test]
    fn usable_as_hashmap_key() {
        use std::collections::HashMap;

        let mut map = HashMap::new();
        map.insert(FileType::Skill, "skill");
        map.insert(FileType::Unknown, "unknown");

        assert_eq!(map.get(&FileType::Skill), Some(&"skill"));
        assert_eq!(map.get(&FileType::Unknown), Some(&"unknown"));
    }

    /// FileType is Copy (no move semantics).
    #[test]
    fn file_type_is_copy() {
        let a = FileType::Skill;
        let b = a; // Copy
        assert_eq!(a, b); // `a` is still usable
    }
}
