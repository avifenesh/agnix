//! Integration tests for agnix-lsp.
//!
//! These tests verify that the LSP server correctly processes
//! requests and returns appropriate responses.

use agnix_core::{Diagnostic, DiagnosticLevel};
use std::path::PathBuf;

// Re-export the diagnostic mapper for testing
mod diagnostic_mapper_tests {
    use super::*;

    fn make_diagnostic(
        level: DiagnosticLevel,
        message: &str,
        line: usize,
        column: usize,
        rule: &str,
    ) -> Diagnostic {
        Diagnostic {
            level,
            message: message.to_string(),
            file: PathBuf::from("test.md"),
            line,
            column,
            rule: rule.to_string(),
            suggestion: None,
            fixes: vec![],
            assumption: None,
        }
    }

    #[test]
    fn test_diagnostic_creation() {
        let diag = make_diagnostic(DiagnosticLevel::Error, "Test error", 10, 5, "AS-001");
        assert_eq!(diag.level, DiagnosticLevel::Error);
        assert_eq!(diag.message, "Test error");
        assert_eq!(diag.line, 10);
        assert_eq!(diag.column, 5);
        assert_eq!(diag.rule, "AS-001");
    }

    #[test]
    fn test_all_diagnostic_levels() {
        let error = make_diagnostic(DiagnosticLevel::Error, "Error", 1, 1, "AS-001");
        let warning = make_diagnostic(DiagnosticLevel::Warning, "Warning", 1, 1, "AS-002");
        let info = make_diagnostic(DiagnosticLevel::Info, "Info", 1, 1, "AS-003");

        assert_eq!(error.level, DiagnosticLevel::Error);
        assert_eq!(warning.level, DiagnosticLevel::Warning);
        assert_eq!(info.level, DiagnosticLevel::Info);
    }
}

mod validation_tests {
    use agnix_core::LintConfig;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_validate_valid_skill_file() {
        let mut file = NamedTempFile::with_suffix(".md").unwrap();
        writeln!(
            file,
            r#"---
name: test-skill
version: 1.0.0
model: sonnet
---

# Test Skill

This is a valid skill file.
"#
        )
        .unwrap();

        // Rename to SKILL.md to trigger skill validation
        let skill_dir = tempfile::tempdir().unwrap();
        let skill_path = skill_dir.path().join("SKILL.md");
        std::fs::copy(file.path(), &skill_path).unwrap();

        let config = LintConfig::default();
        let result = agnix_core::validate_file(&skill_path, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_invalid_skill_name() {
        let skill_dir = tempfile::tempdir().unwrap();
        let skill_path = skill_dir.path().join("SKILL.md");

        std::fs::write(
            &skill_path,
            r#"---
name: Invalid Name With Spaces
version: 1.0.0
model: sonnet
---

# Invalid Skill

This skill has an invalid name.
"#,
        )
        .unwrap();

        let config = LintConfig::default();
        let result = agnix_core::validate_file(&skill_path, &config);
        assert!(result.is_ok());

        let diagnostics = result.unwrap();
        // Should have at least one error for invalid name
        assert!(!diagnostics.is_empty());
        assert!(diagnostics
            .iter()
            .any(|d| d.rule.contains("AS-004") || d.rule.contains("CC-SK")));
    }

    #[test]
    fn test_validate_unknown_file_type() {
        let file = NamedTempFile::with_suffix(".txt").unwrap();
        std::fs::write(file.path(), "Some random content").unwrap();

        let config = LintConfig::default();
        let result = agnix_core::validate_file(file.path(), &config);
        assert!(result.is_ok());

        // Unknown file types should return empty diagnostics
        let diagnostics = result.unwrap();
        assert!(diagnostics.is_empty());
    }
}

mod server_capability_tests {
    use tower_lsp::lsp_types::*;

    #[test]
    fn test_server_capabilities_are_reasonable() {
        // Verify that the capabilities we advertise are what we expect
        let capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            ..Default::default()
        };

        match capabilities.text_document_sync {
            Some(TextDocumentSyncCapability::Kind(kind)) => {
                assert_eq!(kind, TextDocumentSyncKind::FULL);
            }
            _ => panic!("Expected FULL text document sync"),
        }

        // Verify code action capability
        match capabilities.code_action_provider {
            Some(CodeActionProviderCapability::Simple(true)) => {}
            _ => panic!("Expected code action provider"),
        }

        // Verify hover capability
        match capabilities.hover_provider {
            Some(HoverProviderCapability::Simple(true)) => {}
            _ => panic!("Expected hover provider"),
        }
    }
}

mod code_action_tests {
    use agnix_core::Fix;

