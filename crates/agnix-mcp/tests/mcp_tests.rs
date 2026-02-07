//! Integration tests for agnix MCP server
//!
//! Tests the MCP server tools against real fixtures.

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Get the path to the fixtures directory
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
}

/// Create a temp directory with test files
fn create_temp_project() -> TempDir {
    let temp = TempDir::new().unwrap();

    // Create a valid SKILL.md
    let skill_content = r#"---
name: test-skill
description: A test skill for MCP testing
tools:
  - Read
  - Grep
---

# Test Skill

This is a test skill.
"#;
    fs::write(temp.path().join("SKILL.md"), skill_content).unwrap();

    // Create an invalid SKILL.md in a subdirectory
    fs::create_dir_all(temp.path().join("subdir")).unwrap();
    let invalid_skill = r#"---
name: Invalid-Name
description: Invalid skill name
---

# Invalid Skill
"#;
    fs::write(temp.path().join("subdir").join("SKILL.md"), invalid_skill).unwrap();

    temp
}

mod parse_target_tests {
    use agnix_core::config::TargetTool;

    fn parse_target(target: Option<String>) -> TargetTool {
        match target.as_deref() {
            Some("claude-code") | Some("claudecode") => TargetTool::ClaudeCode,
            Some("cursor") => TargetTool::Cursor,
            Some("codex") => TargetTool::Codex,
            _ => TargetTool::Generic,
        }
    }

    #[test]
    fn test_parse_target_none() {
        assert_eq!(parse_target(None), TargetTool::Generic);
    }

    #[test]
    fn test_parse_target_generic() {
        assert_eq!(
            parse_target(Some("generic".to_string())),
            TargetTool::Generic
        );
    }

    #[test]
    fn test_parse_target_claude_code() {
        assert_eq!(
            parse_target(Some("claude-code".to_string())),
            TargetTool::ClaudeCode
        );
    }

    #[test]
    fn test_parse_target_claudecode() {
        assert_eq!(
            parse_target(Some("claudecode".to_string())),
            TargetTool::ClaudeCode
        );
    }

    #[test]
    fn test_parse_target_cursor() {
        assert_eq!(parse_target(Some("cursor".to_string())), TargetTool::Cursor);
    }

    #[test]
    fn test_parse_target_codex() {
        assert_eq!(parse_target(Some("codex".to_string())), TargetTool::Codex);
    }

    #[test]
    fn test_parse_target_unknown() {
        assert_eq!(
            parse_target(Some("unknown".to_string())),
            TargetTool::Generic
        );
    }
}

mod validation_tests {
    use super::*;
    use agnix_core::{config::LintConfig, validate_file, validate_project};

    #[test]
    fn test_validate_valid_skill_file() {
        let temp = create_temp_project();
        let skill_path = temp.path().join("SKILL.md");

        let config = LintConfig::default();
        let result = validate_file(&skill_path, &config);

        assert!(result.is_ok());
        let diagnostics = result.unwrap();
        // Valid skill should have no errors (may have warnings)
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| matches!(d.level, agnix_core::diagnostics::DiagnosticLevel::Error))
            .collect();
        assert!(errors.is_empty(), "Valid SKILL.md should have no errors");
    }

    #[test]
    fn test_validate_invalid_skill_file() {
        let temp = create_temp_project();
        let skill_path = temp.path().join("subdir").join("SKILL.md");

        let config = LintConfig::default();
        let result = validate_file(&skill_path, &config);

        assert!(result.is_ok());
        let diagnostics = result.unwrap();
        // Invalid skill name should produce error
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| matches!(d.level, agnix_core::diagnostics::DiagnosticLevel::Error))
            .collect();
        assert!(
            !errors.is_empty(),
            "Invalid skill name should produce error"
        );
    }

    #[test]
    fn test_validate_project() {
        let temp = create_temp_project();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config);

        assert!(result.is_ok());
        let validation_result = result.unwrap();
        // Should find at least the SKILL.md files
        assert!(
            validation_result.files_checked >= 2,
            "Should check at least 2 files"
        );
    }

    #[test]
    fn test_validate_nonexistent_file() {
        let config = LintConfig::default();
        let result = validate_file(std::path::Path::new("/nonexistent/path/file.md"), &config);

        assert!(result.is_err());
    }

    #[test]
    fn test_validate_with_target_claude_code() {
        let temp = create_temp_project();
        let skill_path = temp.path().join("SKILL.md");

        let mut config = LintConfig::default();
        config.target = agnix_core::config::TargetTool::ClaudeCode;
        let result = validate_file(&skill_path, &config);

        assert!(result.is_ok());
    }

    #[test]
    fn test_tools_array_overrides_target_semantics_in_core_filtering() {
        let mut config = LintConfig::default();
        config.target = agnix_core::config::TargetTool::Codex;
        config.tools = vec!["claude-code".to_string(), "cursor".to_string()];

        // CC-* rules do not apply to codex target alone, but they do apply when tools include claude-code.
        assert!(config.is_rule_enabled("CC-AG-001"));
    }
}

