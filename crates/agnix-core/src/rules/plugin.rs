//! Plugin manifest validation (CC-PL-001 to CC-PL-005)

use crate::{
    config::LintConfig, diagnostics::Diagnostic, rules::Validator, schemas::plugin::PluginSchema,
};
use regex::Regex;
use std::path::Path;

pub struct PluginValidator;

impl Validator for PluginValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if !config.rules.plugins {
            return diagnostics;
        }

        let plugin_dir = path.parent();
        let is_in_claude_plugin = plugin_dir
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|n| n == ".claude-plugin")
            .unwrap_or(false);

        if config.is_rule_enabled("CC-PL-001") && !is_in_claude_plugin {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "CC-PL-001",
                    "plugin.json must be located in .claude-plugin/ directory".to_string(),
                )
                .with_suggestion("Move plugin.json to .claude-plugin/plugin.json".to_string()),
            );
        }

        if config.is_rule_enabled("CC-PL-002") && is_in_claude_plugin {
            if let Some(plugin_dir) = plugin_dir {
                let disallowed = ["skills", "agents", "hooks", "commands"];
                for entry in disallowed {
                    if plugin_dir.join(entry).exists() {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
                                "CC-PL-002",
                                format!("Component '{}' must not be inside .claude-plugin/", entry),
                            )
                            .with_suggestion(
                                "Move components to the plugin root directory".to_string(),
                            ),
                        );
                    }
                }
            }
        }

        let raw_value: serde_json::Value = match serde_json::from_str(content) {
            Ok(v) => v,
            Err(e) => {
                diagnostics.push(Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "plugin::parse",
                    format!("Failed to parse plugin.json: {}", e),
                ));
                return diagnostics;
            }
        };

        if config.is_rule_enabled("CC-PL-004") {
            check_required_field(&raw_value, "name", path, diagnostics.as_mut());
            check_required_field(&raw_value, "description", path, diagnostics.as_mut());
            check_required_field(&raw_value, "version", path, diagnostics.as_mut());
        }

        if config.is_rule_enabled("CC-PL-005") {
            if let Some(name) = raw_value.get("name").and_then(|v| v.as_str()) {
                if name.trim().is_empty() {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CC-PL-005",
                            "Plugin name must not be empty".to_string(),
                        )
                        .with_suggestion("Set a non-empty plugin name".to_string()),
                    );
                }
            }
        }

        let schema: PluginSchema = match serde_json::from_value(raw_value.clone()) {
            Ok(schema) => schema,
            Err(_) => {
                return diagnostics;
            }
        };

        if config.is_rule_enabled("CC-PL-003") {
            let version = schema.version.trim();
            if !version.is_empty() && !is_valid_semver(version) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-PL-003",
                        format!(
                            "Version must be semver format (major.minor.patch), got '{}'",
                            schema.version
                        ),
                    )
                    .with_suggestion("Use format like 1.0.0".to_string()),
                );
            }
        }

        diagnostics
    }
}

fn check_required_field(
    value: &serde_json::Value,
    field: &str,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let missing = match value.get(field) {
        Some(v) => !v.is_string() || v.as_str().map(|s| s.trim().is_empty()).unwrap_or(true),
        None => true,
    };

    if missing {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "CC-PL-004",
                format!("Missing required field: {}", field),
            )
            .with_suggestion(format!("Add '{}' field to plugin.json", field)),
        );
    }
}

fn is_valid_semver(version: &str) -> bool {
    let re = Regex::new(r"^\d+\.\d+\.\d+$").unwrap();
    re.is_match(version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use std::fs;
    use tempfile::TempDir;

    fn write_plugin(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn test_cc_pl_001_manifest_not_in_claude_plugin() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test-plugin","description":"desc","version":"1.0.0"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-001"));
    }

    #[test]
    fn test_cc_pl_002_components_in_claude_plugin() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test-plugin","description":"desc","version":"1.0.0"}"#,
        );
        fs::create_dir_all(temp.path().join(".claude-plugin").join("skills")).unwrap();

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-002"));
    }

    #[test]
    fn test_cc_pl_003_invalid_semver() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test-plugin","description":"desc","version":"1.0"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-003"));
    }

    #[test]
    fn test_cc_pl_003_skips_empty_version() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test-plugin","description":"desc","version":""}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-004"));
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-003"));
    }

    #[test]
    fn test_cc_pl_004_missing_required_fields() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(&plugin_path, r#"{"name":"test-plugin"}"#);

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-004"));
    }

    #[test]
    fn test_cc_pl_005_empty_name() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"  ","description":"desc","version":"1.0.0"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-005"));
    }
}
