//! Plugin manifest validation (CC-PL-001 to CC-PL-010).
//!
//! Validates `.claude-plugin/plugin.json` manifests.

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

        // CC-PL-007: Invalid component path / CC-PL-008: Component inside .claude-plugin
        let pl_007_enabled = config.is_rule_enabled("CC-PL-007");
        let pl_008_enabled = config.is_rule_enabled("CC-PL-008");
        if pl_007_enabled || pl_008_enabled {
            let path_fields = ["commands", "agents", "skills", "hooks"];
            for field in path_fields {
                if pl_007_enabled {
                    check_component_paths(&raw_value, field, path, content, &mut diagnostics);
                }
                if pl_008_enabled {
                    check_component_inside_claude_plugin(&raw_value, field, path, &mut diagnostics);
                }
            }
        }

        // CC-PL-009: Invalid author object
        if config.is_rule_enabled("CC-PL-009") {
            if let Some(author) = raw_value.get("author") {
                if author.is_object() {
                    let name_empty = author
                        .get("name")
                        .and_then(|n| n.as_str())
                        .map(|n| n.trim().is_empty())
                        .unwrap_or(true);
                    if name_empty {
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                1,
                                0,
                                "CC-PL-009",
                                t!("rules.cc_pl_009.message"),
                            )
                            .with_suggestion(t!("rules.cc_pl_009.suggestion")),
                        );
                    }
                } else {
                    // author is present but not an object
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            1,
                            0,
                            "CC-PL-009",
                            t!("rules.cc_pl_009.message"),
                        )
                        .with_suggestion(t!("rules.cc_pl_009.suggestion")),
                    );
                }
            }
        }

        // CC-PL-010: Invalid homepage URL
        if config.is_rule_enabled("CC-PL-010") {
            if let Some(homepage_val) = raw_value.get("homepage") {
                match homepage_val.as_str() {
                    Some(homepage) => {
                        if !homepage.is_empty() && !is_valid_url(homepage) {
                            diagnostics.push(
                                Diagnostic::warning(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "CC-PL-010",
                                    t!("rules.cc_pl_010.message", url = homepage),
                                )
                                .with_suggestion(t!("rules.cc_pl_010.suggestion")),
                            );
                        }
                    }
                    None => {
                        // homepage is present but not a string (e.g., number, object)
                        let val_str = homepage_val.to_string();
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                1,
                                0,
                                "CC-PL-010",
                                t!("rules.cc_pl_010.message", url = val_str.as_str()),
                            )
                            .with_suggestion(t!("rules.cc_pl_010.suggestion")),
                        );
                    }
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

/// Check if a path string is invalid for a component path.
/// Must be relative (no absolute paths), must not use `..` traversal.
fn is_invalid_component_path(p: &str) -> bool {
    let trimmed = p.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Absolute paths: starts with `/` or `\`
    if trimmed.starts_with('/') || trimmed.starts_with('\\') {
        return true;
    }
    // Windows drive letter paths: C:\... or C:/...
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        if bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
            return true;
        }
    }
    // Check for `..` traversal in any component (split on both / and \)
    trimmed.split(['/', '\\']).any(|part| part == "..")
}

/// Check if a path is a relative path missing a `./` prefix (autofixable).
/// This is separate from `is_invalid_component_path`: paths like `skills/foo`
/// are not absolute or traversal, but should have `./` prepended.
fn is_autofixable_path(p: &str) -> bool {
    let trimmed = p.trim();
    // Must not be empty, not already have ./ prefix, and not be invalid
    !trimmed.is_empty()
        && !trimmed.starts_with("./")
        && !trimmed.starts_with(".\\")
        && !is_invalid_component_path(trimmed)
}

/// Check if a path starts with `.claude-plugin/`.
/// Normalizes an optional leading `./` or `.\\` before checking.
fn path_inside_claude_plugin(p: &str) -> bool {
    let trimmed = p.trim();
    let normalized = trimmed
        .strip_prefix("./")
        .or_else(|| trimmed.strip_prefix(".\\"))
        .unwrap_or(trimmed);
    normalized.starts_with(".claude-plugin/")
        || normalized.starts_with(".claude-plugin\\")
        || normalized == ".claude-plugin"
}

/// Extract string paths from a JSON value that can be a string or array of strings.
fn extract_paths(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::String(s) => vec![s.clone()],
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => vec![],
    }
}

/// CC-PL-007: Validate component paths are relative without `..` traversal.
/// Also flags relative paths missing a `./` prefix (with safe autofix).
fn check_component_paths(
    raw_value: &serde_json::Value,
    field: &str,
    path: &Path,
    content: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(val) = raw_value.get(field) {
        for p in extract_paths(val) {
            if is_invalid_component_path(&p) {
                // Absolute or traversal path: error without autofix
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-PL-007",
                        t!("rules.cc_pl_007.message", field = field, path = p.as_str()),
                    )
                    .with_suggestion(t!("rules.cc_pl_007.suggestion")),
                );
            } else if is_autofixable_path(&p) {
                // Relative path missing ./ prefix: error with safe autofix
                let mut diagnostic = Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "CC-PL-007",
                    t!("rules.cc_pl_007.message", field = field, path = p.as_str()),
                )
                .with_suggestion(t!("rules.cc_pl_007.suggestion"));

                if let Some((start, end, _)) = find_unique_json_string_value_range(content, field) {
                    let fixed = format!("./{}", p.trim());
                    diagnostic = diagnostic.with_fix(Fix::replace(
                        start,
                        end,
                        &fixed,
                        format!("Prepend './' to path: '{}'", p.trim()),
                        true,
                    ));
                }

                diagnostics.push(diagnostic);
            }
        }
    }
}

/// CC-PL-008: Detect component paths pointing inside .claude-plugin/.
fn check_component_inside_claude_plugin(
    raw_value: &serde_json::Value,
    field: &str,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(val) = raw_value.get(field) {
        for p in extract_paths(val) {
            if path_inside_claude_plugin(&p) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-PL-008",
                        t!("rules.cc_pl_008.message", field = field, path = p.as_str()),
                    )
                    .with_suggestion(t!("rules.cc_pl_008.suggestion")),
                );
            }
        }
    }
}

/// Check if a URL is valid (http or https scheme).
fn is_valid_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
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

    // ===== CC-PL-007: Invalid Component Path =====

    #[test]
    fn test_cc_pl_007_absolute_path() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","commands":"/usr/local/bin/cmd"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-007"));
    }

    #[test]
    fn test_cc_pl_007_windows_absolute_path() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","skills":"C:\\Users\\skills"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-007"));
    }

    #[test]
    fn test_cc_pl_007_traversal_path() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","agents":"../outside/agents"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-007"));
    }

    #[test]
    fn test_cc_pl_007_embedded_traversal() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","hooks":"./valid/../escape"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-007"));
    }

    #[test]
    fn test_cc_pl_007_array_of_paths() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","skills":["./valid","../invalid"]}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let pl_007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-PL-007")
            .collect();
        assert_eq!(
            pl_007.len(),
            1,
            "Only the invalid path should trigger CC-PL-007"
        );
    }

    #[test]
    fn test_cc_pl_007_valid_relative_path_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","commands":"./commands/"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-007"));
    }

    #[test]
    fn test_cc_pl_007_no_path_fields_no_error() {
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

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-007"));
    }

    #[test]
    fn test_cc_pl_007_disabled() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","commands":"/absolute"}"#,
        );

        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["CC-PL-007".to_string()];

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &config,
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-007"));
    }

    // ===== CC-PL-008: Component Inside .claude-plugin =====

    #[test]
    fn test_cc_pl_008_path_inside_claude_plugin() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","agents":".claude-plugin/agents"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-008"));
    }

    #[test]
    fn test_cc_pl_008_array_with_mixed_paths() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","skills":["./valid",".claude-plugin/invalid"]}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-008"));
    }

    #[test]
    fn test_cc_pl_008_valid_path_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","skills":"./skills/"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-008"));
    }

    #[test]
    fn test_cc_pl_008_disabled() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","agents":".claude-plugin/agents"}"#,
        );

        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["CC-PL-008".to_string()];

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &config,
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-008"));
    }

    // ===== CC-PL-009: Invalid Author Object =====

    #[test]
    fn test_cc_pl_009_empty_author_name() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","author":{"name":""}}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-009"));
    }

    #[test]
    fn test_cc_pl_009_whitespace_author_name() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","author":{"name":"  "}}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-009"));
    }

    #[test]
    fn test_cc_pl_009_missing_author_name() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","author":{"email":"a@b.com"}}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-009"));
    }

    #[test]
    fn test_cc_pl_009_author_not_object() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","author":"just a string"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-009"));
    }

    #[test]
    fn test_cc_pl_009_valid_author_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","author":{"name":"Test Author"}}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-009"));
    }

    #[test]
    fn test_cc_pl_009_no_author_no_error() {
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

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-009"));
    }

    #[test]
    fn test_cc_pl_009_disabled() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","author":{"name":""}}"#,
        );

        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["CC-PL-009".to_string()];

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &config,
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-009"));
    }

    // ===== CC-PL-010: Invalid Homepage URL =====

    #[test]
    fn test_cc_pl_010_invalid_url() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","homepage":"not-a-url"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-010"));
    }

    #[test]
    fn test_cc_pl_010_ftp_url() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","homepage":"ftp://example.com"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-010"));
    }

    #[test]
    fn test_cc_pl_010_valid_https_url_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","homepage":"https://example.com"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-010"));
    }

    #[test]
    fn test_cc_pl_010_valid_http_url_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","homepage":"http://example.com"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-010"));
    }

    #[test]
    fn test_cc_pl_010_no_homepage_no_error() {
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

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-010"));
    }

    #[test]
    fn test_cc_pl_010_empty_homepage_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","homepage":""}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-010"));
    }

    #[test]
    fn test_cc_pl_010_disabled() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","homepage":"not-a-url"}"#,
        );

        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["CC-PL-010".to_string()];

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &config,
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-010"));
    }

    // ===== Review feedback tests =====

    #[test]
    fn test_cc_pl_007_windows_forward_slash_absolute() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","commands":"C:/Users/skills"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(
            diagnostics.iter().any(|d| d.rule == "CC-PL-007"),
            "C:/ forward-slash Windows paths should be detected"
        );
    }

    #[test]
    fn test_cc_pl_007_trailing_traversal() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","hooks":"./foo/.."}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(
            diagnostics.iter().any(|d| d.rule == "CC-PL-007"),
            "Trailing /.. should be detected as traversal"
        );
    }

    #[test]
    fn test_cc_pl_007_mixed_slash_traversal() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","agents":"./foo/..\\bar"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(
            diagnostics.iter().any(|d| d.rule == "CC-PL-007"),
            "Mixed slash traversal should be detected"
        );
    }

    #[test]
    fn test_cc_pl_007_autofixable_path() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","commands":"commands/run"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let pl_007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-PL-007")
            .collect();
        assert_eq!(
            pl_007.len(),
            1,
            "Missing ./ prefix should trigger CC-PL-007"
        );
        assert!(!pl_007[0].fixes.is_empty(), "Should have a safe autofix");
        assert!(pl_007[0].fixes[0].safe, "Autofix should be safe");
        assert!(
            pl_007[0].fixes[0].replacement.starts_with("./"),
            "Autofix should prepend ./"
        );
    }

    #[test]
    fn test_cc_pl_008_dot_slash_prefix_bypass() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","agents":"./.claude-plugin/agents"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(
            diagnostics.iter().any(|d| d.rule == "CC-PL-008"),
            "./.claude-plugin/ should still be detected"
        );
    }

    #[test]
    fn test_cc_pl_010_non_string_homepage() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","homepage":123}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(
            diagnostics.iter().any(|d| d.rule == "CC-PL-010"),
            "Non-string homepage should trigger CC-PL-010"
        );
    }
}
