//! Amp validation rules (AMP-001 to AMP-004)
//!
//! Validates:
//! - AMP-001: Invalid check file frontmatter (ERROR)
//! - AMP-002: Invalid severity-default value (WARNING)
//! - AMP-003: Invalid AGENTS.md globs frontmatter (WARNING)
//! - AMP-004: Amp settings parse error / unknown keys (ERROR)

use crate::{
    FileType,
    config::LintConfig,
    diagnostics::Diagnostic,
    parsers::frontmatter::split_frontmatter,
    rules::{Validator, ValidatorMetadata},
};
use serde_json::Value as JsonValue;
use serde_yaml::{Mapping, Value as YamlValue};
use std::path::Path;

const RULE_IDS: &[&str] = &["AMP-001", "AMP-002", "AMP-003", "AMP-004"];

const VALID_SEVERITY_DEFAULT: &[&str] = &["low", "medium", "high", "critical"];
const VALID_CHECK_KEYS: &[&str] = &["name", "description", "severity-default", "tools"];
const VALID_AMP_SETTINGS_KEYS: &[&str] = &[
    "model",
    "models",
    "instructions",
    "checks",
    "watch",
    "sandbox",
    "approval",
    "server",
    "parallelism",
    "theme",
    "vim",
    "wsl",
    "env",
    "shell",
    "plugins",
    "lsp",
    "disable_summaries",
    "summarize",
    "trusted",
    "history",
    "notify",
];

pub struct AmpValidator;

impl Validator for AmpValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        if is_amp_check_path(path) {
            return validate_amp_check(path, content, config);
        }

        match crate::detect_file_type(path) {
            FileType::AmpSettings => validate_amp_settings(path, content, config),
            FileType::ClaudeMd => validate_amp_agents_globs(path, content, config),
            _ => Vec::new(),
        }
    }
}

fn validate_amp_check(path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let amp_001_enabled = config.is_rule_enabled("AMP-001");
    let amp_002_enabled = config.is_rule_enabled("AMP-002");
    if !amp_001_enabled && !amp_002_enabled {
        return diagnostics;
    }

    let parts = split_frontmatter(content);
    if !parts.has_frontmatter || !parts.has_closing {
        if amp_001_enabled {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "AMP-001",
                    "Amp check files must include YAML frontmatter",
                )
                .with_suggestion("Add frontmatter with at least `name` and a markdown body."),
            );
        }
        return diagnostics;
    }

    let parsed: YamlValue = match serde_yaml::from_str(&parts.frontmatter) {
        Ok(value) => value,
        Err(error) => {
            if amp_001_enabled {
                let line = error.location().map_or(1, |loc| loc.line());
                let column = error.location().map_or(0, |loc| loc.column());
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        column,
                        "AMP-001",
                        format!("Invalid YAML frontmatter in Amp check file: {error}"),
                    )
                    .with_suggestion("Fix YAML syntax in frontmatter."),
                );
            }
            return diagnostics;
        }
    };

    let Some(mapping) = parsed.as_mapping() else {
        if amp_001_enabled {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "AMP-001",
                    "Amp check frontmatter must be a YAML mapping",
                )
                .with_suggestion(
                    "Use key-value frontmatter fields like `name`, `description`, and `tools`.",
                ),
            );
        }
        return diagnostics;
    };

    if amp_001_enabled {
        for (key_node, value) in mapping {
            let Some(key) = key_node.as_str() else {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "AMP-001",
                        "Amp check frontmatter keys must be strings",
                    )
                    .with_suggestion("Use string keys like `name`, `description`, and `tools`."),
                );
                continue;
            };
            if !VALID_CHECK_KEYS.contains(&key) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        frontmatter_key_line(&parts.frontmatter, key),
                        0,
                        "AMP-001",
                        format!("Unknown Amp check frontmatter key '{key}'"),
                    )
                    .with_suggestion(
                        "Allowed keys are: name, description, severity-default, tools.",
                    ),
                );
            }

            if key == "description" && !value.is_string() {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        frontmatter_key_line(&parts.frontmatter, key),
                        0,
                        "AMP-001",
                        "Amp check `description` must be a string",
                    )
                    .with_suggestion("Set `description` to a string value."),
                );
            }

            if key == "tools" && !is_valid_tools_field(value) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        frontmatter_key_line(&parts.frontmatter, key),
                        0,
                        "AMP-001",
                        "Amp check `tools` must be a string or an array of strings",
                    )
                    .with_suggestion("Set `tools` to a string or list of strings."),
                );
            }
        }
    }

    if amp_001_enabled {
        match mapping_value(mapping, "name") {
            Some(name) if name.as_str().is_some_and(|n| !n.trim().is_empty()) => {}
            _ => diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    frontmatter_key_line(&parts.frontmatter, "name"),
                    0,
                    "AMP-001",
                    "Amp check frontmatter is missing required `name` field",
                )
                .with_suggestion("Add `name: your-check-name` to frontmatter."),
            ),
        }
    }

    if amp_002_enabled && let Some(value) = mapping_value(mapping, "severity-default") {
        match value.as_str() {
            Some(severity) if VALID_SEVERITY_DEFAULT.contains(&severity) => {}
            Some(severity) => diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    frontmatter_key_line(&parts.frontmatter, "severity-default"),
                    0,
                    "AMP-002",
                    format!(
                        "Invalid severity-default value '{severity}' (expected low, medium, high, or critical)"
                    ),
                )
                .with_suggestion(
                    "Set `severity-default` to one of: low, medium, high, critical.",
                ),
            ),
            None => diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    frontmatter_key_line(&parts.frontmatter, "severity-default"),
                    0,
                    "AMP-002",
                    "severity-default must be a string",
                )
                .with_suggestion("Set `severity-default` to a string value."),
            ),
        }
    }

    diagnostics
}

