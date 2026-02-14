//! Linter configuration

use crate::file_utils::safe_read_file;
use crate::fs::{FileSystem, RealFileSystem};
use crate::schemas::mcp::DEFAULT_MCP_PROTOCOL_VERSION;
use rust_i18n::t;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Maximum number of file patterns per list (include_as_memory, include_as_generic, exclude).
/// Exceeding this limit produces a configuration warning.
const MAX_FILE_PATTERNS: usize = 100;

mod builder;
mod rule_filter;
mod schema;

pub use builder::LintConfigBuilder;
pub use schema::{ConfigWarning, generate_schema};
/// Tool version pinning for version-aware validation
///
/// When tool versions are pinned, validators can apply version-specific
/// behavior instead of using default assumptions. When not pinned,
/// validators will use sensible defaults and add assumption notes.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ToolVersions {
    /// Claude Code version (e.g., "1.0.0")
    #[serde(default)]
    #[schemars(description = "Claude Code version for version-aware validation (e.g., \"1.0.0\")")]
    pub claude_code: Option<String>,

    /// Codex CLI version (e.g., "0.1.0")
    #[serde(default)]
    #[schemars(description = "Codex CLI version for version-aware validation (e.g., \"0.1.0\")")]
    pub codex: Option<String>,

    /// Cursor version (e.g., "0.45.0")
    #[serde(default)]
    #[schemars(description = "Cursor version for version-aware validation (e.g., \"0.45.0\")")]
    pub cursor: Option<String>,

    /// GitHub Copilot version (e.g., "1.0.0")
    #[serde(default)]
    #[schemars(
        description = "GitHub Copilot version for version-aware validation (e.g., \"1.0.0\")"
    )]
    pub copilot: Option<String>,
}

/// Specification revision pinning for version-aware validation
///
/// When spec revisions are pinned, validators can apply revision-specific
/// rules. When not pinned, validators use the latest known revision.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct SpecRevisions {
    /// MCP protocol version (e.g., "2025-11-25", "2024-11-05")
    #[serde(default)]
    #[schemars(
        description = "MCP protocol version for revision-specific validation (e.g., \"2025-11-25\", \"2024-11-05\")"
    )]
    pub mcp_protocol: Option<String>,

    /// Agent Skills specification revision
    #[serde(default)]
    #[schemars(description = "Agent Skills specification revision")]
    pub agent_skills_spec: Option<String>,

    /// AGENTS.md specification revision
    #[serde(default)]
    #[schemars(description = "AGENTS.md specification revision")]
    pub agents_md_spec: Option<String>,
}

/// File inclusion/exclusion configuration for non-standard agent files.
///
/// By default, agnix only validates files it recognizes (CLAUDE.md, SKILL.md, etc.).
/// Use this section to include additional files in validation or exclude files
/// that would otherwise be validated.
///
/// Patterns use glob syntax (e.g., `"docs/ai-rules/*.md"`).
/// Paths are matched relative to the project root.
///
/// Priority: `exclude` > `include_as_memory` > `include_as_generic` > built-in detection.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct FilesConfig {
    /// Glob patterns for files to validate as memory/instruction files (ClaudeMd rules).
    ///
    /// Files matching these patterns will be treated as CLAUDE.md-like files,
    /// receiving the full set of memory/instruction validation rules.
    #[serde(default)]
    #[schemars(
        description = "Glob patterns for files to validate as memory/instruction files (ClaudeMd rules)"
    )]
    pub include_as_memory: Vec<String>,

    /// Glob patterns for files to validate as generic markdown (XML, XP, REF rules).
    ///
    /// Files matching these patterns will receive generic markdown validation
    /// (XML balance, import references, cross-platform checks).
    #[serde(default)]
    #[schemars(
        description = "Glob patterns for files to validate as generic markdown (XML, XP, REF rules)"
    )]
    pub include_as_generic: Vec<String>,

    /// Glob patterns for files to exclude from validation.
    ///
    /// Files matching these patterns will be skipped entirely, even if they
    /// would otherwise be recognized by built-in detection.
    #[serde(default)]
    #[schemars(description = "Glob patterns for files to exclude from validation")]
    pub exclude: Vec<String>,
}

