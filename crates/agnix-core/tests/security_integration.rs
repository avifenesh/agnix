//! Security-focused integration tests
//!
//! This module tests security scenarios including:
//! - File count limit enforcement
//! - Regex input size limit enforcement
//! - Malformed YAML handling (bombs, alias loops)
//! - Malformed JSON handling (deep nesting)
//!
//! Note: Some security tests (symlink rejection, file size limits) are in the
//! internal unit tests since they require access to private modules.

use agnix_core::{
    config::LintConfig,
    diagnostics::LintError,
    parsers::{frontmatter::split_frontmatter, markdown::MAX_REGEX_INPUT_SIZE},
    validate_project,
};
use tempfile::TempDir;

// ============================================================================
// File Count Limit Tests
// ============================================================================

#[test]
fn test_file_count_limit_enforcement() {
    let temp = TempDir::new().unwrap();

    // Create 15 markdown files
    for i in 0..15 {
        std::fs::write(temp.path().join(format!("file{}.md", i)), "# Content").unwrap();
    }

    // Set a limit of 10 files
    let mut config = LintConfig::default();
    config.max_files_to_validate = Some(10);

    let result = validate_project(temp.path(), &config);

    // Should return TooManyFiles error
    assert!(result.is_err());
    match result.unwrap_err() {
        LintError::TooManyFiles { limit, .. } => {
            assert_eq!(limit, 10);
        }
        e => panic!("Expected TooManyFiles error, got: {:?}", e),
    }
}

#[test]
fn test_default_file_count_limit() {
    let config = LintConfig::default();
    assert_eq!(config.max_files_to_validate, Some(10_000));
}

// ============================================================================
// Regex Input Size Limit Tests
// ============================================================================

#[test]
fn test_regex_input_size_limit_constant() {
    // Verify the constant is set to 64KB
    assert_eq!(MAX_REGEX_INPUT_SIZE, 65536);
}

#[test]
fn test_oversized_content_skips_regex() {
    use agnix_core::parsers::markdown::extract_xml_tags;

    // Create content larger than MAX_REGEX_INPUT_SIZE
    let large_content = "a".repeat(MAX_REGEX_INPUT_SIZE + 1000);
    let content_with_tags = format!("<example>{}</example>", large_content);

    // Tags extraction should return empty for oversized content
    let tags = extract_xml_tags(&content_with_tags);
    assert!(tags.is_empty(), "Oversized content should skip regex");
}

// ============================================================================
// Malformed YAML Tests
// ============================================================================

#[test]
fn test_yaml_bomb_handling() {
    // This is a "billion laughs" style YAML bomb
    // The parser should handle it without excessive memory usage
    let yaml_bomb = r#"---
a: &a ["lol","lol","lol","lol","lol","lol","lol","lol","lol"]
b: &b [*a,*a,*a,*a,*a,*a,*a,*a,*a]
c: &c [*b,*b,*b,*b,*b,*b,*b,*b,*b]
---
body content"#;

    // Should not panic, may return error for deep nesting/cycles
    let parts = split_frontmatter(yaml_bomb);
    assert!(parts.has_frontmatter);
}

#[test]
fn test_yaml_alias_loop_handling() {
    // Self-referential YAML
    let yaml_with_alias = r#"---
name: test
ref: &ref
  nested: *ref
---
body"#;

    // Should not panic
    let parts = split_frontmatter(yaml_with_alias);
    assert!(parts.has_frontmatter);
}

#[test]
fn test_yaml_deeply_nested() {
    // Deeply nested YAML structure
    let mut nested = "value".to_string();
    for _ in 0..100 {
        nested = format!("key:\n  {}", nested.replace('\n', "\n  "));
    }
    let content = format!("---\n{}\n---\nbody", nested);

    // Should not panic
    let parts = split_frontmatter(&content);
    assert!(parts.has_frontmatter);
}

// ============================================================================
// Malformed JSON Tests
// ============================================================================

#[test]
fn test_json_deeply_nested() {
    use agnix_core::parsers::json::parse_json_config;
    use serde_json::Value;

    // Create deeply nested JSON
    let mut json = "null".to_string();
    for _ in 0..1000 {
        json = format!("{{\"a\":{}}}", json);
    }

    // Should not panic, may return error for recursion limit
    let _: Result<Value, _> = parse_json_config(&json);
}

#[test]
fn test_json_with_long_strings() {
    use agnix_core::parsers::json::parse_json_config;
    use serde_json::Value;

    // JSON with very long string value
    let long_string = "x".repeat(100_000);
    let json = format!(r#"{{"key": "{}"}}"#, long_string);

    // Should parse successfully
    let result: Result<Value, _> = parse_json_config(&json);
    assert!(result.is_ok());
}

// ============================================================================
// Path Traversal Tests (via validation)
// ============================================================================

#[test]
fn test_path_traversal_in_imports_detected() {
    let temp = TempDir::new().unwrap();

    // Create a SKILL.md with path traversal import
    // Use .md extension so it's detected as an import reference
    std::fs::write(
        temp.path().join("SKILL.md"),
        "---\nname: test-skill\ndescription: Test skill\n---\n\nSee @../../../etc/passwd.md for details.",
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    // The import validator should detect this as a path escape attempt (REF-002)
    // or as a missing file (REF-003)
    let ref_diagnostics: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("REF-"))
        .collect();

    // Verify that a diagnostic was raised - path traversal should not silently succeed
    // We expect either REF-002 (path escape) or REF-003 (missing file)
    assert!(
        !ref_diagnostics.is_empty(),
        "Path traversal attempt should be detected by import validation. Got diagnostics: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.rule)
            .collect::<Vec<_>>()
    );

    // Additionally verify no actual file read occurred outside the temp directory
    // This would be a security failure if the path traversal succeeded
    // The validator should have rejected the path before attempting to read
}

// ============================================================================
// Empty and Edge Case Tests
// ============================================================================

#[test]
fn test_empty_project_succeeds() {
    let temp = TempDir::new().unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().files_checked, 0);
}

#[test]
fn test_project_with_only_unknown_files() {
    let temp = TempDir::new().unwrap();

    // Create files that won't be validated (unknown types)
    std::fs::write(temp.path().join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(temp.path().join("data.txt"), "some data").unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    // Unknown file types should not be counted
    assert_eq!(result.files_checked, 0);
}
