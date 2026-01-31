//! Plugin manifest validation (CC-PL-001 to CC-PL-005)
//!
//! Validates Claude Code plugin definitions in `.claude-plugin/plugin.json`

use crate::{config::LintConfig, diagnostics::Diagnostic, rules::Validator};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use std::path::Path;

/// Semver regex pattern: X.Y.Z where X, Y, Z are non-negative integers
static SEMVER_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\d+\.\d+\.\d+$").expect("Invalid semver regex"));

/// Maximum directory traversal depth to prevent unbounded filesystem walking
const MAX_TRAVERSAL_DEPTH: usize = 10;

/// Minimal plugin schema for validation (allows optional/missing fields)
#[derive(Debug, Deserialize, Default)]
struct PluginManifest {
    name: Option<String>,
    description: Option<String>,
    version: Option<String>,
}

pub struct PluginValidator;

impl PluginValidator {
    /// Find the plugin root directory by looking for .claude-plugin parent.
    /// Limited to MAX_TRAVERSAL_DEPTH levels to prevent unbounded traversal.
    fn find_plugin_root(path: &Path) -> Option<&Path> {
        path.ancestors()
            .skip(1) // Start from the parent directory
            .take(MAX_TRAVERSAL_DEPTH)
            .find(|dir| {
                dir.file_name()
                    .map_or(false, |name| name.to_string_lossy().ends_with(".claude-plugin"))
            })
    }

    /// Check if plugin.json is in the correct .claude-plugin/ directory location.
    /// Returns true if properly located, false otherwise.
    fn is_in_claude_plugin_dir(path: &Path) -> bool {
        path.parent()
            .and_then(|p| p.file_name())
            .map_or(false, |name| name.to_string_lossy().ends_with(".claude-plugin"))
    }

    /// Check for misplaced components (skills, agents, hooks) inside .claude-plugin/
    fn check_misplaced_components(plugin_dir: &Path) -> Vec<String> {
        ["skills", "agents", "hooks"]
            .iter()
            .filter(|&&component| plugin_dir.join(component).is_dir())
            .map(|&s| s.to_string())
            .collect()
    }

    /// Validate semver format (X.Y.Z)
    fn is_valid_semver(version: &str) -> bool {
        SEMVER_REGEX.is_match(version)
    }
}

