//! VS Code configuration types for LSP integration.
//!
//! These types mirror the VS Code settings schema defined in package.json,
//! allowing the LSP server to receive and apply configuration updates from
//! the VS Code extension without requiring a server restart.
//!
//! # Design Notes
//!
//! - All fields use `Option<T>` to support partial updates (only override
//!   non-None values)
//! - Uses `#[serde(rename_all = "snake_case")]` to match Rust convention
//!   while accepting the snake_case JSON from the extension's buildLspConfig()
//! - The `merge_into_lint_config` method applies VS Code settings on top of
//!   existing config (from .agnix.toml), giving VS Code settings priority

use agnix_core::LintConfig;
use agnix_core::config::{RuleConfig, SeverityLevel, SpecRevisions, TargetTool, ToolVersions};
use serde::{Deserialize, Serialize};

/// VS Code configuration received from workspace/didChangeConfiguration.
///
/// This structure matches the LspConfig interface in extension.ts.
/// All fields are optional to support partial configuration updates.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct VsCodeConfig {
    /// Minimum severity level for diagnostics
    #[serde(default)]
    pub severity: Option<String>,

    /// Target tool for validation (deprecated)
    #[serde(default)]
    pub target: Option<String>,

    /// Tools to validate for
    #[serde(default)]
    pub tools: Option<Vec<String>>,

    /// Rule category toggles
    #[serde(default)]
    pub rules: Option<VsCodeRules>,

    /// Tool version pins
    #[serde(default)]
    pub versions: Option<VsCodeVersions>,

    /// Spec revision pins
    #[serde(default)]
    pub specs: Option<VsCodeSpecs>,

    /// Output locale for translated messages (e.g., "en", "es", "zh-CN")
    /// Uses Option<Option<String>> to distinguish:
    /// - None = field not in JSON (preserve existing locale)
    /// - Some(None) = field in JSON as null (revert to auto-detection)
    /// - Some(Some(v)) = field in JSON with value (set locale to v)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<Option<String>>,
}

/// Rule category toggles from VS Code settings.
///
/// Maps to RuleConfig in agnix-core.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct VsCodeRules {
    /// Enable skills validation (AS-*, CC-SK-*)
    #[serde(default)]
    pub skills: Option<bool>,

    /// Enable hooks validation (CC-HK-*)
    #[serde(default)]
    pub hooks: Option<bool>,

    /// Enable agents validation (CC-AG-*)
    #[serde(default)]
    pub agents: Option<bool>,

    /// Enable memory validation (CC-MEM-*)
    #[serde(default)]
    pub memory: Option<bool>,

    /// Enable plugins validation (CC-PL-*)
    #[serde(default)]
    pub plugins: Option<bool>,

    /// Enable XML balance checking (XML-*)
    #[serde(default)]
    pub xml: Option<bool>,

    /// Enable MCP validation (MCP-*)
    #[serde(default)]
    pub mcp: Option<bool>,

    /// Enable import reference validation (REF-*)
    #[serde(default)]
    pub imports: Option<bool>,

    /// Enable cross-platform validation (XP-*)
    #[serde(default)]
    pub cross_platform: Option<bool>,

    /// Enable AGENTS.md validation (AGM-*)
    #[serde(default)]
    pub agents_md: Option<bool>,

    /// Enable GitHub Copilot validation (COP-*)
    #[serde(default)]
    pub copilot: Option<bool>,

    /// Enable Cursor project rules validation (CUR-*)
    #[serde(default)]
    pub cursor: Option<bool>,

    /// Enable prompt engineering validation (PE-*)
    #[serde(default)]
    pub prompt_engineering: Option<bool>,

    /// Explicitly disabled rules by ID
    #[serde(default)]
    pub disabled_rules: Option<Vec<String>>,
}

/// Tool version pins from VS Code settings.
///
/// Maps to ToolVersions in agnix-core.
/// Uses Option<Option<String>> to distinguish:
/// - None = field not in JSON (preserve .agnix.toml value)
/// - Some(None) = field in JSON as null (clear pin)
/// - Some(Some(v)) = field in JSON with value (set pin to v)
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct VsCodeVersions {
    /// Claude Code version (e.g., "1.0.0")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_code: Option<Option<String>>,

    /// Codex CLI version (e.g., "0.1.0")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex: Option<Option<String>>,

    /// Cursor version (e.g., "0.45.0")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Option<String>>,

    /// GitHub Copilot version (e.g., "1.0.0")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copilot: Option<Option<String>>,
}