// =============================================================================
// Internal Composition Types (Facade Pattern)
// =============================================================================
//
// LintConfig uses internal composition to separate concerns while maintaining
// a stable public API. These types are private implementation details:
//
// - RuntimeContext: Groups non-serialized runtime state (root_dir, import_cache, fs)
// - DefaultRuleFilter: Encapsulates rule filtering logic (~100 lines)
//
// This pattern provides:
// 1. Better code organization without breaking changes
// 2. Easier testing of individual components
// 3. Clear separation between serialized config and runtime state
// =============================================================================

/// Errors that can occur when building or validating a `LintConfig`.
///
/// These are hard errors (not warnings) that indicate the configuration
/// cannot be used as-is. For soft issues, see [`ConfigWarning`].
#[derive(Debug, Clone)]
pub enum ConfigError {
    /// A glob pattern in the configuration is syntactically invalid.
    InvalidGlobPattern {
        /// The invalid glob pattern string.
        pattern: String,
        /// Description of the parse error.
        error: String,
    },
    /// A glob pattern attempts path traversal (e.g. `../escape`).
    PathTraversal {
        /// The pattern containing path traversal.
        pattern: String,
    },
    /// Validation produced warnings that were promoted to errors.
    ValidationFailed(Vec<ConfigWarning>),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::InvalidGlobPattern { pattern, error } => {
                write!(f, "invalid glob pattern '{}': {}", pattern, error)
            }
            ConfigError::PathTraversal { pattern } => {
                write!(f, "path traversal in pattern '{}'", pattern)
            }
            ConfigError::ValidationFailed(warnings) => {
                if warnings.is_empty() {
                    write!(f, "configuration validation failed with 0 warning(s)")
                } else {
                    write!(
                        f,
                        "configuration validation failed with {} warning(s): {}",
                        warnings.len(),
                        warnings[0].message
                    )
                }
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// Runtime context for validation operations (not serialized).
///
/// Groups non-serialized state that is set up at runtime and shared during
/// validation. This includes the project root, import cache, and filesystem
/// abstraction.
///
/// # Thread Safety
///
/// `RuntimeContext` is `Send + Sync` because:
/// - `PathBuf` and `Option<T>` are `Send + Sync`
/// - `ImportCache` uses interior mutability with thread-safe types
/// - `Arc<dyn FileSystem>` shares the filesystem without deep-cloning
///
/// # Clone Behavior
///
/// When cloned, the `Arc<dyn FileSystem>` is shared (not deep-cloned),
/// maintaining the same filesystem instance across clones.
#[derive(Clone)]
struct RuntimeContext {
    /// Project root directory for validation.
    ///
    /// When set, validators can use this to resolve relative paths and
    /// detect project-escape attempts in import validation.
    root_dir: Option<PathBuf>,

    /// Shared import cache for project-level validation.
    ///
    /// When set, validators can use this cache to share parsed import data
    /// across files, avoiding redundant parsing during import chain traversal.
    import_cache: Option<crate::parsers::ImportCache>,

    /// File system abstraction for testability.
    ///
    /// Validators use this to perform file system operations. Defaults to
    /// `RealFileSystem` which delegates to `std::fs` and `file_utils`.
    fs: Arc<dyn FileSystem>,
}

impl Default for RuntimeContext {
    fn default() -> Self {
        Self {
            root_dir: None,
            import_cache: None,
            fs: Arc::new(RealFileSystem),
        }
    }
}

impl std::fmt::Debug for RuntimeContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeContext")
            .field("root_dir", &self.root_dir)
            .field(
                "import_cache",
                &self.import_cache.as_ref().map(|_| "ImportCache(...)"),
            )
            .field("fs", &"Arc<dyn FileSystem>")
            .finish()
    }
}

