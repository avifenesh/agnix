//! Hooks schema (Claude Code hooks)

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Full settings.json schema (for parsing hooks from settings files)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettingsSchema {
    #[serde(default)]
    pub hooks: HashMap<String, Vec<HookMatcher>>,
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
        #[serde(skip_serializing_if = "Option::is_none")]
        command: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timeout: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<String>,
    },
    #[serde(rename = "prompt")]
    Prompt {
        #[serde(skip_serializing_if = "Option::is_none")]
        prompt: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timeout: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<String>,
    },
    #[serde(rename = "agent")]
    Agent {
        #[serde(skip_serializing_if = "Option::is_none")]
        prompt: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timeout: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<String>,
    },
}

impl SettingsSchema {
    pub fn from_json(content: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(content)
    }

    pub fn to_hooks_schema(&self) -> HooksSchema {
        HooksSchema {
            hooks: self.hooks.clone(),
        }
    }
}

impl Hook {
    pub fn command(&self) -> Option<&str> {
        match self {
            Hook::Command { command, .. } => command.as_deref(),
            Hook::Prompt { .. } | Hook::Agent { .. } => None,
        }
    }

    pub fn prompt(&self) -> Option<&str> {
        match self {
            Hook::Prompt { prompt, .. } | Hook::Agent { prompt, .. } => prompt.as_deref(),
            Hook::Command { .. } => None,
        }
    }

    pub fn is_command(&self) -> bool {
        matches!(self, Hook::Command { .. })
    }

    pub fn is_prompt(&self) -> bool {
        matches!(self, Hook::Prompt { .. })
    }

    pub fn is_agent(&self) -> bool {
        matches!(self, Hook::Agent { .. })
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Hook::Command { .. } => "command",
            Hook::Prompt { .. } => "prompt",
            Hook::Agent { .. } => "agent",
        }
    }
}

impl HooksSchema {
    /// Valid hook event names (case-sensitive)
    pub const VALID_EVENTS: &'static [&'static str] = &[
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
        "TeammateIdle",
        "TaskCompleted",
    ];

    /// Events that require a matcher field (tool-related events)
    pub const TOOL_EVENTS: &'static [&'static str] = &[
        "PreToolUse",
        "PermissionRequest",
        "PostToolUse",
        "PostToolUseFailure",
    ];

    /// Events that support prompt hooks (Stop and SubagentStop only)
    pub const PROMPT_EVENTS: &'static [&'static str] = &["Stop", "SubagentStop"];

    /// Check if an event is a tool event (requires matcher)
    pub fn is_tool_event(event: &str) -> bool {
        Self::TOOL_EVENTS.contains(&event)
    }

    /// Check if an event supports prompt hooks
    pub fn is_prompt_event(event: &str) -> bool {
        Self::PROMPT_EVENTS.contains(&event)
    }

    pub fn from_json(content: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(content)
    }

    pub fn validate_events(&self) -> Vec<String> {
        let mut errors = Vec::new();

        for event in self.hooks.keys() {
            if !Self::VALID_EVENTS.contains(&event.as_str()) {
                errors.push(format!(
                    "Unknown hook event '{}', valid events: {:?}",
                    event,
                    Self::VALID_EVENTS
                ));
            }
        }

        errors
    }

    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        errors.extend(self.validate_events());

        for (event, matchers) in &self.hooks {
            for (i, matcher) in matchers.iter().enumerate() {
                if matcher.hooks.is_empty() {
                    errors.push(format!(
                        "Hook event '{}' matcher {} has empty hooks array",
                        event, i
                    ));
                }
            }
        }

        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_hooks_map_validates_ok() {
        let schema = HooksSchema {
            hooks: HashMap::new(),
        };
        let errors = schema.validate();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_empty_matchers_array_validates_ok() {
        let mut hooks = HashMap::new();
        hooks.insert("PreToolUse".to_string(), vec![]);
        let schema = HooksSchema { hooks };
        let errors = schema.validate();
        assert!(errors.is_empty(), "Empty matchers array is valid");
    }

    #[test]
    fn test_empty_hooks_array_in_matcher_reports_error() {
        let mut hooks = HashMap::new();
        hooks.insert(
            "PreToolUse".to_string(),
            vec![HookMatcher {
                matcher: Some("Bash".to_string()),
                hooks: vec![],
            }],
        );
        let schema = HooksSchema { hooks };
        let errors = schema.validate();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("empty hooks array"));
    }

    #[test]
    fn test_unknown_event_name_reports_error() {
        let mut hooks = HashMap::new();
        hooks.insert(
            "NonExistentEvent".to_string(),
            vec![HookMatcher {
                matcher: None,
                hooks: vec![Hook::Command {
                    command: Some("echo hi".to_string()),
                    timeout: None,
                    model: None,
                }],
            }],
        );
        let schema = HooksSchema { hooks };
        let errors = schema.validate_events();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Unknown hook event"));
    }

    #[test]
    fn test_hook_type_name() {
        let cmd = Hook::Command {
            command: Some("echo".to_string()),
            timeout: None,
            model: None,
        };
        assert_eq!(cmd.type_name(), "command");
        assert!(cmd.is_command());
        assert!(!cmd.is_prompt());
        assert!(!cmd.is_agent());

        let prompt = Hook::Prompt {
            prompt: Some("summarize".to_string()),
            timeout: None,
            model: None,
        };
        assert_eq!(prompt.type_name(), "prompt");
        assert!(prompt.is_prompt());

        let agent = Hook::Agent {
            prompt: Some("review".to_string()),
            timeout: None,
            model: None,
        };
        assert_eq!(agent.type_name(), "agent");
        assert!(agent.is_agent());
    }

    #[test]
    fn test_is_tool_event() {
        assert!(HooksSchema::is_tool_event("PreToolUse"));
        assert!(HooksSchema::is_tool_event("PostToolUse"));
        assert!(!HooksSchema::is_tool_event("Stop"));
        assert!(!HooksSchema::is_tool_event("Notification"));
    }

    #[test]
    fn test_is_prompt_event() {
        assert!(HooksSchema::is_prompt_event("Stop"));
        assert!(HooksSchema::is_prompt_event("SubagentStop"));
        assert!(!HooksSchema::is_prompt_event("PreToolUse"));
    }

    #[test]
    fn test_settings_schema_from_json_with_hooks() {
        let json = r#"{"hooks": {"PreToolUse": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "echo test"}]}]}}"#;
        let settings = SettingsSchema::from_json(json).unwrap();
        assert!(settings.hooks.contains_key("PreToolUse"));
        let hooks_schema = settings.to_hooks_schema();
        assert!(hooks_schema.validate_events().is_empty());
    }

    #[test]
    fn test_settings_schema_from_json_no_hooks() {
        let json = r#"{"other_field": "value"}"#;
        let settings = SettingsSchema::from_json(json).unwrap();
        assert!(settings.hooks.is_empty());
    }
}