/// Spec revision pins from VS Code settings.
///
/// Maps to SpecRevisions in agnix-core.
/// Uses Option<Option<String>> to distinguish:
/// - None = field not in JSON (preserve .agnix.toml value)
/// - Some(None) = field in JSON as null (clear pin)
/// - Some(Some(v)) = field in JSON with value (set pin to v)
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct VsCodeSpecs {
    /// MCP protocol version (e.g., "2025-06-18")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_protocol: Option<Option<String>>,

    /// Agent Skills specification revision
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_skills_spec: Option<Option<String>>,

    /// AGENTS.md specification revision
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_md_spec: Option<Option<String>>,
}

impl VsCodeConfig {
    /// Merge VS Code settings into a LintConfig.
    ///
    /// Only non-None values are applied, preserving any existing config
    /// (e.g., from .agnix.toml). This allows VS Code settings to override
    /// file-based config while keeping unspecified options unchanged.
    ///
    /// # Priority
    ///
    /// VS Code settings take priority over .agnix.toml values.
    pub fn merge_into_lint_config(&self, config: &mut LintConfig) {
        // Merge severity
        if let Some(ref severity) = self.severity {
            if let Some(level) = parse_severity(severity) {
                config.severity = level;
            }
        }

        // Merge target
        if let Some(ref target) = self.target {
            if let Some(tool) = parse_target(target) {
                config.target = tool;
            }
        }

        // Merge tools
        if let Some(ref tools) = self.tools {
            config.tools = tools.clone();
        }

        // Merge rules
        if let Some(ref rules) = self.rules {
            rules.merge_into_rule_config(&mut config.rules);
        }

        // Merge tool versions
        if let Some(ref versions) = self.versions {
            versions.merge_into_tool_versions(&mut config.tool_versions);
        }

        // Merge spec revisions
        if let Some(ref specs) = self.specs {
            specs.merge_into_spec_revisions(&mut config.spec_revisions);
        }

        // Merge locale
        // None = not in JSON (preserve existing)
        // Some(None) = JSON null (clear locale, revert to auto-detection)
        // Some(Some(v)) = JSON value (set locale)
        if let Some(ref locale_opt) = self.locale {
            match locale_opt {
                Some(locale) => {
                    config.locale = Some(locale.clone());
                    crate::locale::init_from_config(locale);
                }
                None => {
                    config.locale = None;
                    crate::locale::init_from_env();
                }
            }
        }
    }
}

impl VsCodeRules {
    /// Merge VS Code rule settings into RuleConfig.
    fn merge_into_rule_config(&self, config: &mut RuleConfig) {
        if let Some(v) = self.skills {
            config.skills = v;
        }
        if let Some(v) = self.hooks {
            config.hooks = v;
        }
        if let Some(v) = self.agents {
            config.agents = v;
        }
        if let Some(v) = self.memory {
            config.memory = v;
        }
        if let Some(v) = self.plugins {
            config.plugins = v;
        }
        if let Some(v) = self.xml {
            config.xml = v;
        }
        if let Some(v) = self.mcp {
            config.mcp = v;
        }
        if let Some(v) = self.imports {
            config.imports = v;
        }
        if let Some(v) = self.cross_platform {
            config.cross_platform = v;
        }
        if let Some(v) = self.agents_md {
            config.agents_md = v;
        }
        if let Some(v) = self.copilot {
            config.copilot = v;
        }
        if let Some(v) = self.cursor {
            config.cursor = v;
        }
        if let Some(v) = self.prompt_engineering {
            config.prompt_engineering = v;
        }
        if let Some(ref v) = self.disabled_rules {
            config.disabled_rules = v.clone();
        }
    }
}

impl VsCodeVersions {
    /// Merge VS Code version pins into ToolVersions.
    /// Uses Option<Option<String>> pattern:
    /// - None = not in JSON (skip, preserve .agnix.toml)
    /// - Some(None) = in JSON as null (apply None, clear pin)
    /// - Some(Some(v)) = in JSON with value (apply value)
    fn merge_into_tool_versions(&self, config: &mut ToolVersions) {
        if let Some(ref value) = self.claude_code {
            config.claude_code = value.clone();
        }
        if let Some(ref value) = self.codex {
            config.codex = value.clone();
        }
        if let Some(ref value) = self.cursor {
            config.cursor = value.clone();
        }
        if let Some(ref value) = self.copilot {
            config.copilot = value.clone();
        }
    }
}

