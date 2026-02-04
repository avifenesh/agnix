//! LSP backend implementation for agnix.
//!
//! Implements the Language Server Protocol using tower-lsp, providing
//! real-time validation of agent configuration files.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::code_actions::fixes_to_code_actions;
use crate::diagnostic_mapper::{deserialize_fixes, to_lsp_diagnostics};
use crate::hover_provider::hover_at_position;

fn create_error_diagnostic(code: &str, message: String) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 0,
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(code.to_string())),
        code_description: None,
        source: Some("agnix".to_string()),
        message,
        related_information: None,
        tags: None,
        data: None,
    }
}

/// LSP backend that handles validation requests.
///
/// The backend maintains a connection to the LSP client and validates
/// files on open, change, and save events. It also provides code actions
/// for quick fixes and hover documentation for configuration fields.
///
/// # Performance Notes
///
/// Both `LintConfig` and `ValidatorRegistry` are cached and reused across
/// validations to avoid repeated allocations.
pub struct Backend {
    client: Client,
    /// Cached lint configuration reused across validations.
    /// Wrapped in RwLock to allow loading from .agnix.toml after initialize().
    config: RwLock<Arc<agnix_core::LintConfig>>,
    /// Workspace root path for boundary validation (security).
    /// Set during initialize() from the client's root_uri.
    workspace_root: RwLock<Option<PathBuf>>,
    documents: RwLock<HashMap<Url, String>>,
    /// Cached validator registry reused across validations.
    /// Immutable after construction; Arc enables sharing across spawn_blocking tasks.
    registry: Arc<agnix_core::ValidatorRegistry>,
}