impl Validator for PluginValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Early return if plugins category is entirely disabled
        if !config.rules.plugins {
            return diagnostics;
        }

        // Parse plugin.json
        let manifest: PluginManifest = match serde_json::from_str(content) {
            Ok(m) => m,
            Err(e) => {
                diagnostics.push(Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "plugin::parse",
                    format!("Failed to parse plugin manifest: {}", e),
                ));
                return diagnostics;
            }
        };

        // CC-PL-001: Plugin manifest must be in .claude-plugin/ directory
        if config.is_rule_enabled("CC-PL-001") {
            if !Self::is_in_claude_plugin_dir(path) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-PL-001",
                        "plugin.json must be located in a .claude-plugin/ directory".to_string(),
                    )
                    .with_suggestion(
                        "Move plugin.json to <plugin-name>.claude-plugin/plugin.json".to_string(),
                    ),
                );
            }
        }

        // CC-PL-002: Components (skills/agents/hooks) must NOT be inside .claude-plugin/
        if config.is_rule_enabled("CC-PL-002") {
            if let Some(plugin_dir) = Self::find_plugin_root(path) {
                let misplaced = Self::check_misplaced_components(plugin_dir);
                for component in misplaced {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CC-PL-002",
                            format!(
                                "'{}/' directory must not be inside .claude-plugin/",
                                component
                            ),
                        )
                        .with_suggestion(format!(
                            "Move '{}/' to the plugin root directory, outside .claude-plugin/",
                            component
                        )),
                    );
                }
            }
        }

        // CC-PL-003: Version must be valid semver (X.Y.Z)
        if config.is_rule_enabled("CC-PL-003") {
            if let Some(version) = &manifest.version {
                if !Self::is_valid_semver(version) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CC-PL-003",
                            format!(
                                "Invalid version '{}'. Version must be semver format (X.Y.Z)",
                                version
                            ),
                        )
                        .with_suggestion("Use semver format like '1.0.0' or '0.1.0'".to_string()),
                    );
                }
            }
        }

        // CC-PL-004: Required fields (name, description, version)
        if config.is_rule_enabled("CC-PL-004") {
            let mut missing_fields = Vec::new();

            if manifest.name.is_none() {
                missing_fields.push("name");
            }
            if manifest.description.is_none() {
                missing_fields.push("description");
            }
            if manifest.version.is_none() {
                missing_fields.push("version");
            }

            if !missing_fields.is_empty() {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-PL-004",
                        format!(
                            "Plugin manifest is missing required field(s): {}",
                            missing_fields.join(", ")
                        ),
                    )
                    .with_suggestion(format!(
                        "Add the following field(s) to plugin.json: {}",
                        missing_fields.join(", ")
                    )),
                );
            }
        }

        // CC-PL-005: Name must not be empty
        if config.is_rule_enabled("CC-PL-005") {
            if manifest.name.as_deref().unwrap_or("").trim().is_empty() && manifest.name.is_some() {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-PL-005",
                        "Plugin name cannot be empty".to_string(),
                    )
                    .with_suggestion("Provide a meaningful plugin name".to_string()),
                );
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;
    use tempfile::TempDir;

    fn validate(content: &str) -> Vec<Diagnostic> {
        let validator = PluginValidator;
        // Use a path that is inside .claude-plugin/ for default tests
        validator.validate(
            Path::new("my-plugin.claude-plugin/plugin.json"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_with_path(path: &Path, content: &str) -> Vec<Diagnostic> {
        let validator = PluginValidator;
        validator.validate(path, content, &LintConfig::default())
    }

    // ===== CC-PL-001 Tests: Plugin manifest location =====

    #[test]
    fn test_cc_pl_001_plugin_not_in_claude_plugin_dir() {
        let content = r#"{
            "name": "my-plugin",
            "description": "A test plugin",
            "version": "1.0.0"
        }"#;

        // Path is not in .claude-plugin/
        let diagnostics = validate_with_path(Path::new("some/other/plugin.json"), content);
        let cc_pl_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-001").collect();

        assert_eq!(cc_pl_001.len(), 1);
        assert_eq!(cc_pl_001[0].level, DiagnosticLevel::Error);
        assert!(cc_pl_001[0].message.contains(".claude-plugin/"));
    }

    #[test]
    fn test_cc_pl_001_plugin_in_claude_plugin_dir() {
        let content = r#"{
            "name": "my-plugin",
            "description": "A test plugin",
            "version": "1.0.0"
        }"#;

        let diagnostics =
            validate_with_path(Path::new("my-plugin.claude-plugin/plugin.json"), content);
        let cc_pl_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-001").collect();

        assert_eq!(cc_pl_001.len(), 0);
    }

    #[test]
    fn test_cc_pl_001_plugin_in_nested_claude_plugin_dir() {
        let content = r#"{
            "name": "my-plugin",
            "description": "A test plugin",
            "version": "1.0.0"
        }"#;

        let diagnostics = validate_with_path(
            Path::new("plugins/my-awesome.claude-plugin/plugin.json"),
            content,
        );
        let cc_pl_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-001").collect();

        assert_eq!(cc_pl_001.len(), 0);
    }

    // ===== CC-PL-002 Tests: Misplaced components =====

    #[test]
    fn test_cc_pl_002_skills_inside_claude_plugin() {
        let temp = TempDir::new().unwrap();
        let plugin_dir = temp.path().join("test.claude-plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::create_dir_all(plugin_dir.join("skills")).unwrap();

        let plugin_json = plugin_dir.join("plugin.json");
        let content = r#"{
            "name": "my-plugin",
            "description": "A test plugin",
            "version": "1.0.0"
        }"#;

        let diagnostics = validate_with_path(&plugin_json, content);
        let cc_pl_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-002").collect();

        assert_eq!(cc_pl_002.len(), 1);
        assert!(cc_pl_002[0].message.contains("skills"));
    }

    #[test]
    fn test_cc_pl_002_agents_inside_claude_plugin() {
        let temp = TempDir::new().unwrap();
        let plugin_dir = temp.path().join("test.claude-plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::create_dir_all(plugin_dir.join("agents")).unwrap();

        let plugin_json = plugin_dir.join("plugin.json");
        let content = r#"{
            "name": "my-plugin",
            "description": "A test plugin",
            "version": "1.0.0"
        }"#;

        let diagnostics = validate_with_path(&plugin_json, content);
        let cc_pl_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-002").collect();

        assert_eq!(cc_pl_002.len(), 1);
        assert!(cc_pl_002[0].message.contains("agents"));
    }

    #[test]
    fn test_cc_pl_002_hooks_inside_claude_plugin() {
        let temp = TempDir::new().unwrap();
        let plugin_dir = temp.path().join("test.claude-plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::create_dir_all(plugin_dir.join("hooks")).unwrap();

        let plugin_json = plugin_dir.join("plugin.json");
        let content = r#"{
            "name": "my-plugin",
            "description": "A test plugin",
            "version": "1.0.0"
        }"#;

        let diagnostics = validate_with_path(&plugin_json, content);
        let cc_pl_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-002").collect();

        assert_eq!(cc_pl_002.len(), 1);
        assert!(cc_pl_002[0].message.contains("hooks"));
    }

    #[test]
    fn test_cc_pl_002_multiple_misplaced_components() {
        let temp = TempDir::new().unwrap();
        let plugin_dir = temp.path().join("test.claude-plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::create_dir_all(plugin_dir.join("skills")).unwrap();
        std::fs::create_dir_all(plugin_dir.join("agents")).unwrap();
        std::fs::create_dir_all(plugin_dir.join("hooks")).unwrap();

        let plugin_json = plugin_dir.join("plugin.json");
        let content = r#"{
            "name": "my-plugin",
            "description": "A test plugin",
            "version": "1.0.0"
        }"#;

        let diagnostics = validate_with_path(&plugin_json, content);
        let cc_pl_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-002").collect();

        assert_eq!(cc_pl_002.len(), 3);
    }

    #[test]
    fn test_cc_pl_002_no_misplaced_components() {
        let temp = TempDir::new().unwrap();
        let plugin_dir = temp.path().join("test.claude-plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();

        let plugin_json = plugin_dir.join("plugin.json");
        let content = r#"{
            "name": "my-plugin",
            "description": "A test plugin",
            "version": "1.0.0"
        }"#;

        let diagnostics = validate_with_path(&plugin_json, content);
        let cc_pl_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-002").collect();

        assert_eq!(cc_pl_002.len(), 0);
    }

    // ===== CC-PL-003 Tests: Semver validation =====

    #[test]
    fn test_cc_pl_003_invalid_semver_missing_patch() {
        let content = r#"{
            "name": "my-plugin",
            "description": "A test plugin",
            "version": "1.0"
        }"#;

        let diagnostics = validate(content);
        let cc_pl_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-003").collect();

        assert_eq!(cc_pl_003.len(), 1);
        assert_eq!(cc_pl_003[0].level, DiagnosticLevel::Error);
        assert!(cc_pl_003[0].message.contains("1.0"));
    }

    #[test]
    fn test_cc_pl_003_invalid_semver_with_v_prefix() {
        let content = r#"{
            "name": "my-plugin",
            "description": "A test plugin",
            "version": "v1.0.0"
        }"#;

        let diagnostics = validate(content);
        let cc_pl_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-003").collect();

        assert_eq!(cc_pl_003.len(), 1);
        assert!(cc_pl_003[0].message.contains("v1.0.0"));
    }

    #[test]
    fn test_cc_pl_003_invalid_semver_with_prerelease() {
        let content = r#"{
            "name": "my-plugin",
            "description": "A test plugin",
            "version": "1.0.0-beta"
        }"#;

        let diagnostics = validate(content);
        let cc_pl_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-003").collect();

        assert_eq!(cc_pl_003.len(), 1);
    }

    #[test]
    fn test_cc_pl_003_invalid_semver_non_numeric() {
        let content = r#"{
            "name": "my-plugin",
            "description": "A test plugin",
            "version": "one.two.three"
        }"#;

        let diagnostics = validate(content);
        let cc_pl_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-003").collect();

        assert_eq!(cc_pl_003.len(), 1);
    }

    #[test]
    fn test_cc_pl_003_valid_semver() {
        let content = r#"{
            "name": "my-plugin",
            "description": "A test plugin",
            "version": "1.0.0"
        }"#;

        let diagnostics = validate(content);
        let cc_pl_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-003").collect();

        assert_eq!(cc_pl_003.len(), 0);
    }

    #[test]
    fn test_cc_pl_003_valid_semver_large_numbers() {
        let content = r#"{
            "name": "my-plugin",
            "description": "A test plugin",
            "version": "10.20.300"
        }"#;

        let diagnostics = validate(content);
        let cc_pl_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-003").collect();

        assert_eq!(cc_pl_003.len(), 0);
    }

    #[test]
    fn test_cc_pl_003_valid_semver_zeros() {
        let content = r#"{
            "name": "my-plugin",
            "description": "A test plugin",
            "version": "0.0.0"
        }"#;

        let diagnostics = validate(content);
        let cc_pl_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-003").collect();

        assert_eq!(cc_pl_003.len(), 0);
    }

    // ===== CC-PL-004 Tests: Required fields =====

    #[test]
    fn test_cc_pl_004_missing_name() {
        let content = r#"{
            "description": "A test plugin",
            "version": "1.0.0"
        }"#;

        let diagnostics = validate(content);
        let cc_pl_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-004").collect();

        assert_eq!(cc_pl_004.len(), 1);
        assert!(cc_pl_004[0].message.contains("name"));
    }

    #[test]
    fn test_cc_pl_004_missing_description() {
        let content = r#"{
            "name": "my-plugin",
            "version": "1.0.0"
        }"#;

        let diagnostics = validate(content);
        let cc_pl_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-004").collect();

        assert_eq!(cc_pl_004.len(), 1);
        assert!(cc_pl_004[0].message.contains("description"));
    }

    #[test]
    fn test_cc_pl_004_missing_version() {
        let content = r#"{
            "name": "my-plugin",
            "description": "A test plugin"
        }"#;

        let diagnostics = validate(content);
        let cc_pl_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-004").collect();

        assert_eq!(cc_pl_004.len(), 1);
        assert!(cc_pl_004[0].message.contains("version"));
    }

    #[test]
    fn test_cc_pl_004_missing_all_required_fields() {
        let content = r#"{}"#;

        let diagnostics = validate(content);
        let cc_pl_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-004").collect();

        assert_eq!(cc_pl_004.len(), 1);
        assert!(cc_pl_004[0].message.contains("name"));
        assert!(cc_pl_004[0].message.contains("description"));
        assert!(cc_pl_004[0].message.contains("version"));
    }

    #[test]
    fn test_cc_pl_004_all_required_fields_present() {
        let content = r#"{
            "name": "my-plugin",
            "description": "A test plugin",
            "version": "1.0.0"
        }"#;

        let diagnostics = validate(content);
        let cc_pl_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-004").collect();

        assert_eq!(cc_pl_004.len(), 0);
    }

    // ===== CC-PL-005 Tests: Empty name =====

    #[test]
    fn test_cc_pl_005_empty_name() {
        let content = r#"{
            "name": "",
            "description": "A test plugin",
            "version": "1.0.0"
        }"#;

        let diagnostics = validate(content);
        let cc_pl_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-005").collect();

        assert_eq!(cc_pl_005.len(), 1);
        assert_eq!(cc_pl_005[0].level, DiagnosticLevel::Error);
        assert!(cc_pl_005[0].message.contains("cannot be empty"));
    }

    #[test]
    fn test_cc_pl_005_whitespace_name() {
        let content = r#"{
            "name": "   ",
            "description": "A test plugin",
            "version": "1.0.0"
        }"#;

        let diagnostics = validate(content);
        let cc_pl_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-005").collect();

        assert_eq!(cc_pl_005.len(), 1);
    }

    #[test]
    fn test_cc_pl_005_valid_name() {
        let content = r#"{
            "name": "my-plugin",
            "description": "A test plugin",
            "version": "1.0.0"
        }"#;

        let diagnostics = validate(content);
        let cc_pl_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-005").collect();

        assert_eq!(cc_pl_005.len(), 0);
    }

    // ===== Parse Error Tests =====

    #[test]
    fn test_invalid_json() {
        let content = r#"{ invalid json }"#;

        let diagnostics = validate(content);
        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "plugin::parse")
            .collect();

        assert_eq!(parse_errors.len(), 1);
        assert!(parse_errors[0].message.contains("Failed to parse"));
    }

    #[test]
    fn test_empty_content() {
        let content = "";

        let diagnostics = validate(content);
        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "plugin::parse")
            .collect();

        assert_eq!(parse_errors.len(), 1);
    }

    // ===== Valid Plugin Tests =====

    #[test]
    fn test_valid_plugin_minimal() {
        let content = r#"{
            "name": "my-plugin",
            "description": "A helpful plugin for testing",
            "version": "1.0.0"
        }"#;

        let diagnostics = validate(content);
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .collect();

        assert!(errors.is_empty());
    }

    #[test]
    fn test_valid_plugin_with_optional_fields() {
        let content = r#"{
            "name": "my-awesome-plugin",
            "description": "A fully configured plugin",
            "version": "2.5.10",
            "author": {
                "name": "Test Author",
                "email": "test@example.com"
            },
            "homepage": "https://example.com",
            "repository": "https://github.com/user/repo",
            "license": "MIT",
            "keywords": ["test", "plugin", "awesome"]
        }"#;

        let diagnostics = validate(content);
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .collect();

        assert!(errors.is_empty());
    }

    // ===== Config Wiring Tests =====

    #[test]
    fn test_config_disabled_plugins_category_returns_empty() {
        let mut config = LintConfig::default();
        config.rules.plugins = false;

        let content = r#"{
            "description": "Missing name and version"
        }"#;

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            Path::new("test.claude-plugin/plugin.json"),
            content,
            &config,
        );

        // No diagnostics should fire when plugins category is disabled
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["CC-PL-004".to_string()];

        let content = r#"{
            "version": "1.0.0"
        }"#;

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            Path::new("test.claude-plugin/plugin.json"),
            content,
            &config,
        );

        // CC-PL-004 should not fire when specifically disabled
        let cc_pl_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-004").collect();
        assert_eq!(cc_pl_004.len(), 0);
    }

    #[test]
    fn test_config_cursor_target_disables_plugin_rules() {
        use crate::config::TargetTool;

        let mut config = LintConfig::default();
        config.target = TargetTool::Cursor;

        let content = r#"{}"#;

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            Path::new("test.claude-plugin/plugin.json"),
            content,
            &config,
        );

        // CC-PL-* rules should not fire for Cursor target
        let plugin_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("CC-PL-"))
            .collect();
        assert_eq!(plugin_rules.len(), 0);
    }

    #[test]
    fn test_config_claude_code_target_enables_plugin_rules() {
        use crate::config::TargetTool;

        let mut config = LintConfig::default();
        config.target = TargetTool::ClaudeCode;

        let content = r#"{}"#;

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            Path::new("test.claude-plugin/plugin.json"),
            content,
            &config,
        );

        // CC-PL-004 should fire for ClaudeCode target (missing required fields)
        let cc_pl_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-004").collect();
        assert_eq!(cc_pl_004.len(), 1);
    }

    // ===== Helper Function Tests =====

    #[test]
    fn test_is_in_claude_plugin_dir() {
        assert!(PluginValidator::is_in_claude_plugin_dir(Path::new(
            "my-plugin.claude-plugin/plugin.json"
        )));
        assert!(PluginValidator::is_in_claude_plugin_dir(Path::new(
            "path/to/test.claude-plugin/plugin.json"
        )));
        assert!(!PluginValidator::is_in_claude_plugin_dir(Path::new(
            "some/other/plugin.json"
        )));
        assert!(!PluginValidator::is_in_claude_plugin_dir(Path::new(
            "plugin.json"
        )));
    }

    #[test]
    fn test_is_valid_semver() {
        assert!(PluginValidator::is_valid_semver("1.0.0"));
        assert!(PluginValidator::is_valid_semver("0.0.0"));
        assert!(PluginValidator::is_valid_semver("10.20.30"));
        assert!(PluginValidator::is_valid_semver("100.200.300"));

        assert!(!PluginValidator::is_valid_semver("1.0"));
        assert!(!PluginValidator::is_valid_semver("1"));
        assert!(!PluginValidator::is_valid_semver("v1.0.0"));
        assert!(!PluginValidator::is_valid_semver("1.0.0-beta"));
        assert!(!PluginValidator::is_valid_semver("1.0.0+build"));
        assert!(!PluginValidator::is_valid_semver("one.two.three"));
        assert!(!PluginValidator::is_valid_semver(""));
    }

    // ===== Fixture Tests =====

    #[test]
    fn test_fixture_valid_plugin() {
        let content = include_str!("../../../../tests/fixtures/plugins/valid-plugin.json");
        let diagnostics = validate_with_path(
            Path::new("test.claude-plugin/plugin.json"),
            content,
        );
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .collect();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_fixture_missing_fields() {
        let content = include_str!("../../../../tests/fixtures/plugins/missing-fields.json");
        let diagnostics = validate_with_path(
            Path::new("test.claude-plugin/plugin.json"),
            content,
        );
        let cc_pl_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-004").collect();
        assert!(!cc_pl_004.is_empty());
    }

    #[test]
    fn test_fixture_empty_name() {
        let content = include_str!("../../../../tests/fixtures/plugins/empty-name.json");
        let diagnostics = validate_with_path(
            Path::new("test.claude-plugin/plugin.json"),
            content,
        );
        let cc_pl_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-005").collect();
        assert!(!cc_pl_005.is_empty());
    }

    #[test]
    fn test_fixture_invalid_semver() {
        let content = include_str!("../../../../tests/fixtures/plugins/invalid-semver.json");
        let diagnostics = validate_with_path(
            Path::new("test.claude-plugin/plugin.json"),
            content,
        );
        let cc_pl_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-003").collect();
        assert!(!cc_pl_003.is_empty());
    }

    #[test]
    fn test_fixture_wrong_location() {
        let content = include_str!("../../../../tests/fixtures/plugins/valid-plugin.json");
        // Test with a path that is NOT in .claude-plugin/
        let diagnostics = validate_with_path(
            Path::new("wrong/location/plugin.json"),
            content,
        );
        let cc_pl_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-001").collect();
        assert!(!cc_pl_001.is_empty());
    }

    #[test]
    fn test_cc_pl_003_version_with_whitespace() {
        let content = r#"{"name": "test", "description": "test", "version": " 1.0.0 "}"#;
        let diagnostics = validate(content);
        let cc_pl_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-003").collect();
        assert!(!cc_pl_003.is_empty(), "Version with whitespace should fail CC-PL-003");
    }

    #[test]
    fn test_cc_pl_005_tab_only_name() {
        let content = r#"{"name": "\t\t", "description": "test", "version": "1.0.0"}"#;
        let diagnostics = validate(content);
        let cc_pl_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-005").collect();
        assert!(!cc_pl_005.is_empty(), "Tab-only name should fail CC-PL-005");
    }

    #[test]
    fn test_cc_pl_005_mixed_whitespace_name() {
        let content = r#"{"name": " \n\r\t ", "description": "test", "version": "1.0.0"}"#;
        let diagnostics = validate(content);
        let cc_pl_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-005").collect();
        assert!(!cc_pl_005.is_empty(), "Mixed whitespace name should fail CC-PL-005");
    }

    #[test]
    fn test_cc_pl_002_component_as_file() {
        use std::fs::File;
        let temp_dir = TempDir::new().unwrap();
        let plugin_dir = temp_dir.path().join("test.claude-plugin");
        std::fs::create_dir(&plugin_dir).unwrap();
        let plugin_json = plugin_dir.join("plugin.json");
        std::fs::write(&plugin_json, r#"{"name": "test", "description": "test", "version": "1.0.0"}"#).unwrap();

        // Create "skills" as a FILE, not a directory
        let skills_file = plugin_dir.join("skills");
        File::create(&skills_file).unwrap();

        let content = std::fs::read_to_string(&plugin_json).unwrap();
        let diagnostics = validate_with_path(&plugin_json, &content);

        // Should NOT raise CC-PL-002 since skills is a file, not a directory
        let cc_pl_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-002").collect();
        assert!(cc_pl_002.is_empty(), "skills as file should not trigger CC-PL-002");
    }

    #[test]
    fn test_cc_pl_003_empty_version_string() {
        let content = r#"{"name": "test", "description": "test", "version": ""}"#;
        let diagnostics = validate(content);
        let cc_pl_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-003").collect();
        assert!(!cc_pl_003.is_empty(), "Empty version string should fail CC-PL-003");
    }

    #[test]
    fn test_cc_pl_003_extremely_large_version_numbers() {
        let content = r#"{"name": "test", "description": "test", "version": "999999999999999.0.0"}"#;
        let diagnostics = validate(content);
        let cc_pl_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-PL-003").collect();
        assert!(cc_pl_003.is_empty(), "Extremely large version numbers pass regex validation");
    }
}
