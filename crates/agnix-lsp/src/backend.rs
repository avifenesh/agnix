//! LSP backend implementation for agnix.
//!
//! Implements the Language Server Protocol using tower-lsp, providing
//! real-time validation of agent configuration files.

use std::collections::{HashMap, HashSet};
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::code_actions::fixes_to_code_actions_with_diagnostic;
use crate::completion_provider::completion_items_for_document;
use crate::diagnostic_mapper::{deserialize_fixes, to_lsp_diagnostic, to_lsp_diagnostics};
use crate::hover_provider::hover_at_position;

mod events;
mod helpers;
mod revalidation;

use helpers::{create_error_diagnostic, normalize_path};
#[cfg(test)]
use revalidation::{
    MAX_CONFIG_REVALIDATION_CONCURRENCY, config_revalidation_concurrency, for_each_bounded,
};
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
#[derive(Clone)]
pub struct Backend {
    client: Client,
    /// Cached lint configuration reused across validations.
    /// Wrapped in RwLock to allow loading from .agnix.toml after initialize().
    config: Arc<RwLock<Arc<agnix_core::LintConfig>>>,
    /// Workspace root path for boundary validation (security).
    /// Set during initialize() from the client's root_uri.
    workspace_root: Arc<RwLock<Option<PathBuf>>>,
    /// Canonicalized workspace root cached at initialize() to avoid blocking I/O on hot paths.
    workspace_root_canonical: Arc<RwLock<Option<PathBuf>>>,
    documents: Arc<RwLock<HashMap<Url, Arc<String>>>>,
    /// Monotonic generation incremented on each config change.
    /// Used to drop stale diagnostics from older revalidation batches.
    config_generation: Arc<AtomicU64>,
    /// Monotonic generation incremented on each project validation.
    /// Used to drop stale project-level diagnostics from slower validation runs.
    project_validation_generation: Arc<AtomicU64>,
    /// Cached validator registry reused across validations.
    /// Immutable after construction; Arc enables sharing across spawn_blocking tasks.
    registry: Arc<agnix_core::ValidatorRegistry>,
    /// Cached project-level diagnostics per URI (from validate_project_rules).
    /// Stored separately so they can be merged with per-file diagnostics at publish time.
    project_level_diagnostics: Arc<RwLock<HashMap<Url, Vec<Diagnostic>>>>,
    /// Tracks which URIs received project-level diagnostics so stale ones can be cleared.
    project_diagnostics_uris: Arc<RwLock<HashSet<Url>>>,
}

