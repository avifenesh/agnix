//! Linter configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

    /// Validate tool names
    #[serde(default = "default_true")]
    pub tool_names: bool,

    /// Check required fields
    #[serde(default = "default_true")]
    pub required_fields: bool,

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
            generic_instructions: true,
            frontmatter_validation: true,
            xml_balance: true,
            import_references: true,
            tool_names: true,
            required_fields: true,
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
    pub fn load(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load config or use default
    pub fn load_or_default(path: Option<&PathBuf>) -> Self {
        path.and_then(|p| Self::load(p).ok()).unwrap_or_default()
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
            s if s.starts_with("XML-") || s.starts_with("xml::") => self.rules.xml,
            s if s.starts_with("MCP-") => self.rules.mcp,
            s if s.starts_with("REF-") || s.starts_with("imports::") => self.rules.imports,
            s if s.starts_with("XP-") => self.rules.cross_platform,
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
        assert!(!config.is_rule_enabled("xml::balance"));

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
}