impl VsCodeSpecs {
    /// Merge VS Code spec pins into SpecRevisions.
    /// Uses Option<Option<String>> pattern:
    /// - None = not in JSON (skip, preserve .agnix.toml)
    /// - Some(None) = in JSON as null (apply None, clear pin)
    /// - Some(Some(v)) = in JSON with value (apply value)
    fn merge_into_spec_revisions(&self, config: &mut SpecRevisions) {
        if let Some(ref value) = self.mcp_protocol {
            config.mcp_protocol = value.clone();
        }
        if let Some(ref value) = self.agent_skills_spec {
            config.agent_skills_spec = value.clone();
        }
        if let Some(ref value) = self.agents_md_spec {
            config.agents_md_spec = value.clone();
        }
    }
}

/// Parse severity level from string.
fn parse_severity(s: &str) -> Option<SeverityLevel> {
    match s {
        "Error" => Some(SeverityLevel::Error),
        "Warning" => Some(SeverityLevel::Warning),
        "Info" => Some(SeverityLevel::Info),
        _ => None,
    }
}

/// Parse target tool from string.
fn parse_target(s: &str) -> Option<TargetTool> {
    match s {
        "Generic" => Some(TargetTool::Generic),
        "ClaudeCode" => Some(TargetTool::ClaudeCode),
        "Cursor" => Some(TargetTool::Cursor),
        "Codex" => Some(TargetTool::Codex),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vscode_config_deserialization_complete() {
        let json = r#"{
            "severity": "Error",
            "target": "ClaudeCode",
            "tools": ["claude-code", "cursor"],
            "locale": "es",
            "rules": {
                "skills": false,
                "hooks": true,
                "agents": false,
                "memory": true,
                "plugins": false,
                "xml": true,
                "mcp": false,
                "imports": true,
                "cross_platform": false,
                "agents_md": true,
                "copilot": false,
                "cursor": true,
                "prompt_engineering": false,
                "disabled_rules": ["AS-001", "PE-003"]
            },
            "versions": {
                "claude_code": "1.0.0",
                "codex": "0.1.0",
                "cursor": "0.45.0",
                "copilot": "1.2.0"
            },
            "specs": {
                "mcp_protocol": "2025-06-18",
                "agent_skills_spec": "1.0",
                "agents_md_spec": "1.0"
            }
        }"#;

        let config: VsCodeConfig = serde_json::from_str(json).expect("should parse");

        assert_eq!(config.severity, Some("Error".to_string()));
        assert_eq!(config.target, Some("ClaudeCode".to_string()));
        assert_eq!(
            config.tools,
            Some(vec!["claude-code".to_string(), "cursor".to_string()])
        );
        assert_eq!(config.locale, Some(Some("es".to_string())));

        let rules = config.rules.expect("rules should be present");
        assert_eq!(rules.skills, Some(false));
        assert_eq!(rules.hooks, Some(true));
        assert_eq!(
            rules.disabled_rules,
            Some(vec!["AS-001".to_string(), "PE-003".to_string()])
        );

        let versions = config.versions.expect("versions should be present");
        assert_eq!(versions.claude_code, Some(Some("1.0.0".to_string())));

        let specs = config.specs.expect("specs should be present");
        assert_eq!(specs.mcp_protocol, Some(Some("2025-06-18".to_string())));
    }

    #[test]
    fn test_vscode_config_deserialization_partial() {
        // Only severity and one rule specified
        let json = r#"{
            "severity": "Warning",
            "rules": {
                "skills": false
            }
        }"#;

        let config: VsCodeConfig = serde_json::from_str(json).expect("should parse");

        assert_eq!(config.severity, Some("Warning".to_string()));
        assert!(config.target.is_none());
        assert!(config.tools.is_none());

        let rules = config.rules.expect("rules should be present");
        assert_eq!(rules.skills, Some(false));
        assert!(rules.hooks.is_none()); // Not specified
    }

    #[test]
    fn test_vscode_config_deserialization_empty() {
        let json = "{}";

        let config: VsCodeConfig = serde_json::from_str(json).expect("should parse");

        assert!(config.severity.is_none());
        assert!(config.target.is_none());
        assert!(config.tools.is_none());
        assert!(config.rules.is_none());
        assert!(config.versions.is_none());
        assert!(config.specs.is_none());
        assert!(config.locale.is_none()); // Option<Option<String>>: outer None = not present
    }

    #[test]
    fn test_merge_into_lint_config_preserves_unspecified() {
        let mut lint_config = LintConfig::default();
        lint_config.severity = SeverityLevel::Error;
        lint_config.rules.skills = false;

        // VS Code config only specifies hooks
        let vscode_config = VsCodeConfig {
            rules: Some(VsCodeRules {
                hooks: Some(false),
                ..Default::default()
            }),
            ..Default::default()
        };

        vscode_config.merge_into_lint_config(&mut lint_config);

        // Original values preserved
        assert_eq!(lint_config.severity, SeverityLevel::Error);
        assert!(!lint_config.rules.skills);

        // New value applied
        assert!(!lint_config.rules.hooks);
    }

    #[test]
    fn test_merge_into_lint_config_overrides() {
        let mut lint_config = LintConfig::default();
        lint_config.severity = SeverityLevel::Warning;
        lint_config.target = TargetTool::Generic;
        lint_config.rules.skills = true;

        // VS Code config overrides everything
        let vscode_config = VsCodeConfig {
            severity: Some("Error".to_string()),
            target: Some("ClaudeCode".to_string()),
            rules: Some(VsCodeRules {
                skills: Some(false),
                ..Default::default()
            }),
            ..Default::default()
        };

        vscode_config.merge_into_lint_config(&mut lint_config);

        // All values overridden
        assert_eq!(lint_config.severity, SeverityLevel::Error);
        assert_eq!(lint_config.target, TargetTool::ClaudeCode);
        assert!(!lint_config.rules.skills);
    }

    #[test]
    fn test_merge_versions() {
        let mut lint_config = LintConfig::default();
        lint_config.tool_versions.claude_code = Some("0.9.0".to_string());

        let vscode_config = VsCodeConfig {
            versions: Some(VsCodeVersions {
                claude_code: Some(Some("1.0.0".to_string())),
                codex: Some(Some("0.1.0".to_string())),
                ..Default::default()
            }),
            ..Default::default()
        };

        vscode_config.merge_into_lint_config(&mut lint_config);

        assert_eq!(
            lint_config.tool_versions.claude_code,
            Some("1.0.0".to_string())
        );
        assert_eq!(lint_config.tool_versions.codex, Some("0.1.0".to_string()));
        assert!(lint_config.tool_versions.cursor.is_none()); // Not specified
    }

    #[test]
    fn test_merge_specs() {
        let mut lint_config = LintConfig::default();

        let vscode_config = VsCodeConfig {
            specs: Some(VsCodeSpecs {
                mcp_protocol: Some(Some("2025-06-18".to_string())),
                ..Default::default()
            }),
            ..Default::default()
        };

        vscode_config.merge_into_lint_config(&mut lint_config);

        assert_eq!(
            lint_config.spec_revisions.mcp_protocol,
            Some("2025-06-18".to_string())
        );
        assert!(lint_config.spec_revisions.agent_skills_spec.is_none());
    }

    #[test]
    fn test_parse_severity() {
        assert_eq!(parse_severity("Error"), Some(SeverityLevel::Error));
        assert_eq!(parse_severity("Warning"), Some(SeverityLevel::Warning));
        assert_eq!(parse_severity("Info"), Some(SeverityLevel::Info));
        assert_eq!(parse_severity("invalid"), None);
    }

    #[test]
    fn test_parse_target() {
        assert_eq!(parse_target("Generic"), Some(TargetTool::Generic));
        assert_eq!(parse_target("ClaudeCode"), Some(TargetTool::ClaudeCode));
        assert_eq!(parse_target("Cursor"), Some(TargetTool::Cursor));
        assert_eq!(parse_target("Codex"), Some(TargetTool::Codex));
        assert_eq!(parse_target("invalid"), None);
    }

    #[test]
    fn test_disabled_rules_merge() {
        let mut lint_config = LintConfig::default();
        lint_config.rules.disabled_rules = vec!["AS-001".to_string()];

        let vscode_config = VsCodeConfig {
            rules: Some(VsCodeRules {
                disabled_rules: Some(vec!["PE-003".to_string(), "MCP-001".to_string()]),
                ..Default::default()
            }),
            ..Default::default()
        };

        vscode_config.merge_into_lint_config(&mut lint_config);

        // VS Code config replaces (not appends) disabled_rules
        assert_eq!(
            lint_config.rules.disabled_rules,
            vec!["PE-003".to_string(), "MCP-001".to_string()]
        );
    }

    #[test]
    fn test_tools_array_merge() {
        let mut lint_config = LintConfig::default();
        lint_config.tools = vec!["generic".to_string()];

        let vscode_config = VsCodeConfig {
            tools: Some(vec!["claude-code".to_string(), "cursor".to_string()]),
            ..Default::default()
        };

        vscode_config.merge_into_lint_config(&mut lint_config);

        assert_eq!(
            lint_config.tools,
            vec!["claude-code".to_string(), "cursor".to_string()]
        );
    }

    #[test]
    fn test_locale_merge() {
        // Pin locale to "en" for test isolation
        rust_i18n::set_locale("en");

        let mut lint_config = LintConfig::default();
        assert!(lint_config.locale.is_none());

        let vscode_config = VsCodeConfig {
            locale: Some(Some("es".to_string())),
            ..Default::default()
        };

        vscode_config.merge_into_lint_config(&mut lint_config);

        assert_eq!(lint_config.locale, Some("es".to_string()));
        assert_eq!(&*rust_i18n::locale(), "es");

        // Reset locale for other tests
        rust_i18n::set_locale("en");
    }

    #[test]
    fn test_locale_null_reverts_to_auto_detect() {
        // Pin locale to "es" to simulate a previously set locale
        rust_i18n::set_locale("es");

        let mut lint_config = LintConfig::default();
        lint_config.locale = Some("es".to_string());

        // User sets locale to null in VS Code (revert to auto-detection)
        let vscode_config = VsCodeConfig {
            locale: Some(None),
            ..Default::default()
        };

        vscode_config.merge_into_lint_config(&mut lint_config);

        // Config locale should be cleared
        assert!(lint_config.locale.is_none());

        // Reset locale for other tests
        rust_i18n::set_locale("en");
    }

    #[test]
    fn test_locale_not_set_preserves_existing() {
        let mut lint_config = LintConfig::default();
        lint_config.locale = Some("zh-CN".to_string());

        let vscode_config = VsCodeConfig {
            severity: Some("Error".to_string()),
            ..Default::default()
        };

        vscode_config.merge_into_lint_config(&mut lint_config);

        // locale not in VsCodeConfig, so existing value preserved
        assert_eq!(lint_config.locale, Some("zh-CN".to_string()));
    }
}