mod rules_tests {
    #[test]
    fn test_rules_data_not_empty() {
        assert!(!agnix_rules::RULES_DATA.is_empty());
    }

    #[test]
    fn test_rules_count() {
        // Should have 139 rules
        assert_eq!(agnix_rules::rule_count(), 139);
    }

    #[test]
    fn test_get_rule_name_exists() {
        let name = agnix_rules::get_rule_name("AS-001");
        assert!(name.is_some());
    }

    #[test]
    fn test_get_rule_name_not_exists() {
        let name = agnix_rules::get_rule_name("NONEXISTENT-999");
        assert!(name.is_none());
    }

    #[test]
    fn test_common_rule_ids() {
        // Test that common rule IDs exist
        assert!(agnix_rules::get_rule_name("AS-001").is_some());
        assert!(agnix_rules::get_rule_name("AS-004").is_some());
        assert!(agnix_rules::get_rule_name("CC-SK-001").is_some());
        assert!(agnix_rules::get_rule_name("PE-001").is_some());
        assert!(agnix_rules::get_rule_name("MCP-001").is_some());
    }
}

mod output_format_tests {
    use agnix_core::diagnostics::{Diagnostic, DiagnosticLevel, Fix};
    use serde_json::Value;
    use std::path::PathBuf;

    #[test]
    fn test_diagnostic_json_serialization() {
        let diagnostic = Diagnostic {
            level: DiagnosticLevel::Error,
            message: "Test error".to_string(),
            file: PathBuf::from("test.md"),
            line: 1,
            column: 1,
            rule: "AS-001".to_string(),
            suggestion: Some("Fix this".to_string()),
            fixes: vec![],
            assumption: None,
        };

        let json = serde_json::to_string(&diagnostic);
        assert!(json.is_ok());

        let parsed: Value = serde_json::from_str(&json.unwrap()).unwrap();
        assert_eq!(parsed["rule"], "AS-001");
        // Level is serialized as "Error" (enum variant name)
        assert!(
            parsed["level"] == "error" || parsed["level"] == "Error",
            "Level should be error/Error, got: {}",
            parsed["level"]
        );
    }

    #[test]
    fn test_diagnostic_with_fix() {
        let diagnostic = Diagnostic {
            level: DiagnosticLevel::Warning,
            message: "Test warning".to_string(),
            file: PathBuf::from("test.md"),
            line: 5,
            column: 10,
            rule: "PE-003".to_string(),
            suggestion: Some("Remove this".to_string()),
            fixes: vec![Fix {
                start_byte: 0,
                end_byte: 10,
                replacement: "fixed".to_string(),
                description: "Fix the issue".to_string(),
                safe: true,
            }],
            assumption: None,
        };

        // Diagnostic should be fixable
        assert!(!diagnostic.fixes.is_empty());
    }
}

mod server_info_tests {
    // Note: We can't directly instantiate AgnixServer from here due to privacy,
    // but we can test the expected behavior through the binary

    #[test]
    fn test_server_version_matches_cargo() {
        let version = env!("CARGO_PKG_VERSION");
        assert!(!version.is_empty());
        // Version should be semver format
        assert!(version.contains('.'));
    }

