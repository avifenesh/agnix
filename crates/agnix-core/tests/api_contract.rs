//! API contract tests for agnix-core.
//!
//! These tests catch accidental public API breakage by verifying that all
//! documented public types, functions, and trait implementations remain
//! importable and have the expected shape.
//!
//! If a test here fails after a code change, it means a public API was
//! modified. Check CONTRIBUTING.md's backward-compatibility policy before
//! proceeding.

// ============================================================================
// Public type importability
// ============================================================================

#[test]
fn public_types_are_importable() {
    // Public/Stable re-exports at crate root
    let _ = std::any::type_name::<agnix_core::LintConfig>();
    let _ = std::any::type_name::<agnix_core::Diagnostic>();
    let _ = std::any::type_name::<agnix_core::DiagnosticLevel>();
    let _ = std::any::type_name::<agnix_core::Fix>();
    let _ = std::any::type_name::<agnix_core::LintError>();
    let _ = std::any::type_name::<agnix_core::ValidationResult>();
    let _ = std::any::type_name::<agnix_core::FileType>();
    let _ = std::any::type_name::<agnix_core::ValidatorRegistry>();
    let _ = std::any::type_name::<agnix_core::FixResult>();
    let _ = std::any::type_name::<agnix_core::ConfigWarning>();
    let _ = std::any::type_name::<agnix_core::FilesConfig>();

    // LintResult type alias
    let _ = std::any::type_name::<agnix_core::LintResult<()>>();

    // ValidatorFactory type alias
    let _ = std::any::type_name::<agnix_core::ValidatorFactory>();

    // Trait objects
    fn _assert_validator_trait(_: &dyn agnix_core::Validator) {}
    fn _assert_filesystem_trait(_: &dyn agnix_core::FileSystem) {}

    // FileSystem implementations
    let _ = std::any::type_name::<agnix_core::MockFileSystem>();
    let _ = std::any::type_name::<agnix_core::RealFileSystem>();
}

// ============================================================================
// Public function signatures
// ============================================================================

#[test]
fn public_functions_compile_with_expected_signatures() {
    use std::path::Path;

    // validate_file(path, config) -> LintResult<Vec<Diagnostic>>
    let _: fn(
        &Path,
        &agnix_core::LintConfig,
    ) -> agnix_core::LintResult<Vec<agnix_core::Diagnostic>> = agnix_core::validate_file;

    // validate_project(path, config) -> LintResult<ValidationResult>
    let _: fn(
        &Path,
        &agnix_core::LintConfig,
    ) -> agnix_core::LintResult<agnix_core::ValidationResult> = agnix_core::validate_project;

    // validate_project_rules(root, config) -> LintResult<Vec<Diagnostic>>
    let _: fn(
        &Path,
        &agnix_core::LintConfig,
    ) -> agnix_core::LintResult<Vec<agnix_core::Diagnostic>> = agnix_core::validate_project_rules;

    // validate_project_with_registry(path, config, registry) -> LintResult<ValidationResult>
    let _: fn(
        &Path,
        &agnix_core::LintConfig,
        &agnix_core::ValidatorRegistry,
    ) -> agnix_core::LintResult<agnix_core::ValidationResult> =
        agnix_core::validate_project_with_registry;

    // validate_file_with_registry(path, config, registry) -> LintResult<Vec<Diagnostic>>
    let _: fn(
        &Path,
        &agnix_core::LintConfig,
        &agnix_core::ValidatorRegistry,
    ) -> agnix_core::LintResult<Vec<agnix_core::Diagnostic>> =
        agnix_core::validate_file_with_registry;

    // detect_file_type(path) -> FileType
    let _: fn(&Path) -> agnix_core::FileType = agnix_core::detect_file_type;

    // resolve_file_type(path, config) -> FileType
    let _: fn(&Path, &agnix_core::LintConfig) -> agnix_core::FileType =
        agnix_core::resolve_file_type;

    // apply_fixes(diagnostics, dry_run, safe_only) -> LintResult<Vec<FixResult>>
    let _: fn(
        &[agnix_core::Diagnostic],
        bool,
        bool,
    ) -> agnix_core::LintResult<Vec<agnix_core::FixResult>> = agnix_core::apply_fixes;

    // generate_schema() -> schemars::schema::RootSchema
    let _: fn() -> schemars::schema::RootSchema = agnix_core::generate_schema;
}

