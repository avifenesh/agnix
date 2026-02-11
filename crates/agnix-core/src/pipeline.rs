//! Validation pipeline: file and project validation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use rayon::iter::ParallelBridge;
use rayon::prelude::*;
use rust_i18n::t;

use crate::config::LintConfig;
use crate::diagnostics::{ConfigError, CoreError, Diagnostic, LintResult, ValidationError};
use crate::file_types::{FileType, detect_file_type};
use crate::file_utils;
use crate::registry::ValidatorRegistry;
use crate::schemas;

/// Result of validating a project, including diagnostics and metadata.
///
/// This struct is marked `#[non_exhaustive]` so that new metadata fields can be
/// added in minor releases without breaking downstream destructuring patterns.
/// Use `ValidationResult::new()` to construct instances in tests.
#[derive(Debug, Clone)]
/// **Breaking change in 0.11.0**: This struct is now marked `#[non_exhaustive]`.
/// Downstream crates using struct literals or exhaustive destructuring must
/// switch to `ValidationResult::new()` or use `..` in patterns.
#[non_exhaustive]
pub struct ValidationResult {
    /// Diagnostics found during validation.
    pub diagnostics: Vec<Diagnostic>,
    /// Number of files that were checked (excludes Unknown file types).
    pub files_checked: usize,
    /// Wall-clock time spent in validation, in milliseconds.
    pub validation_time_ms: Option<u64>,
    /// Number of validator factories registered in the registry (not the count of validators executed).
    pub validator_factories_registered: usize,
}

impl ValidationResult {
    /// Create a new `ValidationResult` with the given diagnostics and file count.
    ///
    /// Metadata fields (`validation_time_ms`, `validator_factories_registered`) default to
    /// `None` / `0` and can be set with the builder-style helpers.
    pub fn new(diagnostics: Vec<Diagnostic>, files_checked: usize) -> Self {
        Self {
            diagnostics,
            files_checked,
            validation_time_ms: None,
            validator_factories_registered: 0,
        }
    }

    /// Set the wall-clock validation time (builder pattern).
    pub fn with_timing(mut self, ms: u64) -> Self {
        self.validation_time_ms = Some(ms);
        self
    }

    /// Set the total number of validator factories registered (builder pattern).
    pub fn with_validator_factories_registered(mut self, count: usize) -> Self {
        self.validator_factories_registered = count;
        self
    }
}

/// Pre-compiled file inclusion/exclusion patterns for efficient matching.
///
/// Used internally by `validate_project_with_registry` to avoid re-compiling
/// glob patterns for every file during parallel validation.
#[derive(Default)]
pub(crate) struct CompiledFilesConfig {
    include_as_memory: Vec<glob::Pattern>,
    include_as_generic: Vec<glob::Pattern>,
    exclude: Vec<glob::Pattern>,
}

impl CompiledFilesConfig {
    fn is_empty(&self) -> bool {
        self.include_as_memory.is_empty()
            && self.include_as_generic.is_empty()
            && self.exclude.is_empty()
    }
}

fn compile_patterns_lenient(patterns: &[String]) -> Vec<glob::Pattern> {
    patterns
        .iter()
        .filter_map(|p| {
            let normalized = p.replace('\\', "/");
            match glob::Pattern::new(&normalized) {
                Ok(pat) => Some(pat),
                Err(e) => {
                    // TODO: Consider returning invalid glob patterns as Diagnostic warnings instead of eprintln!
                    // This would allow library consumers to handle warnings programmatically.
                    // See: https://github.com/avifenesh/agnix/issues
                    eprintln!("warning: ignoring invalid glob pattern '{}' : {}", p, e);
                    None
                }
            }
        })
        .collect()
}

fn compile_files_config(files: &crate::config::FilesConfig) -> CompiledFilesConfig {
    CompiledFilesConfig {
        include_as_memory: compile_patterns_lenient(&files.include_as_memory),
        include_as_generic: compile_patterns_lenient(&files.include_as_generic),
        exclude: compile_patterns_lenient(&files.exclude),
    }
}

