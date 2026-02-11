//! Cross-crate integration tests verifying contracts between workspace crates.
//!
//! These tests simulate how downstream CLI, LSP, and MCP binaries use
//! agnix-core and agnix-rules. For items classified as stable in the
//! backward-compatibility policy, these tests help ensure that the
//! corresponding interfaces remain stable across releases. Some sections
//! also exercise Public/Unstable contracts, whose interfaces may change on
//! minor releases; for those, the tests only assert current behavior rather
//! than long-term stability guarantees.

use std::path::Path;

// ============================================================================
// CLI <-> core contracts
// ============================================================================

#[test]
fn cli_lint_config_default_works() {
    let config = agnix_core::LintConfig::default();
    let dir = tempfile::tempdir().unwrap();
    let result = agnix_core::validate_project(dir.path(), &config);
    assert!(result.is_ok());
    let validation = result.unwrap();
    // An empty directory may produce project-level diagnostics (e.g., VER-001
    // for missing version pins). The contract we test here is that
    // validate_project returns a valid ValidationResult.
    let _files: usize = validation.files_checked;
    let _diags: &[agnix_core::Diagnostic] = &validation.diagnostics;

    // New metadata fields are populated by validate_project
    assert!(
        validation.validation_time_ms.is_some(),
        "validation_time_ms should be populated"
    );
    assert!(
        validation.validator_factories_registered > 0,
        "validator_factories_registered should be positive when using default registry"
    );
}

#[test]
fn cli_validation_result_fields_accessible() {
    let config = agnix_core::LintConfig::default();
    let dir = tempfile::tempdir().unwrap();
    let result = agnix_core::validate_project(dir.path(), &config).unwrap();

    // CLI reads these fields to build output
    let _count: usize = result.files_checked;
    let _diags: &[agnix_core::Diagnostic] = &result.diagnostics;

    // Metadata fields are accessible and populated
    assert!(result.validation_time_ms.is_some());
    assert!(result.validator_factories_registered > 0);
}

#[test]
fn cli_apply_fixes_roundtrip() {
    // CLI calls apply_fixes with empty diagnostics (no-op case)
    let diags: Vec<agnix_core::Diagnostic> = vec![];
    let results = agnix_core::apply_fixes(&diags, true, false).unwrap();
    assert!(results.is_empty());
}

#[test]
fn cli_generate_schema_returns_valid_json() {
    let schema = agnix_core::generate_schema();
    let json = serde_json::to_string_pretty(&schema).unwrap();
    assert!(json.contains("\"type\""));
    assert!(json.contains("\"properties\""));
}

#[test]
fn cli_target_tool_variants_accessible() {
    // CLI parses --target flag into TargetTool variants
    let _generic = agnix_core::config::TargetTool::Generic;
    let _claude = agnix_core::config::TargetTool::ClaudeCode;
    let _cursor = agnix_core::config::TargetTool::Cursor;
    let _codex = agnix_core::config::TargetTool::Codex;
}

// ============================================================================
// LSP <-> core contracts
// ============================================================================

#[test]
fn lsp_diagnostic_fix_field_accessibility() {
    use std::path::PathBuf;

    // LSP maps Diagnostic fields to LSP protocol types
    let diag =
        agnix_core::Diagnostic::error(PathBuf::from("test.md"), 1, 0, "AS-001", "test error")
            .with_suggestion("fix it")
            .with_fix(agnix_core::Fix::replace(0, 5, "fixed", "auto fix", true));

    // Fields that LSP reads for protocol mapping
    let _level: agnix_core::DiagnosticLevel = diag.level;
    let _message: &str = &diag.message;
    let _file: &Path = &diag.file;
    let _line: usize = diag.line;
    let _column: usize = diag.column;
    let _rule: &str = &diag.rule;
    let _suggestion: Option<&String> = diag.suggestion.as_ref();

    // Fix fields for code actions
    for fix in &diag.fixes {
        let _start: usize = fix.start_byte;
        let _end: usize = fix.end_byte;
        let _replacement: &str = &fix.replacement;
        let _description: &str = &fix.description;
        let _safe: bool = fix.safe;
    }
}