// ============================================================================
// Key trait implementations
// ============================================================================

fn assert_serialize<T: serde::Serialize>() {}
fn assert_clone<T: Clone>() {}
fn assert_debug<T: std::fmt::Debug>() {}
fn assert_partial_eq<T: PartialEq>() {}
fn assert_eq_trait<T: Eq>() {}
fn assert_copy<T: Copy>() {}
fn assert_hash<T: std::hash::Hash>() {}
fn assert_default<T: Default>() {}
fn assert_send<T: Send>() {}
fn assert_sync<T: Sync>() {}

#[test]
fn diagnostic_implements_required_traits() {
    assert_serialize::<agnix_core::Diagnostic>();
    assert_clone::<agnix_core::Diagnostic>();
    assert_debug::<agnix_core::Diagnostic>();
}

#[test]
fn diagnostic_level_implements_required_traits() {
    assert_partial_eq::<agnix_core::DiagnosticLevel>();
    assert_eq_trait::<agnix_core::DiagnosticLevel>();
    assert_clone::<agnix_core::DiagnosticLevel>();
    assert_copy::<agnix_core::DiagnosticLevel>();
}

#[test]
fn file_type_implements_required_traits() {
    assert_partial_eq::<agnix_core::FileType>();
    assert_eq_trait::<agnix_core::FileType>();
    assert_hash::<agnix_core::FileType>();
    assert_clone::<agnix_core::FileType>();
    assert_copy::<agnix_core::FileType>();
}

#[test]
fn lint_config_implements_required_traits() {
    assert_default::<agnix_core::LintConfig>();
    assert_debug::<agnix_core::LintConfig>();
}

#[test]
fn validator_registry_implements_required_traits() {
    assert_default::<agnix_core::ValidatorRegistry>();
    assert_send::<agnix_core::ValidatorRegistry>();
    assert_sync::<agnix_core::ValidatorRegistry>();
}

// ============================================================================
// Struct field accessibility (construction by field)
// ============================================================================

#[test]
fn diagnostic_fields_are_accessible() {
    use std::path::PathBuf;

    let diag = agnix_core::Diagnostic {
        level: agnix_core::DiagnosticLevel::Warning,
        message: String::from("test message"),
        file: PathBuf::from("test.md"),
        line: 1,
        column: 0,
        rule: String::from("AS-001"),
        suggestion: Some(String::from("try this")),
        fixes: vec![],
        assumption: None,
    };

    // Read back all fields to verify accessibility
    let _: &agnix_core::DiagnosticLevel = &diag.level;
    let _: &String = &diag.message;
    let _: &PathBuf = &diag.file;
    let _: usize = diag.line;
    let _: usize = diag.column;
    let _: &String = &diag.rule;
    let _: &Option<String> = &diag.suggestion;
    let _: &Vec<agnix_core::Fix> = &diag.fixes;
    let _: &Option<String> = &diag.assumption;
}

#[test]
fn fix_fields_are_accessible() {
    let fix = agnix_core::Fix {
        start_byte: 0,
        end_byte: 10,
        replacement: String::from("new text"),
        description: String::from("replace old text"),
        safe: true,
    };

    // Read back all fields
    let _: usize = fix.start_byte;
    let _: usize = fix.end_byte;
    let _: &String = &fix.replacement;
    let _: &String = &fix.description;
    let _: bool = fix.safe;
}

// ============================================================================
// FileType enum exhaustive match
// ============================================================================

