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
    /// MCP protocol version (e.g., "2025-06-18", "2024-11-05")
    #[serde(default)]
    #[schemars(
        description = "MCP protocol version for revision-specific validation (e.g., \"2025-06-18\", \"2024-11-05\")"
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
                write!(
                    f,
                    "configuration validation failed with {} warning(s)",
                    warnings.len()
                )
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

/// Rule filtering logic encapsulated for clarity.
///
/// This trait and its implementation extract the rule enablement logic
/// from LintConfig, making it easier to test and maintain.
trait RuleFilter {
    /// Check if a specific rule is enabled based on config.
    fn is_rule_enabled(&self, rule_id: &str) -> bool;
}

/// Default implementation of rule filtering logic.
///
/// Determines whether a rule is enabled based on:
/// 1. Explicit disabled_rules list
/// 2. Target tool or tools array filtering
/// 3. Category enablement flags
struct DefaultRuleFilter<'a> {
    rules: &'a RuleConfig,
    target: TargetTool,
    tools: &'a [String],
}

impl<'a> DefaultRuleFilter<'a> {
    fn new(rules: &'a RuleConfig, target: TargetTool, tools: &'a [String]) -> Self {
        Self {
            rules,
            target,
            tools,
        }
    }

    /// Check if a rule applies to the current target tool(s)
    fn is_rule_for_target(&self, rule_id: &str) -> bool {
        // If tools array is specified, use it for filtering
        if !self.tools.is_empty() {
            return self.is_rule_for_tools(rule_id);
        }

        // Legacy: CC-* rules only apply to ClaudeCode or Generic targets
        if rule_id.starts_with("CC-") {
            return matches!(self.target, TargetTool::ClaudeCode | TargetTool::Generic);
        }
        // All other rules apply to all targets (see TOOL_RULE_PREFIXES for tool-specific rules)
        true
    }

    /// Check if a rule applies based on the tools array
    fn is_rule_for_tools(&self, rule_id: &str) -> bool {
        for (prefix, tool) in agnix_rules::TOOL_RULE_PREFIXES {
            if rule_id.starts_with(prefix) {
                // Check if the required tool is in the tools list (case-insensitive)
                // Also accept backward-compat aliases (e.g., "copilot" for "github-copilot")
                return self
                    .tools
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case(tool) || Self::is_tool_alias(t, tool));
            }
        }

        // Generic rules (AS-*, XML-*, REF-*, XP-*, AGM-*, MCP-*, PE-*) apply to all tools
        true
    }

    /// Check if a user-provided tool name is a backward-compatible alias
    /// for the canonical tool name from rules.json.
    ///
    /// Currently only "github-copilot" has an alias ("copilot"). This exists for
    /// backward compatibility: early versions of agnix used the shorter "copilot"
    /// name in configs, and we need to continue supporting that for existing users.
    /// The canonical names in rules.json use the full "github-copilot" to match
    /// the official tool name from GitHub's documentation.
    ///
    /// Note: This function does NOT treat canonical names as aliases of themselves.
    /// For example, "github-copilot" is NOT an alias for "github-copilot" - that's
    /// handled by the direct eq_ignore_ascii_case comparison in is_rule_for_tools().
    fn is_tool_alias(user_tool: &str, canonical_tool: &str) -> bool {
        // Backward compatibility: accept short names as aliases
        match canonical_tool {
            "github-copilot" => user_tool.eq_ignore_ascii_case("copilot"),
            _ => false,
        }
    }

    /// Check if a rule's category is enabled
    fn is_category_enabled(&self, rule_id: &str) -> bool {
        match rule_id {
            s if [
                "AS-", "CC-SK-", "CR-SK-", "CL-SK-", "CP-SK-", "CX-SK-", "OC-SK-", "WS-SK-",
                "KR-SK-", "AMP-SK-", "RC-SK-",
            ]
            .iter()
            .any(|p| s.starts_with(p)) =>
            {
                self.rules.skills
            }
            s if s.starts_with("CC-HK-") => self.rules.hooks,
            s if s.starts_with("CC-AG-") => self.rules.agents,
            s if s.starts_with("CC-MEM-") => self.rules.memory,
            s if s.starts_with("CC-PL-") => self.rules.plugins,
            s if s.starts_with("XML-") => self.rules.xml,
            s if s.starts_with("MCP-") => self.rules.mcp,
            s if s.starts_with("REF-") || s.starts_with("imports::") => self.rules.imports,
            s if s.starts_with("XP-") => self.rules.cross_platform,
            s if s.starts_with("AGM-") => self.rules.agents_md,
            s if s.starts_with("COP-") => self.rules.copilot,
            s if s.starts_with("CUR-") => self.rules.cursor,
            s if s.starts_with("CLN-") => self.rules.cline,
            s if s.starts_with("OC-") => self.rules.opencode,
            s if s.starts_with("GM-") => self.rules.gemini_md,
            s if s.starts_with("CDX-") => self.rules.codex,
            s if s.starts_with("PE-") => self.rules.prompt_engineering,
            // Unknown rules are enabled by default
            _ => true,
        }
    }
}

impl RuleFilter for DefaultRuleFilter<'_> {
    fn is_rule_enabled(&self, rule_id: &str) -> bool {
        // Check if explicitly disabled
        if self.rules.disabled_rules.iter().any(|r| r == rule_id) {
            return false;
        }

        // Check if rule applies to target
        if !self.is_rule_for_target(rule_id) {
            return false;
        }

        // Check if category is enabled
        self.is_category_enabled(rule_id)
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

/// Builder for constructing a [`LintConfig`] with validation.
///
/// Uses the `&mut Self` return pattern (consistent with [`ValidatorRegistryBuilder`])
/// for chaining setter calls, with a terminal `build()` that validates and returns
/// `Result<LintConfig, ConfigError>`.
///
/// # Examples
///
/// ```rust
/// use agnix_core::config::{LintConfig, SeverityLevel};
///
/// let config = LintConfig::builder()
///     .severity(SeverityLevel::Error)
///     .build()
///     .expect("valid config");
/// assert_eq!(config.severity(), SeverityLevel::Error);
/// ```
pub struct LintConfigBuilder {
    severity: Option<SeverityLevel>,
    rules: Option<RuleConfig>,
    exclude: Option<Vec<String>>,
    target: Option<TargetTool>,
    tools: Option<Vec<String>>,
    mcp_protocol_version: Option<Option<String>>,
    tool_versions: Option<ToolVersions>,
    spec_revisions: Option<SpecRevisions>,
    files: Option<FilesConfig>,
    locale: Option<Option<String>>,
    max_files_to_validate: Option<Option<usize>>,
    // Runtime
    root_dir: Option<PathBuf>,
    import_cache: Option<crate::parsers::ImportCache>,
    fs: Option<Arc<dyn FileSystem>>,
    disabled_rules: Vec<String>,
    disabled_validators: Vec<String>,
}

impl LintConfigBuilder {
    /// Create a new builder with all fields unset (defaults will be applied at build time).
    ///
    /// Prefer [`LintConfig::builder()`] over calling this directly.
    fn new() -> Self {
        Self {
            severity: None,
            rules: None,
            exclude: None,
            target: None,
            tools: None,
            mcp_protocol_version: None,
            tool_versions: None,
            spec_revisions: None,
            files: None,
            locale: None,
            max_files_to_validate: None,
            root_dir: None,
            import_cache: None,
            fs: None,
            disabled_rules: Vec::new(),
            disabled_validators: Vec::new(),
        }
    }

    /// Set the severity level threshold.
    pub fn severity(&mut self, severity: SeverityLevel) -> &mut Self {
        self.severity = Some(severity);
        self
    }

    /// Set the rules configuration.
    pub fn rules(&mut self, rules: RuleConfig) -> &mut Self {
        self.rules = Some(rules);
        self
    }

    /// Set the exclude patterns.
    pub fn exclude(&mut self, exclude: Vec<String>) -> &mut Self {
        self.exclude = Some(exclude);
        self
    }

    /// Set the target tool.
    pub fn target(&mut self, target: TargetTool) -> &mut Self {
        self.target = Some(target);
        self
    }

    /// Set the tools list.
    pub fn tools(&mut self, tools: Vec<String>) -> &mut Self {
        self.tools = Some(tools);
        self
    }

    /// Set the MCP protocol version (deprecated field).
    pub fn mcp_protocol_version(&mut self, version: Option<String>) -> &mut Self {
        self.mcp_protocol_version = Some(version);
        self
    }

    /// Set the tool versions configuration.
    pub fn tool_versions(&mut self, versions: ToolVersions) -> &mut Self {
        self.tool_versions = Some(versions);
        self
    }

    /// Set the spec revisions configuration.
    pub fn spec_revisions(&mut self, revisions: SpecRevisions) -> &mut Self {
        self.spec_revisions = Some(revisions);
        self
    }

    /// Set the files configuration.
    pub fn files(&mut self, files: FilesConfig) -> &mut Self {
        self.files = Some(files);
        self
    }

    /// Set the locale.
    pub fn locale(&mut self, locale: Option<String>) -> &mut Self {
        self.locale = Some(locale);
        self
    }

    /// Set the maximum number of files to validate.
    pub fn max_files_to_validate(&mut self, max: Option<usize>) -> &mut Self {
        self.max_files_to_validate = Some(max);
        self
    }

    /// Set the runtime validation root directory.
    pub fn root_dir(&mut self, root_dir: PathBuf) -> &mut Self {
        self.root_dir = Some(root_dir);
        self
    }

    /// Set the shared import cache.
    pub fn import_cache(&mut self, cache: crate::parsers::ImportCache) -> &mut Self {
        self.import_cache = Some(cache);
        self
    }

    /// Set the filesystem abstraction.
    pub fn fs(&mut self, fs: Arc<dyn FileSystem>) -> &mut Self {
        self.fs = Some(fs);
        self
    }

    /// Add a rule ID to the disabled rules list.
    pub fn disable_rule(&mut self, rule_id: impl Into<String>) -> &mut Self {
        self.disabled_rules.push(rule_id.into());
        self
    }

    /// Add a validator name to the disabled validators list.
    pub fn disable_validator(&mut self, name: impl Into<String>) -> &mut Self {
        self.disabled_validators.push(name.into());
        self
    }

    /// Build the `LintConfig`, applying defaults for unset fields and
    /// running validation.
    ///
    /// Returns `Err(ConfigError)` if:
    /// - A glob pattern (in exclude or files config) has invalid syntax
    /// - A glob pattern attempts path traversal (`../`)
    /// - Configuration validation produces warnings (promoted to errors)
    pub fn build(&mut self) -> Result<LintConfig, ConfigError> {
        let config = self.build_inner();

        // Validate all glob pattern lists: exclude + files config
        let pattern_lists: &[(&str, &[String])] = &[
            ("exclude", &config.exclude),
            ("files.include_as_memory", &config.files.include_as_memory),
            ("files.include_as_generic", &config.files.include_as_generic),
            ("files.exclude", &config.files.exclude),
        ];
        for &(field, patterns) in pattern_lists {
            for pattern in patterns {
                let normalized = pattern.replace('\\', "/");
                if let Err(e) = glob::Pattern::new(&normalized) {
                    return Err(ConfigError::InvalidGlobPattern {
                        pattern: pattern.clone(),
                        error: format!("{} (in {})", e, field),
                    });
                }
                if has_path_traversal(&normalized) {
                    return Err(ConfigError::PathTraversal {
                        pattern: pattern.clone(),
                    });
                }
            }
        }

        // Run full config validation
        let warnings = config.validate();
        if !warnings.is_empty() {
            return Err(ConfigError::ValidationFailed(warnings));
        }

        Ok(config)
    }

    /// Build the `LintConfig` without running any validation.
    ///
    /// This is primarily intended for tests that need to construct configs
    /// with intentionally invalid data.
    pub fn build_unchecked(&mut self) -> LintConfig {
        self.build_inner()
    }

    /// Internal: construct the LintConfig from builder state, applying defaults.
    fn build_inner(&mut self) -> LintConfig {
        let defaults = LintConfig::default();

        let mut rules = self.rules.take().unwrap_or(defaults.rules);

        // Apply convenience disabled_rules/disabled_validators (dedup via append+retain)
        if !self.disabled_rules.is_empty() {
            rules.disabled_rules.append(&mut self.disabled_rules);
            let mut seen = std::collections::HashSet::new();
            rules.disabled_rules.retain(|r| seen.insert(r.clone()));
        }
        if !self.disabled_validators.is_empty() {
            rules
                .disabled_validators
                .append(&mut self.disabled_validators);
            let mut seen = std::collections::HashSet::new();
            rules.disabled_validators.retain(|v| seen.insert(v.clone()));
        }

        let mut config = LintConfig {
            severity: self.severity.take().unwrap_or(defaults.severity),
            rules,
            exclude: self.exclude.take().unwrap_or(defaults.exclude),
            target: self.target.take().unwrap_or(defaults.target),
            tools: self.tools.take().unwrap_or(defaults.tools),
            mcp_protocol_version: self
                .mcp_protocol_version
                .take()
                .unwrap_or(defaults.mcp_protocol_version),
            tool_versions: self.tool_versions.take().unwrap_or(defaults.tool_versions),
            spec_revisions: self
                .spec_revisions
                .take()
                .unwrap_or(defaults.spec_revisions),
            files: self.files.take().unwrap_or(defaults.files),
            locale: self.locale.take().unwrap_or(defaults.locale),
            max_files_to_validate: self
                .max_files_to_validate
                .take()
                .unwrap_or(defaults.max_files_to_validate),
            runtime: RuntimeContext::default(),
        };

        // Apply runtime state
        if let Some(root_dir) = self.root_dir.take() {
            config.runtime.root_dir = Some(root_dir);
        }
        if let Some(cache) = self.import_cache.take() {
            config.runtime.import_cache = Some(cache);
        }
        if let Some(fs) = self.fs.take() {
            config.runtime.fs = fs;
        }

        config
    }
}

impl Default for LintConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl LintConfig {
    /// Create a new [`LintConfigBuilder`] for constructing a `LintConfig`.
    pub fn builder() -> LintConfigBuilder {
        LintConfigBuilder::new()
    }

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

    // =========================================================================
    // Rule Filtering (delegates to DefaultRuleFilter)
    // =========================================================================

    /// Check if a specific rule is enabled based on config
    ///
    /// A rule is enabled if:
    /// 1. It's not in the disabled_rules list
    /// 2. It's applicable to the current target tool
    /// 3. Its category is enabled
    ///
    /// This delegates to `DefaultRuleFilter` which encapsulates the filtering logic.
    pub fn is_rule_enabled(&self, rule_id: &str) -> bool {
        let filter = DefaultRuleFilter::new(&self.rules, self.target, &self.tools);
        filter.is_rule_enabled(rule_id)
    }

    /// Check if a user-provided tool name is a backward-compatible alias
    /// for the canonical tool name from rules.json.
    ///
    /// Currently only "github-copilot" has an alias ("copilot"). This exists for
    /// backward compatibility: early versions of agnix used the shorter "copilot"
    /// name in configs, and we need to continue supporting that for existing users.
    /// The canonical names in rules.json use the full "github-copilot" to match
    /// the official tool name from GitHub's documentation.
    ///
    /// Note: This function does NOT treat canonical names as aliases of themselves.
    /// For example, "github-copilot" is NOT an alias for "github-copilot" - that's
    /// handled by the direct eq_ignore_ascii_case comparison in is_rule_for_tools().
    pub fn is_tool_alias(user_tool: &str, canonical_tool: &str) -> bool {
        DefaultRuleFilter::is_tool_alias(user_tool, canonical_tool)
    }

    /// Validate the configuration and return any warnings.
    ///
    /// This performs semantic validation beyond what TOML parsing can check:
    /// - Validates that disabled_rules match known rule ID patterns
    /// - Validates that tools array contains known tool names
    /// - Warns on deprecated fields
    pub fn validate(&self) -> Vec<ConfigWarning> {
        let mut warnings = Vec::new();

        // Validate disabled_rules match known patterns
        // Note: imports:: is a legacy prefix used in some internal diagnostics
        let known_prefixes = [
            "AS-",
            "CC-SK-",
            "CC-HK-",
            "CC-AG-",
            "CC-MEM-",
            "CC-PL-",
            "CDX-",
            "XML-",
            "MCP-",
            "REF-",
            "XP-",
            "AGM-",
            "COP-",
            "CUR-",
            "CLN-",
            "OC-",
            "GM-",
            "PE-",
            "VER-",
            "imports::",
        ];
        for rule_id in &self.rules.disabled_rules {
            let matches_known = known_prefixes
                .iter()
                .any(|prefix| rule_id.starts_with(prefix));
            if !matches_known {
                warnings.push(ConfigWarning {
                    field: "rules.disabled_rules".to_string(),
                    message: t!(
                        "core.config.unknown_rule",
                        rule = rule_id.as_str(),
                        prefixes = known_prefixes.join(", ")
                    )
                    .to_string(),
                    suggestion: Some(t!("core.config.unknown_rule_suggestion").to_string()),
                });
            }
        }

        // Validate tools array contains known tools
        let known_tools = [
            "claude-code",
            "cursor",
            "codex",
            "copilot",
            "github-copilot",
            "cline",
            "opencode",
            "gemini-cli",
            "generic",
        ];
        for tool in &self.tools {
            let tool_lower = tool.to_lowercase();
            if !known_tools
                .iter()
                .any(|k| k.eq_ignore_ascii_case(&tool_lower))
            {
                warnings.push(ConfigWarning {
                    field: "tools".to_string(),
                    message: t!(
                        "core.config.unknown_tool",
                        tool = tool.as_str(),
                        valid = known_tools.join(", ")
                    )
                    .to_string(),
                    suggestion: Some(t!("core.config.unknown_tool_suggestion").to_string()),
                });
            }
        }

        // Warn on deprecated fields
        if self.target != TargetTool::Generic && self.tools.is_empty() {
            // Only warn if target is non-default and tools is empty
            // (if both are set, tools takes precedence silently)
            warnings.push(ConfigWarning {
                field: "target".to_string(),
                message: t!("core.config.deprecated_target").to_string(),
                suggestion: Some(t!("core.config.deprecated_target_suggestion").to_string()),
            });
        }
        if self.mcp_protocol_version.is_some() {
            warnings.push(ConfigWarning {
                field: "mcp_protocol_version".to_string(),
                message: t!("core.config.deprecated_mcp_version").to_string(),
                suggestion: Some(t!("core.config.deprecated_mcp_version_suggestion").to_string()),
            });
        }

        // Validate files config glob patterns
        let pattern_lists = [
            ("files.include_as_memory", &self.files.include_as_memory),
            ("files.include_as_generic", &self.files.include_as_generic),
            ("files.exclude", &self.files.exclude),
        ];
        for (field, patterns) in &pattern_lists {
            // Warn if pattern count exceeds recommended limit
            if patterns.len() > MAX_FILE_PATTERNS {
                warnings.push(ConfigWarning {
                    field: field.to_string(),
                    message: t!(
                        "core.config.files_pattern_count_limit",
                        field = *field,
                        count = patterns.len(),
                        limit = MAX_FILE_PATTERNS
                    )
                    .to_string(),
                    suggestion: Some(
                        t!("core.config.files_pattern_count_limit_suggestion").to_string(),
                    ),
                });
            }
            for pattern in *patterns {
                let normalized = pattern.replace('\\', "/");
                if let Err(e) = glob::Pattern::new(&normalized) {
                    warnings.push(ConfigWarning {
                        field: field.to_string(),
                        message: t!(
                            "core.config.invalid_files_pattern",
                            pattern = pattern.as_str(),
                            message = e.to_string()
                        )
                        .to_string(),
                        suggestion: Some(
                            t!("core.config.invalid_files_pattern_suggestion").to_string(),
                        ),
                    });
                }
                // Reject path traversal patterns
                if has_path_traversal(&normalized) {
                    warnings.push(ConfigWarning {
                        field: field.to_string(),
                        message: t!(
                            "core.config.files_path_traversal",
                            pattern = pattern.as_str()
                        )
                        .to_string(),
                        suggestion: Some(
                            t!("core.config.files_path_traversal_suggestion").to_string(),
                        ),
                    });
                }
                // Reject absolute paths (Unix-style leading slash or Windows drive letter)
                if normalized.starts_with('/')
                    || (normalized.len() >= 3
                        && normalized.as_bytes()[0].is_ascii_alphabetic()
                        && normalized.as_bytes().get(1..3) == Some(b":/"))
                {
                    warnings.push(ConfigWarning {
                        field: field.to_string(),
                        message: t!(
                            "core.config.files_absolute_path",
                            pattern = pattern.as_str()
                        )
                        .to_string(),
                        suggestion: Some(
                            t!("core.config.files_absolute_path_suggestion").to_string(),
                        ),
                    });
                }
            }
        }

        warnings
    }
}

/// Warning from configuration validation.
///
/// These warnings indicate potential issues with the configuration that
/// don't prevent validation from running but may indicate user mistakes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigWarning {
    /// The field path that has the issue (e.g., "rules.disabled_rules")
    pub field: String,
    /// Description of the issue
    pub message: String,
    /// Optional suggestion for how to fix the issue
    pub suggestion: Option<String>,
}