/// Match options for file inclusion/exclusion glob patterns.
///
/// `require_literal_separator` is `true` so that `*` only matches within a
/// single path component. Users must use `**` for recursive matching (e.g.
/// `dir/**/*.md` instead of `dir/*.md` to match nested files).
const FILES_MATCH_OPTIONS: glob::MatchOptions = glob::MatchOptions {
    case_sensitive: true,
    require_literal_separator: true,
    require_literal_leading_dot: false,
};

fn resolve_with_compiled(
    path: &Path,
    root_dir: Option<&Path>,
    compiled: &CompiledFilesConfig,
) -> FileType {
    if compiled.is_empty() {
        return detect_file_type(path);
    }

    let rel_path = if let Some(root) = root_dir {
        normalize_rel_path(path, root)
    } else {
        // No root_dir: use filename only
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string()
    };

    // Priority: exclude > include_as_memory > include_as_generic > detect
    for pattern in &compiled.exclude {
        if pattern.matches_with(&rel_path, FILES_MATCH_OPTIONS) {
            return FileType::Unknown;
        }
    }
    for pattern in &compiled.include_as_memory {
        if pattern.matches_with(&rel_path, FILES_MATCH_OPTIONS) {
            return FileType::ClaudeMd;
        }
    }
    for pattern in &compiled.include_as_generic {
        if pattern.matches_with(&rel_path, FILES_MATCH_OPTIONS) {
            return FileType::GenericMarkdown;
        }
    }

    detect_file_type(path)
}

/// Resolve file type with config-based overrides.
///
/// Applies `[files]` config patterns on top of [`detect_file_type`]:
/// - `files.exclude` patterns map to [`FileType::Unknown`] (skip validation)
/// - `files.include_as_memory` patterns map to [`FileType::ClaudeMd`]
/// - `files.include_as_generic` patterns map to [`FileType::GenericMarkdown`]
/// - Otherwise falls through to [`detect_file_type`]
///
/// Priority: exclude > include_as_memory > include_as_generic > built-in detection.
///
/// When no `[files]` patterns are configured, this is equivalent to
/// calling `detect_file_type(path)` directly.
pub fn resolve_file_type(path: &Path, config: &LintConfig) -> FileType {
    if config.files.include_as_memory.is_empty()
        && config.files.include_as_generic.is_empty()
        && config.files.exclude.is_empty()
    {
        return detect_file_type(path);
    }

    // Compile patterns on-demand for single-file validation.
    // Invalid patterns are silently skipped (validated at config load time).
    let compiled = compile_files_config(&config.files);
    resolve_with_compiled(path, config.root_dir.as_deref(), &compiled)
}

/// Validate a single file
pub fn validate_file(path: &Path, config: &LintConfig) -> LintResult<Vec<Diagnostic>> {
    let mut registry = ValidatorRegistry::with_defaults();
    for name in &config.rules.disabled_validators {
        registry.disable_validator(name);
    }
    validate_file_with_registry(path, config, &registry)
}

/// Validate a single file with a custom validator registry
pub fn validate_file_with_registry(
    path: &Path,
    config: &LintConfig,
    registry: &ValidatorRegistry,
) -> LintResult<Vec<Diagnostic>> {
    let file_type = resolve_file_type(path, config);
    validate_file_with_type(path, file_type, config, registry)
}

/// Validate a single file with a pre-resolved [`FileType`].
///
/// This avoids re-compiling `[files]` glob patterns when the file type has
/// already been determined (e.g. in `validate_project_with_registry` where
/// patterns are pre-compiled for the entire walk).
fn validate_file_with_type(
    path: &Path,
    file_type: FileType,
    config: &LintConfig,
    registry: &ValidatorRegistry,
) -> LintResult<Vec<Diagnostic>> {
    if file_type == FileType::Unknown {
        return Ok(vec![]);
    }

    let content = file_utils::safe_read_file(path)?;

    let validators = registry.validators_for(file_type);
    let mut diagnostics = Vec::new();

    for validator in validators {
        diagnostics.extend(validator.validate(path, &content, config));
    }

    Ok(diagnostics)
}