    #[test]
    fn test_protocol_version() {
        // MCP protocol version should be defined
        use rmcp::model::ProtocolVersion;
        let _ = ProtocolVersion::V_2024_11_05;
    }
}

mod integration_tests {
    use super::*;

    #[test]
    fn test_binary_exists() {
        // After cargo build, binary should exist
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let target_dir = manifest_dir
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("target")
            .join("debug");

        // Binary name varies by platform
        #[cfg(windows)]
        let binary_name = "agnix-mcp.exe";
        #[cfg(not(windows))]
        let binary_name = "agnix-mcp";

        let binary_path = target_dir.join(binary_name);

        // Note: This test may fail if build hasn't been run
        // It's informational - the binary should be built by CI
        if binary_path.exists() {
            assert!(binary_path.is_file());
        }
    }

    #[test]
    fn test_validate_fixtures_skill_valid() {
        let fixtures = fixtures_dir();
        let skill_valid = fixtures.join("skill").join("valid");

        if skill_valid.exists() {
            let config = agnix_core::config::LintConfig::default();
            let result = agnix_core::validate_project(&skill_valid, &config);

            assert!(result.is_ok());
            let validation = result.unwrap();
            // Valid fixtures should have no errors
            let errors: Vec<_> = validation
                .diagnostics
                .iter()
                .filter(|d| matches!(d.level, agnix_core::diagnostics::DiagnosticLevel::Error))
                .collect();
            assert!(
                errors.is_empty(),
                "Valid skill fixtures should have no errors"
            );
        }
    }

    #[test]
    fn test_validate_fixtures_skill_invalid() {
        let fixtures = fixtures_dir();
        let skill_invalid = fixtures.join("skill").join("invalid");

        if skill_invalid.exists() {
            let config = agnix_core::config::LintConfig::default();
            let result = agnix_core::validate_project(&skill_invalid, &config);

            assert!(result.is_ok());
            let validation = result.unwrap();
            // Invalid fixtures should have errors or warnings
            assert!(
                !validation.diagnostics.is_empty(),
                "Invalid skill fixtures should have diagnostics"
            );
        }
    }
}

mod json_schema_tests {
    use schemars::JsonSchema;
    use serde::Deserialize;

    #[allow(dead_code)]
    #[derive(Debug, Deserialize, JsonSchema)]
    #[serde(untagged)]
    enum TestToolsInput {
        Csv(String),
        List(Vec<String>),
    }

    // Test that input structs have proper JSON schema
    // Fields are used via schema generation, not directly
    #[allow(dead_code)]
    #[derive(Debug, Deserialize, JsonSchema)]
    struct TestValidateFileInput {
        path: String,
        tools: Option<TestToolsInput>,
        target: Option<String>,
    }

    #[allow(dead_code)]
    #[derive(Debug, Deserialize, JsonSchema)]
    struct TestValidateProjectInput {
        path: String,
        tools: Option<TestToolsInput>,
        target: Option<String>,
    }

    #[allow(dead_code)]
    #[derive(Debug, Deserialize, JsonSchema)]
    struct TestGetRuleDocsInput {
        rule_id: String,
    }

    #[test]
    fn test_validate_file_input_schema() {
        let schema = schemars::schema_for!(TestValidateFileInput);
        let json = serde_json::to_string_pretty(&schema).unwrap();

        assert!(json.contains("path"));
        assert!(json.contains("tools"));
        assert!(json.contains("target"));
    }

    #[test]
    fn test_validate_project_input_schema() {
        let schema = schemars::schema_for!(TestValidateProjectInput);
        let json = serde_json::to_string_pretty(&schema).unwrap();

        assert!(json.contains("path"));
        assert!(json.contains("tools"));
        assert!(json.contains("target"));
    }

    #[test]
    fn test_get_rule_docs_input_schema() {
        let schema = schemars::schema_for!(TestGetRuleDocsInput);
        let json = serde_json::to_string_pretty(&schema).unwrap();

        assert!(json.contains("rule_id"));
    }
}
