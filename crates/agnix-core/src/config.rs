//! Linter configuration

use crate::file_utils::safe_read_file;
use crate::fs::{FileSystem, RealFileSystem};
use crate::schemas::mcp::DEFAULT_MCP_PROTOCOL_VERSION;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Tool version pinning for version-aware validation
///
/// When tool versions are pinned, validators can apply version-specific
/// behavior instead of using default assumptions. When not pinned,
/// validators will use sensible defaults and add assumption notes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolVersions {
    /// Claude Code version (e.g., "1.0.0")
    #[serde(default)]
    pub claude_code: Option<String>,

    /// Codex CLI version (e.g., "0.1.0")
    #[serde(default)]
    pub codex: Option<String>,

    /// Cursor version (e.g., "0.45.0")
    #[serde(default)]
    pub cursor: Option<String>,

    /// GitHub Copilot version (e.g., "1.0.0")
    #[serde(default)]
    pub copilot: Option<String>,
}

/// Specification revision pinning for version-aware validation
///
/// When spec revisions are pinned, validators can apply revision-specific
/// rules. When not pinned, validators use the latest known revision.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpecRevisions {
    /// MCP protocol version (e.g., "2025-06-18", "2024-11-05")
    #[serde(default)]
    pub mcp_protocol: Option<String>,

    /// Agent Skills specification revision
    #[serde(default)]
    pub agent_skills_spec: Option<String>,

    /// AGENTS.md specification revision
    #[serde(default)]
    pub agents_md_spec: Option<String>,
}

