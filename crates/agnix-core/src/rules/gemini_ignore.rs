//! Gemini CLI ignore file validation rules (GM-006)
//!
//! Validates:
//! - GM-006: Invalid .geminiignore file (LOW) - empty content, syntax errors

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::{Validator, ValidatorMetadata},
    schemas::gemini_ignore::validate_geminiignore,
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &["GM-006"];

pub struct GeminiIgnoreValidator;

impl Validator for GeminiIgnoreValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if !config.is_rule_enabled("GM-006") {
            return diagnostics;
        }

        let path_buf = path.to_path_buf();
        let issues = validate_geminiignore(content);

        for issue in issues {
            let (message, suggestion) = if issue.description == "empty" {
                (
                    t!(
                        "rules.gm_006.message",
                        description = t!("rules.gm_006.empty")
                    ),
                    t!("rules.gm_006.suggestion"),
                )
            } else if let Some(pattern) = issue.description.strip_prefix("syntax_error:") {
                (
                    t!(
                        "rules.gm_006.message",
                        description = t!(
                            "rules.gm_006.syntax_error",
                            line = &issue.line.to_string(),
                            pattern = pattern
                        )
                    ),
                    t!("rules.gm_006.suggestion"),
                )
            } else {
                (
                    t!(
                        "rules.gm_006.message",
                        description = issue.description.as_str()
                    ),
                    t!("rules.gm_006.suggestion"),
                )
            };

            diagnostics.push(
                Diagnostic::info(
                    path_buf.clone(),
                    issue.line,
                    issue.column,
                    "GM-006",
                    message,
                )
                .with_suggestion(suggestion),
            );
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;

    fn validate(content: &str) -> Vec<Diagnostic> {
        let validator = GeminiIgnoreValidator;
        validator.validate(Path::new(".geminiignore"), content, &LintConfig::default())
    }

    fn validate_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = GeminiIgnoreValidator;
        validator.validate(Path::new(".geminiignore"), content, config)
    }

    // ===== GM-006: Invalid .geminiignore =====

    #[test]
    fn test_gm_006_empty() {
        let diagnostics = validate("");
        let gm_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-006").collect();
        assert_eq!(gm_006.len(), 1);
        assert_eq!(gm_006[0].level, DiagnosticLevel::Info);
    }

    #[test]
    fn test_gm_006_only_comments() {
        let diagnostics = validate("# Comment\n# Another\n");
        let gm_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-006").collect();
        assert_eq!(gm_006.len(), 1);
    }

    #[test]
    fn test_gm_006_syntax_error() {
        let diagnostics = validate("[bad\n*.log\n");
        let gm_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-006").collect();
        assert_eq!(gm_006.len(), 1);
        assert_eq!(gm_006[0].line, 1);
    }

    #[test]
    fn test_gm_006_valid_content() {
        let diagnostics = validate("node_modules/\n*.log\n.env\n");
        let gm_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-006").collect();
        assert!(gm_006.is_empty());
    }

    // ===== Config Integration =====

    #[test]
    fn test_config_disabled_gemini_md_category() {
        let mut config = LintConfig::default();
        config.rules_mut().gemini_md = false;

        let diagnostics = validate_with_config("", &config);
        let gm_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("GM-"))
            .collect();
        assert!(gm_rules.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["GM-006".to_string()];

        let diagnostics = validate_with_config("", &config);
        let gm_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-006").collect();
        assert!(gm_006.is_empty());
    }
}
