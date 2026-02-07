//! End-to-end fix application and integration tests
//!
//! This module tests the full pipeline: validate -> collect fixes -> apply -> re-validate.
//! It covers:
//! - E2E fix roundtrips for multiple rules (AS-005, AS-006, CC-MEM-005, XML-003, etc.)
//! - Fix safety flag correctness (safe vs unsafe)
//! - Fix determinism (same input -> same output)
//! - Fix ordering with MockFileSystem integration
//! - Safe-only filtering

use agnix_core::{
    apply_fixes_with_fs, Diagnostic, FileSystem, FileType, Fix, LintConfig, MockFileSystem,
    ValidatorRegistry,
};
use std::path::Path;
use std::sync::Arc;

// ============================================================================
// Shared Helper
// ============================================================================

/// Validate content, apply all fixes for `target_rule`, re-validate, and assert the
/// target rule no longer fires.
fn assert_fix_resolves(file_type: FileType, path: &Path, content: &str, target_rule: &str) {
    let config = LintConfig::default();
    let registry = ValidatorRegistry::with_defaults();
    let validators = registry.validators_for(file_type);

    // Step 1: Validate and collect diagnostics
    let mut all_diags = Vec::new();
    for v in &validators {
        all_diags.extend(v.validate(path, content, &config));
    }

    // Step 2: Filter to target rule and verify it has fixes
    let target_diags: Vec<_> = all_diags.iter().filter(|d| d.rule == target_rule).collect();
    assert!(
        !target_diags.is_empty(),
        "Expected at least one {} diagnostic, got none. All diagnostics: {:?}",
        target_rule,
        all_diags.iter().map(|d| &d.rule).collect::<Vec<_>>()
    );
    assert!(
        target_diags.iter().any(|d| d.has_fixes()),
        "Expected {} to have fixes",
        target_rule
    );

    // Step 3: Build diagnostics for fix application (need owned Diagnostic with file path)
    let fix_diags: Vec<Diagnostic> = all_diags
        .iter()
        .filter(|d| d.rule == target_rule && d.has_fixes())
        .map(|d| {
            let mut cloned = d.clone();
            cloned.file = path.to_path_buf();
            cloned
        })
        .collect();

    // Step 4: Apply fixes via MockFileSystem
    let mock_fs = Arc::new(MockFileSystem::new());
    mock_fs.add_file(path, content);
    let fs_clone: Arc<dyn FileSystem> = Arc::clone(&mock_fs) as Arc<dyn FileSystem>;
    let results = apply_fixes_with_fs(&fix_diags, false, false, Some(fs_clone)).unwrap();
    assert!(
        !results.is_empty(),
        "Expected fixes to be applied for {}",
        target_rule
    );

    // Step 5: Get fixed content
    let fixed_content = mock_fs.read_to_string(path).unwrap();
    assert_ne!(
        fixed_content, content,
        "Fix should have changed the content for {}",
        target_rule
    );

    // Step 6: Re-validate and assert target rule is gone
    let mut re_diags = Vec::new();
    for v in &validators {
        re_diags.extend(v.validate(path, &fixed_content, &config));
    }
    let remaining: Vec<_> = re_diags.iter().filter(|d| d.rule == target_rule).collect();
    assert!(
        remaining.is_empty(),
        "After fix, {} should not fire. Got {} remaining diagnostics. Fixed content:\n{}",
        target_rule,
        remaining.len(),
        fixed_content
    );
}

// ============================================================================
// E2E Pipeline Tests
// ============================================================================

#[test]
fn test_e2e_as_005_fix_trims_leading_trailing_hyphens() {
    let content = "---\nname: -my-skill-\ndescription: A test skill\n---\nBody content";
    assert_fix_resolves(FileType::Skill, Path::new("SKILL.md"), content, "AS-005");
}

#[test]
fn test_e2e_as_006_fix_collapses_consecutive_hyphens() {
    let content = "---\nname: my--skill\ndescription: A test skill\n---\nBody content";
    assert_fix_resolves(FileType::Skill, Path::new("SKILL.md"), content, "AS-006");
}

#[test]
fn test_e2e_cc_mem_005_fix_removes_generic_instruction() {
    let content = "# Project\n\nBe helpful and concise.\n\n## Details\nActual content.";
    assert_fix_resolves(
        FileType::ClaudeMd,
        Path::new("CLAUDE.md"),
        content,
        "CC-MEM-005",
    );
}

