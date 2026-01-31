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

        // Check both new category flag and legacy flag for backward compatibility
        if config.is_rule_enabled("xml::balance") && config.rules.xml_balance {
            let tags = extract_xml_tags(content);
            let errors = check_xml_balance(&tags);

            for error in errors {
                let (message, line, column) = match error {
                    XmlBalanceError::Unclosed { tag, line, column } => {
                        (format!("Unclosed XML tag '<{}>'", tag), line, column)
                    }
                    XmlBalanceError::UnmatchedClosing { tag, line, column } => {
                        (format!("Unmatched closing tag '</{}>'", tag), line, column)
                    }
                    XmlBalanceError::Mismatch {
                        expected,
                        found,
                        line,
                        column,
                    } => (
                        format!("Expected '</{}>' but found '</{}>'", expected, found),
                        line,
                        column,
                    ),
                };

                diagnostics.push(
                    Diagnostic::error(path.to_path_buf(), line, column, "xml::balance", message)
                        .with_suggestion("Ensure all XML tags are properly closed".to_string()),
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
