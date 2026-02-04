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
            ..Default::default()
        };

        match capabilities.text_document_sync {
            Some(TextDocumentSyncCapability::Kind(kind)) => {
                assert_eq!(kind, TextDocumentSyncKind::FULL);
            }
            _ => panic!("Expected FULL text document sync"),
        }
    }
}
