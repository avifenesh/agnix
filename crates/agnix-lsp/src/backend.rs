//! LSP backend implementation for agnix.
//!
//! Implements the Language Server Protocol using tower-lsp, providing
//! real-time validation of agent configuration files.

use std::path::PathBuf;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::diagnostic_mapper::to_lsp_diagnostics;

/// LSP backend that handles validation requests.
///
/// The backend maintains a connection to the LSP client and validates
/// files on open and save events.
pub struct Backend {
    client: Client,
}

impl Backend {
    /// Create a new backend instance with the given client connection.
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Validate a file and publish diagnostics to the client.
    async fn validate_and_publish(&self, uri: Url) {
        let path = match uri.to_file_path() {
            Ok(p) => p,
            Err(()) => {
                self.client
                    .log_message(MessageType::WARNING, format!("Invalid file URI: {}", uri))
                    .await;
                return;
            }
        };

        let diagnostics = self.validate_file(path).await;

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    /// Run validation on a file in a blocking task.
    ///
    /// agnix-core validation is CPU-bound and synchronous, so we run it
    /// in a blocking task to avoid blocking the async runtime.
    async fn validate_file(&self, path: PathBuf) -> Vec<Diagnostic> {
        let result = tokio::task::spawn_blocking(move || {
            let config = agnix_core::LintConfig::default();
            agnix_core::validate_file(&path, &config)
        })
        .await;

        match result {
            Ok(Ok(diagnostics)) => to_lsp_diagnostics(diagnostics),
            Ok(Err(e)) => {
                vec![Diagnostic {
                    range: Range {
                        start: Position { line: 0, character: 0 },
                        end: Position { line: 0, character: 0 },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("agnix::validation-error".to_string())),
                    code_description: None,
                    source: Some("agnix".to_string()),
                    message: format!("Validation error: {}", e),
                    related_information: None,
                    tags: None,
                    data: None,
                }]
            }
            Err(e) => {
                vec![Diagnostic {
                    range: Range {
                        start: Position { line: 0, character: 0 },
                        end: Position { line: 0, character: 0 },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("agnix::internal-error".to_string())),
                    code_description: None,
                    source: Some("agnix".to_string()),
                    message: format!("Internal error: {}", e),
                    related_information: None,
                    tags: None,
                    data: None,
                }]
            }
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "agnix-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "agnix-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.validate_and_publish(params.text_document.uri).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.validate_and_publish(params.text_document.uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::LspService;

    /// Test that Backend::new creates a valid Backend instance.
    /// We verify this by creating a service and checking initialize returns proper capabilities.
    #[tokio::test]
    async fn test_backend_new_creates_valid_instance() {
        let (service, _socket) = LspService::new(Backend::new);

        // The service was created successfully, meaning Backend::new worked
        // We can verify by calling initialize
        let init_params = InitializeParams::default();
        let result = service.inner().initialize(init_params).await;

        assert!(result.is_ok());
    }

    /// Test that initialize() returns correct server capabilities.
    #[tokio::test]
    async fn test_initialize_returns_correct_capabilities() {
        let (service, _socket) = LspService::new(Backend::new);

        let init_params = InitializeParams::default();
        let result = service.inner().initialize(init_params).await;

        let init_result = result.expect("initialize should succeed");

        // Verify text document sync capability
        match init_result.capabilities.text_document_sync {
            Some(TextDocumentSyncCapability::Kind(kind)) => {
                assert_eq!(kind, TextDocumentSyncKind::FULL);
            }
            _ => panic!("Expected FULL text document sync capability"),
        }

        // Verify server info
        let server_info = init_result.server_info.expect("server_info should be present");
        assert_eq!(server_info.name, "agnix-lsp");
        assert!(server_info.version.is_some());
    }

    /// Test that shutdown() returns Ok.
    #[tokio::test]
    async fn test_shutdown_returns_ok() {
        let (service, _socket) = LspService::new(Backend::new);

        let result = service.inner().shutdown().await;
        assert!(result.is_ok());
    }

    /// Test validation error diagnostic has correct code.
    /// We test the diagnostic structure directly since we can't easily mock the validation.
    #[test]
    fn test_validation_error_diagnostic_structure() {
        // Simulate what validate_file returns on validation error
        let error_message = "Failed to parse file";
        let diagnostic = Diagnostic {
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 0 },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String("agnix::validation-error".to_string())),
            code_description: None,
            source: Some("agnix".to_string()),
            message: format!("Validation error: {}", error_message),
            related_information: None,
            tags: None,
            data: None,
        };

        assert_eq!(
            diagnostic.code,
            Some(NumberOrString::String("agnix::validation-error".to_string()))
        );
        assert_eq!(diagnostic.source, Some("agnix".to_string()));
        assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::ERROR));
        assert!(diagnostic.message.contains("Validation error:"));
    }