#[test]
fn test_e2e_xml_003_fix_removes_orphan_tag() {
    let content = "---\nname: test-skill\ndescription: A test skill\n---\nSome text\n</example>\nMore text";
    assert_fix_resolves(FileType::Skill, Path::new("SKILL.md"), content, "XML-003");
}

#[test]
fn test_e2e_cc_hk_001_fix_corrects_event_name() {
    // "pretooluse" is a case-insensitive match for "PreToolUse"
    let content = r#"{
    "hooks": {
        "pretooluse": [
            {
                "matcher": "Bash",
                "hooks": [
                    { "type": "command", "command": "echo test", "timeout": 30 }
                ]
            }
        ]
    }
}"#;
    assert_fix_resolves(
        FileType::Hooks,
        Path::new("settings.json"),
        content,
        "CC-HK-001",
    );
}

#[test]
fn test_e2e_cc_hk_011_fix_timeout_range() {
    let content = r#"{
    "hooks": {
        "PreToolUse": [
            {
                "matcher": "Bash",
                "hooks": [
                    { "type": "command", "command": "echo test", "timeout": -5 }
                ]
            }
        ]
    }
}"#;
    assert_fix_resolves(
        FileType::Hooks,
        Path::new("settings.json"),
        content,
        "CC-HK-011",
    );
}

#[test]
fn test_e2e_mcp_011_fix_transport_case() {
    // "STDIO" is an invalid type, but close to "stdio"
    let content = r#"{
    "mcpServers": {
        "my-server": {
            "type": "Stdio",
            "command": "node"
        }
    }
}"#;
    assert_fix_resolves(
        FileType::Mcp,
        Path::new("test.mcp.json"),
        content,
        "MCP-011",
    );
}

#[test]
fn test_e2e_cc_sk_014_fix_string_boolean() {
    // CC-SK-014 checks disable-model-invocation for quoted boolean strings
    let content = "---\nname: test-skill\ndescription: A test skill\ndisable-model-invocation: \"true\"\n---\nBody content";
    assert_fix_resolves(FileType::Skill, Path::new("SKILL.md"), content, "CC-SK-014");
}

#[test]
fn test_e2e_as_004_fix_name_format() {
    let content = "---\nname: My Skill Name\ndescription: A test skill\n---\nBody content";
    assert_fix_resolves(FileType::Skill, Path::new("SKILL.md"), content, "AS-004");
}

#[test]
fn test_e2e_multi_fix_same_file() {
    // Content with violations at different byte ranges:
    // - AS-005 on the name field (leading/trailing hyphens)
    // - XML-003 orphan closing tag in the body
    // These target non-overlapping regions so both fixes should apply.
    let content =
        "---\nname: -my-skill-\ndescription: A test skill\n---\nSome text\n</orphan>\nMore text";
    let path = Path::new("SKILL.md");
    let config = LintConfig::default();
    let registry = ValidatorRegistry::with_defaults();
    let validators = registry.validators_for(FileType::Skill);

    // Step 1: Validate
    let mut all_diags = Vec::new();
    for v in &validators {
        all_diags.extend(v.validate(path, content, &config));
    }

    // Verify both rules fire
    let as_005: Vec<_> = all_diags.iter().filter(|d| d.rule == "AS-005").collect();
    let xml_003: Vec<_> = all_diags.iter().filter(|d| d.rule == "XML-003").collect();
    assert!(!as_005.is_empty(), "Expected AS-005 diagnostic");
    assert!(!xml_003.is_empty(), "Expected XML-003 diagnostic");

    // Step 2: Collect ALL diagnostics with fixes (both rules)
    let fix_diags: Vec<Diagnostic> = all_diags
        .iter()
        .filter(|d| (d.rule == "AS-005" || d.rule == "XML-003") && d.has_fixes())
        .map(|d| {
            let mut cloned = d.clone();
            cloned.file = path.to_path_buf();
            cloned
        })
        .collect();

    // Step 3: Apply all fixes
    let mock_fs = Arc::new(MockFileSystem::new());
    mock_fs.add_file(path, content);
    let fs_clone: Arc<dyn FileSystem> = Arc::clone(&mock_fs) as Arc<dyn FileSystem>;
    let results = apply_fixes_with_fs(&fix_diags, false, false, Some(fs_clone)).unwrap();
    assert!(!results.is_empty(), "Expected fixes to be applied");

    // Step 4: Re-validate
    let fixed_content = mock_fs.read_to_string(path).unwrap();
    let mut re_diags = Vec::new();
    for v in &validators {
        re_diags.extend(v.validate(path, &fixed_content, &config));
    }

    // Both violations should be resolved
    let remaining_005: Vec<_> = re_diags.iter().filter(|d| d.rule == "AS-005").collect();
    let remaining_003: Vec<_> = re_diags.iter().filter(|d| d.rule == "XML-003").collect();
    assert!(
        remaining_005.is_empty(),
        "AS-005 should not fire after fix. Fixed content:\n{}",
        fixed_content
    );
    assert!(
        remaining_003.is_empty(),
        "XML-003 should not fire after fix. Fixed content:\n{}",
        fixed_content
    );
}