#[test]
fn file_type_enum_covers_all_variants() {
    // This match must cover ALL variants. If a variant is added or removed,
    // this test will fail to compile.
    let variants = [
        agnix_core::FileType::Skill,
        agnix_core::FileType::ClaudeMd,
        agnix_core::FileType::Agent,
        agnix_core::FileType::Hooks,
        agnix_core::FileType::Plugin,
        agnix_core::FileType::Mcp,
        agnix_core::FileType::Copilot,
        agnix_core::FileType::CopilotScoped,
        agnix_core::FileType::ClaudeRule,
        agnix_core::FileType::CursorRule,
        agnix_core::FileType::CursorRulesLegacy,
        agnix_core::FileType::ClineRules,
        agnix_core::FileType::ClineRulesFolder,
        agnix_core::FileType::OpenCodeConfig,
        agnix_core::FileType::GeminiMd,
        agnix_core::FileType::CodexConfig,
        agnix_core::FileType::GenericMarkdown,
        agnix_core::FileType::Unknown,
    ];

    assert_eq!(
        variants.len(),
        18,
        "A new FileType variant may have been added or removed. Please update this test's variant list and the match statement below."
    );

    for variant in &variants {
        match variant {
            agnix_core::FileType::Skill => {}
            agnix_core::FileType::ClaudeMd => {}
            agnix_core::FileType::Agent => {}
            agnix_core::FileType::Hooks => {}
            agnix_core::FileType::Plugin => {}
            agnix_core::FileType::Mcp => {}
            agnix_core::FileType::Copilot => {}
            agnix_core::FileType::CopilotScoped => {}
            agnix_core::FileType::ClaudeRule => {}
            agnix_core::FileType::CursorRule => {}
            agnix_core::FileType::CursorRulesLegacy => {}
            agnix_core::FileType::ClineRules => {}
            agnix_core::FileType::ClineRulesFolder => {}
            agnix_core::FileType::OpenCodeConfig => {}
            agnix_core::FileType::GeminiMd => {}
            agnix_core::FileType::CodexConfig => {}
            agnix_core::FileType::GenericMarkdown => {}
            agnix_core::FileType::Unknown => {}
        }
    }
}

// ============================================================================
// Module accessibility
// ============================================================================

#[test]
fn public_modules_are_accessible() {
    // Public/Stable modules
    let _ = std::any::type_name::<agnix_core::config::LintConfig>();
    let _ = std::any::type_name::<agnix_core::diagnostics::Diagnostic>();
    let _ = std::any::type_name::<agnix_core::fixes::FixResult>();
    let _ = std::any::type_name::<agnix_core::fs::RealFileSystem>();

    // Public/Unstable modules
    let _ = std::any::type_name::<agnix_core::eval::EvalCase>();
    let _ = std::any::type_name::<agnix_core::eval::EvalFormat>();
    let _ = std::any::type_name::<agnix_core::eval::EvalResult>();
    let _ = std::any::type_name::<agnix_core::eval::EvalSummary>();
    let _ = std::any::type_name::<agnix_core::eval::EvalManifest>();
    let _ = std::any::type_name::<agnix_core::eval::EvalError>();
}

// ============================================================================
// Submodule types
// ============================================================================

#[test]
fn config_submodule_types_are_accessible() {
    let _ = std::any::type_name::<agnix_core::config::TargetTool>();
    let _ = std::any::type_name::<agnix_core::config::SeverityLevel>();
    let _ = std::any::type_name::<agnix_core::config::RuleConfig>();
    let _ = std::any::type_name::<agnix_core::config::ToolVersions>();
    let _ = std::any::type_name::<agnix_core::config::SpecRevisions>();
    let _ = std::any::type_name::<agnix_core::config::ConfigWarning>();
    let _ = std::any::type_name::<agnix_core::config::FilesConfig>();
}

#[test]
fn eval_submodule_types_are_accessible() {
    let _ = std::any::type_name::<agnix_core::eval::EvalFormat>();
    let _ = std::any::type_name::<agnix_core::eval::EvalCase>();
    let _ = std::any::type_name::<agnix_core::eval::EvalResult>();
    let _ = std::any::type_name::<agnix_core::eval::EvalSummary>();
    let _ = std::any::type_name::<agnix_core::eval::EvalManifest>();
    let _ = std::any::type_name::<agnix_core::eval::EvalError>();
}

// ============================================================================
// ValidationResult field accessibility
// ============================================================================