#[test]
fn lsp_diagnostic_level_variant_mapping() {
    // LSP maps DiagnosticLevel to LSP severity numbers via explicit matches;
    // this test only asserts that the variants remain accessible and matchable.
    let error = agnix_core::DiagnosticLevel::Error;
    let warning = agnix_core::DiagnosticLevel::Warning;
    let info = agnix_core::DiagnosticLevel::Info;

    fn describe(level: agnix_core::DiagnosticLevel) -> &'static str {
        match level {
            agnix_core::DiagnosticLevel::Error => "error",
            agnix_core::DiagnosticLevel::Warning => "warning",
            agnix_core::DiagnosticLevel::Info => "info",
        }
    }

    assert_eq!(describe(error), "error");
    assert_eq!(describe(warning), "warning");
    assert_eq!(describe(info), "info");
}
#[test]
fn lsp_validator_registry_is_send_sync() {
    // LSP wraps ValidatorRegistry in Arc for sharing across tasks
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<agnix_core::ValidatorRegistry>();
}

#[test]
fn lsp_lint_config_is_send_sync() {
    // LSP wraps LintConfig in RwLock<Arc<LintConfig>>
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<agnix_core::LintConfig>();
}

#[test]
fn lsp_file_type_detection_for_relevant_types() {
    // LSP uses detect_file_type to decide whether to validate a file
    let test_cases = [
        ("CLAUDE.md", agnix_core::FileType::ClaudeMd),
        ("AGENTS.md", agnix_core::FileType::ClaudeMd),
        (".claude/agents/reviewer.md", agnix_core::FileType::Agent),
        ("skills/deploy/SKILL.md", agnix_core::FileType::Skill),
        (".claude/settings.json", agnix_core::FileType::Hooks),
        (
            ".cursor/rules/my-rule.mdc",
            agnix_core::FileType::CursorRule,
        ),
        ("plugin.json", agnix_core::FileType::Plugin),
        ("tools.mcp.json", agnix_core::FileType::Mcp),
        (
            ".github/copilot-instructions.md",
            agnix_core::FileType::Copilot,
        ),
    ];

    for (path, expected_type) in test_cases {
        assert_eq!(
            agnix_core::detect_file_type(Path::new(path)),
            expected_type,
            "Failed for path: {}",
            path
        );
    }
}
#[test]
fn lsp_authoring_completion_candidates_accessible() {
    // LSP calls authoring::completion_candidates for completions
    let candidates =
        agnix_core::authoring::completion_candidates(agnix_core::FileType::Skill, "", 0);
    // Should return some candidates for skill files
    assert!(!candidates.is_empty());
}

#[test]
fn lsp_authoring_hover_doc_accessible() {
    // LSP calls authoring::hover_doc for hover information
    let doc = agnix_core::authoring::hover_doc(agnix_core::FileType::Skill, "name");
    // "name" is a known skill frontmatter key
    assert!(doc.is_some());
}

// ============================================================================
// MCP <-> core contracts
// ============================================================================

#[test]
fn mcp_validate_file_with_target_tool_config() {
    let mut config = agnix_core::LintConfig::default();
    config.set_tools(vec!["claude-code".to_string()]);

    let dir = tempfile::tempdir().unwrap();
    let result = agnix_core::validate_project(dir.path(), &config);
    assert!(result.is_ok());
}

#[test]
fn mcp_diagnostic_json_serialization() {
    use std::path::PathBuf;

    let diag = agnix_core::Diagnostic::warning(
        PathBuf::from("CLAUDE.md"),
        10,
        0,
        "CC-MEM-006",
        "Negative instruction without positive alternative",
    )
    .with_suggestion("Add a positive alternative")
    .with_fix(agnix_core::Fix::replace(
        50,
        80,
        "DO use structured logging",
        "Replace negative with positive",
        true,
    ));

    // MCP server serializes diagnostics as JSON
    let json = serde_json::to_string(&diag).unwrap();
    let roundtrip: agnix_core::Diagnostic = serde_json::from_str(&json).unwrap();

    assert_eq!(roundtrip.level, diag.level);
    assert_eq!(roundtrip.message, diag.message);
    assert_eq!(roundtrip.rule, diag.rule);
    assert_eq!(roundtrip.suggestion, diag.suggestion);
    assert_eq!(roundtrip.fixes.len(), 1);
    assert_eq!(roundtrip.fixes[0].replacement, "DO use structured logging");
}

#[test]
fn mcp_fix_json_serialization() {
    let fix = agnix_core::Fix::replace(10, 20, "new text", "fix description", true);

    let json = serde_json::to_string(&fix).unwrap();
    let roundtrip: agnix_core::Fix = serde_json::from_str(&json).unwrap();

    assert_eq!(roundtrip.start_byte, fix.start_byte);
    assert_eq!(roundtrip.end_byte, fix.end_byte);
    assert_eq!(roundtrip.replacement, fix.replacement);
    assert_eq!(roundtrip.description, fix.description);
    assert_eq!(roundtrip.safe, fix.safe);
}

// ============================================================================
// MCP <-> rules contracts
// ============================================================================