// ============================================================================
// Fix Safety Flag Tests
// ============================================================================

#[test]
fn test_safe_fix_deterministic_as_005() {
    let content = "---\nname: -my-skill-\ndescription: A test skill\n---\nBody content";
    let path = Path::new("SKILL.md");
    let config = LintConfig::default();
    let registry = ValidatorRegistry::with_defaults();
    let validators = registry.validators_for(FileType::Skill);

    let mut reference_fix: Option<Fix> = None;
    for _ in 0..10 {
        let mut diags = Vec::new();
        for v in &validators {
            diags.extend(v.validate(path, content, &config));
        }
        let as_005: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "AS-005" && d.has_fixes())
            .collect();
        assert!(!as_005.is_empty());
        let fix = &as_005[0].fixes[0];

        if let Some(ref prev) = reference_fix {
            assert_eq!(prev.start_byte, fix.start_byte, "start_byte must be deterministic");
            assert_eq!(prev.end_byte, fix.end_byte, "end_byte must be deterministic");
            assert_eq!(prev.replacement, fix.replacement, "replacement must be deterministic");
            assert_eq!(prev.safe, fix.safe, "safe flag must be deterministic");
        } else {
            reference_fix = Some(fix.clone());
        }
    }
}

#[test]
fn test_safe_fix_deterministic_as_006() {
    let content = "---\nname: my--skill\ndescription: A test skill\n---\nBody content";
    let path = Path::new("SKILL.md");
    let config = LintConfig::default();
    let registry = ValidatorRegistry::with_defaults();
    let validators = registry.validators_for(FileType::Skill);

    let mut reference_fix: Option<Fix> = None;
    for _ in 0..10 {
        let mut diags = Vec::new();
        for v in &validators {
            diags.extend(v.validate(path, content, &config));
        }
        let as_006: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "AS-006" && d.has_fixes())
            .collect();
        assert!(!as_006.is_empty());
        let fix = &as_006[0].fixes[0];

        if let Some(ref prev) = reference_fix {
            assert_eq!(prev.start_byte, fix.start_byte, "start_byte must be deterministic");
            assert_eq!(prev.end_byte, fix.end_byte, "end_byte must be deterministic");
            assert_eq!(prev.replacement, fix.replacement, "replacement must be deterministic");
            assert_eq!(prev.safe, fix.safe, "safe flag must be deterministic");
        } else {
            reference_fix = Some(fix.clone());
        }
    }
}

#[test]
fn test_unsafe_fix_correctly_marked_xml_001() {
    // Unclosed tag should produce an unsafe fix
    let content = "<example>test content";
    let path = Path::new("test.md");
    let config = LintConfig::default();
    let registry = ValidatorRegistry::with_defaults();
    // XML validator runs on Skill files (among others)
    let validators = registry.validators_for(FileType::Skill);

    let mut diags = Vec::new();
    for v in &validators {
        diags.extend(v.validate(path, content, &config));
    }
    let xml_001: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "XML-001" && d.has_fixes())
        .collect();
    assert!(!xml_001.is_empty(), "Expected XML-001 diagnostic with fix");
    assert!(!xml_001[0].fixes[0].safe, "XML-001 fix should be unsafe");
}

#[test]
fn test_unsafe_fix_correctly_marked_xml_003() {
    // Orphan closing tag should produce an unsafe fix
    let content = "</orphan>";
    let path = Path::new("test.md");
    let config = LintConfig::default();
    let registry = ValidatorRegistry::with_defaults();
    let validators = registry.validators_for(FileType::Skill);

    let mut diags = Vec::new();
    for v in &validators {
        diags.extend(v.validate(path, content, &config));
    }
    let xml_003: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "XML-003" && d.has_fixes())
        .collect();
    assert!(!xml_003.is_empty(), "Expected XML-003 diagnostic with fix");
    assert!(!xml_003[0].fixes[0].safe, "XML-003 fix should be unsafe");
}

