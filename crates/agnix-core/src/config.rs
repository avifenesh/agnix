//! Linter configuration

use crate::file_utils::safe_read_file;
use crate::schemas::mcp::DEFAULT_MCP_PROTOCOL_VERSION;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Configuration for the linter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintConfig {
    /// Severity level threshold
    pub severity: SeverityLevel,

    /// Rules to enable/disable
    pub rules: RuleConfig,

    /// Paths to exclude
    pub exclude: Vec<String>,

    /// Target tool (claude-code, cursor, codex, generic)
    pub target: TargetTool,

    /// Expected MCP protocol version for validation (MCP-008)
    #[serde(default = "default_mcp_protocol_version")]
    pub mcp_protocol_version: Option<String>,

    /// Runtime-only validation root directory (not serialized)
    #[serde(skip)]
    pub root_dir: Option<PathBuf>,
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
            mcp_protocol_version: default_mcp_protocol_version(),
            root_dir: None,
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

/// Default MCP protocol version (latest stable per MCP spec)
fn default_mcp_protocol_version() -> Option<String> {
    Some(DEFAULT_MCP_PROTOCOL_VERSION.to_string())
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

    /// Get the expected MCP protocol version
    pub fn get_mcp_protocol_version(&self) -> &str {
        self.mcp_protocol_version
            .as_deref()
            .unwrap_or(DEFAULT_MCP_PROTOCOL_VERSION)
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

    /// Check if a rule applies to the current target tool
    fn is_rule_for_target(&self, rule_id: &str) -> bool {
        // CC-* rules only apply to ClaudeCode or Generic targets
        if rule_id.starts_with("CC-") {
            return matches!(self.target, TargetTool::ClaudeCode | TargetTool::Generic);
        }
        // All other rules (AS-*, XML-*, REF-*) apply to all targets
        true
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
}