/// Main entry point for validating a project
pub fn validate_project(path: &Path, config: &LintConfig) -> LintResult<ValidationResult> {
    let mut registry = ValidatorRegistry::with_defaults();
    for name in &config.rules.disabled_validators {
        registry.disable_validator(name);
    }
    validate_project_with_registry(path, config, &registry)
}

struct ExcludePattern {
    pattern: glob::Pattern,
    dir_only_prefix: Option<String>,
    allow_probe: bool,
}

fn normalize_rel_path(entry_path: &Path, root: &Path) -> String {
    let rel_path = entry_path.strip_prefix(root).unwrap_or(entry_path);
    let path_str = rel_path.to_string_lossy().replace('\\', "/");
    match path_str.strip_prefix("./") {
        Some(stripped) => stripped.to_string(),
        None => path_str,
    }
}

fn compile_exclude_patterns(excludes: &[String]) -> LintResult<Vec<ExcludePattern>> {
    excludes
        .iter()
        .map(|pattern| {
            let normalized = pattern.replace('\\', "/");
            let (glob_str, dir_only_prefix) = if let Some(prefix) = normalized.strip_suffix('/') {
                (format!("{}/**", prefix), Some(prefix.to_string()))
            } else {
                (normalized.clone(), None)
            };
            let allow_probe = dir_only_prefix.is_some() || glob_str.contains("**");
            let compiled = glob::Pattern::new(&glob_str).map_err(|e| {
                CoreError::Config(ConfigError::InvalidExcludePattern {
                    pattern: pattern.clone(),
                    message: e.to_string(),
                })
            })?;
            Ok(ExcludePattern {
                pattern: compiled,
                dir_only_prefix,
                allow_probe,
            })
        })
        .collect()
}

fn should_prune_dir(rel_dir: &str, exclude_patterns: &[ExcludePattern]) -> bool {
    if rel_dir.is_empty() {
        return false;
    }
    // Probe path used to detect patterns that match files inside a directory.
    // Only apply it for recursive patterns (e.g. ** or dir-only prefix).
    let probe = format!("{}/__agnix_probe__", rel_dir.trim_end_matches('/'));
    exclude_patterns
        .iter()
        .any(|p| p.pattern.matches(rel_dir) || (p.allow_probe && p.pattern.matches(&probe)))
}

fn is_excluded_file(path_str: &str, exclude_patterns: &[ExcludePattern]) -> bool {
    exclude_patterns
        .iter()
        .any(|p| p.pattern.matches(path_str) && p.dir_only_prefix.as_deref() != Some(path_str))
}