#[test]
fn test_cc_mem_005_fix_is_safe() {
    let content = "Be helpful and concise.";
    let path = Path::new("CLAUDE.md");
    let config = LintConfig::default();
    let registry = ValidatorRegistry::with_defaults();
    let validators = registry.validators_for(FileType::ClaudeMd);

    let mut diags = Vec::new();
    for v in &validators {
        diags.extend(v.validate(path, content, &config));
    }
    let mem_005: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "CC-MEM-005" && d.has_fixes())
        .collect();
    assert!(!mem_005.is_empty(), "Expected CC-MEM-005 diagnostic with fix");
    assert!(mem_005[0].fixes[0].safe, "CC-MEM-005 fix should be safe");
}

#[test]
fn test_safe_only_filter_skips_unsafe() {
    // Create content that has both safe and unsafe fixes
    // XML-001 produces unsafe fix, so we use a skill file with both name issue (safe) and XML (unsafe)
    let content = "---\nname: -my-skill-\ndescription: A test skill\n---\n<example>unclosed tag";
    let path = Path::new("SKILL.md");
    let config = LintConfig::default();
    let registry = ValidatorRegistry::with_defaults();
    let validators = registry.validators_for(FileType::Skill);

    let mut all_diags = Vec::new();
    for v in &validators {
        all_diags.extend(v.validate(path, content, &config));
    }

    // Verify we have both safe and unsafe fixes
    let all_fix_diags: Vec<Diagnostic> = all_diags
        .iter()
        .filter(|d| d.has_fixes())
        .map(|d| {
            let mut cloned = d.clone();
            cloned.file = path.to_path_buf();
            cloned
        })
        .collect();

    let has_safe = all_fix_diags.iter().any(|d| d.fixes.iter().any(|f| f.safe));
    let has_unsafe = all_fix_diags.iter().any(|d| d.fixes.iter().any(|f| !f.safe));
    assert!(has_safe, "Expected at least one safe fix");
    assert!(has_unsafe, "Expected at least one unsafe fix");

    // Apply with safe_only=true
    let mock_fs = Arc::new(MockFileSystem::new());
    mock_fs.add_file(path, content);
    let fs_clone: Arc<dyn FileSystem> = Arc::clone(&mock_fs) as Arc<dyn FileSystem>;
    let results = apply_fixes_with_fs(&all_fix_diags, false, true, Some(fs_clone)).unwrap();

    // Verify that fixes were applied
    assert!(!results.is_empty(), "Expected some safe fixes to be applied");

    // Verify the fixed content still has unclosed XML (unsafe fix was skipped)
    let fixed_content = mock_fs.read_to_string(path).unwrap();

    // Re-validate: XML-001 should still fire (unsafe fix was skipped)
    let mut re_diags = Vec::new();
    for v in &validators {
        re_diags.extend(v.validate(path, &fixed_content, &config));
    }
    let remaining_xml: Vec<_> = re_diags.iter().filter(|d| d.rule == "XML-001").collect();
    assert!(
        !remaining_xml.is_empty(),
        "XML-001 should still fire when safe_only=true (unsafe fix skipped)"
    );
}

#[test]
fn test_all_fixes_filter_applies_both() {
    // Same content as above but with safe_only=false
    let content = "---\nname: -my-skill-\ndescription: A test skill\n---\n<example>unclosed tag";
    let path = Path::new("SKILL.md");
    let config = LintConfig::default();
    let registry = ValidatorRegistry::with_defaults();
    let validators = registry.validators_for(FileType::Skill);

    let mut all_diags = Vec::new();
    for v in &validators {
        all_diags.extend(v.validate(path, content, &config));
    }

    let all_fix_diags: Vec<Diagnostic> = all_diags
        .iter()
        .filter(|d| d.has_fixes())
        .map(|d| {
            let mut cloned = d.clone();
            cloned.file = path.to_path_buf();
            cloned
        })
        .collect();

    // Apply with safe_only=false (all fixes)
    let mock_fs = Arc::new(MockFileSystem::new());
    mock_fs.add_file(path, content);
    let fs_clone: Arc<dyn FileSystem> = Arc::clone(&mock_fs) as Arc<dyn FileSystem>;
    let results = apply_fixes_with_fs(&all_fix_diags, false, false, Some(fs_clone)).unwrap();
    assert!(!results.is_empty(), "Expected fixes to be applied");

    let fixed_content = mock_fs.read_to_string(path).unwrap();

    // The fixed content should have the closing tag appended (XML-001 fix)
    assert!(
        fixed_content.contains("</example>"),
        "Expected closing tag to be added when safe_only=false. Got:\n{}",
        fixed_content
    );
}

