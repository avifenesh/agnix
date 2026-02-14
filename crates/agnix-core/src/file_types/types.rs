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
    /// Amp code review checks (.agents/checks/*.md)
    AmpCheck,
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
    /// Cursor project rules (.cursor/rules/*.md, .cursor/rules/*.mdc, including nested dirs)
    CursorRule,
    /// Cursor hooks configuration (.cursor/hooks.json)
    CursorHooks,
    /// Cursor subagent definitions (.cursor/agents/**/*.md, including nested dirs)
    CursorAgent,
    /// Cursor cloud-agent environment configuration (.cursor/environment.json)
    CursorEnvironment,
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
    /// Gemini CLI settings (.gemini/settings.json)
    GeminiSettings,
    /// Amp settings (.amp/settings.json, .amp/settings.local.json)
    AmpSettings,
    /// Gemini CLI extension manifest (gemini-extension.json)
    GeminiExtension,
    /// Gemini CLI ignore file (.geminiignore)
    GeminiIgnore,
    /// Codex CLI configuration (.codex/config.toml)
    CodexConfig,
    /// Roo Code rules files (.roorules, .roo/rules/*.md)
    RooRules,
    /// Roo Code custom modes configuration (.roomodes)
    RooModes,
    /// Roo Code ignore file (.rooignore)
    RooIgnore,
    /// Roo Code mode-specific rules (.roo/rules-{slug}/*.md)
    RooModeRules,
    /// Roo Code MCP configuration (.roo/mcp.json)
    RooMcp,
    /// Windsurf rule files (.windsurf/rules/*.md)
    WindsurfRule,
    /// Windsurf workflow files (.windsurf/workflows/*.md)
    WindsurfWorkflow,
    /// Legacy Windsurf rules file (.windsurfrules)
    WindsurfRulesLegacy,
    /// Kiro steering files (.kiro/steering/*.md)
    KiroSteering,
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
            FileType::AmpCheck => "AmpCheck",
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
            FileType::CursorHooks => "CursorHooks",
            FileType::CursorAgent => "CursorAgent",
            FileType::CursorEnvironment => "CursorEnvironment",
            FileType::CursorRulesLegacy => "CursorRulesLegacy",
            FileType::ClineRules => "ClineRules",
            FileType::ClineRulesFolder => "ClineRulesFolder",
            FileType::OpenCodeConfig => "OpenCodeConfig",
            FileType::GeminiMd => "GeminiMd",
            FileType::GeminiSettings => "GeminiSettings",
            FileType::AmpSettings => "AmpSettings",
            FileType::GeminiExtension => "GeminiExtension",
            FileType::GeminiIgnore => "GeminiIgnore",
            FileType::CodexConfig => "CodexConfig",
            FileType::RooRules => "RooRules",
            FileType::RooModes => "RooModes",
            FileType::RooIgnore => "RooIgnore",
            FileType::RooModeRules => "RooModeRules",
            FileType::RooMcp => "RooMcp",
            FileType::WindsurfRule => "WindsurfRule",
            FileType::WindsurfWorkflow => "WindsurfWorkflow",
            FileType::WindsurfRulesLegacy => "WindsurfRulesLegacy",
            FileType::KiroSteering => "KiroSteering",
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
            (FileType::AmpCheck, "AmpCheck"),
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
            (FileType::CursorHooks, "CursorHooks"),
            (FileType::CursorAgent, "CursorAgent"),
            (FileType::CursorEnvironment, "CursorEnvironment"),
            (FileType::CursorRulesLegacy, "CursorRulesLegacy"),
            (FileType::ClineRules, "ClineRules"),
            (FileType::ClineRulesFolder, "ClineRulesFolder"),
            (FileType::OpenCodeConfig, "OpenCodeConfig"),
            (FileType::GeminiMd, "GeminiMd"),
            (FileType::GeminiSettings, "GeminiSettings"),
            (FileType::AmpSettings, "AmpSettings"),
            (FileType::GeminiExtension, "GeminiExtension"),
            (FileType::GeminiIgnore, "GeminiIgnore"),
            (FileType::CodexConfig, "CodexConfig"),
            (FileType::RooRules, "RooRules"),
            (FileType::RooModes, "RooModes"),
            (FileType::RooIgnore, "RooIgnore"),
            (FileType::RooModeRules, "RooModeRules"),
            (FileType::RooMcp, "RooMcp"),
            (FileType::WindsurfRule, "WindsurfRule"),
            (FileType::WindsurfWorkflow, "WindsurfWorkflow"),
            (FileType::WindsurfRulesLegacy, "WindsurfRulesLegacy"),
            (FileType::KiroSteering, "KiroSteering"),
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
            FileType::AmpCheck,
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
            FileType::CursorHooks,
            FileType::CursorAgent,
            FileType::CursorEnvironment,
            FileType::CursorRulesLegacy,
            FileType::ClineRules,
            FileType::ClineRulesFolder,
            FileType::OpenCodeConfig,
            FileType::GeminiMd,
            FileType::GeminiSettings,
            FileType::AmpSettings,
            FileType::GeminiExtension,
            FileType::GeminiIgnore,
            FileType::CodexConfig,
            FileType::RooRules,
            FileType::RooModes,
            FileType::RooIgnore,
            FileType::RooModeRules,
            FileType::RooMcp,
            FileType::WindsurfRule,
            FileType::WindsurfWorkflow,
            FileType::WindsurfRulesLegacy,
            FileType::KiroSteering,
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