/// Run project-level checks that require cross-file analysis.
///
/// These checks analyze relationships between multiple files in the project:
/// - AGM-006: Multiple AGENTS.md files
/// - XP-004: Conflicting build/test commands across instruction files
/// - XP-005: Conflicting tool constraints across instruction files
/// - XP-006: Multiple instruction layers without documented precedence
/// - VER-001: No tool/spec versions pinned
///
/// Both `agents_md_paths` and `instruction_file_paths` must be pre-sorted
/// for deterministic output ordering.
fn run_project_level_checks(
    agents_md_paths: &[PathBuf],
    instruction_file_paths: &[PathBuf],
    config: &LintConfig,
    root_dir: &Path,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // AGM-006: Check for multiple AGENTS.md files in the directory tree
    if config.is_rule_enabled("AGM-006") {
        if agents_md_paths.len() > 1 {
            for agents_file in agents_md_paths.iter() {
                let parent_files =
                    schemas::agents_md::check_agents_md_hierarchy(agents_file, agents_md_paths);
                let description = if !parent_files.is_empty() {
                    let parent_paths: Vec<String> = parent_files
                        .iter()
                        .map(|p| p.to_string_lossy().into_owned())
                        .collect();
                    format!(
                        "Nested AGENTS.md detected - parent AGENTS.md files exist at: {}",
                        parent_paths.join(", ")
                    )
                } else {
                    let other_paths: Vec<String> = agents_md_paths
                        .iter()
                        .filter(|p| p.as_path() != agents_file.as_path())
                        .map(|p| p.to_string_lossy().into_owned())
                        .collect();
                    format!(
                        "Multiple AGENTS.md files detected - other AGENTS.md files exist at: {}",
                        other_paths.join(", ")
                    )
                };

                diagnostics.push(
                    Diagnostic::warning(
                        agents_file.clone(),
                        1,
                        0,
                        "AGM-006",
                        description,
                    )
                    .with_suggestion(
                        "Some tools load AGENTS.md hierarchically. Document inheritance behavior or consolidate files.".to_string(),
                    ),
                );
            }
        }
    }

    // XP-004, XP-005, XP-006: Cross-layer contradiction detection
    let xp004_enabled = config.is_rule_enabled("XP-004");
    let xp005_enabled = config.is_rule_enabled("XP-005");
    let xp006_enabled = config.is_rule_enabled("XP-006");

    if xp004_enabled || xp005_enabled || xp006_enabled {
        if instruction_file_paths.len() > 1 {
            // Read content of all instruction files
            let mut file_contents: Vec<(PathBuf, String)> = Vec::new();
            for file_path in instruction_file_paths.iter() {
                match file_utils::safe_read_file(file_path) {
                    Ok(content) => {
                        file_contents.push((file_path.clone(), content));
                    }
                    Err(e) => {
                        diagnostics.push(
                            Diagnostic::error(
                                file_path.clone(),
                                0,
                                0,
                                "XP-004",
                                t!("rules.xp_004_read_error", error = e.to_string()),
                            )
                            .with_suggestion(t!("rules.xp_004_read_error_suggestion")),
                        );
                    }
                }
            }

            // XP-004: Detect conflicting build/test commands
            if xp004_enabled {
                let file_commands: Vec<_> = file_contents
                    .iter()
                    .filter_map(|(path, content)| {
                        let cmds = schemas::cross_platform::extract_build_commands(content);
                        if cmds.is_empty() {
                            None
                        } else {
                            Some((path.clone(), cmds))
                        }
                    })
                    .collect();

                let build_conflicts =
                    schemas::cross_platform::detect_build_conflicts(&file_commands);
                for conflict in build_conflicts {
                    diagnostics.push(
                        Diagnostic::warning(
                            conflict.file1.clone(),
                            conflict.file1_line,
                            0,
                            "XP-004",
                            format!(
                                "Conflicting package managers: {} uses {} but {} uses {} for {} commands",
                                conflict.file1.display(),
                                conflict.file1_manager.as_str(),
                                conflict.file2.display(),
                                conflict.file2_manager.as_str(),
                                match conflict.command_type {
                                    schemas::cross_platform::CommandType::Install => "install",
                                    schemas::cross_platform::CommandType::Build => "build",
                                    schemas::cross_platform::CommandType::Test => "test",
                                    schemas::cross_platform::CommandType::Run => "run",
                                    schemas::cross_platform::CommandType::Other => "other",
                                }
                            ),
                        )
                        .with_suggestion(
                            "Standardize on a single package manager across all instruction files".to_string(),
                        ),
                    );
                }
            }

            // XP-005: Detect conflicting tool constraints
            if xp005_enabled {
                let file_constraints: Vec<_> = file_contents
                    .iter()
                    .filter_map(|(path, content)| {
                        let constraints =
                            schemas::cross_platform::extract_tool_constraints(content);
                        if constraints.is_empty() {
                            None
                        } else {
                            Some((path.clone(), constraints))
                        }
                    })
                    .collect();

                let tool_conflicts =
                    schemas::cross_platform::detect_tool_conflicts(&file_constraints);
                for conflict in tool_conflicts {
                    diagnostics.push(
                        Diagnostic::error(
                            conflict.allow_file.clone(),
                            conflict.allow_line,
                            0,
                            "XP-005",
                            format!(
                                "Conflicting tool constraints: '{}' is allowed in {} but disallowed in {}",
                                conflict.tool_name,
                                conflict.allow_file.display(),
                                conflict.disallow_file.display()
                            ),
                        )
                        .with_suggestion(
                            "Resolve the conflict by consistently allowing or disallowing the tool".to_string(),
                        ),
                    );
                }
            }

            // XP-006: Detect multiple layers without documented precedence
            if xp006_enabled {
                let layers: Vec<_> = file_contents
                    .iter()
                    .map(|(path, content)| schemas::cross_platform::categorize_layer(path, content))
                    .collect();

                if let Some(issue) = schemas::cross_platform::detect_precedence_issues(&layers) {
                    // Report on the first layer file
                    if let Some(first_layer) = issue.layers.first() {
                        diagnostics.push(
                            Diagnostic::warning(
                                first_layer.path.clone(),
                                1,
                                0,
                                "XP-006",
                                issue.description,
                            )
                            .with_suggestion(
                                "Document which file takes precedence (e.g., 'CLAUDE.md takes precedence over AGENTS.md')".to_string(),
                            ),
                        );
                    }
                }
            }
        }
    }

    // VER-001: Warn when no tool/spec versions are explicitly pinned
    if config.is_rule_enabled("VER-001") {
        let has_any_version_pinned = config.is_claude_code_version_pinned()
            || config.tool_versions.codex.is_some()
            || config.tool_versions.cursor.is_some()
            || config.tool_versions.copilot.is_some()
            || config.is_mcp_revision_pinned()
            || config.spec_revisions.agent_skills_spec.is_some()
            || config.spec_revisions.agents_md_spec.is_some();

        if !has_any_version_pinned {
            // Use .agnix.toml path or project root as the file reference
            let config_file = root_dir.join(".agnix.toml");
            let report_path = if config_file.exists() {
                config_file
            } else {
                root_dir.to_path_buf()
            };

            diagnostics.push(
                Diagnostic::info(report_path, 1, 0, "VER-001", t!("rules.ver_001.message"))
                    .with_suggestion(t!("rules.ver_001.suggestion")),
            );
        }
    }

    diagnostics
}