#[test]
fn test_fix_is_insertion_is_deletion_helpers() {
    // Insertion: start == end, non-empty replacement
    let insertion = Fix::insert(5, "hello", "Add word", true);
    assert!(insertion.is_insertion());
    assert!(!insertion.is_deletion());

    // Deletion: empty replacement, start < end
    let deletion = Fix::delete(5, 10, "Remove word", true);
    assert!(deletion.is_deletion());
    assert!(!deletion.is_insertion());

    // Replacement: start != end, non-empty replacement
    let replacement = Fix::replace(5, 10, "world", "Replace word", true);
    assert!(!replacement.is_insertion());
    assert!(!replacement.is_deletion());
}

// ============================================================================
// Fix Ordering Integration Tests
// ============================================================================

#[test]
fn test_ordering_multiple_diagnostics_same_file() {
    // Create 3 diagnostics for the same file with fixes at different positions
    let content = "aaaaa_____bbbbb_____ccccc_____";
    let path = Path::new("/test.md");

    let diags = vec![
        Diagnostic::error(path.to_path_buf(), 1, 1, "TEST-001", "Fix at 20")
            .with_fix(Fix::replace(20, 25, "CCCCC", "Fix third", true)),
        Diagnostic::error(path.to_path_buf(), 1, 1, "TEST-002", "Fix at 10")
            .with_fix(Fix::replace(10, 15, "BBBBB", "Fix second", true)),
        Diagnostic::error(path.to_path_buf(), 1, 1, "TEST-003", "Fix at 0")
            .with_fix(Fix::replace(0, 5, "AAAAA", "Fix first", true)),
    ];

    let mock_fs = Arc::new(MockFileSystem::new());
    mock_fs.add_file("/test.md", content);
    let fs_clone: Arc<dyn FileSystem> = Arc::clone(&mock_fs) as Arc<dyn FileSystem>;
    let results = apply_fixes_with_fs(&diags, false, false, Some(fs_clone)).unwrap();

    assert_eq!(results.len(), 1);
    let fixed = &results[0].fixed;
    assert!(fixed.contains("AAAAA"), "First replacement should be applied");
    assert!(fixed.contains("BBBBB"), "Second replacement should be applied");
    assert!(fixed.contains("CCCCC"), "Third replacement should be applied");
}

#[test]
fn test_ordering_safe_only_integration() {
    let content = "aaaaa_____bbbbb_____ccccc_____";
    let path = Path::new("/test.md");

    let diags = vec![
        // Position 20: safe
        Diagnostic::error(path.to_path_buf(), 1, 1, "TEST-001", "Fix at 20")
            .with_fix(Fix::replace(20, 25, "CCCCC", "Fix third (safe)", true)),
        // Position 10: unsafe
        Diagnostic::error(path.to_path_buf(), 1, 1, "TEST-002", "Fix at 10")
            .with_fix(Fix::replace(10, 15, "BBBBB", "Fix second (unsafe)", false)),
        // Position 0: safe
        Diagnostic::error(path.to_path_buf(), 1, 1, "TEST-003", "Fix at 0")
            .with_fix(Fix::replace(0, 5, "AAAAA", "Fix first (safe)", true)),
    ];

    let mock_fs = Arc::new(MockFileSystem::new());
    mock_fs.add_file("/test.md", content);
    let fs_clone: Arc<dyn FileSystem> = Arc::clone(&mock_fs) as Arc<dyn FileSystem>;
    let results = apply_fixes_with_fs(&diags, false, true, Some(fs_clone)).unwrap();

    assert_eq!(results.len(), 1);
    let fixed = &results[0].fixed;
    // Safe fixes at 0 and 20 should be applied
    assert!(fixed.contains("AAAAA"), "Safe fix at 0 should be applied");
    assert!(fixed.contains("CCCCC"), "Safe fix at 20 should be applied");
    // Unsafe fix at 10 should NOT be applied
    assert!(
        !fixed.contains("BBBBB"),
        "Unsafe fix at 10 should be skipped"
    );
    // Original content at position 10-15 should remain
    assert!(
        fixed.contains("bbbbb"),
        "Original content at 10 should remain"
    );
}
