//! XML tag balance validation

use crate::{
    config::LintConfig,
    diagnostics::{Diagnostic, Fix},
    parsers::markdown::{
        XmlBalanceError, XmlTag, check_xml_balance_with_content_end, extract_xml_tags,
    },
    rules::Validator,
};
use rust_i18n::t;
use std::path::Path;

pub struct XmlValidator;

fn find_unique_closing_tag_span(
    tags: &[XmlTag],
    line: usize,
    column: usize,
    name: &str,
) -> Option<(usize, usize)> {
    let mut matches = tags.iter().filter(|tag| {
        tag.is_closing && tag.line == line && tag.column == column && tag.name == name
    });
    let first = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    Some((first.start_byte, first.end_byte))
}

impl Validator for XmlValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Early return if XML category is disabled or legacy flag is disabled
        if !config.rules.xml || !config.rules.xml_balance {
            return diagnostics;
        }

        let tags = extract_xml_tags(content);
        let errors = check_xml_balance_with_content_end(&tags, Some(content.len()));

        for error in errors {
            match error {
                XmlBalanceError::Unclosed {
                    tag,
                    line,
                    column,
                    content_end_byte,
                    ..
                } => {
                    let rule_id = "XML-001";
                    if !config.is_rule_enabled(rule_id) {
                        continue;
                    }
                    let message = t!("rules.xml_001.message", tag = tag);
                    let suggestion = t!("rules.xml_001.suggestion", tag = tag);
                    let closing_tag = format!("</{}>", tag);

                    // Create fix: insert closing tag at content end
                    // safe=false because we can't be 100% certain where the user wants it
                    // NOTE: When multiple tags are unclosed, all fixes insert at the same position.
                    // The fix application in fixes.rs sorts by descending position, ensuring
                    // correct nesting order (later fixes applied first).
                    let fix = Fix::insert(
                        content_end_byte,
                        closing_tag,
                        t!("rules.xml_001.fix", tag = tag),
                        false,
                    );

                    let diagnostic =
                        Diagnostic::error(path.to_path_buf(), line, column, rule_id, message)
                            .with_suggestion(suggestion)
                            .with_fix(fix);
                    diagnostics.push(diagnostic);
                }
                XmlBalanceError::Mismatch {
                    expected,
                    found,
                    line,
                    column,
                } => {
                    let rule_id = "XML-002";
                    if !config.is_rule_enabled(rule_id) {
                        continue;
                    }
                    let message = t!("rules.xml_002.message", expected = expected, found = found);
                    let suggestion = t!(
                        "rules.xml_002.suggestion",
                        found = found,
                        expected = expected
                    );

                    let mut diagnostic =
                        Diagnostic::error(path.to_path_buf(), line, column, rule_id, message)
                            .with_suggestion(suggestion);

                    // Unsafe auto-fix: rewrite mismatched closing tag to expected closing tag.
                    if let Some((start, end)) =
                        find_unique_closing_tag_span(&tags, line, column, &found)
                    {
                        diagnostic = diagnostic.with_fix(Fix::replace(
                            start,
                            end,
                            format!("</{}>", expected),
                            format!("Replace </{}> with </{}>", found, expected),
                            false,
                        ));
                    }

                    diagnostics.push(diagnostic);
                }
                XmlBalanceError::UnmatchedClosing { tag, line, column } => {
                    let rule_id = "XML-003";
                    if !config.is_rule_enabled(rule_id) {
                        continue;
                    }
                    let message = t!("rules.xml_003.message", tag = tag);
                    let suggestion = t!("rules.xml_003.suggestion", tag = tag);

                    let mut diagnostic =
                        Diagnostic::error(path.to_path_buf(), line, column, rule_id, message)
                            .with_suggestion(suggestion);

                    // Unsafe auto-fix: remove unmatched closing tag.
                    if let Some((start, end)) =
                        find_unique_closing_tag_span(&tags, line, column, &tag)
                    {
                        diagnostic = diagnostic.with_fix(Fix::delete(
                            start,
                            end,
                            format!("Remove unmatched closing tag </{}>", tag),
                            false,
                        ));
                    }

                    diagnostics.push(diagnostic);
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

    // XML-001: Unclosed tag produces XML-001 rule ID
    #[test]
    fn test_xml_001_rule_id() {
        let content = "<example>test";
        let validator = XmlValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, "XML-001");
        assert!(diagnostics[0].message.contains("Unclosed XML tag"));
    }

    // XML-002: Tag mismatch produces XML-002 rule ID
    #[test]
    fn test_xml_002_rule_id() {
        // <a><b></a></b> produces a mismatch: expected </b> but found </a>
        let content = "<outer><inner></outer></inner>";
        let validator = XmlValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        // Find the XML-002 diagnostic
        let xml_002 = diagnostics.iter().find(|d| d.rule == "XML-002");
        assert!(xml_002.is_some(), "Expected XML-002 diagnostic");
        assert!(
            xml_002
                .unwrap()
                .message
                .contains("Expected '</inner>' but found '</outer>'")
        );
    }

    // XML-003: Unmatched closing tag produces XML-003 rule ID
    #[test]
    fn test_xml_003_rule_id() {
        let content = "</orphan>";
        let validator = XmlValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, "XML-003");
        assert!(diagnostics[0].message.contains("Unmatched closing tag"));
    }

    // Test that individual rules can be disabled
    #[test]
    fn test_xml_001_can_be_disabled() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["XML-001".to_string()];

        let content = "<example>test";
        let validator = XmlValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &config);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_xml_002_can_be_disabled() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["XML-002".to_string()];

        let content = "<outer><inner></outer></inner>";
        let validator = XmlValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &config);

        // XML-002 should be filtered out, but other errors may remain
        assert!(!diagnostics.iter().any(|d| d.rule == "XML-002"));
    }

    #[test]
    fn test_xml_003_can_be_disabled() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["XML-003".to_string()];

        let content = "</orphan>";
        let validator = XmlValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &config);

        assert!(diagnostics.is_empty());
    }

    // ===== Auto-fix Tests for XML-001 =====

    #[test]
    fn test_xml_001_has_fix() {
        let content = "<example>test content";
        let validator = XmlValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, "XML-001");
        assert!(diagnostics[0].has_fixes());

        let fix = &diagnostics[0].fixes[0];
        assert_eq!(fix.replacement, "</example>");
        assert_eq!(fix.start_byte, content.len());
        assert_eq!(fix.end_byte, content.len()); // Insertion: start == end
        assert!(!fix.safe); // Not safe, position is heuristic
    }

    #[test]
    fn test_xml_001_fix_correct_byte_position() {
        let content = "<tag>some text here";
        let validator = XmlValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        assert_eq!(diagnostics.len(), 1);
        let fix = &diagnostics[0].fixes[0];

        // After applying the fix, content should be balanced
        let mut fixed_content = content.to_string();
        fixed_content.insert_str(fix.start_byte, &fix.replacement);
        assert_eq!(fixed_content, "<tag>some text here</tag>");
    }

    #[test]
    fn test_xml_001_fix_nested_tags() {
        let content = "<outer><inner>content";
        let validator = XmlValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        // Both tags are unclosed
        assert_eq!(diagnostics.len(), 2);

        // Each should have a fix
        for d in &diagnostics {
            assert!(d.has_fixes());
            let fix = &d.fixes[0];
            assert!(fix.is_insertion());
            // Fix position is at content end
            assert_eq!(fix.start_byte, content.len());
        }
    }

    #[test]
    fn test_xml_001_fix_nested_tags_applied() {
        let content = "<outer><inner>content";
        let validator = XmlValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        // Both tags are unclosed
        assert_eq!(diagnostics.len(), 2);

        // Collect fixes and sort descending by position (like fixes.rs does)
        let mut fixes: Vec<_> = diagnostics.iter().flat_map(|d| &d.fixes).collect();
        fixes.sort_by(|a, b| b.start_byte.cmp(&a.start_byte));

        // Apply fixes manually (simulating apply_fixes_to_content)
        let mut result = content.to_string();
        let mut applied_count = 0;
        let mut last_start = usize::MAX;

        for fix in &fixes {
            // Skip overlapping (end > last_start)
            if fix.end_byte > last_start {
                continue;
            }
            result.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
            last_start = fix.start_byte;
            applied_count += 1;
        }

        // Both fixes should be applied (insertions at same position are allowed)
        assert_eq!(applied_count, 2, "Expected 2 fixes to be applied");

        // Result should have both closing tags
        assert!(
            result.contains("</inner>") && result.contains("</outer>"),
            "Expected both closing tags, got: {}",
            result
        );
    }

    #[test]
    fn test_xml_001_fix_description() {
        let content = "<myTag>incomplete";
        let validator = XmlValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        assert_eq!(diagnostics.len(), 1);
        let fix = &diagnostics[0].fixes[0];
        assert!(fix.description.contains("</myTag>"));
    }

    #[test]
    fn test_xml_002_has_unsafe_fix() {
        let content = "<outer><inner></outer></inner>";
        let validator = XmlValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let xml_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XML-002").collect();
        assert!(!xml_002.is_empty());
        assert!(xml_002[0].has_fixes());
        let fix = &xml_002[0].fixes[0];
        assert_eq!(fix.replacement, "</inner>");
        assert!(!fix.safe);
    }

    #[test]
    fn test_xml_003_has_unsafe_fix() {
        let content = "</orphan>";
        let validator = XmlValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, "XML-003");
        assert!(diagnostics[0].has_fixes());
        let fix = &diagnostics[0].fixes[0];
        assert!(fix.is_deletion());
        assert!(!fix.safe);
    }
}
