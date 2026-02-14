//! Kiro steering file validation rules (KIRO-001 to KIRO-004)
//!
//! Validates:
//! - KIRO-001: Invalid steering file inclusion mode (HIGH/ERROR)
//! - KIRO-002: Missing required fields for inclusion mode (HIGH/ERROR)
//! - KIRO-003: Invalid fileMatchPattern glob (MEDIUM/WARNING)
//! - KIRO-004: Empty Kiro steering file (MEDIUM/WARNING)

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    parsers::frontmatter::split_frontmatter,
    rules::{Validator, ValidatorMetadata},
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &["KIRO-001", "KIRO-002", "KIRO-003", "KIRO-004"];
const VALID_INCLUSION_MODES: &[&str] = &["always", "fileMatch", "manual", "auto"];

pub struct KiroSteeringValidator;

impl Validator for KiroSteeringValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // KIRO-004: Empty steering file (check first, return early)
        if config.is_rule_enabled("KIRO-004") && content.trim().is_empty() {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "KIRO-004",
                    t!("rules.kiro_004.message"),
                )
                .with_suggestion(t!("rules.kiro_004.suggestion")),
            );
            return diagnostics;
        }

        // Parse frontmatter
        let parts = split_frontmatter(content);
        if !parts.has_frontmatter || !parts.has_closing {
            return diagnostics; // No frontmatter - skip frontmatter-based rules
        }

        // Parse YAML
        let yaml: serde_yaml::Value = match serde_yaml::from_str(&parts.frontmatter) {
            Ok(v) => v,
            Err(_) => return diagnostics, // Malformed YAML - skip gracefully
        };

        let mapping = match yaml.as_mapping() {
            Some(m) => m,
            None => return diagnostics,
        };

        // Extract commonly accessed keys once to avoid repeated allocations
        let key_inclusion = serde_yaml::Value::String("inclusion".into());
        let key_name = serde_yaml::Value::String("name".into());
        let key_description = serde_yaml::Value::String("description".into());
        let key_file_match_pattern = serde_yaml::Value::String("fileMatchPattern".into());

        // Look up inclusion once - used by both KIRO-001 and KIRO-002
        let inclusion_str = mapping.get(&key_inclusion).and_then(|v| v.as_str());

        // KIRO-001: Invalid inclusion mode
        if config.is_rule_enabled("KIRO-001") {
            if let Some(inclusion) = inclusion_str {
                if !VALID_INCLUSION_MODES.contains(&inclusion) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "KIRO-001",
                            t!("rules.kiro_001.message", value = inclusion),
                        )
                        .with_suggestion(t!("rules.kiro_001.suggestion")),
                    );
                }
            }
        }

        // KIRO-002: Missing required fields for inclusion mode
        if config.is_rule_enabled("KIRO-002") {
            if let Some(mode) = inclusion_str {
                match mode {
                    "auto" => {
                        if !mapping.contains_key(&key_name) {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "KIRO-002",
                                    t!("rules.kiro_002_auto.message", field = "name"),
                                )
                                .with_suggestion(t!("rules.kiro_002_auto.suggestion")),
                            );
                        }
                        if !mapping.contains_key(&key_description) {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "KIRO-002",
                                    t!("rules.kiro_002_auto.message", field = "description"),
                                )
                                .with_suggestion(t!("rules.kiro_002_auto.suggestion")),
                            );
                        }
                    }
                    "fileMatch" => {
                        if !mapping.contains_key(&key_file_match_pattern) {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "KIRO-002",
                                    t!("rules.kiro_002_filematch.message"),
                                )
                                .with_suggestion(t!("rules.kiro_002_filematch.suggestion")),
                            );
                        }
                    }
                    _ => {} // always and manual have no extra required fields
                }
            }
        }

        // KIRO-003: Invalid fileMatchPattern glob
        if config.is_rule_enabled("KIRO-003") {
            if let Some(pattern_val) = mapping.get(&key_file_match_pattern) {
                if let Some(pattern) = pattern_val.as_str() {
                    if let Err(e) = glob::Pattern::new(pattern) {
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                1,
                                0,
                                "KIRO-003",
                                t!("rules.kiro_003.message", error = e.to_string()),
                            )
                            .with_suggestion(t!("rules.kiro_003.suggestion")),
                        );
                    }
                }
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

    fn validate(path: &str, content: &str) -> Vec<Diagnostic> {
        let validator = KiroSteeringValidator;
        validator.validate(
            Path::new(path),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_steering(content: &str) -> Vec<Diagnostic> {
        validate(".kiro/steering/test.md", content)
    }

    // ===== KIRO-001: Invalid inclusion mode =====

    #[test]
    fn test_kiro_001_invalid_mode() {
        let content = "---\ninclusion: invalid_mode\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-001").collect();
        assert_eq!(kiro_001.len(), 1);
        assert_eq!(kiro_001[0].level, DiagnosticLevel::Error);
        assert!(kiro_001[0].message.contains("invalid_mode"));
    }

    #[test]
    fn test_kiro_001_valid_always() {
        let content = "---\ninclusion: always\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-001").collect();
        assert!(kiro_001.is_empty());
    }

    #[test]
    fn test_kiro_001_valid_auto() {
        let content = "---\ninclusion: auto\nname: test\ndescription: test desc\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-001").collect();
        assert!(kiro_001.is_empty());
    }

    #[test]
    fn test_kiro_001_valid_filematch() {
        let content = "---\ninclusion: fileMatch\nfileMatchPattern: \"**/*.ts\"\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-001").collect();
        assert!(kiro_001.is_empty());
    }

    #[test]
    fn test_kiro_001_valid_manual() {
        let content = "---\ninclusion: manual\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-001").collect();
        assert!(kiro_001.is_empty());
    }

    #[test]
    fn test_kiro_001_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["KIRO-001".to_string()];
        let validator = KiroSteeringValidator;
        let diagnostics = validator.validate(
            Path::new(".kiro/steering/test.md"),
            "---\ninclusion: invalid_mode\n---\n# Steering\n",
            &config,
        );
        let kiro_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-001").collect();
        assert!(kiro_001.is_empty());
    }

    // ===== KIRO-002: Missing required fields =====

    #[test]
    fn test_kiro_002_auto_missing_name() {
        let content = "---\ninclusion: auto\ndescription: test desc\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-002").collect();
        assert_eq!(kiro_002.len(), 1);
        assert_eq!(kiro_002[0].level, DiagnosticLevel::Error);
        assert!(kiro_002[0].message.contains("name"));
    }

    #[test]
    fn test_kiro_002_auto_missing_description() {
        let content = "---\ninclusion: auto\nname: my-steering\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-002").collect();
        assert_eq!(kiro_002.len(), 1);
        assert!(kiro_002[0].message.contains("description"));
    }

    #[test]
    fn test_kiro_002_auto_missing_both() {
        let content = "---\ninclusion: auto\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-002").collect();
        assert_eq!(kiro_002.len(), 2);
    }

    #[test]
    fn test_kiro_002_auto_valid() {
        let content =
            "---\ninclusion: auto\nname: my-steering\ndescription: test desc\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-002").collect();
        assert!(kiro_002.is_empty());
    }

    #[test]
    fn test_kiro_002_filematch_missing_pattern() {
        let content = "---\ninclusion: fileMatch\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-002").collect();
        assert_eq!(kiro_002.len(), 1);
        assert!(kiro_002[0].message.contains("fileMatchPattern"));
    }

    #[test]
    fn test_kiro_002_filematch_valid() {
        let content = "---\ninclusion: fileMatch\nfileMatchPattern: \"**/*.ts\"\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-002").collect();
        assert!(kiro_002.is_empty());
    }

    #[test]
    fn test_kiro_002_always_no_extra_fields_needed() {
        let content = "---\ninclusion: always\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-002").collect();
        assert!(kiro_002.is_empty());
    }

    #[test]
    fn test_kiro_002_manual_no_extra_fields_needed() {
        let content = "---\ninclusion: manual\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-002").collect();
        assert!(kiro_002.is_empty());
    }

    #[test]
    fn test_kiro_002_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["KIRO-002".to_string()];
        let validator = KiroSteeringValidator;
        let diagnostics = validator.validate(
            Path::new(".kiro/steering/test.md"),
            "---\ninclusion: auto\n---\n# Steering\n",
            &config,
        );
        let kiro_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-002").collect();
        assert!(kiro_002.is_empty());
    }

    // ===== KIRO-003: Invalid fileMatchPattern glob =====

    #[test]
    fn test_kiro_003_bad_glob() {
        let content = "---\nfileMatchPattern: \"[unclosed\"\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-003").collect();
        assert_eq!(kiro_003.len(), 1);
        assert_eq!(kiro_003[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_kiro_003_valid_glob() {
        let content = "---\nfileMatchPattern: \"**/*.ts\"\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-003").collect();
        assert!(kiro_003.is_empty());
    }

    #[test]
    fn test_kiro_003_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["KIRO-003".to_string()];
        let validator = KiroSteeringValidator;
        let diagnostics = validator.validate(
            Path::new(".kiro/steering/test.md"),
            "---\nfileMatchPattern: \"[unclosed\"\n---\n# Steering\n",
            &config,
        );
        let kiro_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-003").collect();
        assert!(kiro_003.is_empty());
    }

    // ===== KIRO-004: Empty steering file =====

    #[test]
    fn test_kiro_004_empty_file() {
        let diagnostics = validate_steering("");
        let kiro_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-004").collect();
        assert_eq!(kiro_004.len(), 1);
        assert_eq!(kiro_004[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_kiro_004_whitespace_only() {
        let diagnostics = validate_steering("   \n\n  ");
        let kiro_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-004").collect();
        assert_eq!(kiro_004.len(), 1);
    }

    #[test]
    fn test_kiro_004_valid_file() {
        let diagnostics = validate_steering("---\ninclusion: always\n---\n# Guidelines\n");
        let kiro_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-004").collect();
        assert!(kiro_004.is_empty());
    }

    #[test]
    fn test_kiro_004_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["KIRO-004".to_string()];
        let validator = KiroSteeringValidator;
        let diagnostics =
            validator.validate(Path::new(".kiro/steering/test.md"), "", &config);
        let kiro_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-004").collect();
        assert!(kiro_004.is_empty());
    }

    // ===== Category disable =====

    #[test]
    fn test_kiro_steering_category_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().kiro_steering = false;
        let validator = KiroSteeringValidator;

        let diagnostics = validator.validate(
            Path::new(".kiro/steering/test.md"),
            "",
            &config,
        );
        let kiro_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("KIRO-"))
            .collect();
        assert!(kiro_rules.is_empty());

        let diagnostics = validator.validate(
            Path::new(".kiro/steering/test.md"),
            "---\ninclusion: invalid\n---\n# Test\n",
            &config,
        );
        let kiro_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("KIRO-"))
            .collect();
        assert!(kiro_rules.is_empty());
    }

    // ===== Edge cases =====

    #[test]
    fn test_no_frontmatter_no_diagnostics() {
        let diagnostics = validate_steering("# Just a heading\nSome content.");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_malformed_yaml_no_crash() {
        let content = "---\ninclusion: auto\n  bad: indentation\n---\n# Content\n";
        let diagnostics = validate_steering(content);
        // Malformed YAML is handled gracefully - no panic, no diagnostics
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_non_mapping_yaml_no_crash() {
        let content = "---\n- item1\n- item2\n---\n# Content\n";
        let diagnostics = validate_steering(content);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_kiro_001_non_string_inclusion_ignored() {
        // Non-string inclusion values (number, bool) are silently ignored
        let content = "---\ninclusion: 123\n---\n# Content\n";
        let diagnostics = validate_steering(content);
        let kiro_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-001").collect();
        assert!(kiro_001.is_empty());
    }

    #[test]
    fn test_kiro_001_case_sensitive() {
        // Inclusion modes are case-sensitive - "ALWAYS" is not valid
        let content = "---\ninclusion: ALWAYS\n---\n# Content\n";
        let diagnostics = validate_steering(content);
        let kiro_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-001").collect();
        assert_eq!(kiro_001.len(), 1);
    }

    #[test]
    fn test_kiro_003_non_string_pattern_ignored() {
        let content = "---\nfileMatchPattern: 123\n---\n# Content\n";
        let diagnostics = validate_steering(content);
        let kiro_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-003").collect();
        assert!(kiro_003.is_empty());
    }

    #[test]
    fn test_kiro_003_empty_string_pattern() {
        // Empty string is a valid glob pattern
        let content = "---\nfileMatchPattern: \"\"\n---\n# Content\n";
        let diagnostics = validate_steering(content);
        let kiro_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-003").collect();
        assert!(kiro_003.is_empty());
    }

    #[test]
    fn test_frontmatter_only_no_body_not_empty() {
        // File with frontmatter but no body is not "empty"
        let content = "---\ninclusion: always\n---\n";
        let diagnostics = validate_steering(content);
        let kiro_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "KIRO-004").collect();
        assert!(kiro_004.is_empty());
    }

    // ===== Metadata =====

    #[test]
    fn test_metadata() {
        let v = KiroSteeringValidator;
        let meta = v.metadata();
        assert_eq!(meta.name, "KiroSteeringValidator");
        assert_eq!(
            meta.rule_ids,
            &["KIRO-001", "KIRO-002", "KIRO-003", "KIRO-004"]
        );
    }
}
