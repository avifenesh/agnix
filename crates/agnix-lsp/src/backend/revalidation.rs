use super::*;
use crate::vscode_config::VsCodeConfig;
use std::collections::{HashMap, HashSet};
use std::future::Future;

pub(super) const MAX_CONFIG_REVALIDATION_CONCURRENCY: usize = 8;

pub(super) fn config_revalidation_concurrency(document_count: usize) -> usize {
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
pub(super) async fn for_each_bounded<T, I, F, Fut>(
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

impl Backend {
    /// In config-change batch revalidation mode, only publish if the batch generation is current
    /// and the document is still open.
    pub(super) async fn should_publish_diagnostics(
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
    pub(super) async fn validate_project_rules_and_publish(&self) {
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

    pub(super) async fn handle_did_change_configuration(
        &self,
        params: DidChangeConfigurationParams,
    ) {
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
                new_config.set_root_dir(root.clone());
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
}
