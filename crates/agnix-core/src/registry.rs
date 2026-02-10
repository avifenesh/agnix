//! Validator registry and factory functions.

use std::collections::HashMap;

use crate::file_types::FileType;
use crate::rules::Validator;

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

    /// Return the total number of registered validator factories across all file types.
    pub fn total_factory_count(&self) -> usize {
        self.validators.values().map(|v| v.len()).sum()
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
            (FileType::Skill, per_client_skill_validator),
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
            (FileType::ClaudeRule, claude_rules_validator),
            (FileType::CursorRule, cursor_validator),
            (FileType::CursorRule, prompt_validator),
            (FileType::CursorRule, claude_md_validator),
            (FileType::CursorRulesLegacy, cursor_validator),
            (FileType::CursorRulesLegacy, prompt_validator),
            (FileType::CursorRulesLegacy, claude_md_validator),
            (FileType::ClineRules, cline_validator),
            (FileType::ClineRulesFolder, cline_validator),
            (FileType::OpenCodeConfig, opencode_validator),
            (FileType::GeminiMd, gemini_md_validator),
            (FileType::GeminiMd, prompt_validator),
            (FileType::GeminiMd, xml_validator),
            (FileType::GeminiMd, imports_validator),
            (FileType::GeminiMd, cross_platform_validator),
            (FileType::CodexConfig, codex_validator),
            // CodexValidator on ClaudeMd catches AGENTS.override.md files (CDX-003).
            // The validator early-returns for all other ClaudeMd filenames.
            (FileType::ClaudeMd, codex_validator),
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
    Box::new(crate::rules::skill::SkillValidator)
}

fn per_client_skill_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::per_client_skill::PerClientSkillValidator)
}

fn claude_md_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::claude_md::ClaudeMdValidator)
}

fn agents_md_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::agents_md::AgentsMdValidator)
}

fn agent_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::agent::AgentValidator)
}

fn hooks_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::hooks::HooksValidator)
}

fn plugin_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::plugin::PluginValidator)
}

fn mcp_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::mcp::McpValidator)
}

fn xml_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::xml::XmlValidator)
}

fn imports_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::imports::ImportsValidator)
}

fn cross_platform_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::cross_platform::CrossPlatformValidator)
}

fn prompt_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::prompt::PromptValidator)
}

fn copilot_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::copilot::CopilotValidator)
}

fn claude_rules_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::claude_rules::ClaudeRulesValidator)
}

fn cursor_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::cursor::CursorValidator)
}

fn cline_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::cline::ClineValidator)
}

fn opencode_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::opencode::OpenCodeValidator)
}

fn gemini_md_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::gemini_md::GeminiMdValidator)
}

fn codex_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::codex::CodexValidator)
}