/// Configuration for the linter
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct LintConfig {
    /// Severity level threshold
    #[schemars(description = "Minimum severity level to report (Error, Warning, Info)")]
    severity: SeverityLevel,

    /// Rules to enable/disable
    #[schemars(description = "Configuration for enabling/disabling validation rules by category")]
    rules: RuleConfig,

    /// Paths to exclude
    #[schemars(
        description = "Glob patterns for paths to exclude from validation (e.g., [\"node_modules/**\", \"dist/**\"])"
    )]
    exclude: Vec<String>,

    /// Target tool (claude-code, cursor, codex, generic)
    /// Deprecated: Use `tools` array instead for multi-tool support
    #[schemars(description = "Target tool for validation (deprecated: use 'tools' array instead)")]
    target: TargetTool,

    /// Tools to validate for (e.g., ["claude-code", "cursor"])
    /// When specified, agnix automatically enables rules for these tools
    /// and disables rules for tools not in the list.
    /// Valid values: "claude-code", "cursor", "codex", "copilot", "github-copilot", "cline", "opencode", "gemini-cli", "generic"
    #[serde(default)]
    #[schemars(
        description = "Tools to validate for. Valid values: \"claude-code\", \"cursor\", \"codex\", \"copilot\", \"github-copilot\", \"cline\", \"opencode\", \"gemini-cli\", \"generic\""
    )]
    tools: Vec<String>,

    /// Expected MCP protocol version for validation (MCP-008)
    /// Deprecated: Use spec_revisions.mcp_protocol instead
    #[schemars(
        description = "Expected MCP protocol version (deprecated: use spec_revisions.mcp_protocol instead)"
    )]
    mcp_protocol_version: Option<String>,

    /// Tool version pinning for version-aware validation
    #[serde(default)]
    #[schemars(description = "Pin specific tool versions for version-aware validation")]
    tool_versions: ToolVersions,

    /// Specification revision pinning for version-aware validation
    #[serde(default)]
    #[schemars(description = "Pin specific specification revisions for revision-aware validation")]
    spec_revisions: SpecRevisions,

    /// File inclusion/exclusion configuration for non-standard agent files
    #[serde(default)]
    #[schemars(
        description = "File inclusion/exclusion configuration for non-standard agent files"
    )]
    files: FilesConfig,

    /// Output locale for translated messages (e.g., "en", "es", "zh-CN").
    /// When not set, the CLI locale detection is used.
    #[serde(default)]
    #[schemars(
        description = "Output locale for translated messages (e.g., \"en\", \"es\", \"zh-CN\")"
    )]
    locale: Option<String>,

    /// Maximum number of files to validate before stopping.
    ///
    /// This is a security feature to prevent DoS attacks via projects with
    /// millions of small files. When the limit is reached, validation stops
    /// with a `TooManyFiles` error.
    ///
    /// Default: 10,000 files. Set to `None` to disable the limit (not recommended).
    #[serde(default = "default_max_files")]
    max_files_to_validate: Option<usize>,

    /// Internal runtime context for validation operations (not serialized).
    ///
    /// Groups the filesystem abstraction, project root directory, and import
    /// cache. These are non-serialized runtime state set up before validation.
    #[serde(skip)]
    #[schemars(skip)]
    runtime: RuntimeContext,
}

/// Default maximum files to validate (security limit)
///
/// **Design Decision**: 10,000 files was chosen as a balance between:
/// - Large enough for realistic projects (Linux kernel has ~70k files, but most are not validated)
/// - Small enough to prevent DoS from projects with millions of tiny files
/// - Completes validation in reasonable time (seconds to low minutes on typical hardware)
/// - Atomic counter with SeqCst ordering provides thread-safe counting during parallel validation
///
/// Users can override with `--max-files N` or disable with `--max-files 0` (not recommended).
/// Set to `None` to disable the limit entirely (use with caution).
pub const DEFAULT_MAX_FILES: usize = 10_000;

/// Helper function for serde default
fn default_max_files() -> Option<usize> {
    Some(DEFAULT_MAX_FILES)
}

/// Check if a normalized (forward-slash) path pattern contains path traversal.
///
/// Catches `../`, `..` at the start, `/..` at the end, and standalone `..`.
fn has_path_traversal(normalized: &str) -> bool {
    normalized == ".."
        || normalized.starts_with("../")
        || normalized.contains("/../")
        || normalized.ends_with("/..")
}

