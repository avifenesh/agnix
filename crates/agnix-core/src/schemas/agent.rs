//! Agent definition schema (Claude Code subagents)

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Agent .md file frontmatter schema
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentSchema {
    /// Required: agent name (CC-AG-001)
    #[serde(default)]
    pub name: Option<String>,

    /// Required: description (CC-AG-002)
    #[serde(default)]
    pub description: Option<String>,

    /// Optional: tools list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,

    /// Optional: disallowed tools
    #[serde(skip_serializing_if = "Option::is_none", rename = "disallowedTools")]
    pub disallowed_tools: Option<Vec<String>>,

    /// Optional: model (CC-AG-003)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Optional: permission mode (CC-AG-004)
    #[serde(skip_serializing_if = "Option::is_none", rename = "permissionMode")]
    pub permission_mode: Option<String>,

    /// Optional: skills to preload (CC-AG-005)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<Vec<String>>,

    /// Optional: hooks
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<Value>,
}

impl AgentSchema {
    /// Validate model value
    pub fn validate_model(&self) -> Result<(), String> {
        if let Some(model) = &self.model {
            let valid = ["sonnet", "opus", "haiku", "inherit"];
            if !valid.contains(&model.as_str()) {
                return Err(format!("Model must be one of: {:?}, got '{}'", valid, model));
            }
        }
        Ok(())
    }

    /// Validate permission mode
    pub fn validate_permission_mode(&self) -> Result<(), String> {
        if let Some(mode) = &self.permission_mode {
            let valid = ["default", "acceptEdits", "dontAsk", "bypassPermissions", "plan"];
            if !valid.contains(&mode.as_str()) {
                return Err(format!("Permission mode must be one of: {:?}, got '{}'", valid, mode));
            }
        }
        Ok(())
    }

    /// Run all validations
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if let Err(e) = self.validate_model() {
            errors.push(e);
        }
        if let Err(e) = self.validate_permission_mode() {
            errors.push(e);
        }

        errors
    }
}