fn validate_amp_agents_globs(path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
    if !config.is_rule_enabled("AMP-003") {
        return Vec::new();
    }

    let Some(filename) = path.file_name().and_then(|name| name.to_str()) else {
        return Vec::new();
    };
    if !matches!(
        filename,
        "AGENTS.md" | "AGENTS.local.md" | "AGENTS.override.md"
    ) {
        return Vec::new();
    }

    let parts = split_frontmatter(content);
    if !parts.has_frontmatter || !parts.has_closing {
        return Vec::new();
    }

    let parsed: YamlValue = match serde_yaml::from_str(&parts.frontmatter) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    let Some(mapping) = parsed.as_mapping() else {
        return Vec::new();
    };
    let Some(globs_value) = mapping_value(mapping, "globs") else {
        return Vec::new();
    };

    let globs_line = frontmatter_key_line(&parts.frontmatter, "globs");
    let patterns = match globs_value {
        YamlValue::String(pattern) => vec![pattern.clone()],
        YamlValue::Sequence(values) => {
            let mut patterns = Vec::with_capacity(values.len());
            for value in values {
                let Some(pattern) = value.as_str() else {
                    return vec![
                        Diagnostic::warning(
                            path.to_path_buf(),
                            globs_line,
                            0,
                            "AMP-003",
                            "AGENTS.md frontmatter `globs` must contain only string patterns",
                        )
                        .with_suggestion("Set `globs` to a string or list of string patterns."),
                    ];
                };
                patterns.push(pattern.to_string());
            }
            patterns
        }
        _ => {
            return vec![
                Diagnostic::warning(
                    path.to_path_buf(),
                    globs_line,
                    0,
                    "AMP-003",
                    "AGENTS.md frontmatter `globs` must be a string or array of strings",
                )
                .with_suggestion("Set `globs` to a string or list of string patterns."),
            ];
        }
    };

    let mut diagnostics = Vec::new();
    for pattern in patterns {
        let normalized = normalize_amp_glob(&pattern);
        if let Err(error) = glob::Pattern::new(&normalized) {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    globs_line,
                    0,
                    "AMP-003",
                    format!("Invalid AGENTS.md glob pattern '{pattern}': {error}"),
                )
                .with_suggestion("Fix the glob syntax in `globs` frontmatter."),
            );
        }
    }

    diagnostics
}

fn validate_amp_settings(path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
    if !config.is_rule_enabled("AMP-004") {
        return Vec::new();
    }

    let mut diagnostics = Vec::new();
    let parsed: JsonValue = match serde_json::from_str(content) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    error.line(),
                    error.column(),
                    "AMP-004",
                    format!("Failed to parse Amp settings JSON: {error}"),
                )
                .with_suggestion("Fix JSON syntax in .amp/settings.json."),
            );
            return diagnostics;
        }
    };

    let Some(settings_obj) = parsed.as_object() else {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "AMP-004",
                "Amp settings must be a top-level JSON object",
            )
            .with_suggestion("Wrap settings keys in a JSON object."),
        );
        return diagnostics;
    };

    for key in settings_obj.keys() {
        if !VALID_AMP_SETTINGS_KEYS.contains(&key.as_str()) {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    find_json_key_line(content, key).unwrap_or(1),
                    0,
                    "AMP-004",
                    format!("Unknown Amp settings key '{key}'"),
                )
                .with_suggestion("Remove or rename unknown settings keys."),
            );
        }
    }

    diagnostics
}

fn mapping_value<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a YamlValue> {
    mapping
        .iter()
        .find(|(candidate, _)| candidate.as_str() == Some(key))
        .map(|(_, value)| value)
}