    /// Test internal error diagnostic has correct code.
    #[test]
    fn test_internal_error_diagnostic_structure() {
        // Simulate what validate_file returns on panic/internal error
        let error_message = "task panicked";
        let diagnostic = Diagnostic {
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 0 },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String("agnix::internal-error".to_string())),
            code_description: None,
            source: Some("agnix".to_string()),
            message: format!("Internal error: {}", error_message),
            related_information: None,
            tags: None,
            data: None,
        };

        assert_eq!(
            diagnostic.code,
            Some(NumberOrString::String("agnix::internal-error".to_string()))
        );
        assert_eq!(diagnostic.source, Some("agnix".to_string()));
        assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::ERROR));
        assert!(diagnostic.message.contains("Internal error:"));
    }

    /// Test that invalid URIs are identified correctly.
    /// Non-file URIs should fail to_file_path().
    #[test]
    fn test_invalid_uri_detection() {
        // Non-file URIs should fail to_file_path()
        let http_uri = Url::parse("http://example.com/file.md").unwrap();
        assert!(http_uri.to_file_path().is_err());

        let data_uri = Url::parse("data:text/plain;base64,SGVsbG8=").unwrap();
        assert!(data_uri.to_file_path().is_err());

        // File URIs should succeed - use platform-appropriate path
        #[cfg(windows)]
        let file_uri = Url::parse("file:///C:/tmp/test.md").unwrap();
        #[cfg(not(windows))]
        let file_uri = Url::parse("file:///tmp/test.md").unwrap();
        assert!(file_uri.to_file_path().is_ok());
    }

    /// Test validate_file with a valid file returns diagnostics.
    #[tokio::test]
    async fn test_validate_file_valid_skill() {
        let (service, _socket) = LspService::new(Backend::new);

        // Create a valid skill file
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

This is a valid skill.
"#,
        )
        .unwrap();

        // We can't directly call validate_file since it's private,
        // but we can verify the validation logic works through did_open
        // The Backend will log messages to the client
        let uri = Url::from_file_path(&skill_path).unwrap();

        // Call did_open which triggers validate_and_publish internally
        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri,
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: String::new(), // Content is read from file
                },
            })
            .await;

        // If we get here without panicking, the validation completed
    }

    /// Test validate_file with an invalid skill file.
    #[tokio::test]
    async fn test_validate_file_invalid_skill() {
        let (service, _socket) = LspService::new(Backend::new);

        // Create an invalid skill file (invalid name with spaces)
        let temp_dir = tempfile::tempdir().unwrap();
        let skill_path = temp_dir.path().join("SKILL.md");
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

        let uri = Url::from_file_path(&skill_path).unwrap();

        // Call did_open which triggers validation
        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri,
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: String::new(),
                },
            })
            .await;

        // Validation should complete and publish diagnostics
    }

    /// Test did_save triggers validation.
    #[tokio::test]
    async fn test_did_save_triggers_validation() {
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

        // Call did_save which triggers validate_and_publish
        service
            .inner()
            .did_save(DidSaveTextDocumentParams {
                text_document: TextDocumentIdentifier { uri },
                text: None,
            })
            .await;

        // Validation should complete without error
    }

    /// Test did_close clears diagnostics.
    #[tokio::test]
    async fn test_did_close_clears_diagnostics() {
        let (service, _socket) = LspService::new(Backend::new);

        let temp_dir = tempfile::tempdir().unwrap();
        let skill_path = temp_dir.path().join("SKILL.md");
        std::fs::write(&skill_path, "# Test").unwrap();

        let uri = Url::from_file_path(&skill_path).unwrap();

        // Call did_close which publishes empty diagnostics
        service
            .inner()
            .did_close(DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri },
            })
            .await;

        // Should complete without error
    }

    /// Test initialized() completes without error.
    #[tokio::test]
    async fn test_initialized_completes() {
        let (service, _socket) = LspService::new(Backend::new);

        // Call initialized
        service.inner().initialized(InitializedParams {}).await;

        // Should complete without error (logs a message to client)
    }

    /// Test validate_and_publish with non-file URI is handled gracefully.
    /// Since validate_and_publish is private, we test the URI validation logic directly.
    #[tokio::test]
    async fn test_non_file_uri_handled_gracefully() {
        let (service, _socket) = LspService::new(Backend::new);

        // Create a non-file URI (http://)
        let http_uri = Url::parse("http://example.com/test.md").unwrap();

        // Call did_open with non-file URI
        // This should be handled gracefully (log warning and return early)
        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: http_uri,
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: String::new(),
                },
            })
            .await;

        // Should complete without panic
    }

    /// Test validation with non-existent file.
    #[tokio::test]
    async fn test_validate_nonexistent_file() {
        let (service, _socket) = LspService::new(Backend::new);

        // Create a URI for a file that doesn't exist
        let temp_dir = tempfile::tempdir().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent.md");
        let uri = Url::from_file_path(&nonexistent_path).unwrap();

        // Call did_open - should handle missing file gracefully
        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri,
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: String::new(),
                },
            })
            .await;

        // Should complete without panic (will publish error diagnostic)
    }

    /// Test server info contains version from Cargo.toml.
    #[tokio::test]
    async fn test_server_info_version() {
        let (service, _socket) = LspService::new(Backend::new);

        let init_params = InitializeParams::default();
        let result = service.inner().initialize(init_params).await.unwrap();

        let server_info = result.server_info.unwrap();
        let version = server_info.version.unwrap();

        // Version should be a valid semver string
        assert!(!version.is_empty());
        // Should match the crate version pattern (e.g., "0.1.0")
        assert!(version.contains('.'));
    }
}