impl Backend {
    /// Create a new backend instance with the given client connection.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            config: Arc::new(RwLock::new(Arc::new(agnix_core::LintConfig::default()))),
            workspace_root: Arc::new(RwLock::new(None)),
            workspace_root_canonical: Arc::new(RwLock::new(None)),
            documents: Arc::new(RwLock::new(HashMap::new())),
            config_generation: Arc::new(AtomicU64::new(0)),
            project_validation_generation: Arc::new(AtomicU64::new(0)),
            registry: Arc::new(agnix_core::ValidatorRegistry::with_defaults()),
            project_level_diagnostics: Arc::new(RwLock::new(HashMap::new())),
            project_diagnostics_uris: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Spawn project-level validation in a background task.
    ///
    /// Logs a warning if the spawned task panics, preventing silent failures.
    fn spawn_project_validation(&self) {
        let backend = self.clone();
        let client = self.client.clone();
        tokio::spawn(async move {
            let result = tokio::spawn(async move {
                backend.validate_project_rules_and_publish().await;
            })
            .await;
            if let Err(e) = result {
                client
                    .log_message(
                        MessageType::ERROR,
                        format!("Project-level validation task panicked: {}", e),
                    )
                    .await;
            }
        });
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
    async fn validate_from_content_and_publish(
        &self,
        uri: Url,
        expected_config_generation: Option<u64>,
    ) {
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
            let (canonical_path, canonical_root) = match file_path.canonicalize() {
                Ok(path) => {
                    let root = self
                        .workspace_root_canonical
                        .read()
                        .await
                        .clone()
                        .unwrap_or_else(|| normalize_path(workspace_root));
                    (path, root)
                }
                Err(_) => (normalize_path(&file_path), normalize_path(workspace_root)),
            };

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
        let (content, expected_content) = {
            let docs = self.documents.read().await;
            match docs.get(&uri) {
                Some(cached) => {
                    let snapshot = Arc::clone(cached);
                    (Arc::clone(&snapshot), Some(snapshot))
                }
                None => {
                    // Fall back to file-based validation
                    drop(docs);
                    let diagnostics = self.validate_file(file_path).await;
                    if !self
                        .should_publish_diagnostics(&uri, expected_config_generation, None)
                        .await
                    {
                        return;
                    }
                    self.client
                        .publish_diagnostics(uri, diagnostics, None)
                        .await;
                    return;
                }
            }
        };

        let config = Arc::clone(&*self.config.read().await);
        let registry = Arc::clone(&self.registry);
        let result = tokio::task::spawn_blocking(move || {
            let file_type = agnix_core::resolve_file_type(&file_path, &config);
            if file_type == agnix_core::FileType::Unknown {
                return Ok(vec![]);
            }

            let validators = registry.validators_for(file_type);
            let mut diagnostics = Vec::new();

            for validator in validators {
                diagnostics.extend(validator.validate(&file_path, content.as_str(), &config));
            }

            Ok::<_, agnix_core::LintError>(diagnostics)
        })
        .await;

        let mut diagnostics = match result {
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

        // Merge cached project-level diagnostics for this URI (AGM-006, XP-004/005/006, VER-001)
        {
            let proj_diags = self.project_level_diagnostics.read().await;
            if let Some(project_diags) = proj_diags.get(&uri) {
                diagnostics.extend(project_diags.iter().cloned());
            }
        }

        if !self
            .should_publish_diagnostics(&uri, expected_config_generation, expected_content.as_ref())
            .await
        {
            return;
        }

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // Capture workspace root for path boundary validation
        if let Some(root_uri) = params.root_uri {
            if let Ok(root_path) = root_uri.to_file_path() {
                *self.workspace_root.write().await = Some(root_path.clone());
                *self.workspace_root_canonical.write().await = Some(
                    root_path
                        .canonicalize()
                        .unwrap_or_else(|_| normalize_path(&root_path)),
                );

                // Try to load config from .agnix.toml in workspace root
                let config_path = root_path.join(".agnix.toml");
                if config_path.exists() {
                    match agnix_core::LintConfig::load(&config_path) {
                        Ok(loaded_config) => {
                            // Apply config-specified locale if present
                            if let Some(config_locale) = loaded_config.locale() {
                                crate::locale::init_from_config(config_locale);
                            }
                            let mut config_with_root = loaded_config;
                            config_with_root.set_root_dir(root_path.clone());
                            *self.config.write().await = Arc::new(config_with_root);
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
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),
                        ..Default::default()
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![":".to_string(), "\"".to_string()]),
                    ..Default::default()
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["agnix.validateProjectRules".to_string()],
                    ..Default::default()
                }),
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

        // Run project-level validation on workspace open
        self.spawn_project_validation();
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.handle_did_open(params).await;
    }
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.handle_did_change(params).await;
    }
    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.handle_did_save(params).await;
    }
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.handle_did_close(params).await;
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
                actions.extend(fixes_to_code_actions_with_diagnostic(
                    uri,
                    &fixes,
                    content.as_str(),
                    diag,
                ));
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

        let config = self.config.read().await;
        let file_type = uri
            .to_file_path()
            .ok()
            .map(|path| agnix_core::resolve_file_type(&path, &config))
            .unwrap_or(agnix_core::FileType::Unknown);
        if matches!(file_type, agnix_core::FileType::Unknown) {
            return Ok(None);
        }

        // Get hover info for the position
        Ok(hover_at_position(file_type, content.as_str(), position))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let path = match uri.to_file_path() {
            Ok(path) => path,
            Err(_) => return Ok(None),
        };

        let content = match self.get_document_content(uri).await {
            Some(c) => c,
            None => return Ok(None),
        };

        let config = self.config.read().await;
        let items = completion_items_for_document(&path, content.as_str(), position, &config);
        if items.is_empty() {
            Ok(None)
        } else {
            Ok(Some(CompletionResponse::Array(items)))
        }
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        self.handle_did_change_configuration(params).await;
    }
    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> Result<Option<serde_json::Value>> {
        match params.command.as_str() {
            "agnix.validateProjectRules" => {
                self.client
                    .log_message(
                        MessageType::INFO,
                        "Running project-level validation (via executeCommand)",
                    )
                    .await;
                self.validate_project_rules_and_publish().await;
                Ok(None)
            }
            _ => {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        format!("Unknown command: {}", params.command),
                    )
                    .await;
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod tests;