#[test]
fn test_version_pin_clearing_with_null() {
    // Start with a config that has version pins from .agnix.toml
    let mut lint_config = LintConfig::default();
    lint_config.tool_versions.claude_code = Some("0.9.0".to_string());
    lint_config.tool_versions.codex = Some("0.5.0".to_string());

    // User explicitly sets claude_code to null in VS Code (clears pin)
    // but doesn't touch codex (preserves .agnix.toml value)
    let vscode_config = VsCodeConfig {
        versions: Some(VsCodeVersions {
            claude_code: Some(None), // Explicitly null - clear the pin
            codex: None,             // Not specified - preserve .agnix.toml
            ..Default::default()
        }),
        ..Default::default()
    };

    vscode_config.merge_into_lint_config(&mut lint_config);

    // claude_code should be cleared (None)
    assert!(lint_config.tool_versions.claude_code.is_none());
    // codex should still have the .agnix.toml value
    assert_eq!(lint_config.tool_versions.codex, Some("0.5.0".to_string()));
}

#[test]
fn test_spec_pin_clearing_with_null() {
    // Start with a config that has spec pins from .agnix.toml
    let mut lint_config = LintConfig::default();
    lint_config.spec_revisions.mcp_protocol = Some("2025-01-01".to_string());
    lint_config.spec_revisions.agent_skills_spec = Some("v1".to_string());

    // User explicitly sets mcp_protocol to null (clears pin)
    // but doesn't touch agent_skills_spec (preserves .agnix.toml)
    let vscode_config = VsCodeConfig {
        specs: Some(VsCodeSpecs {
            mcp_protocol: Some(None), // Explicitly null - clear the pin
            agent_skills_spec: None,  // Not specified - preserve .agnix.toml
            ..Default::default()
        }),
        ..Default::default()
    };

    vscode_config.merge_into_lint_config(&mut lint_config);

    // mcp_protocol should be cleared (None)
    assert!(lint_config.spec_revisions.mcp_protocol.is_none());
    // agent_skills_spec should still have the .agnix.toml value
    assert_eq!(
        lint_config.spec_revisions.agent_skills_spec,
        Some("v1".to_string())
    );
}
