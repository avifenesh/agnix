//! Plugin manifest validation (CC-PL-001 to CC-PL-005)

use crate::{
    config::LintConfig,
    diagnostics::{Diagnostic, Fix},
    rules::Validator,
    schemas::plugin::PluginSchema,
};
use regex::Regex;
use rust_i18n::t;
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
                    t!("rules.cc_pl_001.message"),
                )
                .with_suggestion(t!("rules.cc_pl_001.suggestion")),
            );
        }

        #[allow(clippy::collapsible_if)]
        if config.is_rule_enabled("CC-PL-002") && is_in_claude_plugin {
            if let Some(plugin_dir) = plugin_dir {
                let fs = config.fs();
                let disallowed = ["skills", "agents", "hooks", "commands"];
                for entry in disallowed {
                    if fs.exists(&plugin_dir.join(entry)) {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
                                "CC-PL-002",
                                t!("rules.cc_pl_002.message", component = entry),
                            )
                            .with_suggestion(t!("rules.cc_pl_002.suggestion")),
                        );
                    }
                }
            }
        }

        let raw_value: serde_json::Value = match serde_json::from_str(content) {
            Ok(v) => v,
            Err(e) => {
                if config.is_rule_enabled("CC-PL-006") {
                    diagnostics.push(Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-PL-006",
                        t!("rules.cc_pl_006.message", error = e.to_string()),
                    ));
                }
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
                    let mut diagnostic = Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-PL-005",
                        t!("rules.cc_pl_005.message"),
                    )
                    .with_suggestion(t!("rules.cc_pl_005.suggestion"));

                    // Unsafe auto-fix: populate empty plugin name with a deterministic placeholder.
                    if let Some((start, end, _)) =
                        find_unique_json_string_value_range(content, "name")
                    {
                        diagnostic = diagnostic.with_fix(Fix::replace(
                            start,
                            end,
                            "my-plugin",
                            "Set plugin name to 'my-plugin'",
                            false,
                        ));
                    }

                    diagnostics.push(diagnostic);
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
                        t!("rules.cc_pl_003.message", version = schema.version.as_str()),
                    )
                    .with_suggestion(t!("rules.cc_pl_003.suggestion")),
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
                t!("rules.cc_pl_004.message", field = field),
            )
            .with_suggestion(t!("rules.cc_pl_004.suggestion", field = field)),
        );
    }
}

fn is_valid_semver(version: &str) -> bool {
    semver::Version::parse(version).is_ok()
}

/// Find a unique string value span for a JSON key.
/// Returns (value_start, value_end, value_content_without_quotes).
fn find_unique_json_string_value_range(content: &str, key: &str) -> Option<(usize, usize, String)> {
    let pattern = format!(r#""{}"\s*:\s*"([^"]*)""#, regex::escape(key));
    let re = Regex::new(&pattern).ok()?;
    let mut captures = re.captures_iter(content);
    let first = captures.next()?;
    if captures.next().is_some() {
        return None;
    }
    let value_match = first.get(1)?;
    Some((
        value_match.start(),
        value_match.end(),
        value_match.as_str().to_string(),
    ))
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
    fn test_cc_pl_003_valid_prerelease_version() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test-plugin","description":"desc","version":"4.0.0-rc.1"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-003"));
    }

    #[test]
    fn test_cc_pl_003_valid_build_metadata() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test-plugin","description":"desc","version":"1.0.0+build.123"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-003"));
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

        let cc_pl_005 = diagnostics
            .iter()
            .find(|d| d.rule == "CC-PL-005")
            .expect("CC-PL-005 should be reported");
        assert!(cc_pl_005.has_fixes());
        let fix = &cc_pl_005.fixes[0];
        assert_eq!(fix.replacement, "my-plugin");
        assert!(!fix.safe);
    }

    // ===== CC-PL-006: Plugin Parse Error =====

    #[test]
    fn test_cc_pl_006_invalid_json() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(&plugin_path, r#"{ invalid json }"#);

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-PL-006")
            .collect();
        assert_eq!(parse_errors.len(), 1);
        assert!(parse_errors[0].message.contains("Failed to parse"));
    }

    #[test]
    fn test_cc_pl_006_truncated_json() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(&plugin_path, r#"{"name":"test"#);

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-006"));
    }

    #[test]
    fn test_cc_pl_006_empty_file() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(&plugin_path, "");

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-006"));
    }

    #[test]
    fn test_cc_pl_006_valid_json_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-006"));
    }

    #[test]
    fn test_cc_pl_006_disabled() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(&plugin_path, r#"{ invalid }"#);

        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["CC-PL-006".to_string()];

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &config,
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-006"));
    }

    // ===== Additional edge case tests =====

    #[test]
    fn test_cc_pl_001_valid_location_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-001"));
    }

    #[test]
    fn test_cc_pl_001_disabled() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0"}"#,
        );

        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["CC-PL-001".to_string()];

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &config,
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-001"));
    }

    #[test]
    fn test_cc_pl_002_no_components_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0"}"#,
        );
        // No skills/agents/hooks/commands directories

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-002"));
    }

    #[test]
    fn test_cc_pl_002_multiple_components() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0"}"#,
        );
        // Create multiple disallowed directories
        fs::create_dir_all(temp.path().join(".claude-plugin").join("skills")).unwrap();
        fs::create_dir_all(temp.path().join(".claude-plugin").join("agents")).unwrap();
        fs::create_dir_all(temp.path().join(".claude-plugin").join("hooks")).unwrap();
        fs::create_dir_all(temp.path().join(".claude-plugin").join("commands")).unwrap();

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let pl_002_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-PL-002")
            .collect();
        assert_eq!(pl_002_errors.len(), 4);
    }

    #[test]
    fn test_cc_pl_004_all_fields_present_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"A test plugin","version":"1.0.0"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-004"));
    }

    #[test]
    fn test_cc_pl_004_empty_string_values() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"","version":""}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let pl_004_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-PL-004")
            .collect();
        // Both description and version are empty
        assert_eq!(pl_004_errors.len(), 2);
    }

    #[test]
    fn test_cc_pl_005_non_empty_name_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"my-plugin","description":"desc","version":"1.0.0"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-005"));
    }

    #[test]
    fn test_config_disabled_plugins_category() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join("plugin.json");
        write_plugin(&plugin_path, r#"{ invalid json }"#);

        let mut config = LintConfig::default();
        config.rules.plugins = false;

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &config,
        );

        assert!(diagnostics.is_empty());
    }
}
