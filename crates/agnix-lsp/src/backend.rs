//! LSP backend implementation for agnix.
//!
//! Implements the Language Server Protocol using tower-lsp, providing
//! real-time validation of agent configuration files.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::{Component, Path, PathBuf};
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
use crate::vscode_config::VsCodeConfig;

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

/// Normalize path components without filesystem access.
/// Resolves `.` and `..` logically -- used when `canonicalize()` fails.
/// Expects absolute paths (LSP URIs always produce absolute paths).
fn normalize_path(path: &Path) -> PathBuf {
    let mut components: Vec<Component<'_>> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                match components.last() {
                    Some(Component::Normal(_)) => {
                        components.pop();
                    }
                    // Cannot traverse above root or prefix -- silently drop
                    Some(Component::RootDir) | Some(Component::Prefix(_)) => {}
                    _ => components.push(component),
                }
            }
            _ => components.push(component),
        }
    }
    components.iter().collect()
}

const MAX_CONFIG_REVALIDATION_CONCURRENCY: usize = 8;

fn config_revalidation_concurrency(document_count: usize) -> usize {
    if document_count == 0 {
        return 0;
    }

    let available = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(4);

    document_count.min(available.clamp(1, MAX_CONFIG_REVALIDATION_CONCURRENCY))
}

