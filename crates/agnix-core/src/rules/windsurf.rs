//! Windsurf rules directory and workflow validation (WS-001 to WS-004)
//!
//! Validates:
//! - WS-001: Empty Windsurf rule file (MEDIUM/WARNING)
//! - WS-002: Windsurf rule file exceeds character limit (HIGH/ERROR)
//! - WS-003: Empty or oversized Windsurf workflow file (MEDIUM/WARNING)
//! - WS-004: Legacy .windsurfrules detected (LOW/INFO)

use crate::{
    FileType,
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::{Validator, ValidatorMetadata},
    schemas::agents_md::WINDSURF_CHAR_LIMIT,
};
use rust_i18n::t;
use std::path::Path;

/// Character limit for Windsurf workflow files.
const WORKFLOW_CHAR_LIMIT: usize = 12_000;

const RULE_IDS: &[&str] = &["WS-001", "WS-002", "WS-003", "WS-004"];

pub struct WindsurfValidator;

impl Validator for WindsurfValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let file_type = crate::detect_file_type(path);

        match file_type {
            FileType::WindsurfRule => {
                // WS-001: Empty Windsurf rule file (WARNING)
                if config.is_rule_enabled("WS-001") && content.trim().is_empty() {
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            1,
                            0,
                            "WS-001",
                            t!("rules.ws_001.message"),
                        )
                        .with_suggestion(t!("rules.ws_001.suggestion")),
                    );
                }

                // WS-002: Windsurf rule file exceeds character limit (ERROR)
                if config.is_rule_enabled("WS-002") && content.len() > WINDSURF_CHAR_LIMIT {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "WS-002",
                            t!(
                                "rules.ws_002.message",
                                limit = WINDSURF_CHAR_LIMIT,
                                len = content.len()
                            ),
                        )
                        .with_suggestion(t!("rules.ws_002.suggestion")),
                    );
                }
            }
            FileType::WindsurfWorkflow => {
                // WS-003: Empty or oversized Windsurf workflow file (WARNING)
                if config.is_rule_enabled("WS-003") {
                    if content.trim().is_empty() {
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                1,
                                0,
                                "WS-003",
                                t!("rules.ws_003_empty.message"),
                            )
                            .with_suggestion(t!("rules.ws_003_empty.suggestion")),
                        );
                    } else if content.len() > WORKFLOW_CHAR_LIMIT {
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                1,
                                0,
                                "WS-003",
                                t!(
                                    "rules.ws_003_too_long.message",
                                    limit = WORKFLOW_CHAR_LIMIT,
                                    len = content.len()
                                ),
                            )
                            .with_suggestion(t!("rules.ws_003_too_long.suggestion")),
                        );
                    }
                }
            }
            FileType::WindsurfRulesLegacy => {
                // WS-004: Legacy .windsurfrules detected (INFO)
                if config.is_rule_enabled("WS-004") {
                    diagnostics.push(
                        Diagnostic::info(
                            path.to_path_buf(),
                            1,
                            0,
                            "WS-004",
                            t!("rules.ws_004.message"),
                        )
                        .with_suggestion(t!("rules.ws_004.suggestion")),
                    );
                }
            }
            _ => {}
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;

    fn validate_rule(content: &str) -> Vec<Diagnostic> {
        let validator = WindsurfValidator;
        validator.validate(
            Path::new(".windsurf/rules/test.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_workflow(content: &str) -> Vec<Diagnostic> {
        let validator = WindsurfValidator;
        validator.validate(
            Path::new(".windsurf/workflows/test.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_legacy(content: &str) -> Vec<Diagnostic> {
        let validator = WindsurfValidator;
        validator.validate(Path::new(".windsurfrules"), content, &LintConfig::default())
    }

    // ===== WS-001: Empty Windsurf rule file =====

    #[test]
    fn test_ws_001_empty_file() {
        let diagnostics = validate_rule("");
        let ws_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "WS-001").collect();
        assert_eq!(ws_001.len(), 1);
        assert_eq!(ws_001[0].level, DiagnosticLevel::Warning);
        assert!(ws_001[0].message.contains("empty"));
    }

    #[test]
    fn test_ws_001_whitespace_only() {
        let diagnostics = validate_rule("   \n\n  ");
        let ws_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "WS-001").collect();
        assert_eq!(ws_001.len(), 1);
    }

    #[test]
    fn test_ws_001_valid_file() {
        let diagnostics = validate_rule("# TypeScript Guidelines\nUse strict mode.");
        let ws_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "WS-001").collect();
        assert!(ws_001.is_empty());
    }

    #[test]
    fn test_ws_001_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["WS-001".to_string()];
        let validator = WindsurfValidator;
        let diagnostics = validator.validate(Path::new(".windsurf/rules/test.md"), "", &config);
        let ws_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "WS-001").collect();
        assert!(ws_001.is_empty());
    }

    // ===== WS-002: Windsurf rule file exceeds character limit =====

    #[test]
    fn test_ws_002_exceeds_limit() {
        let content = "x".repeat(WINDSURF_CHAR_LIMIT + 1);
        let diagnostics = validate_rule(&content);
        let ws_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "WS-002").collect();
        assert_eq!(ws_002.len(), 1);
        assert_eq!(ws_002[0].level, DiagnosticLevel::Error);
        assert!(ws_002[0].message.contains("exceeds"));
    }

    #[test]
    fn test_ws_002_at_limit() {
        let content = "x".repeat(WINDSURF_CHAR_LIMIT);
        let diagnostics = validate_rule(&content);
        let ws_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "WS-002").collect();
        assert!(ws_002.is_empty(), "Exactly at limit should not trigger");
    }

    #[test]
    fn test_ws_002_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["WS-002".to_string()];
        let content = "x".repeat(WINDSURF_CHAR_LIMIT + 1);
        let validator = WindsurfValidator;
        let diagnostics =
            validator.validate(Path::new(".windsurf/rules/test.md"), &content, &config);
        let ws_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "WS-002").collect();
        assert!(ws_002.is_empty());
    }

    // ===== WS-003: Windsurf workflow file =====

    #[test]
    fn test_ws_003_empty_workflow() {
        let diagnostics = validate_workflow("");
        let ws_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "WS-003").collect();
        assert_eq!(ws_003.len(), 1);
        assert_eq!(ws_003[0].level, DiagnosticLevel::Warning);
        assert!(ws_003[0].message.contains("empty"));
    }

    #[test]
    fn test_ws_003_workflow_too_long() {
        let content = "y".repeat(WORKFLOW_CHAR_LIMIT + 1);
        let diagnostics = validate_workflow(&content);
        let ws_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "WS-003").collect();
        assert_eq!(ws_003.len(), 1);
        assert!(ws_003[0].message.contains("exceeds"));
    }

    #[test]
    fn test_ws_003_whitespace_only() {
        let diagnostics = validate_workflow("   \n\n  ");
        let ws_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "WS-003").collect();
        assert_eq!(ws_003.len(), 1);
    }

    #[test]
    fn test_ws_003_workflow_at_limit() {
        let content = "y".repeat(WORKFLOW_CHAR_LIMIT);
        let diagnostics = validate_workflow(&content);
        let ws_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "WS-003").collect();
        assert!(ws_003.is_empty(), "Exactly at limit should not trigger");
    }

    #[test]
    fn test_ws_003_valid_workflow() {
        let diagnostics = validate_workflow("# Deploy\nRun deploy steps.");
        let ws_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "WS-003").collect();
        assert!(ws_003.is_empty());
    }

    #[test]
    fn test_ws_003_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["WS-003".to_string()];
        let validator = WindsurfValidator;
        let diagnostics = validator.validate(Path::new(".windsurf/workflows/test.md"), "", &config);
        let ws_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "WS-003").collect();
        assert!(ws_003.is_empty());
    }

    // ===== WS-004: Legacy .windsurfrules detected =====

    #[test]
    fn test_ws_004_legacy_detected() {
        let diagnostics = validate_legacy("Some rules content");
        let ws_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "WS-004").collect();
        assert_eq!(ws_004.len(), 1);
        assert_eq!(ws_004[0].level, DiagnosticLevel::Info);
        assert!(ws_004[0].message.contains("Legacy"));
    }

    #[test]
    fn test_ws_004_legacy_empty() {
        let diagnostics = validate_legacy("");
        let ws_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "WS-004").collect();
        assert_eq!(ws_004.len(), 1, "WS-004 should trigger even on empty legacy file");
        assert_eq!(ws_004[0].level, DiagnosticLevel::Info);
    }

    #[test]
    fn test_ws_004_legacy_only_one_diagnostic() {
        let diagnostics = validate_legacy("Some rules content");
        assert_eq!(
            diagnostics.len(),
            1,
            "Legacy file should produce exactly 1 diagnostic (WS-004)"
        );
        assert_eq!(diagnostics[0].rule, "WS-004");
    }

    #[test]
    fn test_ws_004_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["WS-004".to_string()];
        let validator = WindsurfValidator;
        let diagnostics = validator.validate(Path::new(".windsurfrules"), "content", &config);
        let ws_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "WS-004").collect();
        assert!(ws_004.is_empty());
    }

    // ===== Category disable =====

    #[test]
    fn test_windsurf_category_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().windsurf = false;
        let validator = WindsurfValidator;

        let diagnostics = validator.validate(Path::new(".windsurf/rules/test.md"), "", &config);
        let ws_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("WS-"))
            .collect();
        assert!(ws_rules.is_empty());

        let diagnostics = validator.validate(Path::new(".windsurfrules"), "content", &config);
        let ws_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("WS-"))
            .collect();
        assert!(ws_rules.is_empty());
    }

    // ===== Valid files produce no diagnostics =====

    #[test]
    fn test_valid_rule_no_diagnostics() {
        let diagnostics = validate_rule("# TypeScript Guidelines\nUse strict mode.");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_valid_workflow_no_diagnostics() {
        let diagnostics = validate_workflow("# Deploy\nRun deploy steps.");
        assert!(diagnostics.is_empty());
    }

    // ===== Metadata =====

    #[test]
    fn test_metadata() {
        let v = WindsurfValidator;
        let meta = v.metadata();
        assert_eq!(meta.name, "WindsurfValidator");
        assert_eq!(meta.rule_ids, &["WS-001", "WS-002", "WS-003", "WS-004"]);
    }
}
