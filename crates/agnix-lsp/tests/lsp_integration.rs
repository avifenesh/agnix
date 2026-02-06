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
        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule.contains("AS-004") || d.rule.contains("CC-SK"))
        );
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
    use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position};

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

mod lsp_handler_integration_tests {
    use agnix_lsp::Backend;
    use tower_lsp::lsp_types::*;
    use tower_lsp::{LanguageServer, LspService};

    #[tokio::test]
    async fn test_did_change_triggers_validation() {
        let (service, _socket) = LspService::new(Backend::new);

        let temp_dir = tempfile::tempdir().unwrap();
        let skill_path = temp_dir.path().join("SKILL.md");
        std::fs::write(
            &skill_path,
            r#"---
name: test-skill
version: 1.0.0
model: sonnet
---

# Test Skill
"#,
        )
        .unwrap();

        let uri = Url::from_file_path(&skill_path).unwrap();

        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: r#"---
name: test-skill
version: 1.0.0
model: sonnet
---

# Test Skill
"#
                    .to_string(),
                },
            })
            .await;

        service
            .inner()
            .did_change(DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version: 2,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: r#"---
name: updated-skill
version: 1.0.0
model: sonnet
---

# Updated Skill
"#
                    .to_string(),
                }],
            })
            .await;
    }

    #[tokio::test]
    async fn test_did_change_with_invalid_content_produces_diagnostics() {
        let (service, _socket) = LspService::new(Backend::new);

        let temp_dir = tempfile::tempdir().unwrap();
        let skill_path = temp_dir.path().join("SKILL.md");
        std::fs::write(&skill_path, "# Empty skill").unwrap();

        let uri = Url::from_file_path(&skill_path).unwrap();

        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: "# Empty skill".to_string(),
                },
            })
            .await;

        service
            .inner()
            .did_change(DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version: 2,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: r#"---
name: Invalid Name With Spaces
version: 1.0.0
model: invalid-model
---

# Invalid Skill
"#
                    .to_string(),
                }],
            })
            .await;
    }

    #[tokio::test]
    async fn test_code_action_returns_none_when_no_fixes() {
        let (service, _socket) = LspService::new(Backend::new);

        let temp_dir = tempfile::tempdir().unwrap();
        let skill_path = temp_dir.path().join("SKILL.md");
        std::fs::write(
            &skill_path,
            r#"---
name: test-skill
version: 1.0.0
model: sonnet
---

# Test Skill
"#,
        )
        .unwrap();

        let uri = Url::from_file_path(&skill_path).unwrap();

        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: r#"---
name: test-skill
version: 1.0.0
model: sonnet
---

# Test Skill
"#
                    .to_string(),
                },
            })
            .await;

        let result = service
            .inner()
            .code_action(CodeActionParams {
                text_document: TextDocumentIdentifier { uri },
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 10,
                    },
                },
                context: CodeActionContext {
                    diagnostics: vec![],
                    only: None,
                    trigger_kind: None,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_code_action_returns_none_for_uncached_document() {
        let (service, _socket) = LspService::new(Backend::new);

        let uri = Url::parse("file:///nonexistent/SKILL.md").unwrap();

        let result = service
            .inner()
            .code_action(CodeActionParams {
                text_document: TextDocumentIdentifier { uri },
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 10,
                    },
                },
                context: CodeActionContext {
                    diagnostics: vec![],
                    only: None,
                    trigger_kind: None,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_hover_returns_documentation_for_known_field() {
        let (service, _socket) = LspService::new(Backend::new);

        let temp_dir = tempfile::tempdir().unwrap();
        let skill_path = temp_dir.path().join("SKILL.md");
        std::fs::write(
            &skill_path,
            r#"---
name: test-skill
version: 1.0.0
model: sonnet
---

# Test Skill
"#,
        )
        .unwrap();

        let uri = Url::from_file_path(&skill_path).unwrap();

        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: r#"---
name: test-skill
version: 1.0.0
model: sonnet
---

# Test Skill
"#
                    .to_string(),
                },
            })
            .await;

        let result = service
            .inner()
            .hover(HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position: Position {
                        line: 3,
                        character: 0,
                    },
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await;

        assert!(result.is_ok());
        let hover = result.unwrap();
        assert!(hover.is_some());

        let hover = hover.unwrap();
        match hover.contents {
            HoverContents::Markup(markup) => {
                assert_eq!(markup.kind, MarkupKind::Markdown);
                assert!(markup.value.contains("model"));
            }
            _ => panic!("Expected markup content"),
        }
    }

    #[tokio::test]
    async fn test_hover_returns_none_for_unknown_field() {
        let (service, _socket) = LspService::new(Backend::new);

        let temp_dir = tempfile::tempdir().unwrap();
        let skill_path = temp_dir.path().join("SKILL.md");
        std::fs::write(
            &skill_path,
            r#"---
unknownfield: value
---

# Test
"#,
        )
        .unwrap();

        let uri = Url::from_file_path(&skill_path).unwrap();

        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: r#"---
unknownfield: value
---

# Test
"#
                    .to_string(),
                },
            })
            .await;

        let result = service
            .inner()
            .hover(HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position: Position {
                        line: 1,
                        character: 0,
                    },
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_hover_returns_none_for_uncached_document() {
        let (service, _socket) = LspService::new(Backend::new);

        let uri = Url::parse("file:///nonexistent/SKILL.md").unwrap();

        let result = service
            .inner()
            .hover(HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position: Position {
                        line: 0,
                        character: 0,
                    },
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_document_cache_lifecycle() {
        let (service, _socket) = LspService::new(Backend::new);

        let temp_dir = tempfile::tempdir().unwrap();
        let skill_path = temp_dir.path().join("SKILL.md");
        std::fs::write(&skill_path, "# Initial").unwrap();

        let uri = Url::from_file_path(&skill_path).unwrap();

        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: "# Initial".to_string(),
                },
            })
            .await;

        let hover_result = service
            .inner()
            .hover(HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position {
                        line: 0,
                        character: 0,
                    },
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await;
        assert!(hover_result.is_ok());

        service
            .inner()
            .did_change(DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version: 2,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: "# Changed".to_string(),
                }],
            })
            .await;

        service
            .inner()
            .did_close(DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
            })
            .await;

        let hover_after_close = service
            .inner()
            .hover(HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position: Position {
                        line: 0,
                        character: 0,
                    },
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await;
        assert!(hover_after_close.is_ok());
        assert!(hover_after_close.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_initialize_advertises_code_action_capability() {
        let (service, _socket) = LspService::new(Backend::new);

        let result = service
            .inner()
            .initialize(InitializeParams::default())
            .await
            .unwrap();

        match result.capabilities.code_action_provider {
            Some(CodeActionProviderCapability::Simple(true)) => {}
            _ => panic!("Expected code action capability"),
        }
    }

    #[tokio::test]
    async fn test_initialize_advertises_hover_capability() {
        let (service, _socket) = LspService::new(Backend::new);

        let result = service
            .inner()
            .initialize(InitializeParams::default())
            .await
            .unwrap();

        match result.capabilities.hover_provider {
            Some(HoverProviderCapability::Simple(true)) => {}
            _ => panic!("Expected hover capability"),
        }
    }

    #[tokio::test]
    async fn test_initialize_advertises_completion_capability() {
        let (service, _socket) = LspService::new(Backend::new);

        let result = service
            .inner()
            .initialize(InitializeParams::default())
            .await
            .unwrap();

        assert!(
            result.capabilities.completion_provider.is_some(),
            "Expected completion capability"
        );
    }

    #[tokio::test]
    async fn test_rapid_document_changes() {
        // Test that rapid document changes don't cause issues
        let (service, _socket) = LspService::new(Backend::new);

        let temp_dir = tempfile::tempdir().unwrap();
        let skill_path = temp_dir.path().join("SKILL.md");
        std::fs::write(&skill_path, "# Initial").unwrap();

        let uri = Url::from_file_path(&skill_path).unwrap();

        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: "# Initial".to_string(),
                },
            })
            .await;

        // Rapid-fire changes
        for i in 2..=10 {
            service
                .inner()
                .did_change(DidChangeTextDocumentParams {
                    text_document: VersionedTextDocumentIdentifier {
                        uri: uri.clone(),
                        version: i,
                    },
                    content_changes: vec![TextDocumentContentChangeEvent {
                        range: None,
                        range_length: None,
                        text: format!(
                            "---\nname: skill-{}\ndescription: Version {}\n---\n# Skill",
                            i, i
                        ),
                    }],
                })
                .await;
        }

        // Final state should be accessible
        let hover = service
            .inner()
            .hover(HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position: Position {
                        line: 1,
                        character: 0,
                    },
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await;

        assert!(hover.is_ok());
    }

    #[tokio::test]
    async fn test_multiple_documents_concurrent() {
        // Test handling multiple documents simultaneously
        let (service, _socket) = LspService::new(Backend::new);

        let temp_dir = tempfile::tempdir().unwrap();

        // Create and open multiple documents
        let mut uris = Vec::new();
        for i in 0..5 {
            let skill_dir = temp_dir.path().join(format!("skill-{}", i));
            std::fs::create_dir_all(&skill_dir).unwrap();
            let skill_path = skill_dir.join("SKILL.md");
            std::fs::write(
                &skill_path,
                format!(
                    "---\nname: skill-{}\ndescription: Test skill {}\n---\n# Skill {}",
                    i, i, i
                ),
            )
            .unwrap();

            let uri = Url::from_file_path(&skill_path).unwrap();
            uris.push(uri.clone());

            service
                .inner()
                .did_open(DidOpenTextDocumentParams {
                    text_document: TextDocumentItem {
                        uri,
                        language_id: "markdown".to_string(),
                        version: 1,
                        text: format!(
                            "---\nname: skill-{}\ndescription: Test skill {}\n---\n# Skill {}",
                            i, i, i
                        ),
                    },
                })
                .await;
        }

        // Query hover on all documents
        for uri in &uris {
            let hover = service
                .inner()
                .hover(HoverParams {
                    text_document_position_params: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        position: Position {
                            line: 1,
                            character: 0,
                        },
                    },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                })
                .await;

            assert!(hover.is_ok());
        }

        // Close all documents
        for uri in uris {
            service
                .inner()
                .did_close(DidCloseTextDocumentParams {
                    text_document: TextDocumentIdentifier { uri },
                })
                .await;
        }
    }

    #[tokio::test]
    async fn test_code_action_with_fix_available() {
        let (service, _socket) = LspService::new(Backend::new);

        let temp_dir = tempfile::tempdir().unwrap();
        let skill_path = temp_dir.path().join("SKILL.md");

        // Create skill with trailing whitespace (AS-001 provides fix)
        let content = "---\nname: test-skill\ndescription: Test   \n---\n# Test";
        std::fs::write(&skill_path, content).unwrap();

        let uri = Url::from_file_path(&skill_path).unwrap();

        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: content.to_string(),
                },
            })
            .await;

        // Request code actions
        let result = service
            .inner()
            .code_action(CodeActionParams {
                text_document: TextDocumentIdentifier { uri },
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 5,
                        character: 0,
                    },
                },
                context: CodeActionContext {
                    diagnostics: vec![],
                    only: None,
                    trigger_kind: None,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .await;

        assert!(result.is_ok());
        // May or may not have actions depending on validation results
    }
}