#[test]
fn rules_normalize_tool_name_for_known_tools() {
    assert_eq!(
        agnix_rules::normalize_tool_name("claude-code"),
        Some("claude-code")
    );
    assert_eq!(
        agnix_rules::normalize_tool_name("Claude-Code"),
        Some("claude-code")
    );
    assert_eq!(
        agnix_rules::normalize_tool_name("github-copilot"),
        Some("github-copilot")
    );
    assert_eq!(agnix_rules::normalize_tool_name("unknown-tool"), None);
}

#[test]
fn rules_valid_tools_returns_non_empty_list() {
    let tools = agnix_rules::valid_tools();
    assert!(!tools.is_empty());
    assert!(tools.contains(&"claude-code"));
}

#[test]
fn rules_data_accessible_and_non_empty() {
    assert!(agnix_rules::rule_count() > 0);
}

#[test]
fn rules_get_rule_name_returns_some_for_known_rules() {
    // AS-001 is the very first rule and should always exist
    assert!(agnix_rules::get_rule_name("AS-001").is_some());
    // CC-HK-001 is a Claude Code hooks rule
    assert!(agnix_rules::get_rule_name("CC-HK-001").is_some());
    // Unknown rule should return None
    assert!(agnix_rules::get_rule_name("NONEXISTENT-999").is_none());
}

// ============================================================================
// Serialization contracts (Diagnostic + Fix roundtrip)
// ============================================================================

#[test]
fn diagnostic_serde_roundtrip_preserves_all_fields() {
    use std::path::PathBuf;

    let original = agnix_core::Diagnostic {
        level: agnix_core::DiagnosticLevel::Error,
        message: "Agent config issue".to_string(),
        file: PathBuf::from("project/agents/reviewer.md"),
        line: 42,
        column: 7,
        rule: "CC-AG-003".to_string(),
        suggestion: Some("Use a valid model name".to_string()),
        fixes: vec![
            agnix_core::Fix {
                start_byte: 100,
                end_byte: 115,
                replacement: "sonnet".to_string(),
                description: "Replace with valid model".to_string(),
                safe: true,
            },
            agnix_core::Fix {
                start_byte: 200,
                end_byte: 250,
                replacement: String::new(),
                description: "Remove deprecated field".to_string(),
                safe: false,
            },
        ],
        assumption: Some("Assuming Claude Code >= 1.0.0".to_string()),
        metadata: None,
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: agnix_core::Diagnostic = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.level, original.level);
    assert_eq!(deserialized.message, original.message);
    assert_eq!(deserialized.file, original.file);
    assert_eq!(deserialized.line, original.line);
    assert_eq!(deserialized.column, original.column);
    assert_eq!(deserialized.rule, original.rule);
    assert_eq!(deserialized.suggestion, original.suggestion);
    assert_eq!(deserialized.assumption, original.assumption);
    assert_eq!(deserialized.fixes.len(), 2);
    assert_eq!(deserialized.fixes[0].start_byte, 100);
    assert_eq!(deserialized.fixes[0].replacement, "sonnet");
    assert!(deserialized.fixes[0].safe);
    assert_eq!(deserialized.fixes[1].start_byte, 200);
    assert!(deserialized.fixes[1].replacement.is_empty());
    assert!(!deserialized.fixes[1].safe);
}

#[test]
fn fix_serde_roundtrip_preserves_all_fields() {
    let original = agnix_core::Fix {
        start_byte: 42,
        end_byte: 99,
        replacement: "replacement text".to_string(),
        description: "fix the thing".to_string(),
        safe: false,
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: agnix_core::Fix = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.start_byte, original.start_byte);
    assert_eq!(deserialized.end_byte, original.end_byte);
    assert_eq!(deserialized.replacement, original.replacement);
    assert_eq!(deserialized.description, original.description);
    assert_eq!(deserialized.safe, original.safe);
}

// ============================================================================
// Plugin architecture cross-crate contracts (#348)
// ============================================================================

#[test]
fn plugin_types_importable_from_outside_crate() {
    // ValidatorProvider trait is importable and usable as trait bound
    fn _assert_provider(_: &dyn agnix_core::ValidatorProvider) {}

    // ValidatorRegistryBuilder is importable
    let _ = std::any::type_name::<agnix_core::ValidatorRegistryBuilder>();
}