impl Default for LintConfig {
    fn default() -> Self {
        Self {
            severity: SeverityLevel::Warning,
            rules: RuleConfig::default(),
            exclude: vec![
                "node_modules/**".to_string(),
                ".git/**".to_string(),
                "target/**".to_string(),
            ],
            target: TargetTool::Generic,
            tools: Vec::new(),
            mcp_protocol_version: None,
            tool_versions: ToolVersions::default(),
            spec_revisions: SpecRevisions::default(),
            files: FilesConfig::default(),
            locale: None,
            max_files_to_validate: Some(DEFAULT_MAX_FILES),
            runtime: RuntimeContext::default(),
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[schemars(description = "Severity level for filtering diagnostics")]
pub enum SeverityLevel {
    /// Only show errors
    Error,
    /// Show errors and warnings
    Warning,
    /// Show all diagnostics including info
    Info,
}

/// Helper function for serde default
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Configuration for enabling/disabling validation rules by category")]
pub struct RuleConfig {
    /// Enable skills validation (AS-*, CC-SK-*)
    #[serde(default = "default_true")]
    #[schemars(description = "Enable Agent Skills validation rules (AS-*, CC-SK-*)")]
    pub skills: bool,

    /// Enable hooks validation (CC-HK-*)
    #[serde(default = "default_true")]
    #[schemars(description = "Enable Claude Code hooks validation rules (CC-HK-*)")]
    pub hooks: bool,

    /// Enable agents validation (CC-AG-*)
    #[serde(default = "default_true")]
    #[schemars(description = "Enable Claude Code agents validation rules (CC-AG-*)")]
    pub agents: bool,

    /// Enable memory validation (CC-MEM-*)
    #[serde(default = "default_true")]
    #[schemars(description = "Enable Claude Code memory validation rules (CC-MEM-*)")]
    pub memory: bool,

    /// Enable plugins validation (CC-PL-*)
    #[serde(default = "default_true")]
    #[schemars(description = "Enable Claude Code plugins validation rules (CC-PL-*)")]
    pub plugins: bool,

    /// Enable XML balance checking (XML-*)
    #[serde(default = "default_true")]
    #[schemars(description = "Enable XML tag balance validation rules (XML-*)")]
    pub xml: bool,

    /// Enable MCP validation (MCP-*)
    #[serde(default = "default_true")]
    #[schemars(description = "Enable Model Context Protocol validation rules (MCP-*)")]
    pub mcp: bool,

    /// Enable import reference validation (REF-*)
    #[serde(default = "default_true")]
    #[schemars(description = "Enable import reference validation rules (REF-*)")]
    pub imports: bool,

    /// Enable cross-platform validation (XP-*)
    #[serde(default = "default_true")]
    #[schemars(description = "Enable cross-platform validation rules (XP-*)")]
    pub cross_platform: bool,

    /// Enable AGENTS.md validation (AGM-*)
    #[serde(default = "default_true")]
    #[schemars(description = "Enable AGENTS.md validation rules (AGM-*)")]
    pub agents_md: bool,

    /// Enable GitHub Copilot validation (COP-*)
    #[serde(default = "default_true")]
    #[schemars(description = "Enable GitHub Copilot validation rules (COP-*)")]
    pub copilot: bool,

    /// Enable Cursor project rules validation (CUR-*)
    #[serde(default = "default_true")]
    #[schemars(description = "Enable Cursor project rules validation (CUR-*)")]
    pub cursor: bool,

    /// Enable Cline rules validation (CLN-*)
    #[serde(default = "default_true")]
    #[schemars(description = "Enable Cline rules validation (CLN-*)")]
    pub cline: bool,

    /// Enable OpenCode validation (OC-*)
    #[serde(default = "default_true")]
    #[schemars(description = "Enable OpenCode validation rules (OC-*)")]
    pub opencode: bool,

    /// Enable Gemini CLI validation (GM-*)
    #[serde(default = "default_true")]
    #[schemars(description = "Enable Gemini CLI validation rules (GM-*)")]
    pub gemini_md: bool,

    /// Enable Codex CLI validation (CDX-*)
    #[serde(default = "default_true")]
    #[schemars(description = "Enable Codex CLI validation rules (CDX-*)")]
    pub codex: bool,

    /// Enable Roo Code validation (ROO-*)
    #[serde(default = "default_true")]
    #[schemars(description = "Enable Roo Code validation rules (ROO-*)")]
    pub roo_code: bool,

    /// Enable Windsurf validation (WS-*)
    #[serde(default = "default_true")]
    #[schemars(description = "Enable Windsurf validation rules (WS-*)")]
    pub windsurf: bool,

    /// Enable Kiro steering validation (KIRO-*)
    #[serde(default = "default_true")]
    #[schemars(description = "Enable Kiro steering validation rules (KIRO-*)")]
    pub kiro_steering: bool,

    /// Enable Amp checks validation (AMP-*)
    #[serde(default = "default_true")]
    #[schemars(description = "Enable Amp checks validation rules (AMP-*)")]
    pub amp_checks: bool,

    /// Enable prompt engineering validation (PE-*)
    #[serde(default = "default_true")]
    #[schemars(description = "Enable prompt engineering validation rules (PE-*)")]
    pub prompt_engineering: bool,

    /// Detect generic instructions in CLAUDE.md
    #[serde(default = "default_true")]
    #[schemars(description = "Detect generic placeholder instructions in CLAUDE.md")]
    pub generic_instructions: bool,

    /// Validate YAML frontmatter
    #[serde(default = "default_true")]
    #[schemars(description = "Validate YAML frontmatter in skill files")]
    pub frontmatter_validation: bool,

    /// Check XML tag balance (legacy - use xml instead)
    #[serde(default = "default_true")]
    #[schemars(description = "Check XML tag balance (legacy: use 'xml' instead)")]
    pub xml_balance: bool,

    /// Validate @import references (legacy - use imports instead)
    #[serde(default = "default_true")]
    #[schemars(description = "Validate @import references (legacy: use 'imports' instead)")]
    pub import_references: bool,

    /// Explicitly disabled rules by ID (e.g., ["CC-AG-001", "AS-005"])
    #[serde(default)]
    #[schemars(
        description = "List of rule IDs to explicitly disable (e.g., [\"CC-AG-001\", \"AS-005\"])"
    )]
    pub disabled_rules: Vec<String>,

    /// Explicitly disabled validators by name (e.g., ["XmlValidator", "PromptValidator"])
    #[serde(default)]
    #[schemars(
        description = "List of validator names to disable (e.g., [\"XmlValidator\", \"PromptValidator\"])"
    )]
    pub disabled_validators: Vec<String>,
}