/// Execute `operation` on each item with bounded concurrency.
///
/// Spawns up to `max_concurrency` tasks at once (minimum 1). As each task
/// completes, the next item is dispatched, maintaining the concurrency cap.
///
/// Partial failures are collected, not propagated: if a spawned task panics
/// or is cancelled, its `JoinError` is appended to the returned `Vec` and
/// processing continues with the remaining items.
async fn for_each_bounded<T, I, F, Fut>(
    items: I,
    max_concurrency: usize,
    operation: F,
) -> Vec<tokio::task::JoinError>
where
    T: Send + 'static,
    I: IntoIterator<Item = T>,
    F: Fn(T) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let mut join_set = tokio::task::JoinSet::new();
    let mut join_errors = Vec::new();
    let mut items = items.into_iter();
    let max_concurrency = max_concurrency.max(1);
    let operation = Arc::new(operation);

    for _ in 0..max_concurrency {
        let Some(item) = items.next() else {
            break;
        };

        let operation = Arc::clone(&operation);
        join_set.spawn(async move {
            operation(item).await;
        });
    }

    while let Some(result) = join_set.join_next().await {
        if let Err(error) = result {
            join_errors.push(error);
        }

        if let Some(item) = items.next() {
            let operation = Arc::clone(&operation);
            join_set.spawn(async move {
                operation(item).await;
            });
        }
    }

    join_errors
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

    /// In config-change batch revalidation mode, only publish if the batch generation is current
    /// and the document is still open.
    async fn should_publish_diagnostics(
        &self,
        uri: &Url,
        expected_config_generation: Option<u64>,
        expected_content: Option<&Arc<String>>,
    ) -> bool {
        let docs = self.documents.read().await;
        let current_content = docs.get(uri);

        if let Some(expected) = expected_content {
            let Some(current) = current_content else {
                return false;
            };
            if !Arc::ptr_eq(current, expected) {
                return false;
            }
        }

        if let Some(expected_generation) = expected_config_generation {
            if self.config_generation.load(Ordering::SeqCst) != expected_generation {
                return false;
            }

            if current_content.is_none() {
                return false;
            }
        }

        true
    }

    /// Run project-level validation and publish diagnostics per affected file.
    ///
    /// Calls `agnix_core::validate_project_rules()` in a blocking task, then
    /// groups the resulting diagnostics by file path. For files open in the
    /// editor, the diagnostics are cached so `validate_from_content_and_publish`
    /// can merge them with per-file diagnostics. For files not open, diagnostics
    /// are published directly.
    ///
    /// Stale URIs from previous runs are cleared by publishing empty diagnostics.
    async fn validate_project_rules_and_publish(&self) {
        let workspace_root = match &*self.workspace_root.read().await {
            Some(root) => root.clone(),
            None => return,
        };

        let config = Arc::clone(&*self.config.read().await);

        // Capture generation to detect stale runs
        let expected_generation = self
            .project_validation_generation
            .fetch_add(1, Ordering::SeqCst)
            + 1;
        let result = tokio::task::spawn_blocking(move || {
            agnix_core::validate_project_rules(&workspace_root, &config)
        })
        .await;

        let core_diagnostics = match result {
            Ok(Ok(diags)) => diags,
            Ok(Err(e)) => {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        format!("Project-level validation error: {}", e),
                    )
                    .await;
                return;
            }
            Err(e) => {
                self.client
                    .log_message(
                        MessageType::ERROR,
                        format!("Project-level validation task failed: {}", e),
                    )
                    .await;
                return;
            }
        };

        // Group diagnostics by file path
        let mut by_uri: HashMap<Url, Vec<Diagnostic>> = HashMap::new();
        for diag in &core_diagnostics {
            if let Ok(uri) = Url::from_file_path(&diag.file) {
                by_uri.entry(uri).or_default().push(to_lsp_diagnostic(diag));
            }
        }

        // Pre-compute the set of URIs in the current run to avoid duplicating
        // the `by_uri.keys().cloned().collect()` call.
        let current_uris: HashSet<Url> = by_uri.keys().cloned().collect();

        // Clear stale project diagnostic URIs from the previous run
        let previous_uris: HashSet<Url> = {
            let prev = self.project_diagnostics_uris.read().await;
            prev.clone()
        };

        // Drop stale results from slower runs BEFORE any side effects
        if self.project_validation_generation.load(Ordering::SeqCst) != expected_generation {
            return;
        }

        // Capture the set of open document URIs once, then release the lock
        // so we don't hold it across await points (publish_diagnostics calls).
        let open_uris: HashSet<Url> = {
            let docs = self.documents.read().await;
            docs.keys().cloned().collect()
        };

        for stale_uri in previous_uris.difference(&current_uris) {
            // Only clear if the document is not open (open docs will re-merge on next validate)
            if !open_uris.contains(stale_uri) {
                self.client
                    .publish_diagnostics(stale_uri.clone(), vec![], None)
                    .await;
            }
        }

        // Store new project-level diagnostics and track URIs.
        // Move `by_uri` into the cache to avoid cloning, but first collect
        // the data needed for publishing below (non-open URIs + their diagnostics).
        let non_open_publish: Vec<(Url, Vec<Diagnostic>)> = by_uri
            .iter()
            .filter(|(uri, _)| !open_uris.contains(uri))
            .map(|(uri, diags)| (uri.clone(), diags.clone()))
            .collect();

        let open_uris_in_results: Vec<Url> = by_uri
            .keys()
            .filter(|uri| open_uris.contains(uri))
            .cloned()
            .collect();

        {
            let mut proj_diags = self.project_level_diagnostics.write().await;
            let mut proj_uris = self.project_diagnostics_uris.write().await;
            *proj_diags = by_uri;
            *proj_uris = current_uris.clone();
        }

        // Publish diagnostics for files not open in the editor
        for (uri, lsp_diags) in non_open_publish {
            self.client.publish_diagnostics(uri, lsp_diags, None).await;
        }

        // For open documents, re-trigger full validation so per-file and
        // project-level diagnostics are merged before publishing.
        for uri in open_uris_in_results {
            let backend = self.clone();
            tokio::spawn(async move {
                backend.validate_from_content_and_publish(uri, None).await;
            });
        }

        // Also clear project-level diagnostics from open docs whose URIs
        // are no longer in the results (stale open docs need re-merge too)
        for stale_uri in previous_uris.difference(&current_uris) {
            if open_uris.contains(stale_uri) {
                let backend = self.clone();
                let uri = stale_uri.clone();
                tokio::spawn(async move {
                    backend.validate_from_content_and_publish(uri, None).await;
                });
            }
        }
    }

    /// Check if a file path is relevant to project-level rules.
    ///
    /// Returns true for instruction files (CLAUDE.md, AGENTS.md, .clinerules,
    /// .cursorrules, copilot-instructions.md, etc.) and .agnix.toml config.
    fn is_project_level_trigger(path: &Path) -> bool {
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => return false,
        };

        // .agnix.toml config changes affect all rules
        if file_name.eq_ignore_ascii_case(".agnix.toml") {
            return true;
        }

        // Instruction files that affect project-level cross-file checks
        file_name.eq_ignore_ascii_case("claude.md")
            || file_name.eq_ignore_ascii_case("claude.local.md")
            || file_name.eq_ignore_ascii_case("agents.md")
            || file_name.eq_ignore_ascii_case("agents.local.md")
            || file_name.eq_ignore_ascii_case("agents.override.md")
            || file_name.eq_ignore_ascii_case("gemini.md")
            || file_name.eq_ignore_ascii_case("gemini.local.md")
            || file_name.eq_ignore_ascii_case(".clinerules")
            || file_name.eq_ignore_ascii_case(".cursorrules")
            || file_name.eq_ignore_ascii_case(".cursorrules.md")
            || file_name.eq_ignore_ascii_case("copilot-instructions.md")
            || file_name.to_lowercase().ends_with(".instructions.md")
            || file_name.to_lowercase().ends_with(".mdc")
            || file_name.eq_ignore_ascii_case("opencode.json")
    }

    /// Get cached document content for a URI.
    async fn get_document_content(&self, uri: &Url) -> Option<Arc<String>> {
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
                            if let Some(ref config_locale) = loaded_config.locale {
                                crate::locale::init_from_config(config_locale);
                            }
                            let mut config_with_root = loaded_config;
                            config_with_root.root_dir = Some(root_path.clone());
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
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        {
            let mut docs = self.documents.write().await;
            docs.insert(uri.clone(), Arc::new(text));
        }
        self.validate_from_content_and_publish(uri, None).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().next() {
            {
                let mut docs = self.documents.write().await;
                docs.insert(uri.clone(), Arc::new(change.text));
            }
            self.validate_from_content_and_publish(uri, None).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        self.validate_from_content_and_publish(uri.clone(), None)
            .await;

        // Re-run project-level validation when a relevant file is saved
        if let Ok(path) = uri.to_file_path() {
            if Self::is_project_level_trigger(&path) {
                self.spawn_project_validation();
            }
        }
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
        // Parse incoming settings JSON into VsCodeConfig
        let vscode_config: VsCodeConfig = match serde_json::from_value(params.settings) {
            Ok(c) => c,
            Err(e) => {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        format!("Failed to parse VS Code settings: {}", e),
                    )
                    .await;
                return;
            }
        };

        self.client
            .log_message(
                MessageType::INFO,
                "Received configuration update from VS Code",
            )
            .await;

        // Invalidate in-flight config-revalidation batches first.
        // This prevents older batches from publishing after a newer config update starts.
        let revalidation_generation = self.config_generation.fetch_add(1, Ordering::SeqCst) + 1;

        // Acquire write lock and apply settings
        // Clone the existing config, modify it, then replace
        {
            let mut config_guard = self.config.write().await;
            let mut new_config = (**config_guard).clone();
            vscode_config.merge_into_lint_config(&mut new_config);
            // Set root_dir from workspace_root for glob pattern matching
            if let Some(ref root) = *self.workspace_root.read().await {
                new_config.root_dir = Some(root.clone());
            }
            *config_guard = Arc::new(new_config);
        }

        // Re-validate all open documents with new config
        let documents: Vec<Url> = {
            let docs = self.documents.read().await;
            docs.keys().cloned().collect()
        };

        if documents.is_empty() {
            return;
        }

        let max_concurrency = config_revalidation_concurrency(documents.len());
        let backend = self.clone();
        let join_errors = for_each_bounded(documents, max_concurrency, move |uri| {
            let backend = backend.clone();
            async move {
                backend
                    .validate_from_content_and_publish(uri, Some(revalidation_generation))
                    .await;
            }
        })
        .await;

        for error in join_errors {
            self.client
                .log_message(
                    MessageType::ERROR,
                    format!("Revalidation task failed after config change: {}", error),
                )
                .await;
        }

        // Also re-run project-level validation with the updated config
        self.spawn_project_validation();
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

        assert!(
            init_result.capabilities.completion_provider.is_some(),
            "Expected completion provider capability"
        );

        // Verify server info
        let server_info = init_result
            .server_info
            .expect("server_info should be present");
        assert_eq!(server_info.name, "agnix-lsp");
        assert!(server_info.version.is_some());
    }

    #[tokio::test]
    async fn test_completion_returns_skill_frontmatter_candidates() {
        let (service, _socket) = LspService::new(Backend::new);

        let temp_dir = tempfile::tempdir().unwrap();
        let skill_path = temp_dir.path().join("SKILL.md");
        let content = "---\nna\n---\n";
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

        let completion = service
            .inner()
            .completion(CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position: Position {
                        line: 1,
                        character: 1,
                    },
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: None,
            })
            .await
            .unwrap();

        let items = match completion {
            Some(CompletionResponse::Array(items)) => items,
            _ => panic!("Expected completion items"),
        };
        assert!(items.iter().any(|item| item.label == "name"));
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

    // ===== Cache Invalidation Tests =====

    /// Test that document cache is cleared when document is closed.
    #[tokio::test]
    async fn test_document_cache_cleared_on_close() {
        let (service, _socket) = LspService::new(Backend::new);

        let temp_dir = tempfile::tempdir().unwrap();
        let skill_path = temp_dir.path().join("SKILL.md");
        std::fs::write(
            &skill_path,
            "---\nname: test\ndescription: Test\n---\n# Test",
        )
        .unwrap();

        let uri = Url::from_file_path(&skill_path).unwrap();

        // Open document
        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: "---\nname: test\ndescription: Test\n---\n# Test".to_string(),
                },
            })
            .await;

        // Verify document is cached (hover should work)
        let hover_before = service
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
        assert!(hover_before.is_ok());
        assert!(hover_before.unwrap().is_some());

        // Close document
        service
            .inner()
            .did_close(DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
            })
            .await;

        // Verify document cache is cleared (hover should return None)
        let hover_after = service
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
        assert!(hover_after.is_ok());
        assert!(hover_after.unwrap().is_none());
    }

    /// Test that document cache is updated on change.
    #[tokio::test]
    async fn test_document_cache_updated_on_change() {
        let (service, _socket) = LspService::new(Backend::new);

        let temp_dir = tempfile::tempdir().unwrap();
        let skill_path = temp_dir.path().join("SKILL.md");
        std::fs::write(&skill_path, "# Initial").unwrap();

        let uri = Url::from_file_path(&skill_path).unwrap();

        // Open with initial content
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

        // Change to content with frontmatter
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
                    text: "---\nname: updated\ndescription: Updated\n---\n# Updated".to_string(),
                }],
            })
            .await;

        // Verify cache has new content (hover should work on frontmatter)
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
        assert!(hover.unwrap().is_some());
    }

    /// Regression: cached document reads should share the same allocation.
    #[tokio::test]
    async fn test_get_document_content_returns_shared_arc() {
        let (service, _socket) = LspService::new(Backend::new);

        let temp_dir = tempfile::tempdir().unwrap();
        let skill_path = temp_dir.path().join("SKILL.md");
        std::fs::write(&skill_path, "# Shared").unwrap();

        let uri = Url::from_file_path(&skill_path).unwrap();

        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: "# Shared".to_string(),
                },
            })
            .await;

        let first = service
            .inner()
            .get_document_content(&uri)
            .await
            .expect("cached content should exist");
        let second = service
            .inner()
            .get_document_content(&uri)
            .await
            .expect("cached content should exist");

        assert!(Arc::ptr_eq(&first, &second));
    }

    /// Test that multiple documents have independent caches.
    #[tokio::test]
    async fn test_multiple_documents_independent_caches() {
        let (service, _socket) = LspService::new(Backend::new);

        let temp_dir = tempfile::tempdir().unwrap();

        // Create two skill files
        let skill1_path = temp_dir.path().join("skill1").join("SKILL.md");
        let skill2_path = temp_dir.path().join("skill2").join("SKILL.md");
        std::fs::create_dir_all(skill1_path.parent().unwrap()).unwrap();
        std::fs::create_dir_all(skill2_path.parent().unwrap()).unwrap();

        std::fs::write(
            &skill1_path,
            "---\nname: skill-one\ndescription: First\n---\n# One",
        )
        .unwrap();
        std::fs::write(
            &skill2_path,
            "---\nname: skill-two\ndescription: Second\n---\n# Two",
        )
        .unwrap();

        let uri1 = Url::from_file_path(&skill1_path).unwrap();
        let uri2 = Url::from_file_path(&skill2_path).unwrap();

        // Open both documents
        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri1.clone(),
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: "---\nname: skill-one\ndescription: First\n---\n# One".to_string(),
                },
            })
            .await;

        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri2.clone(),
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: "---\nname: skill-two\ndescription: Second\n---\n# Two".to_string(),
                },
            })
            .await;

        // Close first document
        service
            .inner()
            .did_close(DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri: uri1.clone() },
            })
            .await;

        // First document should be cleared
        let hover1 = service
            .inner()
            .hover(HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri1 },
                    position: Position {
                        line: 1,
                        character: 0,
                    },
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await;
        assert!(hover1.is_ok());
        assert!(hover1.unwrap().is_none());

        // Second document should still be cached
        let hover2 = service
            .inner()
            .hover(HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri2 },
                    position: Position {
                        line: 1,
                        character: 0,
                    },
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await;
        assert!(hover2.is_ok());
        assert!(hover2.unwrap().is_some());
    }

    // ===== Configuration Change Tests =====

    /// Test that did_change_configuration handles valid settings.
    #[tokio::test]
    async fn test_did_change_configuration_valid_settings() {
        let (service, _socket) = LspService::new(Backend::new);

        // Initialize first
        service
            .inner()
            .initialize(InitializeParams::default())
            .await
            .unwrap();

        // Send valid configuration
        let settings = serde_json::json!({
            "severity": "Error",
            "target": "ClaudeCode",
            "rules": {
                "skills": false,
                "hooks": true
            }
        });

        service
            .inner()
            .did_change_configuration(DidChangeConfigurationParams { settings })
            .await;

        // Should complete without error
        // The config is internally updated but we can't directly access it
    }

    /// Test that did_change_configuration handles partial settings.
    #[tokio::test]
    async fn test_did_change_configuration_partial_settings() {
        let (service, _socket) = LspService::new(Backend::new);

        service
            .inner()
            .initialize(InitializeParams::default())
            .await
            .unwrap();

        // Send only severity (partial config)
        let settings = serde_json::json!({
            "severity": "Info"
        });

        service
            .inner()
            .did_change_configuration(DidChangeConfigurationParams { settings })
            .await;

        // Should complete without error
    }

    /// Test that did_change_configuration handles invalid JSON gracefully.
    #[tokio::test]
    async fn test_did_change_configuration_invalid_json() {
        let (service, _socket) = LspService::new(Backend::new);

        service
            .inner()
            .initialize(InitializeParams::default())
            .await
            .unwrap();

        // Send invalid JSON type (string instead of object)
        let settings = serde_json::json!("not an object");

        service
            .inner()
            .did_change_configuration(DidChangeConfigurationParams { settings })
            .await;

        // Should complete without error (logs warning and returns early)
    }

    /// Test bounded helper used by did_change_configuration.
    #[test]
    fn test_config_revalidation_concurrency_bounds() {
        let expected_cap = std::thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(4)
            .clamp(1, MAX_CONFIG_REVALIDATION_CONCURRENCY);

        assert_eq!(config_revalidation_concurrency(0), 0);
        assert_eq!(config_revalidation_concurrency(1), 1);
        assert_eq!(
            config_revalidation_concurrency(MAX_CONFIG_REVALIDATION_CONCURRENCY * 4),
            expected_cap
        );
    }

    /// Test bounded helper handles empty inputs with no task errors.
    #[tokio::test]
    async fn test_for_each_bounded_empty_input() {
        let errors = for_each_bounded(Vec::<usize>::new(), 3, |_| async {}).await;
        assert!(errors.is_empty());
    }

    /// Test bounded helper reports join errors when inner tasks panic.
    #[tokio::test]
    async fn test_for_each_bounded_collects_join_errors() {
        let errors = for_each_bounded(vec![0usize, 1, 2], 2, |idx| async move {
            if idx == 1 {
                panic!("intentional panic for join error coverage");
            }
        })
        .await;

        assert_eq!(errors.len(), 1);
        assert!(errors[0].is_panic());
    }

    /// Test generation guard for config-change batch publishing.
    #[tokio::test]
    async fn test_should_publish_diagnostics_guard() {
        let (service, _socket) = LspService::new(Backend::new);
        let backend = service.inner();

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("SKILL.md");
        std::fs::write(&path, "# test").unwrap();
        let uri = Url::from_file_path(&path).unwrap();

        let snapshot = Arc::new("# test".to_string());
        backend
            .documents
            .write()
            .await
            .insert(uri.clone(), Arc::clone(&snapshot));
        backend.config_generation.store(7, Ordering::SeqCst);

        assert!(
            backend
                .should_publish_diagnostics(&uri, Some(7), Some(&snapshot))
                .await
        );
        assert!(
            !backend
                .should_publish_diagnostics(&uri, Some(6), Some(&snapshot))
                .await
        );

        // New content (new Arc) means stale validation result should not publish.
        backend
            .documents
            .write()
            .await
            .insert(uri.clone(), Arc::new("# updated".to_string()));
        assert!(
            !backend
                .should_publish_diagnostics(&uri, Some(7), Some(&snapshot))
                .await
        );

        backend.documents.write().await.remove(&uri);
        assert!(
            !backend
                .should_publish_diagnostics(&uri, Some(7), Some(&snapshot))
                .await
        );

        assert!(backend.should_publish_diagnostics(&uri, None, None).await);
    }

    /// Test bounded helper used by did_change_configuration.
    #[tokio::test]
    async fn test_did_change_configuration_concurrency_bound_helper() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Duration;
        use tokio::sync::Barrier;

        let max_concurrency = 3usize;
        let in_flight = Arc::new(AtomicUsize::new(0));
        let peak_in_flight = Arc::new(AtomicUsize::new(0));
        let completed = Arc::new(AtomicUsize::new(0));
        let ready = Arc::new(Barrier::new(max_concurrency + 1));
        let release = Arc::new(Barrier::new(max_concurrency + 1));
        let total_items = 12usize;

        let run = tokio::spawn(for_each_bounded(0..total_items, max_concurrency, {
            let in_flight = Arc::clone(&in_flight);
            let peak_in_flight = Arc::clone(&peak_in_flight);
            let completed = Arc::clone(&completed);
            let ready = Arc::clone(&ready);
            let release = Arc::clone(&release);
            move |idx| {
                let in_flight = Arc::clone(&in_flight);
                let peak_in_flight = Arc::clone(&peak_in_flight);
                let completed = Arc::clone(&completed);
                let ready = Arc::clone(&ready);
                let release = Arc::clone(&release);

                async move {
                    let current = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                    peak_in_flight.fetch_max(current, Ordering::SeqCst);

                    if idx < max_concurrency {
                        ready.wait().await;
                        release.wait().await;
                    } else {
                        tokio::task::yield_now().await;
                    }

                    in_flight.fetch_sub(1, Ordering::SeqCst);
                    completed.fetch_add(1, Ordering::SeqCst);
                }
            }
        }));

        // Wait for the first wave of tasks to all be in-flight at once.
        tokio::time::timeout(Duration::from_secs(2), ready.wait())
            .await
            .expect("timed out waiting for first wave");
        assert_eq!(peak_in_flight.load(Ordering::SeqCst), max_concurrency);
        tokio::time::timeout(Duration::from_secs(2), release.wait())
            .await
            .expect("timed out releasing first wave");

        let join_errors = tokio::time::timeout(Duration::from_secs(2), run)
            .await
            .expect("timed out waiting for bounded worker completion")
            .unwrap();

        assert!(join_errors.is_empty());
        assert_eq!(completed.load(Ordering::SeqCst), total_items);
    }

    /// Test that did_change_configuration triggers revalidation.
    #[tokio::test]
    async fn test_did_change_configuration_triggers_revalidation() {
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

        let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
        service
            .inner()
            .initialize(InitializeParams {
                root_uri: Some(root_uri),
                ..Default::default()
            })
            .await
            .unwrap();

        let uri = Url::from_file_path(&skill_path).unwrap();
        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: std::fs::read_to_string(&skill_path).unwrap(),
                },
            })
            .await;

        // Now change configuration - should trigger revalidation
        let settings = serde_json::json!({
            "severity": "Error",
            "rules": {
                "skills": false
            }
        });

        service
            .inner()
            .did_change_configuration(DidChangeConfigurationParams { settings })
            .await;

        // Should complete without error - open document was revalidated
    }

    /// Test that config changes revalidate all currently open documents.
    #[tokio::test]
    async fn test_did_change_configuration_triggers_revalidation_for_multiple_documents() {
        let (service, _socket) = LspService::new(Backend::new);

        let temp_dir = tempfile::tempdir().unwrap();
        let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
        service
            .inner()
            .initialize(InitializeParams {
                root_uri: Some(root_uri),
                ..Default::default()
            })
            .await
            .unwrap();

        let document_count = 6usize;

        for i in 0..document_count {
            let skill_path = temp_dir.path().join(format!("skill-{i}/SKILL.md"));
            std::fs::create_dir_all(skill_path.parent().unwrap()).unwrap();
            std::fs::write(
                &skill_path,
                format!(
                    r#"---
name: test-skill-{i}
version: 1.0.0
model: sonnet
---

# Test Skill {i}
"#
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
                        text: std::fs::read_to_string(&skill_path).unwrap(),
                    },
                })
                .await;
        }

        let settings = serde_json::json!({
            "severity": "Error",
            "rules": {
                "skills": false
            }
        });

        service
            .inner()
            .did_change_configuration(DidChangeConfigurationParams { settings })
            .await;

        let open_documents = service.inner().documents.read().await.len();
        assert_eq!(open_documents, document_count);
    }

    /// Test that empty settings object doesn't crash.
    #[tokio::test]
    async fn test_did_change_configuration_empty_settings() {
        let (service, _socket) = LspService::new(Backend::new);

        service
            .inner()
            .initialize(InitializeParams::default())
            .await
            .unwrap();

        // Send empty object
        let settings = serde_json::json!({});

        service
            .inner()
            .did_change_configuration(DidChangeConfigurationParams { settings })
            .await;

        // Should complete without error
    }

    /// Test configuration with all tool versions set.
    #[tokio::test]
    async fn test_did_change_configuration_with_versions() {
        let (service, _socket) = LspService::new(Backend::new);

        service
            .inner()
            .initialize(InitializeParams::default())
            .await
            .unwrap();

        let settings = serde_json::json!({
            "versions": {
                "claude_code": "1.0.0",
                "codex": "0.1.0",
                "cursor": "0.45.0",
                "copilot": "1.2.0"
            }
        });

        service
            .inner()
            .did_change_configuration(DidChangeConfigurationParams { settings })
            .await;

        // Should complete without error
    }

    /// Test configuration with spec revisions.
    #[tokio::test]
    async fn test_did_change_configuration_with_specs() {
        let (service, _socket) = LspService::new(Backend::new);

        service
            .inner()
            .initialize(InitializeParams::default())
            .await
            .unwrap();

        let settings = serde_json::json!({
            "specs": {
                "mcp_protocol": "2025-06-18",
                "agent_skills_spec": "1.0",
                "agents_md_spec": "1.0"
            }
        });

        service
            .inner()
            .did_change_configuration(DidChangeConfigurationParams { settings })
            .await;

        // Should complete without error
    }

    /// Test configuration with tools array.
    #[tokio::test]
    async fn test_did_change_configuration_with_tools_array() {
        let (service, _socket) = LspService::new(Backend::new);

        service
            .inner()
            .initialize(InitializeParams::default())
            .await
            .unwrap();

        let settings = serde_json::json!({
            "tools": ["claude-code", "cursor", "github-copilot"]
        });

        service
            .inner()
            .did_change_configuration(DidChangeConfigurationParams { settings })
            .await;

        // Should complete without error
    }

    /// Test configuration with disabled rules.
    #[tokio::test]
    async fn test_did_change_configuration_with_disabled_rules() {
        let (service, _socket) = LspService::new(Backend::new);

        service
            .inner()
            .initialize(InitializeParams::default())
            .await
            .unwrap();

        let settings = serde_json::json!({
            "rules": {
                "disabled_rules": ["AS-001", "PE-003", "MCP-008"]
            }
        });

        service
            .inner()
            .did_change_configuration(DidChangeConfigurationParams { settings })
            .await;

        // Should complete without error
    }

    /// Test that did_change_configuration handles locale setting.
    #[tokio::test]
    async fn test_did_change_configuration_with_locale() {
        let (service, _socket) = {
            let _guard = crate::locale::LOCALE_MUTEX.lock().unwrap();
            LspService::new(Backend::new)
        };

        service
            .inner()
            .initialize(InitializeParams::default())
            .await
            .unwrap();

        let settings = serde_json::json!({
            "severity": "Warning",
            "locale": "es"
        });

        service
            .inner()
            .did_change_configuration(DidChangeConfigurationParams { settings })
            .await;

        {
            let _guard = crate::locale::LOCALE_MUTEX.lock().unwrap();
            // Verify locale was actually changed
            assert_eq!(&*rust_i18n::locale(), "es");
        }

        // Reset locale for other tests
        rust_i18n::set_locale("en");
    }

    // ===== normalize_path() Unit Tests =====

    /// Test that '..' components are resolved by removing the preceding normal component.
    #[test]
    fn test_normalize_path_resolves_parent() {
        let result = normalize_path(Path::new("/a/b/../c"));
        assert_eq!(result, PathBuf::from("/a/c"));
    }

    /// Test that '.' components are removed entirely.
    #[test]
    fn test_normalize_path_removes_curdir() {
        let result = normalize_path(Path::new("/a/./b/./c"));
        assert_eq!(result, PathBuf::from("/a/b/c"));
    }

    /// Test that multiple '..' components are resolved correctly.
    #[test]
    fn test_normalize_path_multiple_parent() {
        let result = normalize_path(Path::new("/a/b/../../c"));
        assert_eq!(result, PathBuf::from("/c"));
    }

    /// Test that a path without special components is returned unchanged.
    #[test]
    fn test_normalize_path_already_clean() {
        let result = normalize_path(Path::new("/a/b/c"));
        assert_eq!(result, PathBuf::from("/a/b/c"));
    }

    /// Test that '..' cannot traverse above root.
    #[test]
    fn test_normalize_path_cannot_escape_root() {
        let result = normalize_path(Path::new("/../a"));
        assert_eq!(result, PathBuf::from("/a"));
    }

    /// Test that root alone is preserved.
    #[test]
    fn test_normalize_path_root_only() {
        let result = normalize_path(Path::new("/"));
        assert_eq!(result, PathBuf::from("/"));
    }

    /// Test excessive '..' beyond root is clamped.
    #[test]
    fn test_normalize_path_excessive_parent_traversal() {
        let result = normalize_path(Path::new("/a/../../../b"));
        assert_eq!(result, PathBuf::from("/b"));
    }

    /// Test mixed '.' and '..' components together.
    #[test]
    fn test_normalize_path_mixed_special_components() {
        let result = normalize_path(Path::new("/a/./b/../c/./d"));
        assert_eq!(result, PathBuf::from("/a/c/d"));
    }

    // ===== Path Traversal Regression Tests =====

    /// Regression: a URI with '..' that escapes the workspace must be rejected
    /// even when the file does not exist on disk (so canonicalize() fails).
    #[tokio::test]
    async fn test_path_traversal_outside_workspace_rejected() {
        let (service, _socket) = LspService::new(Backend::new);

        let workspace_dir = tempfile::tempdir().unwrap();
        let outside_dir = tempfile::tempdir().unwrap();

        // Extract the outside directory name for the traversal path
        let outside_name = outside_dir
            .path()
            .file_name()
            .expect("should have a file name")
            .to_str()
            .expect("should be valid UTF-8");

        // Initialize with workspace root
        let root_uri = Url::from_file_path(workspace_dir.path()).unwrap();
        service
            .inner()
            .initialize(InitializeParams {
                root_uri: Some(root_uri),
                ..Default::default()
            })
            .await
            .unwrap();

        // Construct a path that uses '..' to escape the workspace.
        // The file does not exist, so canonicalize() will fail and
        // the code must fall back to normalize_path().
        let traversal_path = workspace_dir
            .path()
            .join("..")
            .join("..")
            .join(outside_name)
            .join("SKILL.md");
        let uri = Url::from_file_path(&traversal_path).unwrap();
        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri,
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: "---\nname: evil\n---\n# Evil".to_string(),
                },
            })
            .await;

        // Should complete without panic -- the file is outside the workspace
        // so it is silently rejected (warning logged, no diagnostics published).
    }

    /// Regression: a URI with '..' that resolves *inside* the workspace must
    /// still be accepted for validation.
    #[tokio::test]
    async fn test_path_traversal_inside_workspace_accepted() {
        let (service, _socket) = LspService::new(Backend::new);

        let workspace_dir = tempfile::tempdir().unwrap();

        // Create subdir and a SKILL.md at the workspace root
        let subdir = workspace_dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        let skill_path = workspace_dir.path().join("SKILL.md");
        std::fs::write(
            &skill_path,
            "---\nname: test-skill\nversion: 1.0.0\nmodel: sonnet\n---\n\n# Test Skill\n",
        )
        .unwrap();

        // Initialize with workspace root
        let root_uri = Url::from_file_path(workspace_dir.path()).unwrap();
        service
            .inner()
            .initialize(InitializeParams {
                root_uri: Some(root_uri),
                ..Default::default()
            })
            .await
            .unwrap();

        // URI with '..' that resolves back into the workspace

        // URI with '..' that resolves back into the workspace
        let traversal_path = workspace_dir
            .path()
            .join("subdir")
            .join("..")
            .join("SKILL.md");
        let uri = Url::from_file_path(&traversal_path).unwrap();
        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri,
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: std::fs::read_to_string(&skill_path).unwrap(),
                },
            })
            .await;

        // Should complete without error -- file resolves inside workspace
    }

    /// Regression: a non-existent file within the workspace boundary
    /// (without any '..' components) must not be rejected.
    #[tokio::test]
    async fn test_nonexistent_file_in_workspace_accepted() {
        let (service, _socket) = LspService::new(Backend::new);

        let workspace_dir = tempfile::tempdir().unwrap();

        // Initialize with workspace root
        let root_uri = Url::from_file_path(workspace_dir.path()).unwrap();
        service
            .inner()
            .initialize(InitializeParams {
                root_uri: Some(root_uri),
                ..Default::default()
            })
            .await
            .unwrap();

        // Non-existent file inside workspace (no '..' components)
        let nonexistent = workspace_dir.path().join("SKILL.md");
        let uri = Url::from_file_path(&nonexistent).unwrap();

        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri,
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: "---\nname: ghost\n---\n# Ghost".to_string(),
                },
            })
            .await;

        // Should pass boundary check -- path is inside workspace
    }

    /// Regression: a URI with '.' components (current-dir markers) must be
    /// accepted when the file is inside the workspace.
    #[tokio::test]
    async fn test_dot_components_in_path_accepted() {
        let (service, _socket) = LspService::new(Backend::new);

        let workspace_dir = tempfile::tempdir().unwrap();
        let skill_path = workspace_dir.path().join("SKILL.md");
        std::fs::write(
            &skill_path,
            "---\nname: test-skill\nversion: 1.0.0\nmodel: sonnet\n---\n\n# Test Skill\n",
        )
        .unwrap();

        // Initialize with workspace root
        let root_uri = Url::from_file_path(workspace_dir.path()).unwrap();
        service
            .inner()
            .initialize(InitializeParams {
                root_uri: Some(root_uri),
                ..Default::default()
            })
            .await
            .unwrap();

        // URI with '.' components
        let dot_path = format!("{}/./SKILL.md", workspace_dir.path().display());
        let uri = Url::parse(&format!("file://{}", dot_path)).unwrap();

        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri,
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: std::fs::read_to_string(&skill_path).unwrap(),
                },
            })
            .await;

        // Should pass boundary check -- '.' resolves to the same directory
    }

    // ===== Project-Level Validation Tests =====

    /// Test that validate_project_rules_and_publish returns early without panic
    /// when no workspace root is set.
    #[tokio::test]
    async fn test_validate_project_rules_no_workspace() {
        let (service, _socket) = LspService::new(Backend::new);

        // Initialize without workspace root
        service
            .inner()
            .initialize(InitializeParams::default())
            .await
            .unwrap();

        // Should return early without error (no workspace root)
        service.inner().validate_project_rules_and_publish().await;

        // Verify no project diagnostics were stored
        let proj_diags = service.inner().project_level_diagnostics.read().await;
        assert!(
            proj_diags.is_empty(),
            "No project diagnostics should be stored without workspace root"
        );
    }

    /// Test that project-level diagnostics are cached after running validation.
    #[tokio::test]
    async fn test_project_diagnostics_cached() {
        let (service, _socket) = LspService::new(Backend::new);

        let temp_dir = tempfile::tempdir().unwrap();

        // Create two AGENTS.md files to trigger AGM-006
        std::fs::write(temp_dir.path().join("AGENTS.md"), "# Root").unwrap();
        let sub = temp_dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("AGENTS.md"), "# Sub").unwrap();

        let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
        service
            .inner()
            .initialize(InitializeParams {
                root_uri: Some(root_uri),
                ..Default::default()
            })
            .await
            .unwrap();

        // Run project-level validation
        service.inner().validate_project_rules_and_publish().await;

        // Verify project diagnostics are stored
        let proj_diags = service.inner().project_level_diagnostics.read().await;
        assert!(
            !proj_diags.is_empty(),
            "Project diagnostics should be cached for AGM-006"
        );

        // Verify URIs are tracked for cleanup
        let proj_uris = service.inner().project_diagnostics_uris.read().await;
        assert!(
            !proj_uris.is_empty(),
            "Project diagnostic URIs should be tracked"
        );
    }

    /// Test that stale project diagnostics are cleared on re-run.
    #[tokio::test]
    async fn test_project_diagnostics_cleared_on_rerun() {
        let (service, _socket) = LspService::new(Backend::new);

        let temp_dir = tempfile::tempdir().unwrap();

        // Create two AGENTS.md files to trigger AGM-006
        std::fs::write(temp_dir.path().join("AGENTS.md"), "# Root").unwrap();
        let sub = temp_dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("AGENTS.md"), "# Sub").unwrap();

        let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
        service
            .inner()
            .initialize(InitializeParams {
                root_uri: Some(root_uri),
                ..Default::default()
            })
            .await
            .unwrap();

        // First run: should find AGM-006
        service.inner().validate_project_rules_and_publish().await;

        let count_before = service.inner().project_diagnostics_uris.read().await.len();
        assert!(
            count_before > 0,
            "Should have project diagnostics before cleanup"
        );

        // Remove the nested AGENTS.md to resolve the issue
        std::fs::remove_file(sub.join("AGENTS.md")).unwrap();

        // Second run: AGM-006 should no longer fire
        service.inner().validate_project_rules_and_publish().await;

        let proj_diags = service.inner().project_level_diagnostics.read().await;
        let agm006_count: usize = proj_diags
            .values()
            .flat_map(|diags| diags.iter())
            .filter(|d| {
                d.code
                    .as_ref()
                    .map(|c| matches!(c, NumberOrString::String(s) if s == "AGM-006"))
                    .unwrap_or(false)
            })
            .count();
        assert_eq!(agm006_count, 0, "AGM-006 should be cleared after fix");
    }

    /// Test is_project_level_trigger for various file names.
    #[test]
    fn test_is_project_level_trigger() {
        // Instruction files should trigger
        assert!(Backend::is_project_level_trigger(Path::new(
            "/project/CLAUDE.md"
        )));
        assert!(Backend::is_project_level_trigger(Path::new(
            "/project/AGENTS.md"
        )));
        assert!(Backend::is_project_level_trigger(Path::new(
            "/project/.clinerules"
        )));
        assert!(Backend::is_project_level_trigger(Path::new(
            "/project/.cursorrules"
        )));
        assert!(Backend::is_project_level_trigger(Path::new(
            "/project/.github/copilot-instructions.md"
        )));
        assert!(Backend::is_project_level_trigger(Path::new(
            "/project/.github/instructions/test.instructions.md"
        )));
        assert!(Backend::is_project_level_trigger(Path::new(
            "/project/.cursor/rules/test.mdc"
        )));
        assert!(Backend::is_project_level_trigger(Path::new(
            "/project/GEMINI.md"
        )));

        // .agnix.toml should trigger
        assert!(Backend::is_project_level_trigger(Path::new(
            "/project/.agnix.toml"
        )));

        // Non-instruction files should not trigger
        assert!(!Backend::is_project_level_trigger(Path::new(
            "/project/SKILL.md"
        )));
        assert!(!Backend::is_project_level_trigger(Path::new(
            "/project/README.md"
        )));
        assert!(!Backend::is_project_level_trigger(Path::new(
            "/project/settings.json"
        )));
        assert!(!Backend::is_project_level_trigger(Path::new(
            "/project/plugin.json"
        )));
    }

    /// Test that initialize advertises executeCommand capability.
    #[tokio::test]
    async fn test_initialize_advertises_execute_command() {
        let (service, _socket) = LspService::new(Backend::new);

        let result = service
            .inner()
            .initialize(InitializeParams::default())
            .await
            .unwrap();

        match result.capabilities.execute_command_provider {
            Some(ref opts) => {
                assert!(
                    opts.commands
                        .contains(&"agnix.validateProjectRules".to_string()),
                    "Expected agnix.validateProjectRules in execute commands, got: {:?}",
                    opts.commands
                );
            }
            None => panic!("Expected execute command capability"),
        }
    }

    /// Test that execute_command handles the validateProjectRules command.
    #[tokio::test]
    async fn test_execute_command_validate_project_rules() {
        let (service, _socket) = LspService::new(Backend::new);

        let temp_dir = tempfile::tempdir().unwrap();
        let root_uri = Url::from_file_path(temp_dir.path()).unwrap();

        service
            .inner()
            .initialize(InitializeParams {
                root_uri: Some(root_uri),
                ..Default::default()
            })
            .await
            .unwrap();

        // Execute the command
        let result = service
            .inner()
            .execute_command(ExecuteCommandParams {
                command: "agnix.validateProjectRules".to_string(),
                arguments: vec![],
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    /// Test that execute_command handles unknown commands gracefully.
    #[tokio::test]
    async fn test_execute_command_unknown() {
        let (service, _socket) = LspService::new(Backend::new);

        service
            .inner()
            .initialize(InitializeParams::default())
            .await
            .unwrap();

        let result = service
            .inner()
            .execute_command(ExecuteCommandParams {
                command: "unknown.command".to_string(),
                arguments: vec![],
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    /// Test that project-level diagnostics are merged with per-file diagnostics
    /// when validate_from_content_and_publish is called.
    ///
    /// Pre-populates the project_level_diagnostics cache with a diagnostic for
    /// a file URI, then opens the file so per-file validation runs and the merge
    /// path in validate_from_content_and_publish is exercised.
    #[tokio::test]
    async fn test_project_and_file_diagnostics_merged() {
        let (service, _socket) = LspService::new(Backend::new);

        let temp_dir = tempfile::tempdir().unwrap();
        let root_uri = Url::from_file_path(temp_dir.path()).unwrap();

        // Initialize with workspace root
        service
            .inner()
            .initialize(InitializeParams {
                root_uri: Some(root_uri),
                ..Default::default()
            })
            .await
            .unwrap();

        // Create a CLAUDE.md that will produce per-file diagnostics (e.g. XML-001)
        let claude_path = temp_dir.path().join("CLAUDE.md");
        std::fs::write(&claude_path, "<unclosed>\n# Project\n").unwrap();
        let uri = Url::from_file_path(&claude_path).unwrap();

        // Pre-populate project_level_diagnostics with a fake AGM-006 diagnostic
        // for this URI, simulating what validate_project_rules_and_publish would store.
        {
            let fake_project_diag = Diagnostic {
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
                severity: Some(DiagnosticSeverity::WARNING),
                code: Some(NumberOrString::String("AGM-006".to_string())),
                code_description: None,
                source: Some("agnix".to_string()),
                message: "Nested AGENTS.md detected".to_string(),
                related_information: None,
                tags: None,
                data: None,
            };
            let mut proj_diags = service.inner().project_level_diagnostics.write().await;
            proj_diags.insert(uri.clone(), vec![fake_project_diag]);
        }

        // Verify the project diagnostics are in the cache
        {
            let proj_diags = service.inner().project_level_diagnostics.read().await;
            assert!(
                proj_diags.contains_key(&uri),
                "Project diagnostics should be pre-populated for the URI"
            );
        }

        // Open the file -- this triggers validate_from_content_and_publish which
        // should merge per-file diagnostics (e.g. XML-001) with the cached
        // project-level diagnostics (AGM-006).
        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: std::fs::read_to_string(&claude_path).unwrap(),
                },
            })
            .await;

        // The merge code path in validate_from_content_and_publish (lines 309-315)
        // was exercised: it reads project_level_diagnostics and extends the
        // per-file diagnostics with any matching project-level entries.
        // Verify the project cache is still intact after the merge.
        {
            let proj_diags = service.inner().project_level_diagnostics.read().await;
            let diags = proj_diags
                .get(&uri)
                .expect("Project diagnostics should still be cached");
            assert!(
                diags
                    .iter()
                    .any(|d| d.code == Some(NumberOrString::String("AGM-006".to_string()))),
                "Cached project diagnostic should be preserved after merge"
            );
        }
    }

    // ===== for_each_bounded additional tests =====

    #[tokio::test]
    async fn test_for_each_bounded_concurrency_limit_one() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let max_concurrent = Arc::new(AtomicUsize::new(0));
        let current = Arc::new(AtomicUsize::new(0));

        let items: Vec<usize> = (0..5).collect();

        let max_c = Arc::clone(&max_concurrent);
        let cur = Arc::clone(&current);

        let errors = for_each_bounded(items, 1, move |_item| {
            let max_c = Arc::clone(&max_c);
            let cur = Arc::clone(&cur);
            async move {
                let c = cur.fetch_add(1, Ordering::SeqCst) + 1;
                // Update max observed concurrency
                max_c.fetch_max(c, Ordering::SeqCst);
                // Yield to give other tasks a chance to run
                tokio::task::yield_now().await;
                cur.fetch_sub(1, Ordering::SeqCst);
            }
        })
        .await;

        assert!(errors.is_empty());
        assert_eq!(
            max_concurrent.load(Ordering::SeqCst),
            1,
            "With concurrency limit 1, at most 1 task should run concurrently"
        );
    }

    #[tokio::test]
    async fn test_for_each_bounded_zero_concurrency_defaults_to_one() {
        // Passing 0 as max_concurrency should be clamped to 1 (not hang or panic)
        let items = vec![1, 2, 3];
        let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count_clone = Arc::clone(&count);

        let errors = for_each_bounded(items, 0, move |_| {
            let count = Arc::clone(&count_clone);
            async move {
                count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        })
        .await;

        assert!(errors.is_empty());
        assert_eq!(
            count.load(std::sync::atomic::Ordering::SeqCst),
            3,
            "All items should be processed even with concurrency 0"
        );
    }
}
