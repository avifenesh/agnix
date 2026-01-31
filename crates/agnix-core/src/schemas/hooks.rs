//! Hooks schema (Claude Code hooks)

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Full settings.json schema (for parsing hooks from settings files)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettingsSchema {
    #[serde(default)]
    pub hooks: HashMap<String, Vec<HookMatcher>>,
    /// Allow other fields in settings.json to be ignored
    #[serde(flatten)]
    pub _extra: HashMap<String, Value>,
}

/// Hooks configuration schema (standalone .claude/hooks.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HooksSchema {
    pub hooks: HashMap<String, Vec<HookMatcher>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookMatcher {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,
    pub hooks: Vec<Hook>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Hook {
    #[serde(rename = "command")]
    Command {
        /// Command to execute (optional to allow validation of missing field)
        #[serde(skip_serializing_if = "Option::is_none")]
        command: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timeout: Option<u64>,
    },
    #[serde(rename = "prompt")]
    Prompt {
        /// Prompt text (optional to allow validation of missing field)
        #[serde(skip_serializing_if = "Option::is_none")]
        prompt: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timeout: Option<u64>,
    },
}

impl SettingsSchema {
    /// Parse from JSON content
    pub fn from_json(content: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(content)
    }

    /// Convert to HooksSchema for validation
    pub fn to_hooks_schema(&self) -> HooksSchema {
        HooksSchema {
            hooks: self.hooks.clone(),
        }
    }
}

impl Hook {
    /// Get the command string if this is a command hook
    pub fn command(&self) -> Option<&str> {
        match self {
            Hook::Command { command, .. } => command.as_deref(),
            Hook::Prompt { .. } => None,
        }
    }

    /// Get the prompt string if this is a prompt hook
    pub fn prompt(&self) -> Option<&str> {
        match self {
            Hook::Prompt { prompt, .. } => prompt.as_deref(),
            Hook::Command { .. } => None,
        }
    }

    /// Check if this is a command hook
    pub fn is_command(&self) -> bool {
        matches!(self, Hook::Command { .. })
    }

    /// Check if this is a prompt hook
    pub fn is_prompt(&self) -> bool {
        matches!(self, Hook::Prompt { .. })
    }
}

impl HooksSchema {
    /// Valid hook events
    const VALID_EVENTS: &'static [&'static str] = &[
        "PreToolUse",
        "PermissionRequest",
        "PostToolUse",
        "PostToolUseFailure",
        "Notification",
        "UserPromptSubmit",
        "Stop",
        "SubagentStart",
        "SubagentStop",
        "PreCompact",
        "Setup",
        "SessionStart",
        "SessionEnd",
    ];

    /// Parse from JSON content
    pub fn from_json(content: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(content)
    }

    /// Validate hook events
    pub fn validate_events(&self) -> Vec<String> {
        let mut errors = Vec::new();

        for event in self.hooks.keys() {
            if !Self::VALID_EVENTS.contains(&event.as_str()) {
                errors.push(format!("Unknown hook event '{}', valid events: {:?}", event, Self::VALID_EVENTS));
            }
        }

        errors
    }

    /// Validate hook structure
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        errors.extend(self.validate_events());

        // Validate each hook has required fields
        for (event, matchers) in &self.hooks {
            for (i, matcher) in matchers.iter().enumerate() {
                if matcher.hooks.is_empty() {
                    errors.push(format!("Hook event '{}' matcher {} has empty hooks array", event, i));
                }
            }
        }

        errors
    }
}