fn is_amp_check_path(path: &Path) -> bool {
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if !filename.ends_with(".md") {
        return false;
    }

    path.components()
        .zip(path.components().skip(1))
        .any(|(a, b)| {
            matches!(
                (a, b),
                (std::path::Component::Normal(a_os), std::path::Component::Normal(b_os))
                if a_os
                    .to_str()
                    .is_some_and(|s| s.eq_ignore_ascii_case(".agents"))
                    && b_os
                        .to_str()
                        .is_some_and(|s| s.eq_ignore_ascii_case("checks"))
            )
        })
}

fn is_valid_tools_field(value: &YamlValue) -> bool {
    value.is_string()
        || value
            .as_sequence()
            .is_some_and(|values| values.iter().all(YamlValue::is_string))
}

fn normalize_amp_glob(pattern: &str) -> String {
    if pattern.starts_with("./") || pattern.starts_with("../") || pattern.starts_with("**/") {
        pattern.to_string()
    } else {
        format!("**/{pattern}")
    }
}

fn frontmatter_key_line(frontmatter: &str, key: &str) -> usize {
    frontmatter
        .lines()
        .enumerate()
        .find_map(|(idx, line)| {
            let trimmed = line.trim_start();
            let after = trimmed.strip_prefix(key)?;
            if after.trim_start().starts_with(':') {
                Some(idx + 1)
            } else {
                None
            }
        })
        .unwrap_or(1)
}