#[test]
fn validation_result_fields_are_accessible() {
    let result = agnix_core::ValidationResult::new(vec![], 0);

    let _: &Vec<agnix_core::Diagnostic> = &result.diagnostics;
    let _: usize = result.files_checked;

    // New metadata fields default to None/0
    assert!(result.validation_time_ms.is_none());
    assert_eq!(result.validator_factories_registered, 0);

    // Builder-style setters
    let result = agnix_core::ValidationResult::new(vec![], 5)
        .with_timing(42)
        .with_validator_factories_registered(10);
    assert_eq!(result.validation_time_ms, Some(42));
    assert_eq!(result.validator_factories_registered, 10);
    assert_eq!(result.files_checked, 5);
}

// ============================================================================
// FixResult field accessibility
// ============================================================================

#[test]
fn fix_result_fields_are_accessible() {
    use std::path::PathBuf;

    let result = agnix_core::FixResult {
        path: PathBuf::from("test.md"),
        original: String::from("old"),
        fixed: String::from("new"),
        applied: vec![String::from("applied a fix")],
    };

    let _: &PathBuf = &result.path;
    let _: &String = &result.original;
    let _: &String = &result.fixed;
    let _: &Vec<String> = &result.applied;
    let _: bool = result.has_changes();
}

// ============================================================================
// ConfigWarning field accessibility
// ============================================================================

#[test]
fn config_warning_fields_are_accessible() {
    let warning = agnix_core::ConfigWarning {
        field: String::from("rules.disabled_rules"),
        message: String::from("Unknown rule ID"),
        suggestion: Some(String::from("Did you mean AS-001?")),
    };

    let _: &String = &warning.field;
    let _: &String = &warning.message;
    let _: &Option<String> = &warning.suggestion;
}

// ============================================================================
// DiagnosticLevel enum exhaustive match
// ============================================================================

#[test]
fn diagnostic_level_covers_all_variants() {
    let levels = [
        agnix_core::DiagnosticLevel::Error,
        agnix_core::DiagnosticLevel::Warning,
        agnix_core::DiagnosticLevel::Info,
    ];

    for level in &levels {
        match level {
            agnix_core::DiagnosticLevel::Error => {}
            agnix_core::DiagnosticLevel::Warning => {}
            agnix_core::DiagnosticLevel::Info => {}
        }
    }
}

// ============================================================================
// TargetTool enum exhaustive match
// ============================================================================

#[test]
fn target_tool_covers_all_variants() {
    let tools = [
        agnix_core::config::TargetTool::Generic,
        agnix_core::config::TargetTool::ClaudeCode,
        agnix_core::config::TargetTool::Cursor,
        agnix_core::config::TargetTool::Codex,
    ];

    for tool in &tools {
        match tool {
            agnix_core::config::TargetTool::Generic => {}
            agnix_core::config::TargetTool::ClaudeCode => {}
            agnix_core::config::TargetTool::Cursor => {}
            agnix_core::config::TargetTool::Codex => {}
        }
    }
}

// ============================================================================
// SeverityLevel enum exhaustive match
// ============================================================================

#[test]
fn severity_level_covers_all_variants() {
    let levels = [
        agnix_core::config::SeverityLevel::Error,
        agnix_core::config::SeverityLevel::Warning,
        agnix_core::config::SeverityLevel::Info,
    ];

    for level in &levels {
        match level {
            agnix_core::config::SeverityLevel::Error => {}
            agnix_core::config::SeverityLevel::Warning => {}
            agnix_core::config::SeverityLevel::Info => {}
        }
    }
}

// ============================================================================
// Fix constructor and helper methods
// ============================================================================

#[test]
fn fix_constructors_and_helpers() {
    // Fix::replace
    let replace = agnix_core::Fix::replace(0, 10, "new", "replace text", true);
    assert_eq!(replace.start_byte, 0);
    assert_eq!(replace.end_byte, 10);
    assert!(!replace.is_insertion());
    assert!(!replace.is_deletion());

    // Fix::insert (start_byte == end_byte)
    let insert = agnix_core::Fix::insert(5, "inserted", "insert text", true);
    assert_eq!(insert.start_byte, 5);
    assert_eq!(insert.end_byte, 5);
    assert!(insert.is_insertion());
    assert!(!insert.is_deletion());

    // Fix::delete (replacement is empty)
    let delete = agnix_core::Fix::delete(10, 20, "delete text", false);
    assert_eq!(delete.start_byte, 10);
    assert_eq!(delete.end_byte, 20);
    assert!(!delete.is_insertion());
    assert!(delete.is_deletion());
}