/// Configuration for the linter
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LintConfig {
    /// Severity level threshold
    pub severity: SeverityLevel,

    /// Rules to enable/disable
    pub rules: RuleConfig,

    /// Paths to exclude
    pub exclude: Vec<String>,

    /// Target tool (claude-code, cursor, codex, generic)
    /// Deprecated: Use `tools` array instead for multi-tool support
    pub target: TargetTool,

    /// Tools to validate for (e.g., ["claude-code", "cursor"])
    /// When specified, agnix automatically enables rules for these tools
    /// and disables rules for tools not in the list.
    /// Valid values: "claude-code", "cursor", "codex", "copilot", "generic"
    #[serde(default)]
    pub tools: Vec<String>,

    /// Expected MCP protocol version for validation (MCP-008)
    /// Deprecated: Use spec_revisions.mcp_protocol instead
    pub mcp_protocol_version: Option<String>,

    /// Tool version pinning for version-aware validation
    #[serde(default)]
    pub tool_versions: ToolVersions,

    /// Specification revision pinning for version-aware validation
    #[serde(default)]
    pub spec_revisions: SpecRevisions,

    /// Runtime-only validation root directory (not serialized)
    #[serde(skip)]
    pub root_dir: Option<PathBuf>,

    /// Shared import cache for project-level validation (not serialized).
    ///
    /// When set, validators can use this cache to share parsed import data
    /// across files, avoiding redundant parsing during import chain traversal.
    /// This is typically initialized by `validate_project_with_registry` and
    /// shared across all file validations in a project.
    #[serde(skip)]
    pub import_cache: Option<crate::parsers::ImportCache>,

    /// File system abstraction for testability (not serialized).
    ///
    /// Validators use this to perform file system operations. Defaults to
    /// `RealFileSystem` which delegates to `std::fs` and `file_utils`.
    /// For testing, this can be replaced with `MockFileSystem`.
    #[serde(skip)]
    fs: Arc<dyn FileSystem>,
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
            root_dir: None,
            import_cache: None,
            fs: Arc::new(RealFileSystem),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SeverityLevel {
    Error,
    Warning,
    Info,
}

/// Helper function for serde default
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConfig {
    /// Enable skills validation (AS-*, CC-SK-*)
    #[serde(default = "default_true")]
    pub skills: bool,

    /// Enable hooks validation (CC-HK-*)
    #[serde(default = "default_true")]
    pub hooks: bool,

    /// Enable agents validation (CC-AG-*)
    #[serde(default = "default_true")]
    pub agents: bool,

    /// Enable memory validation (CC-MEM-*)
    #[serde(default = "default_true")]
    pub memory: bool,

    /// Enable plugins validation (CC-PL-*)
    #[serde(default = "default_true")]
    pub plugins: bool,

    /// Enable XML balance checking (XML-*)
    #[serde(default = "default_true")]
    pub xml: bool,

    /// Enable MCP validation (MCP-*)
    #[serde(default = "default_true")]
    pub mcp: bool,

    /// Enable import reference validation (REF-*)
    #[serde(default = "default_true")]
    pub imports: bool,

    /// Enable cross-platform validation (XP-*)
    #[serde(default = "default_true")]
    pub cross_platform: bool,

    /// Enable AGENTS.md validation (AGM-*)
    #[serde(default = "default_true")]
    pub agents_md: bool,

    /// Enable GitHub Copilot validation (COP-*)
    #[serde(default = "default_true")]
    pub copilot: bool,

    /// Enable Cursor project rules validation (CUR-*)
    #[serde(default = "default_true")]
    pub cursor: bool,

    /// Enable prompt engineering validation (PE-*)
    #[serde(default = "default_true")]
    pub prompt_engineering: bool,

    /// Detect generic instructions in CLAUDE.md
    #[serde(default = "default_true")]
    pub generic_instructions: bool,

    /// Validate YAML frontmatter
    #[serde(default = "default_true")]
    pub frontmatter_validation: bool,

    /// Check XML tag balance (legacy - use xml instead)
    #[serde(default = "default_true")]
    pub xml_balance: bool,

    /// Validate @import references (legacy - use imports instead)
    #[serde(default = "default_true")]
    pub import_references: bool,

    /// Explicitly disabled rules by ID (e.g., ["CC-AG-001", "AS-005"])
    #[serde(default)]
    pub disabled_rules: Vec<String>,
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
            prompt_engineering: true,
            generic_instructions: true,
            frontmatter_validation: true,
            xml_balance: true,
            import_references: true,
            disabled_rules: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
                    let warning = format!(
                        "Failed to parse config '{}': {}. Using defaults.",
                        p.display(),
                        e
                    );
                    (Self::default(), Some(warning))
                }
            },
            None => (Self::default(), None),
        }
    }

    /// Set the runtime validation root directory (not persisted)
    pub fn set_root_dir(&mut self, root_dir: PathBuf) {
        self.root_dir = Some(root_dir);
    }

    /// Set the shared import cache for project-level validation (not persisted).
    ///
    /// When set, the ImportsValidator will use this cache to share parsed
    /// import data across files, improving performance by avoiding redundant
    /// parsing during import chain traversal.
    pub fn set_import_cache(&mut self, cache: crate::parsers::ImportCache) {
        self.import_cache = Some(cache);
    }

    /// Get the shared import cache, if one has been set.
    ///
    /// Returns `None` for single-file validation or when the cache hasn't
    /// been initialized. Returns `Some(&ImportCache)` during project-level
    /// validation where import results are shared across files.
    pub fn import_cache(&self) -> Option<&crate::parsers::ImportCache> {
        self.import_cache.as_ref()
    }

    /// Get the file system abstraction.
    ///
    /// Validators should use this for file system operations instead of
    /// directly calling `std::fs` functions. This enables unit testing
    /// with `MockFileSystem`.
    pub fn fs(&self) -> &Arc<dyn FileSystem> {
        &self.fs
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
        self.fs = fs;
    }

    /// Get the expected MCP protocol version
    ///
    /// Priority: spec_revisions.mcp_protocol > mcp_protocol_version > default
    pub fn get_mcp_protocol_version(&self) -> &str {
        self.spec_revisions
            .mcp_protocol
            .as_deref()
            .or(self.mcp_protocol_version.as_deref())
            .unwrap_or(DEFAULT_MCP_PROTOCOL_VERSION)
    }

    /// Check if MCP protocol revision is explicitly pinned
    pub fn is_mcp_revision_pinned(&self) -> bool {
        self.spec_revisions.mcp_protocol.is_some() || self.mcp_protocol_version.is_some()
    }

    /// Check if Claude Code version is explicitly pinned
    pub fn is_claude_code_version_pinned(&self) -> bool {
        self.tool_versions.claude_code.is_some()
    }

    /// Get the pinned Claude Code version, if any
    pub fn get_claude_code_version(&self) -> Option<&str> {
        self.tool_versions.claude_code.as_deref()
    }

    /// Check if a specific rule is enabled based on config
    ///
    /// A rule is enabled if:
    /// 1. It's not in the disabled_rules list
    /// 2. It's applicable to the current target tool
    /// 3. Its category is enabled
    pub fn is_rule_enabled(&self, rule_id: &str) -> bool {
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
        // All other rules (AS-*, XML-*, REF-*) apply to all targets
        true
    }

    /// Check if a rule applies based on the tools array
    fn is_rule_for_tools(&self, rule_id: &str) -> bool {
        // Use TOOL_RULE_PREFIXES derived from rules.json at compile time.
        // Note: TOOL_RULE_PREFIXES is small (~6 entries), so linear search is acceptable.
        // If this grows significantly larger, consider using a HashMap for O(1) lookups.
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
            s if s.starts_with("AS-") || s.starts_with("CC-SK-") => self.rules.skills,
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
            s if s.starts_with("PE-") => self.rules.prompt_engineering,
            // Unknown rules are enabled by default
            _ => true,
        }
    }
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
        assert!(config
            .rules
            .disabled_rules
            .contains(&"CC-AG-002".to_string()));

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
}
