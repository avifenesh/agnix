//! XML tag balance validation

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    parsers::markdown::{check_xml_balance, extract_xml_tags, XmlBalanceError},
    rules::Validator,
};
use std::path::Path;

pub struct XmlValidator;

impl Validator for XmlValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Early return if XML category is disabled or legacy flag is disabled
        if !config.rules.xml || !config.rules.xml_balance {
            return diagnostics;
        }

        let tags = extract_xml_tags(content);
        let errors = check_xml_balance(&tags);

        for error in errors {
            let (rule_id, message, line, column, suggestion) = match error {
                XmlBalanceError::Unclosed { tag, line, column } => (
                    "XML-001",
                    format!("Unclosed XML tag '<{}>'", tag),
                    line,
                    column,
                    format!("Add closing tag '</{}>", tag),
                ),
                XmlBalanceError::Mismatch {
                    expected,
                    found,
                    line,
                    column,
                } => (
                    "XML-002",
                    format!("Expected '</{}>' but found '</{}>'", expected, found),
                    line,
                    column,
                    format!("Replace '</{}>' with '</{}>'", found, expected),
                ),
                XmlBalanceError::UnmatchedClosing { tag, line, column } => (
                    "XML-003",
                    format!("Unmatched closing tag '</{}>'", tag),
                    line,
                    column,
                    format!("Remove '</{}>' or add matching opening tag '<{}>'", tag, tag),
                ),
            };

            // Check if specific rule is enabled before adding diagnostic
            if config.is_rule_enabled(rule_id) {
                diagnostics.push(
                    Diagnostic::error(path.to_path_buf(), line, column, rule_id, message)
                        .with_suggestion(suggestion),
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

    #[test]
    fn test_unclosed_tag() {
        let content = "<example>test";
        let validator = XmlValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn test_balanced_tags() {
        let content = "<example>test</example>";
        let validator = XmlValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_config_disabled_xml_category() {
        let mut config = LintConfig::default();
        config.rules.xml = false;

        let content = "<example>test";
        let validator = XmlValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &config);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_legacy_xml_balance_flag() {
        let mut config = LintConfig::default();
        config.rules.xml_balance = false;

        let content = "<example>test";
        let validator = XmlValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &config);

        assert!(diagnostics.is_empty());
    }
}
