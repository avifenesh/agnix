//! Agent Skills schema (agentskills.io spec)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// SKILL.md frontmatter schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSchema {
    /// Required: skill name (lowercase, hyphens, 1-64 chars)
    pub name: String,

    /// Required: description (1-1024 chars)
    pub description: String,

    /// Optional: license identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Optional: compatibility notes (1-500 chars)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<String>,

    /// Optional: arbitrary metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Optional: space-delimited list of allowed tools (experimental)
    #[serde(skip_serializing_if = "Option::is_none", rename = "allowed-tools")]
    pub allowed_tools: Option<String>,

    // Claude Code extensions
    /// Optional: argument hint for autocomplete
    #[serde(skip_serializing_if = "Option::is_none", rename = "argument-hint")]
    pub argument_hint: Option<String>,

    /// Optional: disable model invocation
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "disable-model-invocation"
    )]
    pub disable_model_invocation: Option<bool>,

    /// Optional: user invocable
    #[serde(skip_serializing_if = "Option::is_none", rename = "user-invocable")]
    pub user_invocable: Option<bool>,

    /// Optional: model override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Optional: context mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,

    /// Optional: agent type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
}

impl SkillSchema {
    /// Validate skill name format
    pub fn validate_name(&self) -> Result<(), String> {
        let name = &self.name;

        // Length check
        if name.is_empty() || name.len() > 64 {
            return Err(format!("Name must be 1-64 characters, got {}", name.len()));
        }

        // Character check
        for ch in name.chars() {
            if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '-' {
                return Err(format!(
                    "Name must contain only lowercase letters, digits, and hyphens, found '{}'",
                    ch
                ));
            }
        }

        // Start/end check
        if name.starts_with('-') || name.ends_with('-') {
            return Err("Name cannot start or end with hyphen".to_string());
        }

        // Consecutive hyphens
        if name.contains("--") {
            return Err("Name cannot contain consecutive hyphens".to_string());
        }

        Ok(())
    }

    /// Validate description length
    pub fn validate_description(&self) -> Result<(), String> {
        let len = self.description.len();
        if len == 0 || len > 1024 {
            return Err(format!(
                "Description must be 1-1024 characters, got {}",
                len
            ));
        }
        Ok(())
    }

    /// Validate compatibility length
    pub fn validate_compatibility(&self) -> Result<(), String> {
        if let Some(compat) = &self.compatibility {
            let len = compat.len();
            if len == 0 || len > 500 {
                return Err(format!(
                    "Compatibility must be 1-500 characters, got {}",
                    len
                ));
            }
        }
        Ok(())
    }

    /// Validate model value
    pub fn validate_model(&self) -> Result<(), String> {
        if let Some(model) = &self.model {
            let valid = ["sonnet", "opus", "haiku", "inherit"];
            if !valid.contains(&model.as_str()) {
                return Err(format!(
                    "Model must be one of: {:?}, got '{}'",
                    valid, model
                ));
            }
        }
        Ok(())
    }

    /// Validate context value
    pub fn validate_context(&self) -> Result<(), String> {
        if let Some(context) = &self.context {
            if context != "fork" {
                return Err(format!("Context must be 'fork', got '{}'", context));
            }
        }
        Ok(())
    }

    /// Run all validations
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if let Err(e) = self.validate_name() {
            errors.push(e);
        }
        if let Err(e) = self.validate_description() {
            errors.push(e);
        }
        if let Err(e) = self.validate_compatibility() {
            errors.push(e);
        }
        if let Err(e) = self.validate_model() {
            errors.push(e);
        }
        if let Err(e) = self.validate_context() {
            errors.push(e);
        }

        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_skill_name() {
        let skill = SkillSchema {
            name: "code-review".to_string(),
            description: "Reviews code".to_string(),
            license: None,
            compatibility: None,
            metadata: None,
            allowed_tools: None,
            argument_hint: None,
            disable_model_invocation: None,
            user_invocable: None,
            model: None,
            context: None,
            agent: None,
        };
        assert!(skill.validate_name().is_ok());
    }

    #[test]
    fn test_invalid_skill_name_uppercase() {
        let skill = SkillSchema {
            name: "Code-Review".to_string(),
            description: "Reviews code".to_string(),
            license: None,
            compatibility: None,
            metadata: None,
            allowed_tools: None,
            argument_hint: None,
            disable_model_invocation: None,
            user_invocable: None,
            model: None,
            context: None,
            agent: None,
        };
        assert!(skill.validate_name().is_err());
    }

    #[test]
    fn test_invalid_model() {
        let skill = SkillSchema {
            name: "test".to_string(),
            description: "Test".to_string(),
            license: None,
            compatibility: None,
            metadata: None,
            allowed_tools: None,
            argument_hint: None,
            disable_model_invocation: None,
            user_invocable: None,
            model: Some("gpt-4".to_string()),
            context: None,
            agent: None,
        };
        assert!(skill.validate_model().is_err());
    }
}
