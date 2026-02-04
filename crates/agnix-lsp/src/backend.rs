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