#[test]
fn builder_usable_from_outside_crate() {
    let registry = agnix_core::ValidatorRegistry::builder()
        .with_defaults()
        .without_validator("XmlValidator")
        .build();

    // Verify the builder produced a valid registry
    assert!(registry.total_factory_count() > 0);
    assert_eq!(registry.disabled_validator_count(), 1);

    // XmlValidator should be excluded from Skill validators
    let skill_validators = registry.validators_for(agnix_core::FileType::Skill);
    let names: Vec<&str> = skill_validators.iter().map(|v| v.name()).collect();
    assert!(!names.contains(&"XmlValidator"));
    assert!(names.contains(&"SkillValidator"));
}

#[test]
fn custom_provider_from_outside_crate() {
    use agnix_core::ValidatorProvider;

    struct ExternalProvider;

    impl agnix_core::ValidatorProvider for ExternalProvider {
        fn validators(&self) -> Vec<(agnix_core::FileType, agnix_core::ValidatorFactory)> {
            vec![]
        }
    }

    let provider = ExternalProvider;
    assert_eq!(provider.name(), "ExternalProvider");

    let registry = agnix_core::ValidatorRegistry::builder()
        .with_defaults()
        .with_provider(&provider)
        .build();

    // Should match default count (empty provider adds nothing)
    let defaults = agnix_core::ValidatorRegistry::with_defaults();
    assert_eq!(
        registry.total_factory_count(),
        defaults.total_factory_count()
    );
}

#[test]
fn disabled_validators_config_accessible_from_outside_crate() {
    let mut config = agnix_core::LintConfig::default();
    assert!(config.rules().disabled_validators.is_empty());

    config.rules_mut().disabled_validators = vec!["XmlValidator".to_string()];
    assert_eq!(config.rules().disabled_validators.len(), 1);
}

#[test]
fn validator_name_accessible_from_outside_crate() {
    let registry = agnix_core::ValidatorRegistry::with_defaults();
    let validators = registry.validators_for(agnix_core::FileType::Skill);

    for v in &validators {
        let name = v.name();
        assert!(!name.is_empty());
        assert!(name.is_ascii());
    }
}

// ============================================================================
// LintConfigBuilder cross-crate tests
// ============================================================================

#[test]
fn builder_accessible_from_outside_crate() {
    // Verify the builder type and ConfigError are exported
    let config: agnix_core::LintConfig = agnix_core::LintConfig::builder().build().unwrap();
    assert_eq!(config.severity(), agnix_core::LintConfig::default().severity());
}

#[test]
fn builder_via_lint_config_factory() {
    // target is deprecated so build() would reject it; use build_unchecked()
    let config = agnix_core::LintConfig::builder()
        .target(agnix_core::config::TargetTool::ClaudeCode)
        .tools(vec!["claude-code".to_string()])
        .build_unchecked();

    assert_eq!(config.target(), agnix_core::config::TargetTool::ClaudeCode);
    assert_eq!(config.tools(), &["claude-code"]);
}

#[test]
fn builder_config_works_with_validate_project() {
    let dir = tempfile::tempdir().unwrap();
    let config = agnix_core::LintConfig::builder()
        .root_dir(dir.path().to_path_buf())
        .build()
        .unwrap();

    let result = agnix_core::validate_project(dir.path(), &config);
    assert!(result.is_ok());
}

#[test]
fn builder_invalid_glob_returns_config_error() {
    let result = agnix_core::LintConfig::builder()
        .exclude(vec!["[bad-pattern".to_string()])
        .build();

    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("[bad-pattern"), "Error should mention the pattern: {}", msg);
}

#[test]
fn builder_path_traversal_returns_config_error() {
    let result = agnix_core::LintConfig::builder()
        .exclude(vec!["../secret/**".to_string()])
        .build();

    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("../secret/**"), "Error should mention the pattern: {}", msg);
}

#[test]
fn builder_disable_rule_affects_validation() {
    let config = agnix_core::LintConfig::builder()
        .disable_rule("AS-001")
        .build()
        .unwrap();

    assert!(!config.is_rule_enabled("AS-001"));
    // Other rules should still be enabled
    assert!(config.is_rule_enabled("AS-004"));
}

#[test]
fn builder_disable_validator_accessible() {
    let config = agnix_core::LintConfig::builder()
        .disable_validator("XmlValidator")
        .build()
        .unwrap();

    assert!(config.rules().disabled_validators.contains(&"XmlValidator".to_string()));
}

#[test]
fn builder_build_unchecked_allows_invalid_patterns() {
    let config = agnix_core::LintConfig::builder()
        .exclude(vec!["[invalid".to_string()])
        .build_unchecked();

    assert_eq!(config.exclude(), &["[invalid".to_string()]);
}

#[test]
fn config_error_is_std_error() {
    // Verify ConfigError implements std::error::Error
    fn assert_error<T: std::error::Error>() {}
    assert_error::<agnix_core::ConfigError>();
}