// ============================================================================
// Diagnostic builder methods
// ============================================================================

#[test]
fn diagnostic_builder_methods() {
    use std::path::PathBuf;

    // Constructor variants
    let _err = agnix_core::Diagnostic::error(PathBuf::from("a.md"), 1, 0, "R-001", "err");
    let _warn = agnix_core::Diagnostic::warning(PathBuf::from("b.md"), 2, 0, "R-002", "warn");
    let _info = agnix_core::Diagnostic::info(PathBuf::from("c.md"), 3, 0, "R-003", "info");

    // Builder chain
    let diag = agnix_core::Diagnostic::error(PathBuf::from("d.md"), 1, 0, "R-004", "test")
        .with_suggestion("try this")
        .with_assumption("assuming v1")
        .with_fix(agnix_core::Fix::replace(0, 5, "fixed", "auto", true))
        .with_fixes(vec![agnix_core::Fix::insert(
            10,
            "extra",
            "add extra",
            true,
        )]);

    assert!(diag.has_fixes());
    assert!(diag.has_safe_fixes());
    assert_eq!(diag.fixes.len(), 2);
    assert_eq!(diag.assumption, Some("assuming v1".to_string()));
    assert_eq!(diag.suggestion, Some("try this".to_string()));
}

// ============================================================================
// ValidatorRegistry method signatures
// ============================================================================

#[test]
fn validator_registry_methods() {
    // new() and with_defaults()
    let empty = agnix_core::ValidatorRegistry::new();
    let defaults = agnix_core::ValidatorRegistry::with_defaults();

    // validators_for returns Vec<Box<dyn Validator>>
    let empty_validators = empty.validators_for(agnix_core::FileType::Skill);
    assert!(empty_validators.is_empty());

    let default_validators = defaults.validators_for(agnix_core::FileType::Skill);
    assert!(!default_validators.is_empty());

    // register() signature check: accepts FileType + ValidatorFactory (fn pointer)
    let _: fn(
        &mut agnix_core::ValidatorRegistry,
        agnix_core::FileType,
        agnix_core::ValidatorFactory,
    ) = agnix_core::ValidatorRegistry::register;

    // total_factory_count()
    assert_eq!(empty.total_factory_count(), 0);
    assert!(defaults.total_factory_count() > 0);

    // disable_validator() and disabled_validator_count()
    let mut registry = agnix_core::ValidatorRegistry::with_defaults();
    assert_eq!(registry.disabled_validator_count(), 0);
    registry.disable_validator("XmlValidator");
    assert_eq!(registry.disabled_validator_count(), 1);

    // builder()
    let _builder_registry = agnix_core::ValidatorRegistry::builder()
        .with_defaults()
        .build();
}

// ============================================================================
// ValidatorProvider and ValidatorRegistryBuilder importability
// ============================================================================

#[test]
fn new_plugin_types_are_importable() {
    let _ = std::any::type_name::<agnix_core::ValidatorRegistryBuilder>();

    // ValidatorProvider is a trait - verify it can be used as trait bound
    fn _assert_provider_trait(_: &dyn agnix_core::ValidatorProvider) {}
}

#[test]
fn builder_method_signatures_compile() {
    // builder() -> ValidatorRegistryBuilder
    let mut builder = agnix_core::ValidatorRegistry::builder();

    // with_defaults() -> &mut Self
    let _: &mut agnix_core::ValidatorRegistryBuilder = builder.with_defaults();

    // without_validator() -> &mut Self
    let _: &mut agnix_core::ValidatorRegistryBuilder = builder.without_validator("XmlValidator");

    // build() -> ValidatorRegistry
    let _: agnix_core::ValidatorRegistry = builder.build();
}

#[test]
fn builder_built_registry_matches_with_defaults_factory_count() {
    let via_builder = agnix_core::ValidatorRegistry::builder()
        .with_defaults()
        .build();
    let via_direct = agnix_core::ValidatorRegistry::with_defaults();

    assert_eq!(
        via_builder.total_factory_count(),
        via_direct.total_factory_count(),
    );
}