/// Run only project-level validation checks without per-file validation.
///
/// This is a lightweight alternative to [`validate_project`] that only runs
/// cross-file analysis rules (AGM-006, XP-004/005/006, VER-001). It does
/// not validate individual file contents.
///
/// Designed for the LSP server to provide project-level diagnostics that
/// require workspace-wide analysis, without the overhead of full per-file
/// validation (which the LSP handles incrementally via `did_open`/`did_change`).
pub fn validate_project_rules(root: &Path, config: &LintConfig) -> LintResult<Vec<Diagnostic>> {
    use ignore::WalkBuilder;
    use std::sync::Arc;

    let root_dir = resolve_validation_root(root);
    let mut config = config.clone();
    config.set_root_dir(root_dir.clone());

    // Pre-compile exclude patterns once (Arc for filter_entry 'static bound)
    let exclude_patterns = Arc::new(compile_exclude_patterns(&config.exclude)?);

    let walk_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let root_path = root_dir.clone();

    let mut agents_md_paths: Vec<PathBuf> = Vec::new();
    let mut instruction_file_paths: Vec<PathBuf> = Vec::new();
    let max_files = config.max_files_to_validate;

    // Walk directory tree collecting only paths relevant to project-level checks.
    // No per-file validation is performed -- this walk is lightweight.
    // Respects the same max_files_to_validate limit as validate_project_with_registry
    // to prevent unbounded directory traversal in large workspaces.
    for (files_seen, entry) in WalkBuilder::new(&walk_root)
        .hidden(false)
        .git_ignore(true)
        .git_exclude(false)
        .filter_entry({
            let exclude_patterns = Arc::clone(&exclude_patterns);
            let root_path = root_path.clone();
            move |entry| {
                let entry_path = entry.path();
                if entry_path == root_path {
                    return true;
                }
                if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                    let rel_path = normalize_rel_path(entry_path, &root_path);
                    return !should_prune_dir(&rel_path, exclude_patterns.as_slice());
                }
                true
            }
        })
        .build()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .enumerate()
    {
        // Enforce file count limit to prevent unbounded traversal
        if let Some(limit) = max_files {
            if files_seen >= limit {
                return Err(CoreError::Validation(ValidationError::TooManyFiles {
                    count: files_seen,
                    limit,
                }));
            }
        }
        let file_path = entry.path().to_path_buf();

        let path_str = normalize_rel_path(&file_path, &root_path);
        if is_excluded_file(&path_str, exclude_patterns.as_slice()) {
            continue;
        }

        // Collect AGENTS.md paths for AGM-006 check
        if file_path.file_name().and_then(|n| n.to_str()) == Some("AGENTS.md") {
            agents_md_paths.push(file_path.clone());
        }

        // Collect instruction file paths for XP-004/005/006 checks
        if schemas::cross_platform::is_instruction_file(&file_path) {
            instruction_file_paths.push(file_path);
        }
    }

    // Sort for deterministic ordering
    agents_md_paths.sort();
    instruction_file_paths.sort();

    Ok(run_project_level_checks(
        &agents_md_paths,
        &instruction_file_paths,
        &config,
        &root_dir,
    ))
}