impl Default for RuleConfig {
    fn default() -> Self {
        Self {
            skills: true,
            hooks: true,
            agents: true,
            memory: true,
            plugins: true,
            xml: true,
            mcp: true,
            imports: true,
            cross_platform: true,
            agents_md: true,
            copilot: true,
            cursor: true,
            cline: true,
            opencode: true,
            gemini_md: true,
            codex: true,
            roo_code: true,
            windsurf: true,
            kiro_steering: true,
            amp_checks: true,
            prompt_engineering: true,
            generic_instructions: true,
            frontmatter_validation: true,
            xml_balance: true,
            import_references: true,
            disabled_rules: Vec::new(),
            disabled_validators: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(
    description = "Target tool for validation (deprecated: use 'tools' array for multi-tool support)"
)]
pub enum TargetTool {
    /// Generic Agent Skills standard
    Generic,
    /// Claude Code specific
    ClaudeCode,
    /// Cursor specific
    Cursor,
    /// Codex specific
    Codex,
}

impl LintConfig {
    /// Load config from file
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = safe_read_file(path.as_ref())?;
        let config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load config or use default, returning any parse warning
    ///
    /// Returns a tuple of (config, optional_warning). If a config path is provided
    /// but the file cannot be loaded or parsed, returns the default config with a
    /// warning message describing the error. This prevents silent fallback to
    /// defaults on config typos or missing/unreadable config files.
    pub fn load_or_default(path: Option<&PathBuf>) -> (Self, Option<String>) {
        match path {
            Some(p) => match Self::load(p) {
                Ok(config) => (config, None),
                Err(e) => {
                    let warning = t!(
                        "core.config.load_warning",
                        path = p.display().to_string(),
                        error = e.to_string()
                    );
                    (Self::default(), Some(warning.to_string()))
                }
            },
            None => (Self::default(), None),
        }
    }

    // =========================================================================
    // Runtime Context Accessors
    // =========================================================================
    //
    // These methods delegate to RuntimeContext, maintaining the same public API.
    // =========================================================================

    /// Get the runtime validation root directory, if set.
    #[inline]
    pub fn root_dir(&self) -> Option<&PathBuf> {
        self.runtime.root_dir.as_ref()
    }

    /// Alias for `root_dir()` for consistency with other accessors.
    #[inline]
    pub fn get_root_dir(&self) -> Option<&PathBuf> {
        self.root_dir()
    }

    /// Set the runtime validation root directory (not persisted).
    pub fn set_root_dir(&mut self, root_dir: PathBuf) {
        self.runtime.root_dir = Some(root_dir);
    }

    /// Set the shared import cache for project-level validation (not persisted).
    ///
    /// When set, the ImportsValidator will use this cache to share parsed
    /// import data across files, improving performance by avoiding redundant
    /// parsing during import chain traversal.
    pub fn set_import_cache(&mut self, cache: crate::parsers::ImportCache) {
        self.runtime.import_cache = Some(cache);
    }

    /// Get the shared import cache, if one has been set.
    ///
    /// Returns `None` for single-file validation or when the cache hasn't
    /// been initialized. Returns `Some(&ImportCache)` during project-level
    /// validation where import results are shared across files.
    #[inline]
    pub fn import_cache(&self) -> Option<&crate::parsers::ImportCache> {
        self.runtime.import_cache.as_ref()
    }

    /// Alias for `import_cache()` for consistency with other accessors.
    #[inline]
    pub fn get_import_cache(&self) -> Option<&crate::parsers::ImportCache> {
        self.import_cache()
    }

    /// Get the file system abstraction.
    ///
    /// Validators should use this for file system operations instead of
    /// directly calling `std::fs` functions. This enables unit testing
    /// with `MockFileSystem`.
    pub fn fs(&self) -> &Arc<dyn FileSystem> {
        &self.runtime.fs
    }

    /// Set the file system abstraction (not persisted).
    ///
    /// This is primarily used for testing with `MockFileSystem`.
    ///
    /// # Important
    ///
    /// This should only be called during configuration setup, before validation
    /// begins. Changing the filesystem during validation may cause inconsistent
    /// results if validators have already cached file state.
    pub fn set_fs(&mut self, fs: Arc<dyn FileSystem>) {
        self.runtime.fs = fs;
    }

    // =========================================================================
    // Serializable Field Getters
    // =========================================================================

    /// Get the severity level threshold.
    #[inline]
    pub fn severity(&self) -> SeverityLevel {
        self.severity
    }

    /// Get the rules configuration.
    #[inline]
    pub fn rules(&self) -> &RuleConfig {
        &self.rules
    }

    /// Get the exclude patterns.
    #[inline]
    pub fn exclude(&self) -> &[String] {
        &self.exclude
    }

    /// Get the target tool.
    #[inline]
    pub fn target(&self) -> TargetTool {
        self.target
    }

    /// Get the tools list.
    #[inline]
    pub fn tools(&self) -> &[String] {
        &self.tools
    }

    /// Get the tool versions configuration.
    #[inline]
    pub fn tool_versions(&self) -> &ToolVersions {
        &self.tool_versions
    }

    /// Get the spec revisions configuration.
    #[inline]
    pub fn spec_revisions(&self) -> &SpecRevisions {
        &self.spec_revisions
    }

    /// Get the files configuration.
    #[inline]
    pub fn files_config(&self) -> &FilesConfig {
        &self.files
    }

    /// Get the locale, if set.
    #[inline]
    pub fn locale(&self) -> Option<&str> {
        self.locale.as_deref()
    }

    /// Get the maximum number of files to validate.
    #[inline]
    pub fn max_files_to_validate(&self) -> Option<usize> {
        self.max_files_to_validate
    }

    /// Get the raw `mcp_protocol_version` field value (without fallback logic).
    ///
    /// For the resolved version with fallback, use [`get_mcp_protocol_version()`](Self::get_mcp_protocol_version).
    #[inline]
    pub fn mcp_protocol_version_raw(&self) -> Option<&str> {
        self.mcp_protocol_version.as_deref()
    }

    // =========================================================================
    // Serializable Field Setters
    // =========================================================================

    /// Set the severity level threshold.
    pub fn set_severity(&mut self, severity: SeverityLevel) {
        self.severity = severity;
    }

    /// Set the target tool.
    pub fn set_target(&mut self, target: TargetTool) {
        self.target = target;
    }

    /// Set the tools list.
    pub fn set_tools(&mut self, tools: Vec<String>) {
        self.tools = tools;
    }

    /// Get a mutable reference to the tools list.
    pub fn tools_mut(&mut self) -> &mut Vec<String> {
        &mut self.tools
    }

    /// Set the exclude patterns.
    ///
    /// Note: This does not validate the patterns. Call [`validate()`](Self::validate)
    /// after using this if validation is needed.
    pub fn set_exclude(&mut self, exclude: Vec<String>) {
        self.exclude = exclude;
    }

    /// Set the locale.
    pub fn set_locale(&mut self, locale: Option<String>) {
        self.locale = locale;
    }

    /// Set the maximum number of files to validate.
    pub fn set_max_files_to_validate(&mut self, max: Option<usize>) {
        self.max_files_to_validate = max;
    }

    /// Set the MCP protocol version (deprecated field).
    pub fn set_mcp_protocol_version(&mut self, version: Option<String>) {
        self.mcp_protocol_version = version;
    }

    /// Get a mutable reference to the rules configuration.
    pub fn rules_mut(&mut self) -> &mut RuleConfig {
        &mut self.rules
    }

    /// Get a mutable reference to the tool versions configuration.
    pub fn tool_versions_mut(&mut self) -> &mut ToolVersions {
        &mut self.tool_versions
    }

    /// Get a mutable reference to the spec revisions configuration.
    pub fn spec_revisions_mut(&mut self) -> &mut SpecRevisions {
        &mut self.spec_revisions
    }

    /// Get a mutable reference to the files configuration.
    ///
    /// Note: Mutations bypass builder validation. Call [`validate()`](Self::validate)
    /// after modifying if validation is needed.
    pub fn files_mut(&mut self) -> &mut FilesConfig {
        &mut self.files
    }

    // =========================================================================
    // Derived / Computed Accessors
    // =========================================================================

    /// Get the expected MCP protocol version
    ///
    /// Priority: spec_revisions.mcp_protocol > mcp_protocol_version > default
    #[inline]
    pub fn get_mcp_protocol_version(&self) -> &str {
        self.spec_revisions
            .mcp_protocol
            .as_deref()
            .or(self.mcp_protocol_version.as_deref())
            .unwrap_or(DEFAULT_MCP_PROTOCOL_VERSION)
    }

    /// Check if MCP protocol revision is explicitly pinned
    #[inline]
    pub fn is_mcp_revision_pinned(&self) -> bool {
        self.spec_revisions.mcp_protocol.is_some() || self.mcp_protocol_version.is_some()
    }

    /// Check if Claude Code version is explicitly pinned
    #[inline]
    pub fn is_claude_code_version_pinned(&self) -> bool {
        self.tool_versions.claude_code.is_some()
    }

    /// Get the pinned Claude Code version, if any
    #[inline]
    pub fn get_claude_code_version(&self) -> Option<&str> {
        self.tool_versions.claude_code.as_deref()
    }
}

#[cfg(test)]
mod tests;