fn find_json_key_line(content: &str, key: &str) -> Option<usize> {
    let needle = format!("\"{key}\"");
    for (index, line) in content.lines().enumerate() {
        let Some(position) = line.find(&needle) else {
            continue;
        };
        let after = &line[position + needle.len()..];
        if after.trim_start().starts_with(':') {
            return Some(index + 1);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::LintConfig, diagnostics::DiagnosticLevel};

    fn validate(path: &str, content: &str) -> Vec<Diagnostic> {
        let validator = AmpValidator;
        validator.validate(Path::new(path), content, &LintConfig::default())
    }

    fn validate_with_config(path: &str, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = AmpValidator;
        validator.validate(Path::new(path), content, config)
    }

    #[test]
    fn test_amp_001_missing_frontmatter() {
        let diagnostics = validate(".agents/checks/security.md", "# Security check");
        let amp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AMP-001").collect();
        assert_eq!(amp_001.len(), 1);
        assert_eq!(amp_001[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_amp_001_invalid_yaml_frontmatter() {
        let content = "---\nname: [unclosed\n---\n# Body";
        let diagnostics = validate(".agents/checks/security.md", content);
        let amp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AMP-001").collect();
        assert_eq!(amp_001.len(), 1);
        assert!(amp_001[0].message.contains("Invalid YAML frontmatter"));
    }

    #[test]
    fn test_amp_001_frontmatter_must_be_mapping() {
        let content = "---\n- security\n- check\n---\n# Body";
        let diagnostics = validate(".agents/checks/security.md", content);
        let amp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AMP-001").collect();
        assert_eq!(amp_001.len(), 1);
        assert!(amp_001[0].message.contains("must be a YAML mapping"));
    }

    #[test]
    fn test_amp_001_missing_name() {
        let content = "---\ndescription: Security checks\n---\n# Body";
        let diagnostics = validate(".agents/checks/security.md", content);
        let amp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AMP-001").collect();
        assert_eq!(amp_001.len(), 1);
        assert!(amp_001[0].message.contains("missing required `name`"));
    }

    #[test]
    fn test_amp_001_unknown_frontmatter_key() {
        let content = "---\nname: security\ndescription: Security checks\nfoo: bar\n---\n# Body";
        let diagnostics = validate(".agents/checks/security.md", content);
        let amp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AMP-001").collect();
        assert_eq!(amp_001.len(), 1);
        assert!(
            amp_001[0]
                .message
                .contains("Unknown Amp check frontmatter key")
        );
    }

    #[test]
    fn test_amp_001_non_string_frontmatter_key() {
        let content = "---\n1: true\nname: security\n---\n# Body";
        let diagnostics = validate(".agents/checks/security.md", content);
        let amp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AMP-001").collect();
        assert_eq!(amp_001.len(), 1);
        assert!(amp_001[0].message.contains("keys must be strings"));
    }

    #[test]
    fn test_amp_001_description_must_be_string() {
        let content = "---\nname: security\ndescription: [not, string]\n---\n# Body";
        let diagnostics = validate(".agents/checks/security.md", content);
        let amp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AMP-001").collect();
        assert_eq!(amp_001.len(), 1);
        assert!(
            amp_001[0]
                .message
                .contains("`description` must be a string")
        );
    }

    #[test]
    fn test_amp_001_tools_must_be_string_or_string_array() {
        let content = "---\nname: security\ntools:\n  - rg\n  - 7\n---\n# Body";
        let diagnostics = validate(".agents/checks/security.md", content);
        let amp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AMP-001").collect();
        assert_eq!(amp_001.len(), 1);
        assert!(amp_001[0].message.contains("`tools` must be a string"));
    }

    #[test]
    fn test_amp_002_invalid_severity_default() {
        let content = "---\nname: security\nseverity-default: urgent\n---\n# Body";
        let diagnostics = validate(".agents/checks/security.md", content);
        let amp_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AMP-002").collect();
        assert_eq!(amp_002.len(), 1);
        assert_eq!(amp_002[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_amp_002_severity_default_must_be_string() {
        let content = "---\nname: security\nseverity-default:\n  level: high\n---\n# Body";
        let diagnostics = validate(".agents/checks/security.md", content);
        let amp_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AMP-002").collect();
        assert_eq!(amp_002.len(), 1);
        assert!(amp_002[0].message.contains("must be a string"));
    }

    #[test]
    fn test_amp_check_valid_file() {
        let content = "---\nname: security\ndescription: Security checks\nseverity-default: high\ntools:\n  - rg\n---\n# Body";
        let diagnostics = validate(".agents/checks/security.md", content);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_amp_003_invalid_agents_glob() {
        let content = "---\nglobs: \"[unclosed\"\n---\n# Instructions";
        let diagnostics = validate("AGENTS.md", content);
        let amp_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AMP-003").collect();
        assert_eq!(amp_003.len(), 1);
        assert_eq!(amp_003[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_amp_003_non_agents_file_noop() {
        let content = "---\nglobs: \"[unclosed\"\n---\n# Instructions";
        let diagnostics = validate("CLAUDE.md", content);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_amp_001_applies_to_agents_file_under_checks() {
        let diagnostics = validate(".agents/checks/AGENTS.md", "# Missing frontmatter");
        let amp_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AMP-001").collect();
        assert_eq!(amp_001.len(), 1);
    }

    #[test]
    fn test_amp_003_globs_array_must_be_strings() {
        let content = "---\nglobs:\n  - \"src/**/*.rs\"\n  - 42\n---\n# Instructions";
        let diagnostics = validate("AGENTS.md", content);
        let amp_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AMP-003").collect();
        assert_eq!(amp_003.len(), 1);
        assert!(
            amp_003[0]
                .message
                .contains("must contain only string patterns")
        );
    }

    #[test]
    fn test_amp_003_globs_must_be_string_or_array() {
        let content = "---\nglobs: true\n---\n# Instructions";
        let diagnostics = validate("AGENTS.md", content);
        let amp_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AMP-003").collect();
        assert_eq!(amp_003.len(), 1);
        assert!(
            amp_003[0]
                .message
                .contains("must be a string or array of strings")
        );
    }

    #[test]
    fn test_amp_003_agents_local_file() {
        let content = "---\nglobs: \"[unclosed\"\n---\n# Instructions";
        let diagnostics = validate("AGENTS.local.md", content);
        let amp_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AMP-003").collect();
        assert_eq!(amp_003.len(), 1);
    }

    #[test]
    fn test_amp_003_agents_override_file() {
        let content = "---\nglobs: \"[unclosed\"\n---\n# Instructions";
        let diagnostics = validate("AGENTS.override.md", content);
        let amp_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AMP-003").collect();
        assert_eq!(amp_003.len(), 1);
    }

    #[test]
    fn test_amp_004_settings_parse_error() {
        let diagnostics = validate(".amp/settings.json", "{ invalid json");
        let amp_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AMP-004").collect();
        assert_eq!(amp_004.len(), 1);
        assert_eq!(amp_004[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_amp_004_settings_top_level_must_be_object() {
        let diagnostics = validate(".amp/settings.json", r#"["not", "object"]"#);
        let amp_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AMP-004").collect();
        assert_eq!(amp_004.len(), 1);
        assert!(amp_004[0].message.contains("top-level JSON object"));
    }

    #[test]
    fn test_amp_004_unknown_settings_key() {
        let diagnostics = validate(".amp/settings.json", r#"{"model":"x","badKey":true}"#);
        let amp_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AMP-004").collect();
        assert_eq!(amp_004.len(), 1);
        assert!(amp_004[0].message.contains("badKey"));
    }

    #[test]
    fn test_amp_004_valid_settings() {
        let diagnostics = validate(".amp/settings.json", r#"{"model":"x","notify":true}"#);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_amp_rules_respect_disabled_config() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["AMP-004".to_string()];
        let diagnostics = validate_with_config(".amp/settings.json", "{ invalid json", &config);
        assert!(diagnostics.is_empty());
    }
}