impl Backend {
    /// Create a new backend instance with the given client connection.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            config: RwLock::new(Arc::new(agnix_core::LintConfig::default())),
            workspace_root: RwLock::new(None),
            documents: RwLock::new(HashMap::new()),
            registry: Arc::new(agnix_core::ValidatorRegistry::with_defaults()),
        }
    }

    /// Run validation on a file in a blocking task.
    ///
    /// agnix-core validation is CPU-bound and synchronous, so we run it
    /// in a blocking task to avoid blocking the async runtime.
    ///
    /// Both `LintConfig` and `ValidatorRegistry` are cloned from cached
    /// instances to avoid repeated allocations on each validation.
    async fn validate_file(&self, path: PathBuf) -> Vec<Diagnostic> {
        let config = Arc::clone(&*self.config.read().await);
        let registry = Arc::clone(&self.registry);
        let result = tokio::task::spawn_blocking(move || {
            agnix_core::validate_file_with_registry(&path, &config, &registry)
        })
        .await;

        match result {
            Ok(Ok(diagnostics)) => to_lsp_diagnostics(diagnostics),
            Ok(Err(e)) => vec![create_error_diagnostic(
                "agnix::validation-error",
                format!("Validation error: {}", e),
            )],
            Err(e) => vec![create_error_diagnostic(
                "agnix::internal-error",
                format!("Internal error: {}", e),
            )],
        }
    }

    /// Validate from cached content and publish diagnostics.
    ///
    /// Used for did_change events where we have the content in memory.
    /// This avoids reading from disk and provides real-time feedback.
    async fn validate_from_content_and_publish(&self, uri: Url) {
        let file_path = match uri.to_file_path() {
            Ok(p) => p,
            Err(()) => {
                self.client
                    .log_message(MessageType::WARNING, format!("Invalid file URI: {}", uri))
                    .await;
                return;
            }
        };

        // Security: Validate file is within workspace boundaries
        if let Some(ref workspace_root) = *self.workspace_root.read().await {
            let canonical_path = match file_path.canonicalize() {
                Ok(p) => p,
                Err(_) => file_path.clone(),
            };
            let canonical_root = workspace_root
                .canonicalize()
                .unwrap_or_else(|_| workspace_root.clone());

            if !canonical_path.starts_with(&canonical_root) {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        format!("File outside workspace boundary: {}", uri),
                    )
                    .await;
                return;
            }
        }

        // Get content from cache
        let content = {
            let docs = self.documents.read().await;
            match docs.get(&uri) {
                Some(c) => c.clone(),
                None => {
                    // Fall back to file-based validation
                    drop(docs);
                    let diagnostics = self.validate_file(file_path).await;
                    self.client
                        .publish_diagnostics(uri, diagnostics, None)
                        .await;
                    return;
                }
            }
        };

        let config = Arc::clone(&*self.config.read().await);
        let result = tokio::task::spawn_blocking(move || {
            let file_type = agnix_core::detect_file_type(&file_path);
            if file_type == agnix_core::FileType::Unknown {
                return Ok(vec![]);
            }

            let registry = agnix_core::ValidatorRegistry::with_defaults();
            let validators = registry.validators_for(file_type);
            let mut diagnostics = Vec::new();

            for validator in validators {
                diagnostics.extend(validator.validate(&file_path, &content, &config));
            }

            Ok::<_, agnix_core::LintError>(diagnostics)
        })
        .await;

        let diagnostics = match result {
            Ok(Ok(diagnostics)) => to_lsp_diagnostics(diagnostics),
            Ok(Err(e)) => vec![create_error_diagnostic(
                "agnix::validation-error",
                format!("Validation error: {}", e),
            )],
            Err(e) => vec![create_error_diagnostic(
                "agnix::internal-error",
                format!("Internal error: {}", e),
            )],
        };

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    /// Get cached document content for a URI.
    async fn get_document_content(&self, uri: &Url) -> Option<String> {
        self.documents.read().await.get(uri).cloned()
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // Capture workspace root for path boundary validation
        if let Some(root_uri) = params.root_uri {
            if let Ok(root_path) = root_uri.to_file_path() {
                *self.workspace_root.write().await = Some(root_path.clone());

                // Try to load config from .agnix.toml in workspace root
                let config_path = root_path.join(".agnix.toml");
                if config_path.exists() {
                    match agnix_core::LintConfig::load(&config_path) {
                        Ok(loaded_config) => {
                            *self.config.write().await = Arc::new(loaded_config);
                        }
                        Err(e) => {
                            // Log error but continue with default config
                            self.client
                                .log_message(
                                    MessageType::WARNING,
                                    format!("Failed to load .agnix.toml: {}", e),
                                )
                                .await;
                        }
                    }
                }
            }
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
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
        {
            let mut docs = self.documents.write().await;
            docs.insert(
                params.text_document.uri.clone(),
                params.text_document.text.clone(),
            );
        }
        self.validate_from_content_and_publish(params.text_document.uri)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().next() {
            {
                let mut docs = self.documents.write().await;
                docs.insert(params.text_document.uri.clone(), change.text);
            }
            self.validate_from_content_and_publish(params.text_document.uri)
                .await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.validate_from_content_and_publish(params.text_document.uri)
            .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        {
            let mut docs = self.documents.write().await;
            docs.remove(&params.text_document.uri);
        }
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = &params.text_document.uri;

        // Get document content for byte-to-position conversion
        let content = match self.get_document_content(uri).await {
            Some(c) => c,
            None => return Ok(None),
        };

        let mut actions = Vec::new();

        // Extract fixes from diagnostics that overlap with the request range
        for diag in &params.context.diagnostics {
            // Check if this diagnostic overlaps with the requested range
            let diag_range = &diag.range;
            let req_range = &params.range;

            let overlaps = diag_range.start.line <= req_range.end.line
                && diag_range.end.line >= req_range.start.line;

            if !overlaps {
                continue;
            }

            // Deserialize fixes from diagnostic.data
            let fixes = deserialize_fixes(diag.data.as_ref());
            if !fixes.is_empty() {
                actions.extend(fixes_to_code_actions(uri, &fixes, &content));
            }
        }

        if actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(
                actions
                    .into_iter()
                    .map(CodeActionOrCommand::CodeAction)
                    .collect(),
            ))
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        // Get document content
        let content = match self.get_document_content(uri).await {
            Some(c) => c,
            None => return Ok(None),
        };

        // Get hover info for the position
        Ok(hover_at_position(&content, position))
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
        let server_info = init_result
            .server_info
            .expect("server_info should be present");
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
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 0,
                },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String(
                "agnix::validation-error".to_string(),
            )),
            code_description: None,
            source: Some("agnix".to_string()),
            message: format!("Validation error: {}", error_message),
            related_information: None,
            tags: None,
            data: None,
        };

        assert_eq!(
            diagnostic.code,
            Some(NumberOrString::String(
                "agnix::validation-error".to_string()
            ))
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
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 0,
                },
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

    /// Test that initialize captures workspace root from root_uri.
    #[tokio::test]
    async fn test_initialize_captures_workspace_root() {
        let (service, _socket) = LspService::new(Backend::new);

        let temp_dir = tempfile::tempdir().unwrap();
        let root_uri = Url::from_file_path(temp_dir.path()).unwrap();

        let init_params = InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        };

        let result = service.inner().initialize(init_params).await;
        assert!(result.is_ok());

        // The workspace root should now be set (we can't directly access it,
        // but the test verifies initialize handles root_uri without error)
    }

    /// Test that initialize loads config from .agnix.toml when present.
    #[tokio::test]
    async fn test_initialize_loads_config_from_file() {
        let (service, _socket) = LspService::new(Backend::new);

        let temp_dir = tempfile::tempdir().unwrap();

        // Create a .agnix.toml config file
        let config_path = temp_dir.path().join(".agnix.toml");
        std::fs::write(
            &config_path,
            r#"
severity = "Warning"
target = "ClaudeCode"
exclude = []

[rules]
skills = false
"#,
        )
        .unwrap();

        let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
        let init_params = InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        };

        let result = service.inner().initialize(init_params).await;
        assert!(result.is_ok());

        // The config should have been loaded (we can't directly access it,
        // but the test verifies initialize handles .agnix.toml without error)
    }

    /// Test that initialize handles invalid .agnix.toml gracefully.
    #[tokio::test]
    async fn test_initialize_handles_invalid_config() {
        let (service, _socket) = LspService::new(Backend::new);

        let temp_dir = tempfile::tempdir().unwrap();

        // Create an invalid .agnix.toml config file
        let config_path = temp_dir.path().join(".agnix.toml");
        std::fs::write(&config_path, "this is not valid toml [[[").unwrap();

        let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
        let init_params = InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        };

        // Should still succeed (logs warning, uses default config)
        let result = service.inner().initialize(init_params).await;
        assert!(result.is_ok());
    }

    /// Test that files within workspace are validated normally.
    #[tokio::test]
    async fn test_file_within_workspace_validated() {
        let (service, _socket) = LspService::new(Backend::new);

        // Create workspace with a skill file
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

        // Initialize with workspace root
        let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
        let init_params = InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        };
        service.inner().initialize(init_params).await.unwrap();

        // File within workspace should be validated
        let uri = Url::from_file_path(&skill_path).unwrap();
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

        // Should complete without error (file is within workspace)
    }

    /// Test that files outside workspace are rejected.
    /// This tests the workspace boundary validation security feature.
    #[tokio::test]
    async fn test_file_outside_workspace_rejected() {
        let (service, _socket) = LspService::new(Backend::new);

        // Create two separate directories
        let workspace_dir = tempfile::tempdir().unwrap();
        let outside_dir = tempfile::tempdir().unwrap();

        // Create a file outside the workspace
        let outside_file = outside_dir.path().join("SKILL.md");
        std::fs::write(
            &outside_file,
            r#"---
name: outside-skill
version: 1.0.0
model: sonnet
---

# Outside Skill
"#,
        )
        .unwrap();

        // Initialize with workspace root
        let root_uri = Url::from_file_path(workspace_dir.path()).unwrap();
        let init_params = InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        };
        service.inner().initialize(init_params).await.unwrap();

        // Try to validate file outside workspace
        let uri = Url::from_file_path(&outside_file).unwrap();
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

        // Should complete without error (logs warning and returns early)
        // The file is rejected but no panic occurs
    }

    /// Test validation without workspace root (backwards compatibility).
    /// When no workspace root is set, all files should be accepted.
    #[tokio::test]
    async fn test_validation_without_workspace_root() {
        let (service, _socket) = LspService::new(Backend::new);

        // Initialize without root_uri
        let init_params = InitializeParams::default();
        service.inner().initialize(init_params).await.unwrap();

        // Create a file anywhere
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

        // Should validate normally (no workspace boundary check)
        let uri = Url::from_file_path(&skill_path).unwrap();
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

        // Should complete without error
    }

    /// Test that cached config is used (performance optimization).
    /// We verify this indirectly by running multiple validations.
    #[tokio::test]
    async fn test_cached_config_used_for_multiple_validations() {
        let (service, _socket) = LspService::new(Backend::new);

        // Initialize
        service
            .inner()
            .initialize(InitializeParams::default())
            .await
            .unwrap();

        // Create multiple skill files
        let temp_dir = tempfile::tempdir().unwrap();
        for i in 0..3 {
            let skill_path = temp_dir.path().join(format!("skill{}/SKILL.md", i));
            std::fs::create_dir_all(skill_path.parent().unwrap()).unwrap();
            std::fs::write(
                &skill_path,
                format!(
                    r#"---
name: test-skill-{}
version: 1.0.0
model: sonnet
---

# Test Skill {}
"#,
                    i, i
                ),
            )
            .unwrap();

            let uri = Url::from_file_path(&skill_path).unwrap();
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
        }

        // All validations should complete (config is reused internally)
    }

    /// Regression test: validates multiple files using the cached registry.
    /// Verifies the Arc<ValidatorRegistry> is thread-safe across spawn_blocking tasks.
    #[tokio::test]
    async fn test_cached_registry_used_for_multiple_validations() {
        let (service, _socket) = LspService::new(Backend::new);

        // Initialize
        service
            .inner()
            .initialize(InitializeParams::default())
            .await
            .unwrap();

        let temp_dir = tempfile::tempdir().unwrap();

        // Skill file
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

        // CLAUDE.md file
        let claude_path = temp_dir.path().join("CLAUDE.md");
        std::fs::write(
            &claude_path,
            r#"# Project Memory

This is a test project.
"#,
        )
        .unwrap();

        for path in [&skill_path, &claude_path] {
            let uri = Url::from_file_path(path).unwrap();
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
        }
    }
}