/// Generate a JSON Schema for the LintConfig type.
///
/// This can be used to provide editor autocompletion and validation
/// for `.agnix.toml` configuration files.
///
/// # Example
///
/// ```rust
/// use agnix_core::config::generate_schema;
///
/// let schema = generate_schema();
/// let json = serde_json::to_string_pretty(&schema).unwrap();
/// println!("{}", json);
/// ```
pub fn generate_schema() -> schemars::schema::RootSchema {
    schemars::schema_for!(LintConfig)
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_enables_all_rules() {
        let config = LintConfig::default();

        // Test various rule IDs
        assert!(config.is_rule_enabled("CC-AG-001"));
        assert!(config.is_rule_enabled("CC-HK-001"));
        assert!(config.is_rule_enabled("AS-005"));
        assert!(config.is_rule_enabled("CC-SK-006"));
        assert!(config.is_rule_enabled("CC-MEM-005"));
        assert!(config.is_rule_enabled("CC-PL-001"));
        assert!(config.is_rule_enabled("XML-001"));
        assert!(config.is_rule_enabled("REF-001"));
    }

    #[test]
    fn test_disabled_rules_list() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["CC-AG-001".to_string(), "AS-005".to_string()];

        assert!(!config.is_rule_enabled("CC-AG-001"));
        assert!(!config.is_rule_enabled("AS-005"));
        assert!(config.is_rule_enabled("CC-AG-002"));
        assert!(config.is_rule_enabled("AS-006"));
    }

    #[test]
    fn test_category_disabled_skills() {
        let mut config = LintConfig::default();
        config.rules.skills = false;

        assert!(!config.is_rule_enabled("AS-005"));
        assert!(!config.is_rule_enabled("AS-006"));
        assert!(!config.is_rule_enabled("CC-SK-006"));
        assert!(!config.is_rule_enabled("CC-SK-007"));

        // Other categories still enabled
        assert!(config.is_rule_enabled("CC-AG-001"));
        assert!(config.is_rule_enabled("CC-HK-001"));
    }

    #[test]
    fn test_category_disabled_hooks() {
        let mut config = LintConfig::default();
        config.rules.hooks = false;

        assert!(!config.is_rule_enabled("CC-HK-001"));
        assert!(!config.is_rule_enabled("CC-HK-009"));

        // Other categories still enabled
        assert!(config.is_rule_enabled("CC-AG-001"));
        assert!(config.is_rule_enabled("AS-005"));
    }

    #[test]
    fn test_category_disabled_agents() {
        let mut config = LintConfig::default();
        config.rules.agents = false;

        assert!(!config.is_rule_enabled("CC-AG-001"));
        assert!(!config.is_rule_enabled("CC-AG-006"));

        // Other categories still enabled
        assert!(config.is_rule_enabled("CC-HK-001"));
        assert!(config.is_rule_enabled("AS-005"));
    }

    #[test]
    fn test_category_disabled_memory() {
        let mut config = LintConfig::default();
        config.rules.memory = false;

        assert!(!config.is_rule_enabled("CC-MEM-005"));

        // Other categories still enabled
        assert!(config.is_rule_enabled("CC-AG-001"));
    }

    #[test]
    fn test_category_disabled_plugins() {
        let mut config = LintConfig::default();
        config.rules.plugins = false;

        assert!(!config.is_rule_enabled("CC-PL-001"));

        // Other categories still enabled
        assert!(config.is_rule_enabled("CC-AG-001"));
    }

    #[test]
    fn test_category_disabled_xml() {
        let mut config = LintConfig::default();
        config.rules.xml = false;

        assert!(!config.is_rule_enabled("XML-001"));
        assert!(!config.is_rule_enabled("XML-002"));
        assert!(!config.is_rule_enabled("XML-003"));

        // Other categories still enabled
        assert!(config.is_rule_enabled("CC-AG-001"));
    }

    #[test]
    fn test_category_disabled_imports() {
        let mut config = LintConfig::default();
        config.rules.imports = false;

        assert!(!config.is_rule_enabled("REF-001"));
        assert!(!config.is_rule_enabled("imports::not_found"));

        // Other categories still enabled
        assert!(config.is_rule_enabled("CC-AG-001"));
    }

    #[test]
    fn test_target_cursor_disables_cc_rules() {
        let mut config = LintConfig::default();
        config.target = TargetTool::Cursor;

        // CC-* rules should be disabled for Cursor
        assert!(!config.is_rule_enabled("CC-AG-001"));
        assert!(!config.is_rule_enabled("CC-HK-001"));
        assert!(!config.is_rule_enabled("CC-SK-006"));
        assert!(!config.is_rule_enabled("CC-MEM-005"));

        // AS-* rules should still work
        assert!(config.is_rule_enabled("AS-005"));
        assert!(config.is_rule_enabled("AS-006"));

        // XML and imports should still work
        assert!(config.is_rule_enabled("XML-001"));
        assert!(config.is_rule_enabled("REF-001"));
    }

    #[test]
    fn test_target_codex_disables_cc_rules() {
        let mut config = LintConfig::default();
        config.target = TargetTool::Codex;

        // CC-* rules should be disabled for Codex
        assert!(!config.is_rule_enabled("CC-AG-001"));
        assert!(!config.is_rule_enabled("CC-HK-001"));

        // AS-* rules should still work
        assert!(config.is_rule_enabled("AS-005"));
    }

    #[test]
    fn test_target_claude_code_enables_cc_rules() {
        let mut config = LintConfig::default();
        config.target = TargetTool::ClaudeCode;

        // All rules should be enabled
        assert!(config.is_rule_enabled("CC-AG-001"));
        assert!(config.is_rule_enabled("CC-HK-001"));
        assert!(config.is_rule_enabled("AS-005"));
    }

    #[test]
    fn test_target_generic_enables_all() {
        let config = LintConfig::default(); // Default is Generic

        // All rules should be enabled
        assert!(config.is_rule_enabled("CC-AG-001"));
        assert!(config.is_rule_enabled("CC-HK-001"));
        assert!(config.is_rule_enabled("AS-005"));
        assert!(config.is_rule_enabled("XML-001"));
    }

    #[test]
    fn test_unknown_rules_enabled_by_default() {
        let config = LintConfig::default();

        // Unknown rule IDs should be enabled
        assert!(config.is_rule_enabled("UNKNOWN-001"));
        assert!(config.is_rule_enabled("skill::schema"));
        assert!(config.is_rule_enabled("agent::parse"));
    }

    #[test]
    fn test_disabled_rules_takes_precedence() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["AS-005".to_string()];

        // Even with skills enabled, this specific rule is disabled
        assert!(config.rules.skills);
        assert!(!config.is_rule_enabled("AS-005"));
        assert!(config.is_rule_enabled("AS-006"));
    }

    #[test]
    fn test_toml_deserialization_with_new_fields() {
        let toml_str = r#"
severity = "Warning"
target = "ClaudeCode"
exclude = []

[rules]
skills = true
hooks = false
agents = true
disabled_rules = ["CC-AG-002"]
"#;

        let config: LintConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.target, TargetTool::ClaudeCode);
        assert!(config.rules.skills);
        assert!(!config.rules.hooks);
        assert!(config.rules.agents);
        assert!(
            config
                .rules
                .disabled_rules
                .contains(&"CC-AG-002".to_string())
        );

        // Check rule enablement
        assert!(config.is_rule_enabled("CC-AG-001"));
        assert!(!config.is_rule_enabled("CC-AG-002")); // Disabled in list
        assert!(!config.is_rule_enabled("CC-HK-001")); // hooks category disabled
    }

    #[test]
    fn test_toml_deserialization_defaults() {
        // Minimal config should use defaults
        let toml_str = r#"
severity = "Warning"
target = "Generic"
exclude = []

[rules]
"#;

        let config: LintConfig = toml::from_str(toml_str).unwrap();

        // All categories should default to true
        assert!(config.rules.skills);
        assert!(config.rules.hooks);
        assert!(config.rules.agents);
        assert!(config.rules.memory);
        assert!(config.rules.plugins);
        assert!(config.rules.xml);
        assert!(config.rules.mcp);
        assert!(config.rules.imports);
        assert!(config.rules.cross_platform);
        assert!(config.rules.prompt_engineering);
        assert!(config.rules.disabled_rules.is_empty());
    }

    // ===== MCP Category Tests =====

    #[test]
    fn test_category_disabled_mcp() {
        let mut config = LintConfig::default();
        config.rules.mcp = false;

        assert!(!config.is_rule_enabled("MCP-001"));
        assert!(!config.is_rule_enabled("MCP-002"));
        assert!(!config.is_rule_enabled("MCP-003"));
        assert!(!config.is_rule_enabled("MCP-004"));
        assert!(!config.is_rule_enabled("MCP-005"));
        assert!(!config.is_rule_enabled("MCP-006"));

        // Other categories still enabled
        assert!(config.is_rule_enabled("CC-AG-001"));
        assert!(config.is_rule_enabled("AS-005"));
    }

    #[test]
    fn test_mcp_rules_enabled_by_default() {
        let config = LintConfig::default();

        assert!(config.is_rule_enabled("MCP-001"));
        assert!(config.is_rule_enabled("MCP-002"));
        assert!(config.is_rule_enabled("MCP-003"));
        assert!(config.is_rule_enabled("MCP-004"));
        assert!(config.is_rule_enabled("MCP-005"));
        assert!(config.is_rule_enabled("MCP-006"));
        assert!(config.is_rule_enabled("MCP-007"));
        assert!(config.is_rule_enabled("MCP-008"));
    }

    // ===== MCP Protocol Version Config Tests =====

    #[test]
    fn test_default_mcp_protocol_version() {
        let config = LintConfig::default();
        assert_eq!(config.get_mcp_protocol_version(), "2025-06-18");
    }

    #[test]
    fn test_custom_mcp_protocol_version() {
        let mut config = LintConfig::default();
        config.mcp_protocol_version = Some("2024-11-05".to_string());
        assert_eq!(config.get_mcp_protocol_version(), "2024-11-05");
    }

    #[test]
    fn test_mcp_protocol_version_none_fallback() {
        let mut config = LintConfig::default();
        config.mcp_protocol_version = None;
        // Should fall back to default when None
        assert_eq!(config.get_mcp_protocol_version(), "2025-06-18");
    }

    #[test]
    fn test_toml_deserialization_mcp_protocol_version() {
        let toml_str = r#"
severity = "Warning"
target = "Generic"
exclude = []
mcp_protocol_version = "2024-11-05"

[rules]
"#;

        let config: LintConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.get_mcp_protocol_version(), "2024-11-05");
    }

    #[test]
    fn test_toml_deserialization_mcp_protocol_version_default() {
        // Without specifying mcp_protocol_version, should use default
        let toml_str = r#"
severity = "Warning"
target = "Generic"
exclude = []

[rules]
"#;

        let config: LintConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.get_mcp_protocol_version(), "2025-06-18");
    }

    // ===== Cross-Platform Category Tests =====

    #[test]
    fn test_default_config_enables_xp_rules() {
        let config = LintConfig::default();

        assert!(config.is_rule_enabled("XP-001"));
        assert!(config.is_rule_enabled("XP-002"));
        assert!(config.is_rule_enabled("XP-003"));
    }

    #[test]
    fn test_category_disabled_cross_platform() {
        let mut config = LintConfig::default();
        config.rules.cross_platform = false;

        assert!(!config.is_rule_enabled("XP-001"));
        assert!(!config.is_rule_enabled("XP-002"));
        assert!(!config.is_rule_enabled("XP-003"));

        // Other categories still enabled
        assert!(config.is_rule_enabled("CC-AG-001"));
        assert!(config.is_rule_enabled("AS-005"));
    }

    #[test]
    fn test_xp_rules_work_with_all_targets() {
        // XP-* rules are NOT target-specific (unlike CC-* rules)
        // They should work with Cursor, Codex, and all targets
        let targets = [
            TargetTool::Generic,
            TargetTool::ClaudeCode,
            TargetTool::Cursor,
            TargetTool::Codex,
        ];

        for target in targets {
            let mut config = LintConfig::default();
            config.target = target;

            assert!(
                config.is_rule_enabled("XP-001"),
                "XP-001 should be enabled for {:?}",
                target
            );
            assert!(
                config.is_rule_enabled("XP-002"),
                "XP-002 should be enabled for {:?}",
                target
            );
            assert!(
                config.is_rule_enabled("XP-003"),
                "XP-003 should be enabled for {:?}",
                target
            );
        }
    }

    #[test]
    fn test_disabled_specific_xp_rule() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["XP-001".to_string()];

        assert!(!config.is_rule_enabled("XP-001"));
        assert!(config.is_rule_enabled("XP-002"));
        assert!(config.is_rule_enabled("XP-003"));
    }

    #[test]
    fn test_toml_deserialization_cross_platform() {
        let toml_str = r#"
severity = "Warning"
target = "Generic"
exclude = []

[rules]
cross_platform = false
"#;

        let config: LintConfig = toml::from_str(toml_str).unwrap();

        assert!(!config.rules.cross_platform);
        assert!(!config.is_rule_enabled("XP-001"));
        assert!(!config.is_rule_enabled("XP-002"));
        assert!(!config.is_rule_enabled("XP-003"));
    }

    // ===== AGENTS.md Category Tests =====

    #[test]
    fn test_default_config_enables_agm_rules() {
        let config = LintConfig::default();

        assert!(config.is_rule_enabled("AGM-001"));
        assert!(config.is_rule_enabled("AGM-002"));
        assert!(config.is_rule_enabled("AGM-003"));
        assert!(config.is_rule_enabled("AGM-004"));
        assert!(config.is_rule_enabled("AGM-005"));
        assert!(config.is_rule_enabled("AGM-006"));
    }

    #[test]
    fn test_category_disabled_agents_md() {
        let mut config = LintConfig::default();
        config.rules.agents_md = false;

        assert!(!config.is_rule_enabled("AGM-001"));
        assert!(!config.is_rule_enabled("AGM-002"));
        assert!(!config.is_rule_enabled("AGM-003"));
        assert!(!config.is_rule_enabled("AGM-004"));
        assert!(!config.is_rule_enabled("AGM-005"));
        assert!(!config.is_rule_enabled("AGM-006"));

        // Other categories still enabled
        assert!(config.is_rule_enabled("CC-AG-001"));
        assert!(config.is_rule_enabled("AS-005"));
        assert!(config.is_rule_enabled("XP-001"));
    }

    #[test]
    fn test_agm_rules_work_with_all_targets() {
        // AGM-* rules are NOT target-specific (unlike CC-* rules)
        // They should work with Cursor, Codex, and all targets
        let targets = [
            TargetTool::Generic,
            TargetTool::ClaudeCode,
            TargetTool::Cursor,
            TargetTool::Codex,
        ];

        for target in targets {
            let mut config = LintConfig::default();
            config.target = target;

            assert!(
                config.is_rule_enabled("AGM-001"),
                "AGM-001 should be enabled for {:?}",
                target
            );
            assert!(
                config.is_rule_enabled("AGM-006"),
                "AGM-006 should be enabled for {:?}",
                target
            );
        }
    }

    #[test]
    fn test_disabled_specific_agm_rule() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["AGM-001".to_string()];

        assert!(!config.is_rule_enabled("AGM-001"));
        assert!(config.is_rule_enabled("AGM-002"));
        assert!(config.is_rule_enabled("AGM-003"));
        assert!(config.is_rule_enabled("AGM-004"));
        assert!(config.is_rule_enabled("AGM-005"));
        assert!(config.is_rule_enabled("AGM-006"));
    }

    #[test]
    fn test_toml_deserialization_agents_md() {
        let toml_str = r#"
severity = "Warning"
target = "Generic"
exclude = []

[rules]
agents_md = false
"#;

        let config: LintConfig = toml::from_str(toml_str).unwrap();

        assert!(!config.rules.agents_md);
        assert!(!config.is_rule_enabled("AGM-001"));
        assert!(!config.is_rule_enabled("AGM-006"));
    }

    // ===== Prompt Engineering Category Tests =====

    #[test]
    fn test_default_config_enables_pe_rules() {
        let config = LintConfig::default();

        assert!(config.is_rule_enabled("PE-001"));
        assert!(config.is_rule_enabled("PE-002"));
        assert!(config.is_rule_enabled("PE-003"));
        assert!(config.is_rule_enabled("PE-004"));
    }

    #[test]
    fn test_category_disabled_prompt_engineering() {
        let mut config = LintConfig::default();
        config.rules.prompt_engineering = false;

        assert!(!config.is_rule_enabled("PE-001"));
        assert!(!config.is_rule_enabled("PE-002"));
        assert!(!config.is_rule_enabled("PE-003"));
        assert!(!config.is_rule_enabled("PE-004"));

        // Other categories still enabled
        assert!(config.is_rule_enabled("CC-AG-001"));
        assert!(config.is_rule_enabled("AS-005"));
        assert!(config.is_rule_enabled("XP-001"));
    }

    #[test]
    fn test_pe_rules_work_with_all_targets() {
        // PE-* rules are NOT target-specific
        let targets = [
            TargetTool::Generic,
            TargetTool::ClaudeCode,
            TargetTool::Cursor,
            TargetTool::Codex,
        ];

        for target in targets {
            let mut config = LintConfig::default();
            config.target = target;

            assert!(
                config.is_rule_enabled("PE-001"),
                "PE-001 should be enabled for {:?}",
                target
            );
            assert!(
                config.is_rule_enabled("PE-002"),
                "PE-002 should be enabled for {:?}",
                target
            );
            assert!(
                config.is_rule_enabled("PE-003"),
                "PE-003 should be enabled for {:?}",
                target
            );
            assert!(
                config.is_rule_enabled("PE-004"),
                "PE-004 should be enabled for {:?}",
                target
            );
        }
    }

    #[test]
    fn test_disabled_specific_pe_rule() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["PE-001".to_string()];

        assert!(!config.is_rule_enabled("PE-001"));
        assert!(config.is_rule_enabled("PE-002"));
        assert!(config.is_rule_enabled("PE-003"));
        assert!(config.is_rule_enabled("PE-004"));
    }

    #[test]
    fn test_toml_deserialization_prompt_engineering() {
        let toml_str = r#"
severity = "Warning"
target = "Generic"
exclude = []

[rules]
prompt_engineering = false
"#;

        let config: LintConfig = toml::from_str(toml_str).unwrap();

        assert!(!config.rules.prompt_engineering);
        assert!(!config.is_rule_enabled("PE-001"));
        assert!(!config.is_rule_enabled("PE-002"));
        assert!(!config.is_rule_enabled("PE-003"));
        assert!(!config.is_rule_enabled("PE-004"));
    }

    // ===== GitHub Copilot Category Tests =====

    #[test]
    fn test_default_config_enables_cop_rules() {
        let config = LintConfig::default();

        assert!(config.is_rule_enabled("COP-001"));
        assert!(config.is_rule_enabled("COP-002"));
        assert!(config.is_rule_enabled("COP-003"));
        assert!(config.is_rule_enabled("COP-004"));
    }

    #[test]
    fn test_category_disabled_copilot() {
        let mut config = LintConfig::default();
        config.rules.copilot = false;

        assert!(!config.is_rule_enabled("COP-001"));
        assert!(!config.is_rule_enabled("COP-002"));
        assert!(!config.is_rule_enabled("COP-003"));
        assert!(!config.is_rule_enabled("COP-004"));

        // Other categories still enabled
        assert!(config.is_rule_enabled("CC-AG-001"));
        assert!(config.is_rule_enabled("AS-005"));
        assert!(config.is_rule_enabled("XP-001"));
    }

    #[test]
    fn test_cop_rules_work_with_all_targets() {
        // COP-* rules are NOT target-specific
        let targets = [
            TargetTool::Generic,
            TargetTool::ClaudeCode,
            TargetTool::Cursor,
            TargetTool::Codex,
        ];

        for target in targets {
            let mut config = LintConfig::default();
            config.target = target;

            assert!(
                config.is_rule_enabled("COP-001"),
                "COP-001 should be enabled for {:?}",
                target
            );
            assert!(
                config.is_rule_enabled("COP-002"),
                "COP-002 should be enabled for {:?}",
                target
            );
            assert!(
                config.is_rule_enabled("COP-003"),
                "COP-003 should be enabled for {:?}",
                target
            );
            assert!(
                config.is_rule_enabled("COP-004"),
                "COP-004 should be enabled for {:?}",
                target
            );
        }
    }

    #[test]
    fn test_disabled_specific_cop_rule() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["COP-001".to_string()];

        assert!(!config.is_rule_enabled("COP-001"));
        assert!(config.is_rule_enabled("COP-002"));
        assert!(config.is_rule_enabled("COP-003"));
        assert!(config.is_rule_enabled("COP-004"));
    }

    #[test]
    fn test_toml_deserialization_copilot() {
        let toml_str = r#"
severity = "Warning"
target = "Generic"
exclude = []

[rules]
copilot = false
"#;

        let config: LintConfig = toml::from_str(toml_str).unwrap();

        assert!(!config.rules.copilot);
        assert!(!config.is_rule_enabled("COP-001"));
        assert!(!config.is_rule_enabled("COP-002"));
        assert!(!config.is_rule_enabled("COP-003"));
        assert!(!config.is_rule_enabled("COP-004"));
    }

    // ===== Cursor Category Tests =====

    #[test]
    fn test_default_config_enables_cur_rules() {
        let config = LintConfig::default();

        assert!(config.is_rule_enabled("CUR-001"));
        assert!(config.is_rule_enabled("CUR-002"));
        assert!(config.is_rule_enabled("CUR-003"));
        assert!(config.is_rule_enabled("CUR-004"));
        assert!(config.is_rule_enabled("CUR-005"));
        assert!(config.is_rule_enabled("CUR-006"));
    }

    #[test]
    fn test_category_disabled_cursor() {
        let mut config = LintConfig::default();
        config.rules.cursor = false;

        assert!(!config.is_rule_enabled("CUR-001"));
        assert!(!config.is_rule_enabled("CUR-002"));
        assert!(!config.is_rule_enabled("CUR-003"));
        assert!(!config.is_rule_enabled("CUR-004"));
        assert!(!config.is_rule_enabled("CUR-005"));
        assert!(!config.is_rule_enabled("CUR-006"));

        // Other categories still enabled
        assert!(config.is_rule_enabled("CC-AG-001"));
        assert!(config.is_rule_enabled("AS-005"));
        assert!(config.is_rule_enabled("COP-001"));
    }

    #[test]
    fn test_cur_rules_work_with_all_targets() {
        // CUR-* rules are NOT target-specific
        let targets = [
            TargetTool::Generic,
            TargetTool::ClaudeCode,
            TargetTool::Cursor,
            TargetTool::Codex,
        ];

        for target in targets {
            let mut config = LintConfig::default();
            config.target = target;

            assert!(
                config.is_rule_enabled("CUR-001"),
                "CUR-001 should be enabled for {:?}",
                target
            );
            assert!(
                config.is_rule_enabled("CUR-006"),
                "CUR-006 should be enabled for {:?}",
                target
            );
        }
    }

    #[test]
    fn test_disabled_specific_cur_rule() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["CUR-001".to_string()];

        assert!(!config.is_rule_enabled("CUR-001"));
        assert!(config.is_rule_enabled("CUR-002"));
        assert!(config.is_rule_enabled("CUR-003"));
        assert!(config.is_rule_enabled("CUR-004"));
        assert!(config.is_rule_enabled("CUR-005"));
        assert!(config.is_rule_enabled("CUR-006"));
    }

    #[test]
    fn test_toml_deserialization_cursor() {
        let toml_str = r#"
severity = "Warning"
target = "Generic"
exclude = []

[rules]
cursor = false
"#;

        let config: LintConfig = toml::from_str(toml_str).unwrap();

        assert!(!config.rules.cursor);
        assert!(!config.is_rule_enabled("CUR-001"));
        assert!(!config.is_rule_enabled("CUR-002"));
        assert!(!config.is_rule_enabled("CUR-003"));
        assert!(!config.is_rule_enabled("CUR-004"));
        assert!(!config.is_rule_enabled("CUR-005"));
        assert!(!config.is_rule_enabled("CUR-006"));
    }

    // ===== Config Load Warning Tests =====

    #[test]
    fn test_invalid_toml_returns_warning() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join(".agnix.toml");
        std::fs::write(&config_path, "this is not valid toml [[[").unwrap();

        let (config, warning) = LintConfig::load_or_default(Some(&config_path));

        // Should return default config
        assert_eq!(config.target, TargetTool::Generic);
        assert!(config.rules.skills);

        // Should have a warning message
        assert!(warning.is_some());
        let msg = warning.unwrap();
        assert!(msg.contains("Failed to parse config"));
        assert!(msg.contains("Using defaults"));
    }

    #[test]
    fn test_missing_config_no_warning() {
        let (config, warning) = LintConfig::load_or_default(None);

        assert_eq!(config.target, TargetTool::Generic);
        assert!(warning.is_none());
    }

    #[test]
    fn test_valid_config_no_warning() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join(".agnix.toml");
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

        let (config, warning) = LintConfig::load_or_default(Some(&config_path));

        assert_eq!(config.target, TargetTool::ClaudeCode);
        assert!(!config.rules.skills);
        assert!(warning.is_none());
    }

    #[test]
    fn test_nonexistent_config_file_returns_warning() {
        let nonexistent = PathBuf::from("/nonexistent/path/.agnix.toml");
        let (config, warning) = LintConfig::load_or_default(Some(&nonexistent));

        // Should return default config
        assert_eq!(config.target, TargetTool::Generic);

        // Should have a warning about the missing file
        assert!(warning.is_some());
        let msg = warning.unwrap();
        assert!(msg.contains("Failed to parse config"));
    }

    // ===== Backward Compatibility Tests =====

    #[test]
    fn test_old_config_with_removed_fields_still_parses() {
        // Test that configs with the removed tool_names and required_fields
        // options still parse correctly (serde ignores unknown fields by default)
        let toml_str = r#"
severity = "Warning"
target = "Generic"
exclude = []

[rules]
skills = true
hooks = true
tool_names = true
required_fields = true
"#;

        let config: LintConfig = toml::from_str(toml_str)
            .expect("Failed to parse config with removed fields for backward compatibility");

        // Config should parse successfully with expected values
        assert_eq!(config.target, TargetTool::Generic);
        assert!(config.rules.skills);
        assert!(config.rules.hooks);
        // The removed fields are simply ignored
    }

    // ===== Tool Versions Tests =====

    #[test]
    fn test_tool_versions_default_unpinned() {
        let config = LintConfig::default();

        assert!(config.tool_versions.claude_code.is_none());
        assert!(config.tool_versions.codex.is_none());
        assert!(config.tool_versions.cursor.is_none());
        assert!(config.tool_versions.copilot.is_none());
        assert!(!config.is_claude_code_version_pinned());
    }

    #[test]
    fn test_tool_versions_claude_code_pinned() {
        let toml_str = r#"
severity = "Warning"
target = "ClaudeCode"
exclude = []

[rules]

[tool_versions]
claude_code = "1.0.0"
"#;

        let config: LintConfig = toml::from_str(toml_str).unwrap();
        assert!(config.is_claude_code_version_pinned());
        assert_eq!(config.get_claude_code_version(), Some("1.0.0"));
    }

    #[test]
    fn test_tool_versions_multiple_pinned() {
        let toml_str = r#"
severity = "Warning"
target = "Generic"
exclude = []

[rules]

[tool_versions]
claude_code = "1.0.0"
codex = "0.1.0"
cursor = "0.45.0"
copilot = "1.0.0"
"#;

        let config: LintConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.tool_versions.claude_code, Some("1.0.0".to_string()));
        assert_eq!(config.tool_versions.codex, Some("0.1.0".to_string()));
        assert_eq!(config.tool_versions.cursor, Some("0.45.0".to_string()));
        assert_eq!(config.tool_versions.copilot, Some("1.0.0".to_string()));
    }

    // ===== Tool Versions: Pre-release, Build Metadata, Invalid Semver =====

    #[test]
    fn test_tool_versions_prerelease_version() {
        let toml_str = r#"
severity = "Warning"
target = "Generic"
exclude = []

[rules]

[tool_versions]
claude_code = "1.0.0-rc1"
codex = "0.2.0-beta.3"
"#;

        let config: LintConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.tool_versions.claude_code,
            Some("1.0.0-rc1".to_string())
        );
        assert_eq!(config.tool_versions.codex, Some("0.2.0-beta.3".to_string()));
        // Pre-release strings are valid semver, confirm they parse
        assert!(semver::Version::parse("1.0.0-rc1").is_ok());
        assert!(semver::Version::parse("0.2.0-beta.3").is_ok());
    }

    #[test]
    fn test_tool_versions_build_metadata() {
        let toml_str = r#"
severity = "Warning"
target = "Generic"
exclude = []

[rules]

[tool_versions]
claude_code = "1.0.0+build123"
cursor = "0.45.0+20250101"
"#;

        let config: LintConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.tool_versions.claude_code,
            Some("1.0.0+build123".to_string())
        );
        assert_eq!(
            config.tool_versions.cursor,
            Some("0.45.0+20250101".to_string())
        );
        // Build metadata is valid semver
        assert!(semver::Version::parse("1.0.0+build123").is_ok());
        assert!(semver::Version::parse("0.45.0+20250101").is_ok());
    }

    #[test]
    fn test_tool_versions_prerelease_with_build_metadata() {
        let toml_str = r#"
severity = "Warning"
target = "Generic"
exclude = []

[rules]

[tool_versions]
copilot = "2.0.0-alpha.1+build456"
"#;

        let config: LintConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.tool_versions.copilot,
            Some("2.0.0-alpha.1+build456".to_string())
        );
        assert!(semver::Version::parse("2.0.0-alpha.1+build456").is_ok());
    }

    #[test]
    fn test_invalid_semver_rejected_by_parser() {
        // ToolVersions stores strings (no validation at deserialization time),
        // but the semver crate correctly rejects invalid formats
        let invalid_versions = vec![
            "not-a-version",
            "1.0",
            "1",
            "v1.0.0",
            "1.0.0.0",
            "",
            "abc",
            "1.0.0-",
            "1.0.0+",
        ];

        for v in &invalid_versions {
            assert!(
                semver::Version::parse(v).is_err(),
                "Expected '{}' to be rejected as invalid semver",
                v
            );
        }
    }

    #[test]
    fn test_valid_semver_accepted_by_parser() {
        let valid_versions = vec![
            "0.0.0",
            "1.0.0",
            "99.99.99",
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-0.3.7",
            "1.0.0-x.7.z.92",
            "1.0.0+build",
            "1.0.0+build.123",
            "1.0.0-beta+exp.sha.5114f85",
        ];

        for v in &valid_versions {
            assert!(
                semver::Version::parse(v).is_ok(),
                "Expected '{}' to be accepted as valid semver",
                v
            );
        }
    }

    #[test]
    fn test_tool_versions_invalid_string_still_deserializes() {
        // ToolVersions fields are plain strings, so invalid semver still deserializes
        // (validation happens at usage time, not parse time)
        let toml_str = r#"
severity = "Warning"
target = "Generic"
exclude = []

[rules]

[tool_versions]
claude_code = "not-valid-semver"
"#;

        let config: LintConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.tool_versions.claude_code,
            Some("not-valid-semver".to_string())
        );
        // But semver parsing would fail
        assert!(semver::Version::parse("not-valid-semver").is_err());
    }

    // ===== Spec Revisions Tests =====

    #[test]
    fn test_spec_revisions_default_unpinned() {
        let config = LintConfig::default();

        assert!(config.spec_revisions.mcp_protocol.is_none());
        assert!(config.spec_revisions.agent_skills_spec.is_none());
        assert!(config.spec_revisions.agents_md_spec.is_none());
        // mcp_protocol_version is None by default, so is_mcp_revision_pinned returns false
        assert!(!config.is_mcp_revision_pinned());
    }

    #[test]
    fn test_spec_revisions_mcp_pinned() {
        let toml_str = r#"
severity = "Warning"
target = "Generic"
exclude = []

[rules]

[spec_revisions]
mcp_protocol = "2024-11-05"
"#;

        let config: LintConfig = toml::from_str(toml_str).unwrap();
        assert!(config.is_mcp_revision_pinned());
        assert_eq!(config.get_mcp_protocol_version(), "2024-11-05");
    }

    #[test]
    fn test_spec_revisions_precedence_over_legacy() {
        // spec_revisions.mcp_protocol should take precedence over mcp_protocol_version
        let toml_str = r#"
severity = "Warning"
target = "Generic"
exclude = []
mcp_protocol_version = "2024-11-05"

[rules]

[spec_revisions]
mcp_protocol = "2025-06-18"
"#;

        let config: LintConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.get_mcp_protocol_version(), "2025-06-18");
    }

    #[test]
    fn test_spec_revisions_fallback_to_legacy() {
        // When spec_revisions.mcp_protocol is not set, fall back to mcp_protocol_version
        let toml_str = r#"
severity = "Warning"
target = "Generic"
exclude = []
mcp_protocol_version = "2024-11-05"

[rules]

[spec_revisions]
"#;

        let config: LintConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.get_mcp_protocol_version(), "2024-11-05");
    }

    #[test]
    fn test_spec_revisions_multiple_pinned() {
        let toml_str = r#"
severity = "Warning"
target = "Generic"
exclude = []

[rules]

[spec_revisions]
mcp_protocol = "2024-11-05"
agent_skills_spec = "1.0.0"
agents_md_spec = "1.0.0"
"#;

        let config: LintConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.spec_revisions.mcp_protocol,
            Some("2024-11-05".to_string())
        );
        assert_eq!(
            config.spec_revisions.agent_skills_spec,
            Some("1.0.0".to_string())
        );
        assert_eq!(
            config.spec_revisions.agents_md_spec,
            Some("1.0.0".to_string())
        );
    }

    // ===== Backward Compatibility with New Fields =====

    #[test]
    fn test_config_without_tool_versions_defaults() {
        // Old configs without tool_versions section should still work
        let toml_str = r#"
severity = "Warning"
target = "ClaudeCode"
exclude = []

[rules]
skills = true
"#;

        let config: LintConfig = toml::from_str(toml_str).unwrap();
        assert!(!config.is_claude_code_version_pinned());
        assert!(config.tool_versions.claude_code.is_none());
    }

    #[test]
    fn test_config_without_spec_revisions_defaults() {
        // Old configs without spec_revisions section should still work
        let toml_str = r#"
severity = "Warning"
target = "Generic"
exclude = []

[rules]
"#;

        let config: LintConfig = toml::from_str(toml_str).unwrap();
        // mcp_protocol_version is None when not specified, so is_mcp_revision_pinned returns false
        assert!(!config.is_mcp_revision_pinned());
        // get_mcp_protocol_version still returns default value
        assert_eq!(config.get_mcp_protocol_version(), "2025-06-18");
    }

    #[test]
    fn test_is_mcp_revision_pinned_with_none_mcp_protocol_version() {
        // When both spec_revisions.mcp_protocol and mcp_protocol_version are None
        let mut config = LintConfig::default();
        config.mcp_protocol_version = None;
        config.spec_revisions.mcp_protocol = None;

        assert!(!config.is_mcp_revision_pinned());
        // Should still return default
        assert_eq!(config.get_mcp_protocol_version(), "2025-06-18");
    }

    // ===== Tools Array Tests =====

    #[test]
    fn test_tools_array_empty_uses_target() {
        // When tools is empty, fall back to target behavior
        let mut config = LintConfig::default();
        config.tools = vec![];
        config.target = TargetTool::Cursor;

        // With Cursor target and empty tools, CC-* rules should be disabled
        assert!(!config.is_rule_enabled("CC-AG-001"));
        assert!(!config.is_rule_enabled("CC-HK-001"));

        // AS-* rules should still work
        assert!(config.is_rule_enabled("AS-005"));
    }

    #[test]
    fn test_tools_array_claude_code_only() {
        let mut config = LintConfig::default();
        config.tools = vec!["claude-code".to_string()];

        // CC-* rules should be enabled
        assert!(config.is_rule_enabled("CC-AG-001"));
        assert!(config.is_rule_enabled("CC-HK-001"));
        assert!(config.is_rule_enabled("CC-SK-006"));

        // COP-* and CUR-* rules should be disabled
        assert!(!config.is_rule_enabled("COP-001"));
        assert!(!config.is_rule_enabled("CUR-001"));

        // Generic rules should still be enabled
        assert!(config.is_rule_enabled("AS-005"));
        assert!(config.is_rule_enabled("XP-001"));
        assert!(config.is_rule_enabled("AGM-001"));
    }

    #[test]
    fn test_tools_array_cursor_only() {
        let mut config = LintConfig::default();
        config.tools = vec!["cursor".to_string()];

        // CUR-* rules should be enabled
        assert!(config.is_rule_enabled("CUR-001"));
        assert!(config.is_rule_enabled("CUR-006"));

        // CC-* and COP-* rules should be disabled
        assert!(!config.is_rule_enabled("CC-AG-001"));
        assert!(!config.is_rule_enabled("COP-001"));

        // Generic rules should still be enabled
        assert!(config.is_rule_enabled("AS-005"));
        assert!(config.is_rule_enabled("XP-001"));
    }

    #[test]
    fn test_tools_array_copilot_only() {
        let mut config = LintConfig::default();
        config.tools = vec!["copilot".to_string()];

        // COP-* rules should be enabled
        assert!(config.is_rule_enabled("COP-001"));
        assert!(config.is_rule_enabled("COP-002"));

        // CC-* and CUR-* rules should be disabled
        assert!(!config.is_rule_enabled("CC-AG-001"));
        assert!(!config.is_rule_enabled("CUR-001"));

        // Generic rules should still be enabled
        assert!(config.is_rule_enabled("AS-005"));
        assert!(config.is_rule_enabled("XP-001"));
    }

    #[test]
    fn test_tools_array_multiple_tools() {
        let mut config = LintConfig::default();
        config.tools = vec!["claude-code".to_string(), "cursor".to_string()];

        // CC-* and CUR-* rules should both be enabled
        assert!(config.is_rule_enabled("CC-AG-001"));
        assert!(config.is_rule_enabled("CC-HK-001"));
        assert!(config.is_rule_enabled("CUR-001"));
        assert!(config.is_rule_enabled("CUR-006"));

        // COP-* rules should be disabled (not in tools)
        assert!(!config.is_rule_enabled("COP-001"));

        // Generic rules should still be enabled
        assert!(config.is_rule_enabled("AS-005"));
        assert!(config.is_rule_enabled("XP-001"));
    }

    #[test]
    fn test_tools_array_case_insensitive() {
        let mut config = LintConfig::default();
        config.tools = vec!["Claude-Code".to_string(), "CURSOR".to_string()];

        // Should work case-insensitively
        assert!(config.is_rule_enabled("CC-AG-001"));
        assert!(config.is_rule_enabled("CUR-001"));
    }

    #[test]
    fn test_tools_array_overrides_target() {
        let mut config = LintConfig::default();
        config.target = TargetTool::Cursor; // Legacy: would disable CC-*
        config.tools = vec!["claude-code".to_string()]; // New: should enable CC-*

        // tools array should override target
        assert!(config.is_rule_enabled("CC-AG-001"));
        assert!(!config.is_rule_enabled("CUR-001")); // Cursor not in tools
    }

    #[test]
    fn test_tools_toml_deserialization() {
        let toml_str = r#"
severity = "Warning"
target = "Generic"
exclude = []
tools = ["claude-code", "cursor"]

[rules]
"#;

        let config: LintConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.tools.len(), 2);
        assert!(config.tools.contains(&"claude-code".to_string()));
        assert!(config.tools.contains(&"cursor".to_string()));

        // Verify rule enablement
        assert!(config.is_rule_enabled("CC-AG-001"));
        assert!(config.is_rule_enabled("CUR-001"));
        assert!(!config.is_rule_enabled("COP-001"));
    }

    #[test]
    fn test_tools_toml_backward_compatible() {
        // Old configs without tools field should still work
        let toml_str = r#"
severity = "Warning"
target = "ClaudeCode"
exclude = []

[rules]
"#;

        let config: LintConfig = toml::from_str(toml_str).unwrap();

        assert!(config.tools.is_empty());
        // Falls back to target behavior
        assert!(config.is_rule_enabled("CC-AG-001"));
    }

    #[test]
    fn test_tools_disabled_rules_still_works() {
        let mut config = LintConfig::default();
        config.tools = vec!["claude-code".to_string()];
        config.rules.disabled_rules = vec!["CC-AG-001".to_string()];

        // CC-AG-001 is explicitly disabled even though claude-code is in tools
        assert!(!config.is_rule_enabled("CC-AG-001"));
        // Other CC-* rules should still work
        assert!(config.is_rule_enabled("CC-AG-002"));
        assert!(config.is_rule_enabled("CC-HK-001"));
    }

    #[test]
    fn test_tools_category_disabled_still_works() {
        let mut config = LintConfig::default();
        config.tools = vec!["claude-code".to_string()];
        config.rules.hooks = false;

        // CC-HK-* rules should be disabled because hooks category is disabled
        assert!(!config.is_rule_enabled("CC-HK-001"));
        // Other CC-* rules should still work
        assert!(config.is_rule_enabled("CC-AG-001"));
    }

    // ===== is_tool_alias Edge Case Tests =====

    #[test]
    fn test_is_tool_alias_unknown_alias_returns_false() {
        // Unknown aliases should return false
        assert!(!LintConfig::is_tool_alias("unknown", "github-copilot"));
        assert!(!LintConfig::is_tool_alias("gh-copilot", "github-copilot"));
        assert!(!LintConfig::is_tool_alias("", "github-copilot"));
    }

    #[test]
    fn test_is_tool_alias_canonical_name_not_alias_of_itself() {
        // Canonical name "github-copilot" is NOT treated as an alias of itself.
        // This is by design - canonical names match via direct comparison in
        // is_rule_for_tools(), not through the alias mechanism.
        assert!(!LintConfig::is_tool_alias(
            "github-copilot",
            "github-copilot"
        ));
        assert!(!LintConfig::is_tool_alias(
            "GitHub-Copilot",
            "github-copilot"
        ));
    }

    #[test]
    fn test_is_tool_alias_copilot_is_alias_for_github_copilot() {
        // "copilot" is an alias for "github-copilot" (backward compatibility)
        assert!(LintConfig::is_tool_alias("copilot", "github-copilot"));
        assert!(LintConfig::is_tool_alias("Copilot", "github-copilot"));
        assert!(LintConfig::is_tool_alias("COPILOT", "github-copilot"));
    }

    #[test]
    fn test_is_tool_alias_no_aliases_for_other_tools() {
        // Other tools have no aliases defined
        assert!(!LintConfig::is_tool_alias("claude", "claude-code"));
        assert!(!LintConfig::is_tool_alias("cc", "claude-code"));
        assert!(!LintConfig::is_tool_alias("cur", "cursor"));
    }

    // ===== Partial Config Tests =====

    #[test]
    fn test_partial_config_only_rules_section() {
        let toml_str = r#"
[rules]
disabled_rules = ["CC-MEM-006"]
"#;
        let config: LintConfig = toml::from_str(toml_str).unwrap();

        // Should use defaults for unspecified fields
        assert_eq!(config.severity, SeverityLevel::Warning);
        assert_eq!(config.target, TargetTool::Generic);
        assert!(config.rules.skills);
        assert!(config.rules.hooks);

        // disabled_rules should be set
        assert_eq!(config.rules.disabled_rules, vec!["CC-MEM-006"]);
        assert!(!config.is_rule_enabled("CC-MEM-006"));
    }

    #[test]
    fn test_partial_config_only_severity() {
        let toml_str = r#"severity = "Error""#;
        let config: LintConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.severity, SeverityLevel::Error);
        assert_eq!(config.target, TargetTool::Generic);
        assert!(config.rules.skills);
    }

    #[test]
    fn test_partial_config_only_target() {
        let toml_str = r#"target = "ClaudeCode""#;
        let config: LintConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.target, TargetTool::ClaudeCode);
        assert_eq!(config.severity, SeverityLevel::Warning);
    }

    #[test]
    fn test_partial_config_only_exclude() {
        let toml_str = r#"exclude = ["vendor/**", "dist/**"]"#;
        let config: LintConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.exclude, vec!["vendor/**", "dist/**"]);
        assert_eq!(config.severity, SeverityLevel::Warning);
    }

    #[test]
    fn test_partial_config_only_disabled_rules() {
        let toml_str = r#"
[rules]
disabled_rules = ["AS-001", "CC-SK-007", "PE-003"]
"#;
        let config: LintConfig = toml::from_str(toml_str).unwrap();

        assert!(!config.is_rule_enabled("AS-001"));
        assert!(!config.is_rule_enabled("CC-SK-007"));
        assert!(!config.is_rule_enabled("PE-003"));
        // Other rules should still be enabled
        assert!(config.is_rule_enabled("AS-002"));
        assert!(config.is_rule_enabled("CC-SK-001"));
    }

    #[test]
    fn test_partial_config_disable_single_category() {
        let toml_str = r#"
[rules]
skills = false
"#;
        let config: LintConfig = toml::from_str(toml_str).unwrap();

        assert!(!config.rules.skills);
        // Other categories should still be enabled (default true)
        assert!(config.rules.hooks);
        assert!(config.rules.agents);
        assert!(config.rules.memory);
    }

    #[test]
    fn test_partial_config_tools_array() {
        let toml_str = r#"tools = ["claude-code", "cursor"]"#;
        let config: LintConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.tools, vec!["claude-code", "cursor"]);
        assert!(config.is_rule_enabled("CC-SK-001")); // Claude Code rule
        assert!(config.is_rule_enabled("CUR-001")); // Cursor rule
    }

    #[test]
    fn test_partial_config_combined_options() {
        let toml_str = r#"
severity = "Error"
target = "ClaudeCode"

[rules]
xml = false
disabled_rules = ["CC-MEM-006"]
"#;
        let config: LintConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.severity, SeverityLevel::Error);
        assert_eq!(config.target, TargetTool::ClaudeCode);
        assert!(!config.rules.xml);
        assert!(!config.is_rule_enabled("CC-MEM-006"));
        // exclude should use default
        assert!(config.exclude.contains(&"node_modules/**".to_string()));
    }

    // ===== Disabled Validators TOML Deserialization =====

    #[test]
    fn test_disabled_validators_toml_deserialization() {
        let toml_str = r#"
[rules]
disabled_validators = ["XmlValidator", "PromptValidator"]
"#;
        let config: LintConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.rules.disabled_validators,
            vec!["XmlValidator", "PromptValidator"]
        );
    }

    #[test]
    fn test_disabled_validators_defaults_to_empty() {
        let toml_str = r#"
[rules]
skills = true
"#;
        let config: LintConfig = toml::from_str(toml_str).unwrap();
        assert!(config.rules.disabled_validators.is_empty());
    }

    #[test]
    fn test_disabled_validators_empty_array() {
        let toml_str = r#"
[rules]
disabled_validators = []
"#;
        let config: LintConfig = toml::from_str(toml_str).unwrap();
        assert!(config.rules.disabled_validators.is_empty());
    }

    // ===== Disabled Rules Edge Cases =====

    #[test]
    fn test_disabled_rules_empty_array() {
        let toml_str = r#"
[rules]
disabled_rules = []
"#;
        let config: LintConfig = toml::from_str(toml_str).unwrap();

        assert!(config.rules.disabled_rules.is_empty());
        assert!(config.is_rule_enabled("AS-001"));
        assert!(config.is_rule_enabled("CC-SK-001"));
    }

    #[test]
    fn test_disabled_rules_case_sensitive() {
        let toml_str = r#"
[rules]
disabled_rules = ["as-001"]
"#;
        let config: LintConfig = toml::from_str(toml_str).unwrap();

        // Rule IDs are case-sensitive
        assert!(config.is_rule_enabled("AS-001")); // Not disabled (different case)
        assert!(!config.is_rule_enabled("as-001")); // Disabled
    }

    #[test]
    fn test_disabled_rules_multiple_from_same_category() {
        let toml_str = r#"
[rules]
disabled_rules = ["AS-001", "AS-002", "AS-003", "AS-004"]
"#;
        let config: LintConfig = toml::from_str(toml_str).unwrap();

        assert!(!config.is_rule_enabled("AS-001"));
        assert!(!config.is_rule_enabled("AS-002"));
        assert!(!config.is_rule_enabled("AS-003"));
        assert!(!config.is_rule_enabled("AS-004"));
        // AS-005 should still be enabled
        assert!(config.is_rule_enabled("AS-005"));
    }

    #[test]
    fn test_disabled_rules_across_categories() {
        let toml_str = r#"
[rules]
disabled_rules = ["AS-001", "CC-SK-007", "MCP-001", "PE-003", "XP-001"]
"#;
        let config: LintConfig = toml::from_str(toml_str).unwrap();

        assert!(!config.is_rule_enabled("AS-001"));
        assert!(!config.is_rule_enabled("CC-SK-007"));
        assert!(!config.is_rule_enabled("MCP-001"));
        assert!(!config.is_rule_enabled("PE-003"));
        assert!(!config.is_rule_enabled("XP-001"));
    }

    #[test]
    fn test_disabled_rules_nonexistent_rule() {
        let toml_str = r#"
[rules]
disabled_rules = ["FAKE-001", "NONEXISTENT-999"]
"#;
        let config: LintConfig = toml::from_str(toml_str).unwrap();

        // Should parse without error, nonexistent rules just have no effect
        assert!(!config.is_rule_enabled("FAKE-001"));
        assert!(!config.is_rule_enabled("NONEXISTENT-999"));
        // Real rules still work
        assert!(config.is_rule_enabled("AS-001"));
    }

    #[test]
    fn test_disabled_rules_with_category_disabled() {
        let toml_str = r#"
[rules]
skills = false
disabled_rules = ["AS-001"]
"#;
        let config: LintConfig = toml::from_str(toml_str).unwrap();

        // Both category disabled AND individual rule disabled
        assert!(!config.is_rule_enabled("AS-001"));
        assert!(!config.is_rule_enabled("AS-002")); // Category disabled
    }

    // ===== Config File Loading Edge Cases =====

    #[test]
    fn test_config_file_empty() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join(".agnix.toml");
        std::fs::write(&config_path, "").unwrap();

        let (config, warning) = LintConfig::load_or_default(Some(&config_path));

        // Empty file should use all defaults
        assert_eq!(config.severity, SeverityLevel::Warning);
        assert_eq!(config.target, TargetTool::Generic);
        assert!(config.rules.skills);
        assert!(warning.is_none());
    }

    #[test]
    fn test_config_file_only_comments() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join(".agnix.toml");
        std::fs::write(
            &config_path,
            r#"
# This is a comment
# Another comment
"#,
        )
        .unwrap();

        let (config, warning) = LintConfig::load_or_default(Some(&config_path));

        // Comments-only file should use all defaults
        assert_eq!(config.severity, SeverityLevel::Warning);
        assert!(warning.is_none());
    }

    #[test]
    fn test_config_file_with_comments() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join(".agnix.toml");
        std::fs::write(
            &config_path,
            r#"
# Severity level
severity = "Error"

# Disable specific rules
[rules]
# Disable negative instruction warnings
disabled_rules = ["CC-MEM-006"]
"#,
        )
        .unwrap();

        let (config, warning) = LintConfig::load_or_default(Some(&config_path));

        assert_eq!(config.severity, SeverityLevel::Error);
        assert!(!config.is_rule_enabled("CC-MEM-006"));
        assert!(warning.is_none());
    }

    #[test]
    fn test_config_invalid_severity_value() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join(".agnix.toml");
        std::fs::write(&config_path, r#"severity = "InvalidLevel""#).unwrap();

        let (config, warning) = LintConfig::load_or_default(Some(&config_path));

        // Should fall back to defaults with warning
        assert_eq!(config.severity, SeverityLevel::Warning);
        assert!(warning.is_some());
    }

    #[test]
    fn test_config_invalid_target_value() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join(".agnix.toml");
        std::fs::write(&config_path, r#"target = "InvalidTool""#).unwrap();

        let (config, warning) = LintConfig::load_or_default(Some(&config_path));

        // Should fall back to defaults with warning
        assert_eq!(config.target, TargetTool::Generic);
        assert!(warning.is_some());
    }

    #[test]
    fn test_config_wrong_type_for_disabled_rules() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join(".agnix.toml");
        std::fs::write(
            &config_path,
            r#"
[rules]
disabled_rules = "AS-001"
"#,
        )
        .unwrap();

        let (config, warning) = LintConfig::load_or_default(Some(&config_path));

        // Should fall back to defaults with warning (wrong type)
        assert!(config.rules.disabled_rules.is_empty());
        assert!(warning.is_some());
    }

    #[test]
    fn test_config_wrong_type_for_exclude() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join(".agnix.toml");
        std::fs::write(&config_path, r#"exclude = "node_modules""#).unwrap();

        let (config, warning) = LintConfig::load_or_default(Some(&config_path));

        // Should fall back to defaults with warning (wrong type)
        assert!(warning.is_some());
        // Config should have default exclude values
        assert!(config.exclude.contains(&"node_modules/**".to_string()));
    }

    // ===== Config Interaction Tests =====

    #[test]
    fn test_target_and_tools_interaction() {
        // When both target and tools are set, tools takes precedence
        let toml_str = r#"
target = "Cursor"
tools = ["claude-code"]
"#;
        let config: LintConfig = toml::from_str(toml_str).unwrap();

        // Claude Code rules should be enabled (from tools)
        assert!(config.is_rule_enabled("CC-SK-001"));
        // Cursor rules should be disabled (not in tools)
        assert!(!config.is_rule_enabled("CUR-001"));
    }

    #[test]
    fn test_category_disabled_overrides_target() {
        let toml_str = r#"
target = "ClaudeCode"

[rules]
skills = false
"#;
        let config: LintConfig = toml::from_str(toml_str).unwrap();

        // Even with ClaudeCode target, skills category is disabled
        assert!(!config.is_rule_enabled("AS-001"));
        assert!(!config.is_rule_enabled("CC-SK-001"));
    }

    #[test]
    fn test_disabled_rules_overrides_category_enabled() {
        let toml_str = r#"
[rules]
skills = true
disabled_rules = ["AS-001"]
"#;
        let config: LintConfig = toml::from_str(toml_str).unwrap();

        // Category is enabled but specific rule is disabled
        assert!(!config.is_rule_enabled("AS-001"));
        assert!(config.is_rule_enabled("AS-002"));
    }

    // ===== Serialization Round-Trip Tests =====

    #[test]
    fn test_config_serialize_deserialize_roundtrip() {
        let mut config = LintConfig::default();
        config.severity = SeverityLevel::Error;
        config.target = TargetTool::ClaudeCode;
        config.rules.skills = false;
        config.rules.disabled_rules = vec!["CC-MEM-006".to_string()];

        let serialized = toml::to_string(&config).unwrap();
        let deserialized: LintConfig = toml::from_str(&serialized).unwrap();

        assert_eq!(deserialized.severity, SeverityLevel::Error);
        assert_eq!(deserialized.target, TargetTool::ClaudeCode);
        assert!(!deserialized.rules.skills);
        assert_eq!(deserialized.rules.disabled_rules, vec!["CC-MEM-006"]);
    }

    #[test]
    fn test_default_config_serializes_cleanly() {
        let config = LintConfig::default();
        let serialized = toml::to_string(&config).unwrap();

        // Should be valid TOML
        let _: LintConfig = toml::from_str(&serialized).unwrap();
    }

    // ===== Real-World Config Scenarios =====

    #[test]
    fn test_minimal_disable_warnings_config() {
        // Common use case: user just wants to disable some noisy warnings
        let toml_str = r#"
[rules]
disabled_rules = [
    "CC-MEM-006",  # Negative instructions
    "PE-003",      # Weak language
    "XP-001",      # Hard-coded paths
]
"#;
        let config: LintConfig = toml::from_str(toml_str).unwrap();

        assert!(!config.is_rule_enabled("CC-MEM-006"));
        assert!(!config.is_rule_enabled("PE-003"));
        assert!(!config.is_rule_enabled("XP-001"));
        // Everything else should work normally
        assert!(config.is_rule_enabled("AS-001"));
        assert!(config.is_rule_enabled("MCP-001"));
    }

    #[test]
    fn test_multi_tool_project_config() {
        // Project that targets both Claude Code and Cursor
        let toml_str = r#"
tools = ["claude-code", "cursor"]
exclude = ["node_modules/**", ".git/**", "dist/**"]

[rules]
disabled_rules = ["VER-001"]  # Don't warn about version pinning
"#;
        let config: LintConfig = toml::from_str(toml_str).unwrap();

        assert!(config.is_rule_enabled("CC-SK-001"));
        assert!(config.is_rule_enabled("CUR-001"));
        assert!(!config.is_rule_enabled("VER-001"));
    }

    #[test]
    fn test_strict_ci_config() {
        // Strict config for CI pipeline
        let toml_str = r#"
severity = "Error"
target = "ClaudeCode"

[rules]
# Enable everything
skills = true
hooks = true
memory = true
xml = true
mcp = true
disabled_rules = []
"#;
        let config: LintConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.severity, SeverityLevel::Error);
        assert!(config.rules.skills);
        assert!(config.rules.hooks);
        assert!(config.rules.disabled_rules.is_empty());
    }

    // ===== FileSystem Abstraction Tests =====

    #[test]
    fn test_default_config_uses_real_filesystem() {
        let config = LintConfig::default();

        // Default fs() should be RealFileSystem
        let fs = config.fs();

        // Verify it works by checking a file that should exist
        assert!(fs.exists(Path::new("Cargo.toml")));
        assert!(!fs.exists(Path::new("nonexistent_xyz_abc.txt")));
    }

    #[test]
    fn test_set_fs_replaces_filesystem() {
        use crate::fs::{FileSystem, MockFileSystem};

        let mut config = LintConfig::default();

        // Create a mock filesystem with a test file
        let mock_fs = Arc::new(MockFileSystem::new());
        mock_fs.add_file("/mock/test.md", "mock content");

        // Replace the filesystem (coerce to trait object)
        let fs_arc: Arc<dyn FileSystem> = Arc::clone(&mock_fs) as Arc<dyn FileSystem>;
        config.set_fs(fs_arc);

        // Verify fs() returns the mock
        let fs = config.fs();
        assert!(fs.exists(Path::new("/mock/test.md")));
        assert!(!fs.exists(Path::new("Cargo.toml"))); // Real file shouldn't exist in mock

        // Verify we can read from the mock
        let content = fs.read_to_string(Path::new("/mock/test.md")).unwrap();
        assert_eq!(content, "mock content");
    }

    #[test]
    fn test_set_fs_is_not_serialized() {
        use crate::fs::MockFileSystem;

        let mut config = LintConfig::default();
        config.set_fs(Arc::new(MockFileSystem::new()));

        // Serialize and deserialize
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: LintConfig = toml::from_str(&serialized).unwrap();

        // Deserialized config should have RealFileSystem (default)
        // because fs is marked with #[serde(skip)]
        let fs = deserialized.fs();
        // RealFileSystem can see Cargo.toml, MockFileSystem cannot
        assert!(fs.exists(Path::new("Cargo.toml")));
    }

    #[test]
    fn test_fs_can_be_shared_across_threads() {
        use crate::fs::{FileSystem, MockFileSystem};
        use std::thread;

        let mut config = LintConfig::default();
        let mock_fs = Arc::new(MockFileSystem::new());
        mock_fs.add_file("/test/file.md", "content");

        // Coerce to trait object and set
        let fs_arc: Arc<dyn FileSystem> = mock_fs;
        config.set_fs(fs_arc);

        // Get fs reference
        let fs = Arc::clone(config.fs());

        // Spawn a thread that uses the filesystem
        let handle = thread::spawn(move || {
            assert!(fs.exists(Path::new("/test/file.md")));
            let content = fs.read_to_string(Path::new("/test/file.md")).unwrap();
            assert_eq!(content, "content");
        });

        handle.join().unwrap();
    }

    #[test]
    fn test_config_fs_returns_arc_ref() {
        let config = LintConfig::default();

        // fs() returns &Arc<dyn FileSystem>
        let fs1 = config.fs();
        let fs2 = config.fs();

        // Both should point to the same Arc
        assert!(Arc::ptr_eq(fs1, fs2));
    }

    // ===== RuntimeContext Tests =====
    //
    // These tests verify the internal RuntimeContext type works correctly.
    // RuntimeContext is private, but we test it through LintConfig's public API.

    #[test]
    fn test_runtime_context_default_values() {
        let config = LintConfig::default();

        // Default RuntimeContext should have:
        // - root_dir: None
        // - import_cache: None
        // - fs: RealFileSystem
        assert!(config.root_dir().is_none());
        assert!(config.import_cache().is_none());
        // fs should work with real files
        assert!(config.fs().exists(Path::new("Cargo.toml")));
    }

    #[test]
    fn test_runtime_context_root_dir_accessor() {
        let mut config = LintConfig::default();
        assert!(config.root_dir().is_none());

        config.set_root_dir(PathBuf::from("/test/path"));
        assert_eq!(config.root_dir(), Some(&PathBuf::from("/test/path")));
    }

    #[test]
    fn test_runtime_context_clone_shares_fs() {
        use crate::fs::{FileSystem, MockFileSystem};

        let mut config = LintConfig::default();
        let mock_fs = Arc::new(MockFileSystem::new());
        mock_fs.add_file("/shared/file.md", "content");

        let fs_arc: Arc<dyn FileSystem> = Arc::clone(&mock_fs) as Arc<dyn FileSystem>;
        config.set_fs(fs_arc);

        // Clone the config
        let cloned = config.clone();

        // Both should share the same filesystem Arc
        assert!(Arc::ptr_eq(config.fs(), cloned.fs()));

        // Both can access the same file
        assert!(config.fs().exists(Path::new("/shared/file.md")));
        assert!(cloned.fs().exists(Path::new("/shared/file.md")));
    }

    #[test]
    fn test_runtime_context_not_serialized() {
        let mut config = LintConfig::default();
        config.set_root_dir(PathBuf::from("/test/root"));

        // Serialize
        let serialized = toml::to_string(&config).unwrap();

        // The serialized TOML should NOT contain root_dir
        assert!(!serialized.contains("root_dir"));
        assert!(!serialized.contains("/test/root"));

        // Deserialize
        let deserialized: LintConfig = toml::from_str(&serialized).unwrap();

        // Deserialized config should have default RuntimeContext (root_dir = None)
        assert!(deserialized.root_dir().is_none());
    }

    // ===== DefaultRuleFilter Tests =====
    //
    // These tests verify the internal DefaultRuleFilter logic through
    // LintConfig's public is_rule_enabled() method.

    #[test]
    fn test_rule_filter_disabled_rules_checked_first() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["AS-001".to_string()];

        // Rule should be disabled regardless of category or target
        assert!(!config.is_rule_enabled("AS-001"));

        // Other AS-* rules should still be enabled
        assert!(config.is_rule_enabled("AS-002"));
    }

    #[test]
    fn test_rule_filter_target_checked_second() {
        let mut config = LintConfig::default();
        config.target = TargetTool::Cursor;

        // CC-* rules should be disabled for Cursor target
        assert!(!config.is_rule_enabled("CC-SK-001"));

        // But AS-* rules (generic) should still work
        assert!(config.is_rule_enabled("AS-001"));
    }

    #[test]
    fn test_rule_filter_category_checked_third() {
        let mut config = LintConfig::default();
        config.rules.skills = false;

        // Skills category disabled
        assert!(!config.is_rule_enabled("AS-001"));
        assert!(!config.is_rule_enabled("CC-SK-001"));

        // Other categories still enabled
        assert!(config.is_rule_enabled("CC-HK-001"));
        assert!(config.is_rule_enabled("MCP-001"));
    }

    #[test]
    fn test_rule_filter_order_of_checks() {
        let mut config = LintConfig::default();
        config.target = TargetTool::ClaudeCode;
        config.rules.skills = true;
        config.rules.disabled_rules = vec!["CC-SK-001".to_string()];

        // disabled_rules takes precedence over everything
        assert!(!config.is_rule_enabled("CC-SK-001"));

        // Other CC-SK-* rules are enabled (category enabled + target matches)
        assert!(config.is_rule_enabled("CC-SK-002"));
    }

    #[test]
    fn test_rule_filter_is_tool_alias_works_through_config() {
        // Test that is_tool_alias is properly exposed
        assert!(LintConfig::is_tool_alias("copilot", "github-copilot"));
        assert!(!LintConfig::is_tool_alias("unknown", "github-copilot"));
    }

    // ===== Serde Round-Trip Tests =====

    #[test]
    fn test_serde_roundtrip_preserves_all_public_fields() {
        let mut config = LintConfig::default();
        config.severity = SeverityLevel::Error;
        config.target = TargetTool::ClaudeCode;
        config.tools = vec!["claude-code".to_string(), "cursor".to_string()];
        config.exclude = vec!["custom/**".to_string()];
        config.mcp_protocol_version = Some("2024-11-05".to_string());
        config.tool_versions.claude_code = Some("1.0.0".to_string());
        config.spec_revisions.mcp_protocol = Some("2025-06-18".to_string());
        config.rules.skills = false;
        config.rules.disabled_rules = vec!["MCP-001".to_string()];

        // Also set runtime values (should NOT be serialized)
        config.set_root_dir(PathBuf::from("/test/root"));

        // Serialize
        let serialized = toml::to_string(&config).unwrap();

        // Deserialize
        let deserialized: LintConfig = toml::from_str(&serialized).unwrap();

        // All public fields should be preserved
        assert_eq!(deserialized.severity, SeverityLevel::Error);
        assert_eq!(deserialized.target, TargetTool::ClaudeCode);
        assert_eq!(deserialized.tools, vec!["claude-code", "cursor"]);
        assert_eq!(deserialized.exclude, vec!["custom/**"]);
        assert_eq!(
            deserialized.mcp_protocol_version,
            Some("2024-11-05".to_string())
        );
        assert_eq!(
            deserialized.tool_versions.claude_code,
            Some("1.0.0".to_string())
        );
        assert_eq!(
            deserialized.spec_revisions.mcp_protocol,
            Some("2025-06-18".to_string())
        );
        assert!(!deserialized.rules.skills);
        assert_eq!(deserialized.rules.disabled_rules, vec!["MCP-001"]);

        // Runtime values should be reset to defaults
        assert!(deserialized.root_dir().is_none());
    }

    #[test]
    fn test_serde_runtime_fields_not_included() {
        use crate::fs::MockFileSystem;

        let mut config = LintConfig::default();
        config.set_root_dir(PathBuf::from("/test"));
        config.set_fs(Arc::new(MockFileSystem::new()));

        let serialized = toml::to_string(&config).unwrap();

        // Runtime fields should not appear in serialized output
        assert!(!serialized.contains("runtime"));
        assert!(!serialized.contains("root_dir"));
        assert!(!serialized.contains("import_cache"));
        assert!(!serialized.contains("fs"));
    }

    // ===== JSON Schema Generation Tests =====

    #[test]
    fn test_generate_schema_produces_valid_json() {
        let schema = super::generate_schema();
        let json = serde_json::to_string_pretty(&schema).unwrap();

        // Verify it's valid JSON by parsing it back
        let _: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Verify basic schema structure
        assert!(json.contains("\"$schema\""));
        assert!(json.contains("\"title\": \"LintConfig\""));
        assert!(json.contains("\"type\": \"object\""));
    }

    #[test]
    fn test_generate_schema_includes_all_fields() {
        let schema = super::generate_schema();
        let json = serde_json::to_string(&schema).unwrap();

        // Check main config fields
        assert!(json.contains("\"severity\""));
        assert!(json.contains("\"rules\""));
        assert!(json.contains("\"exclude\""));
        assert!(json.contains("\"target\""));
        assert!(json.contains("\"tools\""));
        assert!(json.contains("\"tool_versions\""));
        assert!(json.contains("\"spec_revisions\""));

        // Check runtime fields are NOT included
        assert!(!json.contains("\"root_dir\""));
        assert!(!json.contains("\"import_cache\""));
        assert!(!json.contains("\"runtime\""));
    }

    #[test]
    fn test_generate_schema_includes_definitions() {
        let schema = super::generate_schema();
        let json = serde_json::to_string(&schema).unwrap();

        // Check definitions for nested types
        assert!(json.contains("\"RuleConfig\""));
        assert!(json.contains("\"SeverityLevel\""));
        assert!(json.contains("\"TargetTool\""));
        assert!(json.contains("\"ToolVersions\""));
        assert!(json.contains("\"SpecRevisions\""));
    }

    #[test]
    fn test_generate_schema_includes_descriptions() {
        let schema = super::generate_schema();
        let json = serde_json::to_string(&schema).unwrap();

        // Check that descriptions are present
        assert!(json.contains("\"description\""));
        assert!(json.contains("Minimum severity level to report"));
        assert!(json.contains("Glob patterns for paths to exclude"));
        assert!(json.contains("Enable Agent Skills validation rules"));
    }

    // ===== Config Validation Tests =====

    #[test]
    fn test_validate_empty_config_no_warnings() {
        let config = LintConfig::default();
        let warnings = config.validate();

        // Default config should have no warnings
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_validate_valid_disabled_rules() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec![
            "AS-001".to_string(),
            "CC-SK-007".to_string(),
            "MCP-001".to_string(),
            "PE-003".to_string(),
            "XP-001".to_string(),
            "AGM-001".to_string(),
            "COP-001".to_string(),
            "CUR-001".to_string(),
            "XML-001".to_string(),
            "REF-001".to_string(),
            "VER-001".to_string(),
        ];

        let warnings = config.validate();

        // All these are valid rule IDs
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_validate_invalid_disabled_rule_pattern() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["INVALID-001".to_string(), "UNKNOWN-999".to_string()];

        let warnings = config.validate();

        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].field.contains("disabled_rules"));
        assert!(warnings[0].message.contains("Unknown rule ID pattern"));
        assert!(warnings[1].message.contains("UNKNOWN-999"));
    }

    #[test]
    fn test_validate_ver_prefix_accepted() {
        // Regression test for #233
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["VER-001".to_string()];

        let warnings = config.validate();

        assert!(warnings.is_empty());
    }

    #[test]
    fn test_validate_valid_tools() {
        let mut config = LintConfig::default();
        config.tools = vec![
            "claude-code".to_string(),
            "cursor".to_string(),
            "codex".to_string(),
            "copilot".to_string(),
            "github-copilot".to_string(),
            "generic".to_string(),
        ];

        let warnings = config.validate();

        // All these are valid tool names
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_validate_invalid_tool() {
        let mut config = LintConfig::default();
        config.tools = vec!["unknown-tool".to_string(), "invalid".to_string()];

        let warnings = config.validate();

        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].field == "tools");
        assert!(warnings[0].message.contains("Unknown tool"));
        assert!(warnings[0].message.contains("unknown-tool"));
    }

    #[test]
    fn test_validate_deprecated_mcp_protocol_version() {
        let mut config = LintConfig::default();
        config.mcp_protocol_version = Some("2024-11-05".to_string());

        let warnings = config.validate();

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].field == "mcp_protocol_version");
        assert!(warnings[0].message.contains("deprecated"));
        assert!(
            warnings[0]
                .suggestion
                .as_ref()
                .unwrap()
                .contains("spec_revisions.mcp_protocol")
        );
    }

    #[test]
    fn test_validate_mixed_valid_invalid() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec![
            "AS-001".to_string(),    // Valid
            "INVALID-1".to_string(), // Invalid
            "CC-SK-001".to_string(), // Valid
        ];
        config.tools = vec![
            "claude-code".to_string(), // Valid
            "bad-tool".to_string(),    // Invalid
        ];

        let warnings = config.validate();

        // Should have exactly 2 warnings: one for invalid rule, one for invalid tool
        assert_eq!(warnings.len(), 2);
    }

    #[test]
    fn test_config_warning_has_suggestion() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["INVALID-001".to_string()];

        let warnings = config.validate();

        assert!(!warnings.is_empty());
        assert!(warnings[0].suggestion.is_some());
    }

    #[test]
    fn test_validate_case_insensitive_tools() {
        // Tools should be validated case-insensitively
        let mut config = LintConfig::default();
        config.tools = vec![
            "CLAUDE-CODE".to_string(),
            "CuRsOr".to_string(),
            "COPILOT".to_string(),
        ];

        let warnings = config.validate();

        // All should be valid (case-insensitive)
        assert!(
            warnings.is_empty(),
            "Expected no warnings for valid tools with different cases, got: {:?}",
            warnings
        );
    }

    #[test]
    fn test_validate_multiple_warnings_same_category() {
        // Test that multiple invalid items of the same type are all reported
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec![
            "INVALID-001".to_string(),
            "FAKE-RULE".to_string(),
            "NOT-A-RULE".to_string(),
        ];

        let warnings = config.validate();

        // Should have 3 warnings, one for each invalid rule
        assert_eq!(warnings.len(), 3, "Expected 3 warnings for 3 invalid rules");

        // Verify each invalid rule is mentioned
        let warning_messages: Vec<&str> = warnings.iter().map(|w| w.message.as_str()).collect();
        assert!(warning_messages.iter().any(|m| m.contains("INVALID-001")));
        assert!(warning_messages.iter().any(|m| m.contains("FAKE-RULE")));
        assert!(warning_messages.iter().any(|m| m.contains("NOT-A-RULE")));
    }

    #[test]
    fn test_validate_multiple_invalid_tools() {
        let mut config = LintConfig::default();
        config.tools = vec![
            "unknown-tool".to_string(),
            "bad-editor".to_string(),
            "claude-code".to_string(), // This one is valid
        ];

        let warnings = config.validate();

        // Should have 2 warnings for the 2 invalid tools
        assert_eq!(warnings.len(), 2, "Expected 2 warnings for 2 invalid tools");
    }

    #[test]
    fn test_validate_empty_string_in_tools() {
        // Empty strings should be flagged as invalid
        let mut config = LintConfig::default();
        config.tools = vec!["".to_string(), "claude-code".to_string()];

        let warnings = config.validate();

        // Empty string is not a valid tool
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("Unknown tool ''"));
    }

    #[test]
    fn test_validate_deprecated_target_field() {
        let mut config = LintConfig::default();
        config.target = TargetTool::ClaudeCode;
        // tools is empty, so target deprecation warning should fire

        let warnings = config.validate();

        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].field, "target");
        assert!(warnings[0].message.contains("deprecated"));
        assert!(warnings[0].suggestion.as_ref().unwrap().contains("tools"));
    }

    #[test]
    fn test_validate_target_with_tools_no_warning() {
        // When both target and tools are set, don't warn about target
        // because tools takes precedence
        let mut config = LintConfig::default();
        config.target = TargetTool::ClaudeCode;
        config.tools = vec!["claude-code".to_string()];

        let warnings = config.validate();

        // No warning because tools is set
        assert!(warnings.is_empty());
    }

    // =========================================================================
    // FilesConfig tests
    // =========================================================================

    #[test]
    fn test_files_config_default_is_empty() {
        let files = FilesConfig::default();
        assert!(files.include_as_memory.is_empty());
        assert!(files.include_as_generic.is_empty());
        assert!(files.exclude.is_empty());
    }

    #[test]
    fn test_lint_config_default_has_empty_files() {
        let config = LintConfig::default();
        assert!(config.files.include_as_memory.is_empty());
        assert!(config.files.include_as_generic.is_empty());
        assert!(config.files.exclude.is_empty());
    }

    #[test]
    fn test_files_config_toml_deserialization() {
        let toml_str = r#"
[files]
include_as_memory = ["docs/ai-rules/*.md", "custom/INSTRUCTIONS.md"]
include_as_generic = ["internal/*.md"]
exclude = ["drafts/**"]
"#;
        let config: LintConfig = toml::from_str(toml_str).expect("should parse");
        assert_eq!(config.files.include_as_memory.len(), 2);
        assert_eq!(config.files.include_as_memory[0], "docs/ai-rules/*.md");
        assert_eq!(config.files.include_as_memory[1], "custom/INSTRUCTIONS.md");
        assert_eq!(config.files.include_as_generic.len(), 1);
        assert_eq!(config.files.include_as_generic[0], "internal/*.md");
        assert_eq!(config.files.exclude.len(), 1);
        assert_eq!(config.files.exclude[0], "drafts/**");
    }

    #[test]
    fn test_files_config_partial_toml() {
        let toml_str = r#"
[files]
include_as_memory = ["custom.md"]
"#;
        let config: LintConfig = toml::from_str(toml_str).expect("should parse");
        assert_eq!(config.files.include_as_memory.len(), 1);
        assert!(config.files.include_as_generic.is_empty());
        assert!(config.files.exclude.is_empty());
    }

    #[test]
    fn test_files_config_empty_section() {
        let toml_str = r#"
[files]
"#;
        let config: LintConfig = toml::from_str(toml_str).expect("should parse");
        assert!(config.files.include_as_memory.is_empty());
        assert!(config.files.include_as_generic.is_empty());
        assert!(config.files.exclude.is_empty());
    }

    #[test]
    fn test_files_config_omitted_section() {
        let toml_str = r#"
severity = "Warning"
"#;
        let config: LintConfig = toml::from_str(toml_str).expect("should parse");
        assert!(config.files.include_as_memory.is_empty());
    }

    #[test]
    fn test_validate_files_invalid_glob() {
        let mut config = LintConfig::default();
        config.files.include_as_memory = vec!["[invalid".to_string()];

        let warnings = config.validate();
        assert!(
            warnings
                .iter()
                .any(|w| w.field == "files.include_as_memory"),
            "should warn about invalid glob pattern"
        );
    }

    #[test]
    fn test_validate_files_valid_globs_no_warnings() {
        let mut config = LintConfig::default();
        config.files.include_as_memory = vec!["docs/**/*.md".to_string()];
        config.files.include_as_generic = vec!["internal/*.md".to_string()];
        config.files.exclude = vec!["drafts/**".to_string()];

        let warnings = config.validate();
        assert!(
            warnings.is_empty(),
            "valid globs should not produce warnings: {:?}",
            warnings
        );
    }

    #[test]
    fn test_validate_files_path_traversal_rejected() {
        let mut config = LintConfig::default();
        config.files.include_as_memory = vec!["../outside/secrets.md".to_string()];

        let warnings = config.validate();
        assert!(
            warnings
                .iter()
                .any(|w| w.field == "files.include_as_memory" && w.message.contains("../")),
            "should warn about path traversal pattern: {:?}",
            warnings
        );
    }

    #[test]
    fn test_validate_files_absolute_path_rejected() {
        let mut config = LintConfig::default();
        config.files.include_as_generic = vec!["/etc/passwd".to_string()];

        let warnings = config.validate();
        assert!(
            warnings
                .iter()
                .any(|w| w.field == "files.include_as_generic" && w.message.contains("absolute")),
            "should warn about absolute path pattern: {:?}",
            warnings
        );

        // Also test Windows drive letter
        let mut config2 = LintConfig::default();
        config2.files.exclude = vec!["C:\\Users\\secret".to_string()];

        let warnings2 = config2.validate();
        assert!(
            warnings2
                .iter()
                .any(|w| w.field == "files.exclude" && w.message.contains("absolute")),
            "should warn about Windows absolute path pattern: {:?}",
            warnings2
        );
    }

    #[test]
    fn test_validate_files_pattern_count_limit() {
        let mut config = LintConfig::default();
        // Create 101 patterns to exceed MAX_FILE_PATTERNS (100)
        config.files.include_as_memory = (0..101).map(|i| format!("pattern-{}.md", i)).collect();

        let warnings = config.validate();
        assert!(
            warnings.iter().any(|w| w.field == "files.include_as_memory"
                && w.message.contains("101")
                && w.message.contains("100")),
            "should warn about exceeding pattern count limit: {:?}",
            warnings
        );

        // 100 patterns should not produce a count warning
        let mut config2 = LintConfig::default();
        config2.files.include_as_memory = (0..100).map(|i| format!("pattern-{}.md", i)).collect();

        let warnings2 = config2.validate();
        assert!(
            !warnings2.iter().any(|w| w.message.contains("exceeds")),
            "100 patterns should not produce a count warning: {:?}",
            warnings2
        );
    }

    // =========================================================================
    // LintConfigBuilder tests
    // =========================================================================

    #[test]
    fn test_builder_default_matches_default() {
        let from_builder = LintConfig::builder().build().unwrap();
        let from_default = LintConfig::default();

        assert_eq!(from_builder.severity(), from_default.severity());
        assert_eq!(from_builder.target(), from_default.target());
        assert_eq!(from_builder.tools(), from_default.tools());
        assert_eq!(from_builder.exclude(), from_default.exclude());
        assert_eq!(from_builder.locale(), from_default.locale());
        assert_eq!(
            from_builder.max_files_to_validate(),
            from_default.max_files_to_validate()
        );
        assert_eq!(
            from_builder.rules().disabled_rules,
            from_default.rules().disabled_rules
        );
        assert_eq!(
            from_builder.rules().disabled_validators,
            from_default.rules().disabled_validators
        );
    }

    #[test]
    fn test_builder_custom_severity() {
        let config = LintConfig::builder()
            .severity(SeverityLevel::Error)
            .build()
            .unwrap();

        assert_eq!(config.severity(), SeverityLevel::Error);
    }

    #[test]
    fn test_builder_custom_target() {
        // target is deprecated, so build() validates and rejects it;
        // use build_unchecked() to test the setter works
        let config = LintConfig::builder()
            .target(TargetTool::ClaudeCode)
            .build_unchecked();

        assert_eq!(config.target(), TargetTool::ClaudeCode);
    }

    #[test]
    fn test_builder_deprecated_target_rejected_by_build() {
        let result = LintConfig::builder().target(TargetTool::ClaudeCode).build();

        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::ValidationFailed(warnings) => {
                assert!(warnings.iter().any(|w| w.field == "target"));
            }
            other => panic!("Expected ValidationFailed, got: {:?}", other),
        }
    }

    #[test]
    fn test_builder_custom_tools() {
        let config = LintConfig::builder()
            .tools(vec!["claude-code".to_string(), "cursor".to_string()])
            .build()
            .unwrap();

        assert_eq!(config.tools(), &["claude-code", "cursor"]);
    }

    #[test]
    fn test_builder_custom_exclude() {
        let config = LintConfig::builder()
            .exclude(vec!["node_modules/**".to_string(), ".git/**".to_string()])
            .build()
            .unwrap();

        assert_eq!(
            config.exclude(),
            &["node_modules/**".to_string(), ".git/**".to_string()]
        );
    }

    #[test]
    fn test_builder_invalid_glob_returns_error() {
        let result = LintConfig::builder()
            .exclude(vec!["[invalid".to_string()])
            .build();

        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::InvalidGlobPattern { pattern, .. } => {
                assert_eq!(pattern, "[invalid");
            }
            other => panic!("Expected InvalidGlobPattern, got: {:?}", other),
        }
    }

    #[test]
    fn test_builder_path_traversal_returns_error() {
        let result = LintConfig::builder()
            .exclude(vec!["../secret/**".to_string()])
            .build();

        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::PathTraversal { pattern } => {
                assert_eq!(pattern, "../secret/**");
            }
            other => panic!("Expected PathTraversal, got: {:?}", other),
        }
    }

    #[test]
    fn test_builder_disable_rule() {
        let config = LintConfig::builder()
            .disable_rule("AS-001")
            .disable_rule("PE-003")
            .build()
            .unwrap();

        assert!(
            config
                .rules()
                .disabled_rules
                .contains(&"AS-001".to_string())
        );
        assert!(
            config
                .rules()
                .disabled_rules
                .contains(&"PE-003".to_string())
        );
        assert!(!config.is_rule_enabled("AS-001"));
        assert!(!config.is_rule_enabled("PE-003"));
    }

    #[test]
    fn test_builder_disable_validator() {
        let config = LintConfig::builder()
            .disable_validator("XmlValidator")
            .build()
            .unwrap();

        assert!(
            config
                .rules()
                .disabled_validators
                .contains(&"XmlValidator".to_string())
        );
    }

    #[test]
    fn test_builder_chaining() {
        // Uses build_unchecked() because target is a deprecated field
        let config = LintConfig::builder()
            .severity(SeverityLevel::Error)
            .target(TargetTool::Cursor)
            .tools(vec!["cursor".to_string()])
            .locale(Some("es".to_string()))
            .max_files_to_validate(Some(50))
            .disable_rule("PE-003")
            .build_unchecked();

        assert_eq!(config.severity(), SeverityLevel::Error);
        assert_eq!(config.target(), TargetTool::Cursor);
        assert_eq!(config.tools(), &["cursor"]);
        assert_eq!(config.locale(), Some("es"));
        assert_eq!(config.max_files_to_validate(), Some(50));
        assert!(
            config
                .rules()
                .disabled_rules
                .contains(&"PE-003".to_string())
        );
    }

    #[test]
    fn test_builder_build_unchecked_skips_validation() {
        // build_unchecked allows invalid patterns that build() would reject
        let config = LintConfig::builder()
            .exclude(vec!["[invalid".to_string()])
            .build_unchecked();

        assert_eq!(config.exclude(), &["[invalid".to_string()]);
    }

    #[test]
    fn test_builder_root_dir() {
        let config = LintConfig::builder()
            .root_dir(PathBuf::from("/my/project"))
            .build()
            .unwrap();

        assert_eq!(config.root_dir(), Some(&PathBuf::from("/my/project")));
    }

    #[test]
    fn test_builder_locale_none() {
        let config = LintConfig::builder().locale(None).build().unwrap();

        assert!(config.locale().is_none());
    }

    #[test]
    fn test_builder_locale_some() {
        let config = LintConfig::builder()
            .locale(Some("fr".to_string()))
            .build()
            .unwrap();

        assert_eq!(config.locale(), Some("fr"));
    }

    #[test]
    fn test_builder_mcp_protocol_version() {
        // mcp_protocol_version is deprecated, so use build_unchecked()
        let config = LintConfig::builder()
            .mcp_protocol_version(Some("2024-11-05".to_string()))
            .build_unchecked();

        assert_eq!(config.mcp_protocol_version_raw(), Some("2024-11-05"));
    }

    #[test]
    fn test_builder_deprecated_mcp_protocol_rejected_by_build() {
        let result = LintConfig::builder()
            .mcp_protocol_version(Some("2024-11-05".to_string()))
            .build();

        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::ValidationFailed(warnings) => {
                assert!(warnings.iter().any(|w| w.field == "mcp_protocol_version"));
            }
            other => panic!("Expected ValidationFailed, got: {:?}", other),
        }
    }

    #[test]
    fn test_builder_files_config() {
        let files = FilesConfig {
            include_as_memory: vec!["memory.md".to_string()],
            include_as_generic: vec!["generic.md".to_string()],
            exclude: vec!["drafts/**".to_string()],
        };

        let config = LintConfig::builder().files(files.clone()).build().unwrap();

        assert_eq!(
            config.files_config().include_as_memory,
            files.include_as_memory
        );
        assert_eq!(
            config.files_config().include_as_generic,
            files.include_as_generic
        );
        assert_eq!(config.files_config().exclude, files.exclude);
    }

    #[test]
    fn test_builder_duplicate_disable_rule_deduplicates() {
        let config = LintConfig::builder()
            .disable_rule("AS-001")
            .disable_rule("AS-001")
            .build()
            .unwrap();

        let count = config
            .rules()
            .disabled_rules
            .iter()
            .filter(|r| *r == "AS-001")
            .count();
        assert_eq!(count, 1, "Duplicate disable_rule should be deduplicated");
    }

    #[test]
    fn test_builder_duplicate_disable_validator_deduplicates() {
        let config = LintConfig::builder()
            .disable_validator("XmlValidator")
            .disable_validator("XmlValidator")
            .build()
            .unwrap();

        let count = config
            .rules()
            .disabled_validators
            .iter()
            .filter(|v| *v == "XmlValidator")
            .count();
        assert_eq!(
            count, 1,
            "Duplicate disable_validator should be deduplicated"
        );
    }

    #[test]
    fn test_builder_backslash_exclude_normalized() {
        // Windows-style path separators should be accepted
        let result = LintConfig::builder()
            .exclude(vec!["node_modules\\**".to_string()])
            .build();

        // Glob validation normalizes backslashes to forward slashes
        assert!(result.is_ok());
    }

    #[test]
    fn test_builder_path_traversal_with_backslash() {
        let result = LintConfig::builder()
            .exclude(vec!["..\\secret\\**".to_string()])
            .build();

        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::PathTraversal { .. } => {}
            other => panic!("Expected PathTraversal, got: {:?}", other),
        }
    }

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::InvalidGlobPattern {
            pattern: "[bad".to_string(),
            error: "unclosed bracket".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("[bad"));
        assert!(msg.contains("unclosed bracket"));

        let err = ConfigError::PathTraversal {
            pattern: "../etc/passwd".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("../etc/passwd"));

        let warnings = vec![ConfigWarning {
            field: "test".to_string(),
            message: "bad config".to_string(),
            suggestion: None,
        }];
        let err = ConfigError::ValidationFailed(warnings);
        let msg = err.to_string();
        assert!(msg.contains("1 warning(s)"));
    }

    #[test]
    fn test_builder_tool_versions() {
        let tv = ToolVersions {
            claude_code: Some("1.2.3".to_string()),
            ..ToolVersions::default()
        };
        let config = LintConfig::builder()
            .tool_versions(tv.clone())
            .build_unchecked();
        assert_eq!(config.tool_versions().claude_code, tv.claude_code);
    }

    #[test]
    fn test_builder_spec_revisions() {
        let sr = SpecRevisions {
            mcp_protocol: Some("2025-03-26".to_string()),
            ..SpecRevisions::default()
        };
        let config = LintConfig::builder()
            .spec_revisions(sr.clone())
            .build_unchecked();
        assert_eq!(config.spec_revisions().mcp_protocol, sr.mcp_protocol);
    }

    #[test]
    fn test_builder_rules() {
        let mut rules = RuleConfig::default();
        rules.skills = false;
        rules.hooks = false;
        let config = LintConfig::builder().rules(rules).build_unchecked();
        assert!(!config.rules().skills);
        assert!(!config.rules().hooks);
    }

    #[test]
    fn test_builder_import_cache() {
        let cache = crate::parsers::ImportCache::default();
        let config = LintConfig::builder().import_cache(cache).build_unchecked();
        assert!(config.import_cache().is_some());
    }

    #[test]
    fn test_builder_fs() {
        use crate::fs::MockFileSystem;
        let fs = Arc::new(MockFileSystem::new());
        let config = LintConfig::builder().fs(fs).build_unchecked();
        // Verify fs was set (we can't directly compare Arc<dyn FileSystem>,
        // but if it compiled and didn't panic, the builder method works)
        let _ = config.fs();
    }

    #[test]
    fn test_builder_files_include_invalid_glob_rejected() {
        let files = FilesConfig {
            include_as_memory: vec!["[invalid".to_string()],
            ..FilesConfig::default()
        };
        let result = LintConfig::builder().files(files).build();
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::InvalidGlobPattern { pattern, error } => {
                assert_eq!(pattern, "[invalid");
                assert!(error.contains("files.include_as_memory"));
            }
            other => panic!("Expected InvalidGlobPattern, got: {:?}", other),
        }
    }

    #[test]
    fn test_builder_files_include_path_traversal_rejected() {
        let files = FilesConfig {
            include_as_generic: vec!["../secret.md".to_string()],
            ..FilesConfig::default()
        };
        let result = LintConfig::builder().files(files).build();
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::PathTraversal { pattern } => {
                assert_eq!(pattern, "../secret.md");
            }
            other => panic!("Expected PathTraversal, got: {:?}", other),
        }
    }

    #[test]
    fn test_builder_unknown_tool_rejected() {
        let result = LintConfig::builder()
            .tools(vec!["fake-tool".to_string()])
            .build();
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::ValidationFailed(warnings) => {
                assert!(warnings.iter().any(|w| w.field == "tools"));
            }
            other => panic!("Expected ValidationFailed, got: {:?}", other),
        }
    }

    #[test]
    fn test_builder_unknown_rule_rejected() {
        let result = LintConfig::builder().disable_rule("FAKE-001").build();
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::ValidationFailed(warnings) => {
                assert!(warnings.iter().any(|w| w.field == "rules.disabled_rules"));
            }
            other => panic!("Expected ValidationFailed, got: {:?}", other),
        }
    }

    #[test]
    fn test_builder_multiple_validation_errors() {
        let result = LintConfig::builder()
            .tools(vec!["fake-tool".to_string()])
            .disable_rule("FAKE-001")
            .build();
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::ValidationFailed(warnings) => {
                assert!(
                    warnings.len() >= 2,
                    "Expected at least 2 warnings, got {}",
                    warnings.len()
                );
            }
            other => panic!("Expected ValidationFailed, got: {:?}", other),
        }
    }

    #[test]
    fn test_builder_reuse_after_build() {
        let mut builder = LintConfig::builder();
        builder.severity(SeverityLevel::Error);
        let config1 = builder.build_unchecked();
        assert_eq!(config1.severity(), SeverityLevel::Error);

        // After build, builder state is drained - building again gives defaults
        let config2 = builder.build_unchecked();
        assert_eq!(config2.severity(), SeverityLevel::Warning);
    }

    #[test]
    fn test_builder_empty_exclude() {
        let config = LintConfig::builder().exclude(vec![]).build_unchecked();
        assert!(config.exclude().is_empty());
    }

    #[test]
    fn test_path_traversal_edge_cases() {
        // ".." alone
        let result = LintConfig::builder()
            .exclude(vec!["..".to_string()])
            .build();
        assert!(matches!(result, Err(ConfigError::PathTraversal { .. })));

        // "foo/../bar"
        let result = LintConfig::builder()
            .exclude(vec!["foo/../bar".to_string()])
            .build();
        assert!(matches!(result, Err(ConfigError::PathTraversal { .. })));

        // "foo/.."
        let result = LintConfig::builder()
            .exclude(vec!["foo/..".to_string()])
            .build();
        assert!(matches!(result, Err(ConfigError::PathTraversal { .. })));

        // "..foo" is NOT path traversal (just a name starting with ..)
        let result = LintConfig::builder()
            .exclude(vec!["..foo".to_string()])
            .build_unchecked();
        assert_eq!(result.exclude(), &["..foo".to_string()]);
    }
}