/// Main entry point for validating a project with a custom validator registry
pub fn validate_project_with_registry(
    path: &Path,
    config: &LintConfig,
    registry: &ValidatorRegistry,
) -> LintResult<ValidationResult> {
    use ignore::WalkBuilder;
    use std::sync::Arc;
    use std::time::Instant;

    let validation_start = Instant::now();

    let root_dir = resolve_validation_root(path);
    let mut config = config.clone();
    config.set_root_dir(root_dir.clone());

    // Initialize shared import cache for project-level validation.
    // This cache is shared across all file validations, allowing the ImportsValidator
    // to avoid redundant parsing when traversing import chains that reference the same files.
    let import_cache: crate::parsers::ImportCache =
        std::sync::Arc::new(std::sync::RwLock::new(HashMap::new()));
    config.set_import_cache(import_cache);

    // Pre-compile exclude patterns once (avoids N+1 pattern compilation)
    let exclude_patterns = compile_exclude_patterns(&config.exclude)?;
    let exclude_patterns = Arc::new(exclude_patterns);

    // Pre-compile files config patterns once for the parallel walk.
    // Invalid patterns are treated as non-fatal to align with LintConfig::validate()
    // Invalid patterns are silently skipped (validated at config load time).
    let compiled_files = Arc::new(compile_files_config(&config.files));

    let root_path = root_dir.clone();

    // Fallback to relative path is safe: symlink checks and size limits still apply per-file
    let walk_root = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    // Shared atomic state for file-limit enforcement across parallel workers.
    // These must remain atomic (not fold/reduce) because the limit check must
    // be visible immediately to all threads to stop work promptly.
    let files_checked = Arc::new(AtomicUsize::new(0));
    let limit_exceeded = Arc::new(AtomicBool::new(false));

    // Get the file limit from config (None means no limit)
    let max_files = config.max_files_to_validate;

    // Stream file walk directly into parallel validation (no intermediate Vec)
    // Note: hidden(false) includes .github, .codex, .claude, .cursor directories
    // Note: git_exclude(false) prevents .git/info/exclude from hiding config dirs
    //       that users may locally exclude (e.g. .codex/) but still need linting.
    //       Trade-off: this may surface files the user intentionally excluded locally,
    //       but security is still enforced via symlink rejection (file_utils::safe_read)
    //       and file size limits, so the exposure is limited to lint noise, not unsafe I/O.
    //
    // Uses fold/reduce instead of Mutex-protected Vecs to accumulate paths and
    // diagnostics thread-locally, eliminating lock contention in the hot loop.
    let (mut diagnostics, mut agents_md_paths, mut instruction_file_paths) =
        WalkBuilder::new(&walk_root)
            .hidden(false)
            .git_ignore(true)
            .git_exclude(false)
            .filter_entry({
                let exclude_patterns = Arc::clone(&exclude_patterns);
                let root_path = root_path.clone();
                move |entry| {
                    let entry_path = entry.path();
                    if entry_path == root_path {
                        return true;
                    }
                    if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                        let rel_path = normalize_rel_path(entry_path, &root_path);
                        return !should_prune_dir(&rel_path, exclude_patterns.as_slice());
                    }
                    true
                }
            })
            .build()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_file())
            .filter(|entry| {
                let entry_path = entry.path();
                let path_str = normalize_rel_path(entry_path, &root_path);
                !is_excluded_file(&path_str, exclude_patterns.as_slice())
            })
            .map(|entry| entry.path().to_path_buf())
            .par_bridge()
            .fold(
                || {
                    (
                        Vec::<Diagnostic>::new(),
                        Vec::<PathBuf>::new(),
                        Vec::<PathBuf>::new(),
                    )
                },
                |(mut diags, mut agents, mut instructions), file_path| {
                    // Security: Check if file limit has been exceeded
                    // Once exceeded, skip processing additional files
                    // Use SeqCst ordering for consistency with store operations
                    if limit_exceeded.load(Ordering::SeqCst) {
                        return (diags, agents, instructions);
                    }

                    // Count recognized files (resolve_with_compiled is string-only, no I/O)
                    let file_type =
                        resolve_with_compiled(&file_path, Some(&root_path), &compiled_files);
                    if file_type != FileType::Unknown {
                        let count = files_checked.fetch_add(1, Ordering::SeqCst);
                        // Security: Enforce file count limit to prevent DoS
                        if let Some(limit) = max_files {
                            if count >= limit {
                                limit_exceeded.store(true, Ordering::SeqCst);
                                return (diags, agents, instructions);
                            }
                        }
                    }

                    // Collect AGENTS.md paths for AGM-006 check (thread-local, no lock)
                    if file_path.file_name().and_then(|n| n.to_str()) == Some("AGENTS.md") {
                        agents.push(file_path.clone());
                    }

                    // Collect instruction file paths for XP-004/005/006 checks (thread-local, no lock)
                    if schemas::cross_platform::is_instruction_file(&file_path) {
                        instructions.push(file_path.clone());
                    }

                    // Validate the file using the pre-resolved file_type to avoid
                    // re-compiling [files] glob patterns for every file.
                    match validate_file_with_type(&file_path, file_type, &config, registry) {
                        Ok(file_diagnostics) => diags.extend(file_diagnostics),
                        Err(e) => {
                            diags.push(
                                Diagnostic::error(
                                    file_path.clone(),
                                    0,
                                    0,
                                    "file::read",
                                    t!("rules.file_read_error", error = e.to_string()),
                                )
                                .with_suggestion(t!("rules.file_read_error_suggestion")),
                            );
                        }
                    }

                    (diags, agents, instructions)
                },
            )
            .reduce(
                || (Vec::new(), Vec::new(), Vec::new()),
                |(mut d1, mut a1, mut i1), (d2, a2, i2)| {
                    d1.extend(d2);
                    a1.extend(a2);
                    i1.extend(i2);
                    (d1, a1, i1)
                },
            );

    // Check if limit was exceeded and return error
    if limit_exceeded.load(Ordering::Relaxed) {
        if let Some(limit) = max_files {
            return Err(CoreError::Validation(ValidationError::TooManyFiles {
                count: files_checked.load(Ordering::Relaxed),
                limit,
            }));
        }
    }

    // Run project-level checks (AGM-006, XP-004/005/006, VER-001)
    {
        agents_md_paths.sort();
        instruction_file_paths.sort();

        diagnostics.extend(run_project_level_checks(
            &agents_md_paths,
            &instruction_file_paths,
            &config,
            &root_dir,
        ));
    }

    // Sort by severity (errors first), then by file path, then by line/rule for full determinism
    diagnostics.sort_by(|a, b| {
        a.level
            .cmp(&b.level)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.rule.cmp(&b.rule))
    });

    // Extract final count from atomic counter
    let files_checked = files_checked.load(Ordering::Relaxed);

    let elapsed_ms_u128 = validation_start.elapsed().as_millis();
    let elapsed_ms = std::cmp::min(elapsed_ms_u128, u64::MAX as u128) as u64;
    let validator_factories_registered = registry.total_factory_count();

    Ok(ValidationResult::new(diagnostics, files_checked)
        .with_timing(elapsed_ms)
        .with_validator_factories_registered(validator_factories_registered))
}

