//! Plugin manifest schema

use serde::{Deserialize, Serialize};

/// plugin.json schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSchema {
    /// Required: plugin name
    pub name: String,

    /// Required: description
    pub description: String,

    /// Required: version (semver)
    pub version: String,

    /// Optional: author info
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<AuthorInfo>,

    /// Optional: homepage URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,

    /// Optional: repository URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,

    /// Optional: license
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Optional: keywords
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl PluginSchema {
    /// Validate semver format
    pub fn validate_version(&self) -> Result<(), String> {
        // Basic semver check (major.minor.patch)
        let parts: Vec<&str> = self.version.split('.').collect();
        if parts.len() != 3 {
            return Err(format!(
                "Version must be in semver format (e.g., 1.0.0), got '{}'",
                self.version
            ));
        }

        for part in parts {
            if part.parse::<u32>().is_err() {
                return Err(format!("Version parts must be numbers, got '{}'", part));
            }
        }

        Ok(())
    }

    /// Run all validations
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.name.is_empty() {
            errors.push("Plugin name cannot be empty".to_string());
        }

        if self.description.is_empty() {
            errors.push("Plugin description cannot be empty".to_string());
        }

        if let Err(e) = self.validate_version() {
            errors.push(e);
        }

        errors
    }
}