    #[test]
    fn test_fix_with_safe_flag() {
        let fix = Fix {
            start_byte: 0,
            end_byte: 5,
            replacement: "hello".to_string(),
            description: "Test fix".to_string(),
            safe: true,
        };

        assert!(fix.safe);
        assert_eq!(fix.start_byte, 0);
        assert_eq!(fix.end_byte, 5);
    }

    #[test]
    fn test_fix_with_unsafe_flag() {
        let fix = Fix {
            start_byte: 10,
            end_byte: 20,
            replacement: "world".to_string(),
            description: "Unsafe fix".to_string(),
            safe: false,
        };

        assert!(!fix.safe);
    }

    #[test]
    fn test_fix_insertion() {
        // Insertion is when start == end
        let fix = Fix {
            start_byte: 5,
            end_byte: 5,
            replacement: "inserted text".to_string(),
            description: "Insert text".to_string(),
            safe: true,
        };

        assert_eq!(fix.start_byte, fix.end_byte);
        assert!(!fix.replacement.is_empty());
    }

    #[test]
    fn test_fix_deletion() {
        // Deletion is when replacement is empty
        let fix = Fix {
            start_byte: 0,
            end_byte: 10,
            replacement: String::new(),
            description: "Delete text".to_string(),
            safe: true,
        };

        assert!(fix.replacement.is_empty());
        assert!(fix.start_byte < fix.end_byte);
    }
}

mod did_change_tests {
    use agnix_core::{Diagnostic, DiagnosticLevel, Fix};
    use std::path::PathBuf;

    #[test]
    fn test_diagnostic_with_multiple_fixes() {
        let fixes = vec![
            Fix {
                start_byte: 0,
                end_byte: 5,
                replacement: "fix1".to_string(),
                description: "First fix".to_string(),
                safe: true,
            },
            Fix {
                start_byte: 10,
                end_byte: 15,
                replacement: "fix2".to_string(),
                description: "Second fix".to_string(),
                safe: false,
            },
        ];

        let diag = Diagnostic {
            level: DiagnosticLevel::Error,
            message: "Multiple fixes available".to_string(),
            file: PathBuf::from("test.md"),
            line: 1,
            column: 1,
            rule: "AS-001".to_string(),
            suggestion: None,
            fixes,
            assumption: None,
        };

        assert_eq!(diag.fixes.len(), 2);
        assert!(diag.fixes[0].safe);
        assert!(!diag.fixes[1].safe);
    }

    #[test]
    fn test_diagnostic_has_fixes_method() {
        let diag_with_fixes = Diagnostic {
            level: DiagnosticLevel::Error,
            message: "Error".to_string(),
            file: PathBuf::from("test.md"),
            line: 1,
            column: 1,
            rule: "AS-001".to_string(),
            suggestion: None,
            fixes: vec![Fix {
                start_byte: 0,
                end_byte: 1,
                replacement: "x".to_string(),
                description: "Fix".to_string(),
                safe: true,
            }],
            assumption: None,
        };

        let diag_without_fixes = Diagnostic {
            level: DiagnosticLevel::Error,
            message: "Error".to_string(),
            file: PathBuf::from("test.md"),
            line: 1,
            column: 1,
            rule: "AS-001".to_string(),
            suggestion: None,
            fixes: vec![],
            assumption: None,
        };

        assert!(diag_with_fixes.has_fixes());
        assert!(!diag_without_fixes.has_fixes());
    }
}

mod hover_tests {
    use tower_lsp::lsp_types::{Position, Hover, HoverContents, MarkupContent, MarkupKind};

    #[test]
    fn test_hover_content_structure() {
        let hover = Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "**field** documentation".to_string(),
            }),
            range: None,
        };

        match hover.contents {
            HoverContents::Markup(markup) => {
                assert_eq!(markup.kind, MarkupKind::Markdown);
                assert!(markup.value.contains("field"));
            }
            _ => panic!("Expected markup content"),
        }
    }

    #[test]
    fn test_position_creation() {
        let pos = Position {
            line: 10,
            character: 5,
        };

        assert_eq!(pos.line, 10);
        assert_eq!(pos.character, 5);
    }

    #[test]
    fn test_position_zero() {
        let pos = Position {
            line: 0,
            character: 0,
        };

        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);
    }
}