fn resolve_validation_root(path: &Path) -> PathBuf {
    let candidate = if path.is_file() {
        path.parent().unwrap_or(Path::new("."))
    } else {
        path
    };
    std::fs::canonicalize(candidate).unwrap_or_else(|_| candidate.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_prune_dir_with_globbed_patterns() {
        let patterns =
            compile_exclude_patterns(&vec!["target/**".to_string(), "**/target/**".to_string()])
                .unwrap();
        assert!(
            should_prune_dir("target", &patterns),
            "Expected target/** to prune target directory"
        );
        assert!(
            should_prune_dir("sub/target", &patterns),
            "Expected **/target/** to prune nested target directory"
        );
    }

    #[test]
    fn test_should_prune_dir_for_bare_pattern() {
        let patterns = compile_exclude_patterns(&vec!["target".to_string()]).unwrap();
        assert!(
            should_prune_dir("target", &patterns),
            "Bare pattern should prune directory"
        );
        assert!(
            !should_prune_dir("sub/target", &patterns),
            "Bare pattern should not prune nested directories"
        );
    }

    #[test]
    fn test_should_prune_dir_for_trailing_slash_pattern() {
        let patterns = compile_exclude_patterns(&vec!["target/".to_string()]).unwrap();
        assert!(
            should_prune_dir("target", &patterns),
            "Trailing slash pattern should prune directory"
        );
    }

    #[test]
    fn test_should_not_prune_root_dir() {
        let patterns = compile_exclude_patterns(&vec!["target/**".to_string()]).unwrap();
        assert!(
            !should_prune_dir("", &patterns),
            "Root directory should never be pruned"
        );
    }

    #[test]
    fn test_should_not_prune_dir_for_single_level_glob() {
        let patterns = compile_exclude_patterns(&vec!["target/*".to_string()]).unwrap();
        assert!(
            !should_prune_dir("target", &patterns),
            "Single-level glob should not prune directory"
        );
    }

    #[test]
    fn test_dir_only_pattern_does_not_exclude_file_named_dir() {
        let patterns = compile_exclude_patterns(&vec!["target/".to_string()]).unwrap();
        assert!(
            !is_excluded_file("target", &patterns),
            "Directory-only pattern should not exclude a file named target"
        );
    }

    #[test]
    fn test_dir_only_pattern_excludes_files_under_dir() {
        let patterns = compile_exclude_patterns(&vec!["target/".to_string()]).unwrap();
        assert!(
            is_excluded_file("target/file.txt", &patterns),
            "Directory-only pattern should exclude files under target/"
        );
    }

    #[test]
    fn test_compile_exclude_patterns_invalid_pattern_returns_error() {
        let result = compile_exclude_patterns(&vec!["[".to_string()]);
        assert!(matches!(
            result,
            Err(CoreError::Config(ConfigError::InvalidExcludePattern { .. }))
        ));
    }
}
