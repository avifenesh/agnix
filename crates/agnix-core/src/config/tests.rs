#![allow(clippy::field_reassign_with_default)]

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
    assert_eq!(config.get_mcp_protocol_version(), "2025-11-25");
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
    assert_eq!(config.get_mcp_protocol_version(), "2025-11-25");
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
    assert_eq!(config.get_mcp_protocol_version(), "2025-11-25");
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
mcp_protocol = "2025-11-25"
"#;

    let config: LintConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.get_mcp_protocol_version(), "2025-11-25");
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
    assert_eq!(config.get_mcp_protocol_version(), "2025-11-25");
}

#[test]
fn test_is_mcp_revision_pinned_with_none_mcp_protocol_version() {
    // When both spec_revisions.mcp_protocol and mcp_protocol_version are None
    let mut config = LintConfig::default();
    config.mcp_protocol_version = None;
    config.spec_revisions.mcp_protocol = None;

    assert!(!config.is_mcp_revision_pinned());
    // Should still return default
    assert_eq!(config.get_mcp_protocol_version(), "2025-11-25");
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
fn test_tools_array_amp_tool_enables_amp_rules() {
    let mut config = LintConfig::default();
    config.tools = vec!["amp".to_string()];

    assert!(config.is_rule_enabled("AMP-001"));
    assert!(!config.is_rule_enabled("CUR-001"));
    assert!(config.is_rule_enabled("AS-001"));
}

#[test]
fn test_tools_array_amp_respects_disabled_rules() {
    let mut config = LintConfig::default();
    config.tools = vec!["amp".to_string()];
    config.rules.disabled_rules = vec!["AMP-001".to_string()];

    assert!(!config.is_rule_enabled("AMP-001"));
    assert!(config.is_rule_enabled("AMP-002"));
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
    assert!(!config.is_rule_enabled("AMP-001"));

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
    config.spec_revisions.mcp_protocol = Some("2025-11-25".to_string());
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
        Some("2025-11-25".to_string())
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
        "AMP-001".to_string(),
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
        "amp".to_string(),
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
