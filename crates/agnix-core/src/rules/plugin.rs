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
                let mut diag = Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "CC-PL-003",
                    t!("rules.cc_pl_003.message", version = schema.version.as_str()),
                )
                .with_suggestion(t!("rules.cc_pl_003.suggestion"));

                // Add fix if the version can be normalized to valid semver
                if let Some(normalized) = normalize_semver(version) {
                    if let Some((start, end, _)) =
                        find_unique_json_string_value_range(content, "version")
                    {
                        diag = diag.with_fix(Fix::replace(
                            start,
                            end,
                            &normalized,
                            format!("Normalize version to '{}'", normalized),
                            true,
                        ));
                    }
                }

                diagnostics.push(diag);
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

/// Attempt to normalize a version string to valid semver.
/// Returns Some(normalized) if normalization is possible, None otherwise.
/// Examples: "1.0" -> "1.0.0", "v1.0.0" -> "1.0.0", "v2.1" -> "2.1.0"
fn normalize_semver(version: &str) -> Option<String> {
    let trimmed = version.trim();
    // Strip leading 'v' or 'V'
    let stripped = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed);

    // Already valid after stripping prefix?
    if semver::Version::parse(stripped).is_ok() {
        if stripped != trimmed {
            return Some(stripped.to_string());
        }
        return None; // Already valid, no normalization needed
    }

    // Try appending .0 for "X.Y" format
    let with_patch = format!("{}.0", stripped);
    if semver::Version::parse(&with_patch).is_ok() {
        return Some(with_patch);
    }

    // Try appending .0.0 for "X" format
    let with_minor_patch = format!("{}.0.0", stripped);
    if semver::Version::parse(&with_minor_patch).is_ok() {
        return Some(with_minor_patch);
    }

    None
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

    // ===== CC-PL-003 Auto-fix Tests =====

    #[test]
    fn test_cc_pl_003_has_fix_for_missing_patch() {
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

        let cc_pl_003 = diagnostics
            .iter()
            .find(|d| d.rule == "CC-PL-003")
            .expect("CC-PL-003 should be reported");
        assert!(cc_pl_003.has_fixes());
        let fix = &cc_pl_003.fixes[0];
        assert_eq!(fix.replacement, "1.0.0");
        assert!(fix.safe);
    }

    #[test]
    fn test_cc_pl_003_has_fix_for_v_prefix() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test-plugin","description":"desc","version":"v1.0.0"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let cc_pl_003 = diagnostics
            .iter()
            .find(|d| d.rule == "CC-PL-003")
            .expect("CC-PL-003 should be reported");
        assert!(cc_pl_003.has_fixes());
        let fix = &cc_pl_003.fixes[0];
        assert_eq!(fix.replacement, "1.0.0");
        assert!(fix.safe);
    }

    #[test]
    fn test_cc_pl_003_fix_applied_produces_valid_json() {
        let content = r#"{"name":"test-plugin","description":"desc","version":"v2.1"}"#;
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(&plugin_path, content);

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let cc_pl_003 = diagnostics
            .iter()
            .find(|d| d.rule == "CC-PL-003")
            .expect("CC-PL-003 should be reported");
        assert!(cc_pl_003.has_fixes());
        let fix = &cc_pl_003.fixes[0];

        // Apply fix
        let mut fixed = content.to_string();
        fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
        // Verify it's valid JSON with valid semver
        let parsed: serde_json::Value =
            serde_json::from_str(&fixed).expect("Fixed content should be valid JSON");
        let version = parsed.get("version").and_then(|v| v.as_str()).unwrap();
        assert!(
            semver::Version::parse(version).is_ok(),
            "Fixed version '{}' should be valid semver",
            version
        );
    }

    #[test]
    fn test_cc_pl_003_no_fix_for_totally_invalid() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test-plugin","description":"desc","version":"not-a-version"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let cc_pl_003 = diagnostics
            .iter()
            .find(|d| d.rule == "CC-PL-003")
            .expect("CC-PL-003 should be reported");
        // Should have no fix for completely invalid version strings
        assert!(!cc_pl_003.has_fixes());
    }

    #[test]
    fn test_normalize_semver_missing_patch() {
        assert_eq!(normalize_semver("1.0"), Some("1.0.0".to_string()));
        assert_eq!(normalize_semver("2.3"), Some("2.3.0".to_string()));
    }

    #[test]
    fn test_normalize_semver_v_prefix() {
        assert_eq!(normalize_semver("v1.0.0"), Some("1.0.0".to_string()));
        assert_eq!(normalize_semver("V2.0.0"), Some("2.0.0".to_string()));
        assert_eq!(normalize_semver("v1.2"), Some("1.2.0".to_string()));
    }

    #[test]
    fn test_normalize_semver_major_only() {
        assert_eq!(normalize_semver("1"), Some("1.0.0".to_string()));
    }

    #[test]
    fn test_normalize_semver_already_valid() {
        assert_eq!(normalize_semver("1.0.0"), None);
    }

    #[test]
    fn test_normalize_semver_garbage() {
        assert_eq!(normalize_semver("not-a-version"), None);
        assert_eq!(normalize_semver("abc"), None);
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
