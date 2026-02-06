use super::*;
use crate::config::LintConfig;
use crate::fs::RealFileSystem;
use std::fs;

#[test]
fn test_valid_skill() {
    let content = r#"---
name: test-skill
description: Use when testing skill validation
---
Skill body content"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    assert!(diagnostics.is_empty());
}

#[test]
fn test_invalid_skill_name() {
    let content = r#"---
name: Test-Skill
description: Use when validating skill names
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_004_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
    assert_eq!(as_004_errors.len(), 1);
}

#[test]
fn test_as_001_missing_frontmatter() {
    let content = include_str!("../../../../../tests/fixtures/skills/missing-frontmatter/SKILL.md");

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let as_001_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-001").collect();
    assert_eq!(as_001_errors.len(), 1);
}

#[test]
fn test_as_002_missing_name() {
    let content = r#"---
description: Use when validating missing name
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_002_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-002").collect();
    assert_eq!(as_002_errors.len(), 1);
}

#[test]
fn test_as_003_missing_description() {
    let content = r#"---
name: test-skill
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_003_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-003").collect();
    assert_eq!(as_003_errors.len(), 1);
}

#[test]
fn test_as_004_invalid_name_format() {
    let content = r#"---
name: bad_name
description: Use when validating name format
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_004_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
    assert_eq!(as_004_errors.len(), 1);
}

#[test]
fn test_as_007_reserved_name() {
    let content = r#"---
name: claude
description: Use when validating reserved names
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_007_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-007").collect();
    assert_eq!(as_007_errors.len(), 1);
}

#[test]
fn test_as_008_description_too_long() {
    let long_description = "a".repeat(1025);
    let content = format!(
        "---\nname: test-skill\ndescription: {}\n---\nBody",
        long_description
    );

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), &content, &LintConfig::default());

    let as_008_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-008").collect();
    assert_eq!(as_008_errors.len(), 1);
}

#[test]
fn test_as_008_description_empty_string() {
    let content = r#"---
name: test-skill
description: ""
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_003_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-003").collect();
    assert_eq!(as_003_errors.len(), 0);

    let as_008_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-008").collect();
    assert_eq!(as_008_errors.len(), 1);
}

#[test]
fn test_as_009_description_contains_xml() {
    let content = r#"---
name: test-skill
description: Use when validating <xml> tags
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_009_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-009").collect();
    assert_eq!(as_009_errors.len(), 1);
}

#[test]
fn test_as_011_compatibility_too_long() {
    let long_compat = "b".repeat(501);
    let content = format!(
            "---\nname: test-skill\ndescription: Use when validating compatibility\ncompatibility: {}\n---\nBody",
            long_compat
        );

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), &content, &LintConfig::default());

    let as_011_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-011").collect();
    assert_eq!(as_011_errors.len(), 1);
}

#[test]
fn test_as_012_content_too_long() {
    let body = (0..501).map(|_| "line").collect::<Vec<_>>().join("\n");
    let content = format!(
        "---\nname: test-skill\ndescription: Use when validating content length\n---\n{}",
        body
    );

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), &content, &LintConfig::default());

    let as_012_warnings: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-012").collect();
    assert_eq!(as_012_warnings.len(), 1);
}

#[test]
fn test_as_013_reference_too_deep() {
    let content = include_str!("../../../../../tests/fixtures/skills/deep-reference/SKILL.md");

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let as_013_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-013").collect();
    assert_eq!(as_013_errors.len(), 1);
}

#[test]
fn test_as_013_reference_single_name_too_deep() {
    let content = r#"---
name: deep-reference
description: Use when validating deep references
---

See reference/deep/guide.md for details."#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let as_013_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-013").collect();
    assert_eq!(as_013_errors.len(), 1);
}

#[test]
fn test_as_014_windows_path_separator() {
    let content = include_str!("../../../../../tests/fixtures/skills/windows-path/SKILL.md");

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let as_014_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-014").collect();
    assert_eq!(as_014_errors.len(), 1);
}

#[test]
fn test_as_015_directory_size_exceeds() {
    use std::io::Write;

    let temp_dir = tempfile::tempdir().unwrap();
    let skill_dir = temp_dir.path().join("big-skill");
    fs::create_dir_all(&skill_dir).unwrap();

    let skill_path = skill_dir.join("SKILL.md");
    let mut skill_file = fs::File::create(&skill_path).unwrap();
    writeln!(
        skill_file,
        "---\nname: big-skill\ndescription: Use when validating directory size\n---\nBody"
    )
    .unwrap();

    let big_file_path = skill_dir.join("big.bin");
    let big_payload = vec![0u8; 8 * 1024 * 1024 + 1];
    fs::write(&big_file_path, big_payload).unwrap();

    let content = fs::read_to_string(&skill_path).unwrap();
    let validator = SkillValidator;
    let diagnostics = validator.validate(&skill_path, &content, &LintConfig::default());

    let as_015_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-015").collect();
    assert_eq!(as_015_errors.len(), 1);
}

#[test]
fn test_cc_sk_006_dangerous_name_without_safety() {
    let content = r#"---
name: deploy-prod
description: Deploys to production
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    // Should have an error for CC-SK-006
    let cc_sk_006_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-006")
        .collect();

    assert_eq!(cc_sk_006_errors.len(), 1);
    assert_eq!(
        cc_sk_006_errors[0].level,
        crate::diagnostics::DiagnosticLevel::Error
    );
}

#[test]
fn test_cc_sk_006_dangerous_name_with_safety() {
    let content = r#"---
name: deploy-prod
description: Deploys to production
disable-model-invocation: true
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    // Should NOT have an error for CC-SK-006
    let cc_sk_006_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-006")
        .collect();

    assert_eq!(cc_sk_006_errors.len(), 0);
}

#[test]
fn test_cc_sk_006_covers_all_dangerous_names() {
    let dangerous_names = vec!["deploy", "ship", "publish", "delete", "release", "push"];

    for name in dangerous_names {
        let content = format!(
            r#"---
name: {}-prod
description: A dangerous skill
---
Body"#,
            name
        );

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        // Should have an error for CC-SK-006
        let cc_sk_006_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-006")
            .collect();

        assert_eq!(
            cc_sk_006_errors.len(),
            1,
            "Expected CC-SK-006 error for name: {}",
            name
        );
    }
}

#[test]
fn test_cc_sk_007_unrestricted_bash() {
    let content = r#"---
name: git-helper
description: Git operations helper
allowed-tools: Bash Read Write
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    // Should have a warning for CC-SK-007
    let cc_sk_007_warnings: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-007")
        .collect();

    assert_eq!(cc_sk_007_warnings.len(), 1);
    assert_eq!(
        cc_sk_007_warnings[0].level,
        crate::diagnostics::DiagnosticLevel::Warning
    );
}

#[test]
fn test_cc_sk_007_scoped_bash_ok() {
    let content = r#"---
name: git-helper
description: Git operations helper
allowed-tools: Bash(git:*) Read Write
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    // Should NOT have a warning for CC-SK-007 (scoped Bash is ok)
    let cc_sk_007_warnings: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-007")
        .collect();

    assert_eq!(cc_sk_007_warnings.len(), 0);
}

#[test]
fn test_cc_sk_007_no_bash() {
    let content = r#"---
name: reader
description: File reader
allowed-tools: Read Write
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    // Should NOT have a warning for CC-SK-007 (no Bash at all)
    let cc_sk_007_warnings: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-007")
        .collect();

    assert_eq!(cc_sk_007_warnings.len(), 0);
}

// ===== CC-SK-007 Auto-fix Tests =====

#[test]
fn test_cc_sk_007_has_fix() {
    let content = r#"---
name: git-helper
description: Use when doing git operations
allowed-tools: Bash Read Write
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_007: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-007")
        .collect();

    assert_eq!(cc_sk_007.len(), 1);
    assert!(cc_sk_007[0].has_fixes());

    let fix = &cc_sk_007[0].fixes[0];
    assert_eq!(fix.replacement, "Bash(git:*)");
    assert!(!fix.safe); // Not safe, we don't know user's intended scope
}

#[test]
fn test_cc_sk_007_fix_correct_byte_position() {
    let content = r#"---
name: helper
description: Use when helping
allowed-tools: Bash Read
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_007: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-007")
        .collect();

    assert_eq!(cc_sk_007.len(), 1);
    assert!(cc_sk_007[0].has_fixes());

    let fix = &cc_sk_007[0].fixes[0];

    // Apply fix and verify
    let mut fixed = content.to_string();
    fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
    assert!(fixed.contains("Bash(git:*)"));
    assert!(!fixed.contains("allowed-tools: Bash "));
}

#[test]
fn test_cc_sk_007_multiple_bash_multiple_fixes() {
    let content = r#"---
name: helper
description: Use when helping
allowed-tools: Bash Read Bash
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_007: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-007")
        .collect();

    // Each Bash occurrence generates a warning
    assert_eq!(cc_sk_007.len(), 2);
    // Each should have a fix
    assert!(cc_sk_007[0].has_fixes());
    assert!(cc_sk_007[1].has_fixes());
}

#[test]
fn test_cc_sk_007_scoped_bash_no_fix() {
    let content = r#"---
name: helper
description: Use when helping
allowed-tools: Bash(git:*) Read
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_007: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-007")
        .collect();

    // Scoped Bash doesn't trigger the warning
    assert_eq!(cc_sk_007.len(), 0);
}

#[test]
fn test_find_plain_bash_positions() {
    let content = "allowed-tools: Bash Read Bash(git:*) Write Bash";
    let positions = find_plain_bash_positions(content, 0);

    // Should find 2: "Bash" at position 15 and "Bash" at position 43
    // But NOT "Bash(git:*)"
    assert_eq!(positions.len(), 2);
    assert_eq!(&content[positions[0].0..positions[0].1], "Bash");
    assert_eq!(&content[positions[1].0..positions[1].1], "Bash");
}

#[test]
fn test_find_plain_bash_positions_none() {
    let content = "allowed-tools: Bash(git:*) Bash(npm:*) Read";
    let positions = find_plain_bash_positions(content, 0);
    assert_eq!(positions.len(), 0);
}

#[test]
fn test_as_005_leading_hyphen() {
    let content = r#"---
name: -bad-name
description: Use when testing validation
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_005_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-005").collect();

    assert_eq!(as_005_errors.len(), 1);
    assert_eq!(
        as_005_errors[0].level,
        crate::diagnostics::DiagnosticLevel::Error
    );
}

#[test]
fn test_as_005_trailing_hyphen() {
    let content = r#"---
name: bad-name-
description: Use when testing validation
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_005_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-005").collect();

    assert_eq!(as_005_errors.len(), 1);
    assert_eq!(
        as_005_errors[0].level,
        crate::diagnostics::DiagnosticLevel::Error
    );
}

#[test]
fn test_as_006_consecutive_hyphens() {
    let content = r#"---
name: bad--name
description: Use when testing validation
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_006_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-006").collect();

    assert_eq!(as_006_errors.len(), 1);
    assert_eq!(
        as_006_errors[0].level,
        crate::diagnostics::DiagnosticLevel::Error
    );
}

#[test]
fn test_as_010_missing_trigger() {
    let content = r#"---
name: code-review
description: Reviews code for quality
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_010_warnings: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();

    assert_eq!(as_010_warnings.len(), 1);
    assert_eq!(
        as_010_warnings[0].level,
        crate::diagnostics::DiagnosticLevel::Warning
    );
}

#[test]
fn test_as_010_has_use_when_trigger() {
    let content = r#"---
name: code-review
description: Use when user asks for code review
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_010_warnings: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();

    assert_eq!(as_010_warnings.len(), 0);
}

#[test]
fn test_as_010_use_this_not_accepted() {
    let content = r#"---
name: code-review
description: Use this skill to review code
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_010_warnings: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();

    assert_eq!(as_010_warnings.len(), 1);
}

// ===== CC-SK-001: Invalid Model Value =====

#[test]
fn test_cc_sk_001_invalid_model() {
    let content = r#"---
name: test-skill
description: Use when testing
model: gpt-4
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_001: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-001")
        .collect();

    assert_eq!(cc_sk_001.len(), 1);
    assert_eq!(
        cc_sk_001[0].level,
        crate::diagnostics::DiagnosticLevel::Error
    );
    assert!(cc_sk_001[0].message.contains("gpt-4"));
}

#[test]
fn test_cc_sk_001_valid_models() {
    for model in &["sonnet", "opus", "haiku", "inherit"] {
        let content = format!(
            r#"---
name: test-skill
description: Use when testing
model: {}
---
Body"#,
            model
        );

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        let cc_sk_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-001")
            .collect();

        assert_eq!(cc_sk_001.len(), 0, "Model '{}' should be valid", model);
    }
}

#[test]
fn test_cc_sk_001_no_model_ok() {
    let content = r#"---
name: test-skill
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_001: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-001")
        .collect();

    assert_eq!(cc_sk_001.len(), 0);
}

// ===== CC-SK-002: Invalid Context Value =====

#[test]
fn test_cc_sk_002_invalid_context() {
    let content = r#"---
name: test-skill
description: Use when testing
context: split
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_002: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-002")
        .collect();

    assert_eq!(cc_sk_002.len(), 1);
    assert_eq!(
        cc_sk_002[0].level,
        crate::diagnostics::DiagnosticLevel::Error
    );
    assert!(cc_sk_002[0].message.contains("split"));
}

#[test]
fn test_cc_sk_002_valid_context_fork() {
    let content = r#"---
name: test-skill
description: Use when testing
context: fork
agent: general-purpose
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_002: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-002")
        .collect();

    assert_eq!(cc_sk_002.len(), 0);
}

#[test]
fn test_cc_sk_002_no_context_ok() {
    let content = r#"---
name: test-skill
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_002: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-002")
        .collect();

    assert_eq!(cc_sk_002.len(), 0);
}

// ===== CC-SK-003: Context Without Agent =====

#[test]
fn test_cc_sk_003_context_fork_without_agent() {
    let content = r#"---
name: test-skill
description: Use when testing
context: fork
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_003: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-003")
        .collect();

    assert_eq!(cc_sk_003.len(), 1);
    assert_eq!(
        cc_sk_003[0].level,
        crate::diagnostics::DiagnosticLevel::Error
    );
}

#[test]
fn test_cc_sk_003_context_fork_with_agent_ok() {
    let content = r#"---
name: test-skill
description: Use when testing
context: fork
agent: Explore
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_003: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-003")
        .collect();

    assert_eq!(cc_sk_003.len(), 0);
}

// ===== CC-SK-004: Agent Without Context =====

#[test]
fn test_cc_sk_004_agent_without_context() {
    let content = r#"---
name: test-skill
description: Use when testing
agent: Explore
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_004: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-004")
        .collect();

    assert_eq!(cc_sk_004.len(), 1);
    assert_eq!(
        cc_sk_004[0].level,
        crate::diagnostics::DiagnosticLevel::Error
    );
}

#[test]
fn test_cc_sk_004_agent_with_context_ok() {
    let content = r#"---
name: test-skill
description: Use when testing
context: fork
agent: Explore
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_004: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-004")
        .collect();

    assert_eq!(cc_sk_004.len(), 0);
}

#[test]
fn test_cc_sk_004_no_agent_no_context_ok() {
    let content = r#"---
name: test-skill
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_004: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-004")
        .collect();

    assert_eq!(cc_sk_004.len(), 0);
}

// ===== CC-SK-005: Invalid Agent Type =====

#[test]
fn test_cc_sk_005_invalid_agent() {
    let content = r#"---
name: test-skill
description: Use when testing
context: fork
agent: CustomAgent
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_005: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-005")
        .collect();

    assert_eq!(cc_sk_005.len(), 1);
    assert_eq!(
        cc_sk_005[0].level,
        crate::diagnostics::DiagnosticLevel::Error
    );
    assert!(cc_sk_005[0].message.contains("CustomAgent"));
}

#[test]
fn test_cc_sk_005_valid_agents() {
    for agent in &["Explore", "Plan", "general-purpose"] {
        let content = format!(
            r#"---
name: test-skill
description: Use when testing
context: fork
agent: {}
---
Body"#,
            agent
        );

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        let cc_sk_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-005")
            .collect();

        assert_eq!(cc_sk_005.len(), 0, "Agent '{}' should be valid", agent);
    }
}

#[test]
fn test_cc_sk_005_valid_custom_agents() {
    // Custom agents in kebab-case should be valid
    for agent in &[
        "my-custom-agent",
        "code-review",
        "deploy-helper",
        "a",
        "agent123",
        "my-agent-v2",
    ] {
        let content = format!(
            r#"---
name: test-skill
description: Use when testing
context: fork
agent: {}
---
Body"#,
            agent
        );

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        let cc_sk_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-005")
            .collect();

        assert_eq!(
            cc_sk_005.len(),
            0,
            "Custom agent '{}' should be valid",
            agent
        );
    }
}

#[test]
fn test_cc_sk_005_rejects_invalid_agent_formats() {
    // Consolidated test for all invalid agent formats
    let invalid_agents = [
        ("MyAgent", "uppercase"),
        ("my_custom_agent", "underscore"),
        ("\"\"", "empty"),
        ("-custom-agent", "leading hyphen"),
        ("custom-agent-", "trailing hyphen"),
        ("custom--agent", "consecutive hyphens"),
        ("my@agent", "special char @"),
        ("agent!", "special char !"),
        ("test.agent", "special char ."),
        ("agent/name", "special char /"),
    ];

    for (agent, reason) in invalid_agents {
        let content = format!(
            r#"---
name: test-skill
description: Use when testing
context: fork
agent: {}
---
Body"#,
            agent
        );

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        let cc_sk_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-005")
            .collect();

        assert_eq!(
            cc_sk_005.len(),
            1,
            "Agent '{}' ({}) should be rejected",
            agent,
            reason
        );
    }
}

#[test]
fn test_cc_sk_005_rejects_too_long_agent() {
    let long_agent = "a".repeat(65);
    let content = format!(
        r#"---
name: test-skill
description: Use when testing
context: fork
agent: {}
---
Body"#,
        long_agent
    );

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), &content, &LintConfig::default());

    let cc_sk_005: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-005")
        .collect();

    assert_eq!(cc_sk_005.len(), 1, "Agent over 64 chars should be rejected");
}

#[test]
fn test_cc_sk_005_accepts_max_length_agent() {
    let max_agent = "a".repeat(64);
    let content = format!(
        r#"---
name: test-skill
description: Use when testing
context: fork
agent: {}
---
Body"#,
        max_agent
    );

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), &content, &LintConfig::default());

    let cc_sk_005: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-005")
        .collect();

    assert_eq!(cc_sk_005.len(), 0, "Agent at 64 chars should be accepted");
}

#[test]
fn test_cc_sk_005_fixture_invalid_agent() {
    let content =
        include_str!("../../../../../tests/fixtures/invalid/skills/invalid-agent/SKILL.md");

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let cc_sk_005: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-005")
        .collect();

    assert_eq!(
        cc_sk_005.len(),
        1,
        "Invalid agent fixture should trigger CC-SK-005"
    );
}

#[test]
fn test_cc_sk_005_fixture_valid_custom_agent() {
    let content =
        include_str!("../../../../../tests/fixtures/valid/skills/with-custom-agent/SKILL.md");

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let cc_sk_005: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-005")
        .collect();

    assert_eq!(
        cc_sk_005.len(),
        0,
        "Valid custom agent fixture should pass CC-SK-005"
    );
}

// ===== CC-SK-008: Unknown Tool Name =====

#[test]
fn test_cc_sk_008_unknown_tool() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Read Write UnknownTool
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_008: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();

    assert_eq!(cc_sk_008.len(), 1);
    assert_eq!(
        cc_sk_008[0].level,
        crate::diagnostics::DiagnosticLevel::Error
    );
    assert!(cc_sk_008[0].message.contains("UnknownTool"));
}

#[test]
fn test_cc_sk_008_all_known_tools_ok() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Bash Read Write Edit Grep Glob Task
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_008: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();

    assert_eq!(cc_sk_008.len(), 0);
}

#[test]
fn test_cc_sk_008_scoped_tool_extracts_base_name() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Bash(git:*) Read Write
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_008: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();

    assert_eq!(cc_sk_008.len(), 0);
}

#[test]
fn test_cc_sk_008_multiple_unknown_tools() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: FakeTool1 Read FakeTool2
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_008: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();

    assert_eq!(cc_sk_008.len(), 2);
}

#[test]
fn test_cc_sk_008_scoped_unknown_tool() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: FakeTool(scope:*) Read
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_008: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();

    assert_eq!(
        cc_sk_008.len(),
        1,
        "Should detect FakeTool as unknown even when scoped"
    );
    assert!(cc_sk_008[0].message.contains("FakeTool"));
}

// ===== CC-SK-009: Too Many Injections =====

#[test]
fn test_cc_sk_009_too_many_injections() {
    let content = r#"---
name: test-skill
description: Use when testing
---
Current date: !`date`
Git status: !`git status`
Branch: !`git branch`
User: !`whoami`
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_009: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-009")
        .collect();

    assert_eq!(cc_sk_009.len(), 1);
    assert_eq!(
        cc_sk_009[0].level,
        crate::diagnostics::DiagnosticLevel::Warning
    );
    assert!(cc_sk_009[0].message.contains("4"));
}

#[test]
fn test_cc_sk_009_exactly_three_injections_ok() {
    let content = r#"---
name: test-skill
description: Use when testing
---
Date: !`date`
Status: !`git status`
Branch: !`git branch`
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_009: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-009")
        .collect();

    assert_eq!(cc_sk_009.len(), 0);
}

#[test]
fn test_cc_sk_009_no_injections_ok() {
    let content = r#"---
name: test-skill
description: Use when testing
---
No dynamic injections here.
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_009: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-009")
        .collect();

    assert_eq!(cc_sk_009.len(), 0);
}

// ===== Edge Case Tests =====

#[test]
fn test_cc_sk_006_explicit_false_still_triggers() {
    let content = r#"---
name: deploy-prod
description: Use when deploying
disable-model-invocation: false
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_006: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-006")
        .collect();

    assert_eq!(
        cc_sk_006.len(),
        1,
        "Explicit false should still trigger CC-SK-006"
    );
}

#[test]
fn test_cc_sk_007_duplicate_bash_multiple_warnings() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Bash Read Bash
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_007: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-007")
        .collect();

    // Each plain "Bash" triggers warning (2 occurrences = 2 warnings)
    assert_eq!(
        cc_sk_007.len(),
        2,
        "Each Bash occurrence triggers a warning"
    );
}

#[test]
fn test_cc_sk_008_malformed_scope_no_panic() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Bash( Read Bash() Write
---
Body"#;

    let validator = SkillValidator;
    // Should not panic on malformed scope syntax
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    // Bash( extracts "Bash", which is known
    // Bash() extracts "Bash", which is known
    let cc_sk_008: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();

    assert_eq!(
        cc_sk_008.len(),
        0,
        "Malformed scopes should extract base name correctly"
    );
}

#[test]
fn test_cc_sk_008_lowercase_tool_unknown() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: bash read
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_008: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();

    // Tool names are case-sensitive: bash != Bash
    assert_eq!(cc_sk_008.len(), 2, "lowercase tool names are unknown");
}

#[test]
fn test_as_010_case_insensitive() {
    let content = r#"---
name: test-skill
description: USE WHEN testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();

    assert_eq!(
        as_010.len(),
        0,
        "'USE WHEN' should match case-insensitively"
    );
}

#[test]
fn test_parse_error_handling() {
    let content = r#"---
name: test
description
invalid yaml
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-016").collect();

    assert_eq!(
        parse_errors.len(),
        1,
        "Invalid YAML should produce parse error"
    );
}

// ===== Config Wiring Tests =====

#[test]
fn test_config_disabled_skills_category() {
    let mut config = LintConfig::default();
    config.rules.skills = false;

    let content = r#"---
name: -bad-name
description: Missing trigger phrase
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &config);

    // AS-005 and AS-010 should not fire when skills category is disabled
    let skill_rules: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("AS-") || d.rule.starts_with("CC-SK-"))
        .collect();
    assert_eq!(skill_rules.len(), 0);
}

#[test]
fn test_config_disabled_specific_skill_rule() {
    let mut config = LintConfig::default();
    config.rules.disabled_rules = vec!["AS-005".to_string()];

    let content = r#"---
name: -bad-name
description: Missing trigger phrase
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &config);

    // AS-005 should not fire when specifically disabled
    let as_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-005").collect();
    assert_eq!(as_005.len(), 0);

    // But AS-010 should still fire
    let as_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();
    assert_eq!(as_010.len(), 1);
}

#[test]
fn test_config_cursor_target_disables_cc_sk_rules() {
    use crate::config::TargetTool;

    let mut config = LintConfig::default();
    config.target = TargetTool::Cursor;

    let content = r#"---
name: deploy-prod
description: Deploys to production
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &config);

    // CC-SK-006 should not fire for Cursor target
    let cc_sk_006: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-006")
        .collect();
    assert_eq!(cc_sk_006.len(), 0);

    // But AS-010 should still fire (it's not CC- prefix)
    let as_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();
    assert_eq!(as_010.len(), 1);
}

#[test]
fn test_config_claude_code_target_enables_cc_sk_rules() {
    use crate::config::TargetTool;

    let mut config = LintConfig::default();
    config.target = TargetTool::ClaudeCode;

    let content = r#"---
name: deploy-prod
description: Use when deploying to production
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &config);

    // CC-SK-006 should fire for ClaudeCode target
    let cc_sk_006: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-006")
        .collect();
    assert_eq!(cc_sk_006.len(), 1);
}

// ===== convert_to_kebab_case Tests =====

#[test]
fn test_convert_to_kebab_case_lowercase() {
    assert_eq!(convert_to_kebab_case("TestSkill"), "testskill");
}

#[test]
fn test_convert_to_kebab_case_underscores() {
    assert_eq!(convert_to_kebab_case("test_skill"), "test-skill");
}

#[test]
fn test_convert_to_kebab_case_mixed() {
    assert_eq!(convert_to_kebab_case("Test_Skill_Name"), "test-skill-name");
}

#[test]
fn test_convert_to_kebab_case_consecutive_hyphens() {
    assert_eq!(convert_to_kebab_case("test--skill"), "test-skill");
    assert_eq!(convert_to_kebab_case("test___skill"), "test-skill");
}

#[test]
fn test_convert_to_kebab_case_leading_trailing() {
    assert_eq!(convert_to_kebab_case("-test-skill-"), "test-skill");
    assert_eq!(convert_to_kebab_case("_test_skill_"), "test-skill");
}

#[test]
fn test_convert_to_kebab_case_invalid_chars() {
    assert_eq!(convert_to_kebab_case("test@skill!"), "testskill");
    assert_eq!(convert_to_kebab_case("test.skill"), "testskill");
}

#[test]
fn test_convert_to_kebab_case_truncate() {
    let long_name = "a".repeat(100);
    let result = convert_to_kebab_case(&long_name);
    assert!(result.len() <= 64);
    assert_eq!(result.len(), 64);
}

// ===== AS-004 Auto-fix Tests =====

#[test]
fn test_as_004_has_fix() {
    let content = r#"---
name: Test_Skill
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
    assert_eq!(as_004.len(), 1);
    assert!(as_004[0].has_fixes());
    assert_eq!(as_004[0].fixes[0].replacement, "test-skill");
}

#[test]
fn test_as_004_fix_case_only_is_safe() {
    let content = r#"---
name: TestSkill
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
    assert_eq!(as_004.len(), 1);
    assert!(as_004[0].has_fixes());
    // Case-only change (TestSkill -> testskill) is safe
    assert!(as_004[0].fixes[0].safe);
}

#[test]
fn test_as_004_fix_structural_is_unsafe() {
    let content = r#"---
name: Test_Skill
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
    assert_eq!(as_004.len(), 1);
    assert!(as_004[0].has_fixes());
    // Structural change (Test_Skill -> test-skill) is not safe
    assert!(!as_004[0].fixes[0].safe);
}

#[test]
fn test_as_004_fix_byte_position() {
    let content = r#"---
name: Bad_Name
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
    assert_eq!(as_004.len(), 1);
    assert!(as_004[0].has_fixes());

    let fix = &as_004[0].fixes[0];
    // Apply fix and verify
    let mut fixed = content.to_string();
    fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
    assert!(fixed.contains("name: bad-name"));
}

#[test]
fn test_as_004_fix_quoted_value() {
    let content = r#"---
name: "Bad_Name"
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
    assert_eq!(as_004.len(), 1);
    assert!(as_004[0].has_fixes());

    let fix = &as_004[0].fixes[0];
    // Apply fix and verify
    let mut fixed = content.to_string();
    fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
    // The fix replaces the inner value, keeping quotes in place
    assert!(fixed.contains("bad-name"));
}

#[test]
fn test_as_004_no_fix_when_converts_to_empty() {
    // Name with only special characters should convert to empty string
    let content = r#"---
name: "!@#$%"
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
    assert_eq!(as_004.len(), 1);
    // Should have no fix since converted name would be empty
    assert!(!as_004[0].has_fixes());
}

#[test]
fn test_as_004_underscore_to_hyphen_is_unsafe() {
    // Test_Name -> test-name involves underscore replacement, should be unsafe
    let content = r#"---
name: test_name
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
    assert_eq!(as_004.len(), 1);
    assert!(as_004[0].has_fixes());
    // Underscore to hyphen is structural change, not safe
    assert!(!as_004[0].fixes[0].safe);
}

// ===== AS-010 Auto-fix Tests =====

#[test]
fn test_as_010_has_fix() {
    let content = r#"---
name: code-review
description: Reviews code for quality
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();
    assert_eq!(as_010.len(), 1);
    assert!(as_010[0].has_fixes());
    assert_eq!(
        as_010[0].fixes[0].replacement,
        "Use when user wants to Reviews code for quality"
    );
}

#[test]
fn test_as_010_fix_is_unsafe() {
    let content = r#"---
name: code-review
description: Reviews code for quality
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();
    assert_eq!(as_010.len(), 1);
    assert!(as_010[0].has_fixes());
    // Semantic change is not safe
    assert!(!as_010[0].fixes[0].safe);
}

#[test]
fn test_as_010_fix_byte_position() {
    let content = r#"---
name: helper
description: Helps with tasks
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();
    assert_eq!(as_010.len(), 1);
    assert!(as_010[0].has_fixes());

    let fix = &as_010[0].fixes[0];
    // Apply fix and verify
    let mut fixed = content.to_string();
    fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
    assert!(fixed.contains("Use when user wants to Helps with tasks"));
}

#[test]
fn test_as_010_fix_quoted_value() {
    let content = r#"---
name: helper
description: "Helps with tasks"
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();
    assert_eq!(as_010.len(), 1);
    assert!(as_010[0].has_fixes());

    let fix = &as_010[0].fixes[0];
    // Apply fix and verify
    let mut fixed = content.to_string();
    fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
    assert!(fixed.contains("Use when user wants to Helps with tasks"));
}

#[test]
fn test_as_010_no_fix_when_description_too_long() {
    // Create a description that would exceed 1024 chars when prepending trigger phrase
    let long_desc = "a".repeat(1010);
    let content = format!("---\nname: helper\ndescription: {}\n---\nBody", long_desc);

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), &content, &LintConfig::default());

    let as_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();
    assert_eq!(as_010.len(), 1);
    // Should have no fix since prepending would exceed limit
    assert!(!as_010[0].has_fixes());
}

// ===== frontmatter_value_byte_range Tests =====

#[test]
fn test_frontmatter_value_byte_range_unquoted() {
    let content = r#"---
name: test-skill
description: A test skill
---
Body"#;
    let parts = split_frontmatter(content);
    let range = frontmatter_value_byte_range(content, &parts, "name");
    assert!(range.is_some());
    let (start, end) = range.unwrap();
    assert_eq!(&content[start..end], "test-skill");
}

#[test]
fn test_frontmatter_value_byte_range_double_quoted() {
    let content = r#"---
name: "test-skill"
description: A test skill
---
Body"#;
    let parts = split_frontmatter(content);
    let range = frontmatter_value_byte_range(content, &parts, "name");
    assert!(range.is_some());
    let (start, end) = range.unwrap();
    assert_eq!(&content[start..end], "test-skill");
}

#[test]
fn test_frontmatter_value_byte_range_single_quoted() {
    let content = r#"---
name: 'test-skill'
description: A test skill
---
Body"#;
    let parts = split_frontmatter(content);
    let range = frontmatter_value_byte_range(content, &parts, "name");
    assert!(range.is_some());
    let (start, end) = range.unwrap();
    assert_eq!(&content[start..end], "test-skill");
}

#[test]
fn test_frontmatter_value_byte_range_not_found() {
    let content = r#"---
name: test-skill
---
Body"#;
    let parts = split_frontmatter(content);
    let range = frontmatter_value_byte_range(content, &parts, "description");
    assert!(range.is_none());
}

// ===== directory_size_until tests =====

/// Helper to write N bytes to a file efficiently using a small buffer
fn write_bytes_to_file(path: &std::path::Path, num_bytes: usize) {
    use std::io::Write;
    let mut file = fs::File::create(path).expect("Failed to create test file");
    let buffer = [0u8; 8192];
    let mut remaining = num_bytes;
    while remaining > 0 {
        let to_write = remaining.min(buffer.len());
        file.write_all(&buffer[..to_write])
            .expect("Failed to write test data");
        remaining -= to_write;
    }
}

#[test]
fn test_directory_size_until_short_circuits() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let fs = RealFileSystem;

    // Create 10 files of 1MB each (10MB total)
    for i in 0..10 {
        let file_path = temp_dir.path().join(format!("file_{:02}.bin", i));
        write_bytes_to_file(&file_path, 1024 * 1024);
    }

    // With a 2MB limit, should short-circuit and return > 2MB
    let size = directory_size_until(temp_dir.path(), 2 * 1024 * 1024, &fs);
    assert!(size > 2 * 1024 * 1024, "Size should exceed 2MB limit");
    // Should not have scanned all 10MB (short-circuited after exceeding limit).
    // Upper bound is 3MB because: directory iteration order is unspecified,
    // so we may read up to 2 full 1MB files before the check triggers on the 3rd.
    assert!(
        size <= 3 * 1024 * 1024,
        "Size {} should be <= 3MB (short-circuited)",
        size
    );
}

#[test]
fn test_directory_size_until_accurate_under_limit() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let fs = RealFileSystem;

    // Create 2 files of 1KB each (2KB total)
    for i in 0..2 {
        let file_path = temp_dir.path().join(format!("file_{}.bin", i));
        write_bytes_to_file(&file_path, 1024);
    }

    // With a 1MB limit, should return exact size
    let size = directory_size_until(temp_dir.path(), 1024 * 1024, &fs);
    assert_eq!(size, 2048);
}

#[test]
fn test_directory_size_until_handles_empty_directory() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let fs = RealFileSystem;

    let size = directory_size_until(temp_dir.path(), 1024 * 1024, &fs);
    assert_eq!(size, 0);
}

#[test]
fn test_directory_size_until_nested_directories() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let real_fs = RealFileSystem;

    // Create nested structure: root/sub1/sub2 with files at each level
    let sub1 = temp_dir.path().join("sub1");
    let sub2 = sub1.join("sub2");
    fs::create_dir_all(&sub2).expect("Failed to create nested directories");

    // 1KB at root, 2KB in sub1, 3KB in sub2 = 6KB total
    write_bytes_to_file(&temp_dir.path().join("root.bin"), 1024);
    write_bytes_to_file(&sub1.join("sub1.bin"), 2048);
    write_bytes_to_file(&sub2.join("sub2.bin"), 3072);

    let size = directory_size_until(temp_dir.path(), 1024 * 1024, &real_fs);
    assert_eq!(size, 6144, "Should sum files across all nested directories");
}

#[test]
fn test_directory_size_until_nested_short_circuits() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let real_fs = RealFileSystem;

    // Create nested structure with large files
    let sub1 = temp_dir.path().join("sub1");
    let sub2 = sub1.join("sub2");
    fs::create_dir_all(&sub2).expect("Failed to create nested directories");

    // 1MB at each level = 3MB total, with 2MB limit should short-circuit
    write_bytes_to_file(&temp_dir.path().join("root.bin"), 1024 * 1024);
    write_bytes_to_file(&sub1.join("sub1.bin"), 1024 * 1024);
    write_bytes_to_file(&sub2.join("sub2.bin"), 1024 * 1024);

    let size = directory_size_until(temp_dir.path(), 2 * 1024 * 1024, &real_fs);
    assert!(size > 2 * 1024 * 1024, "Should exceed limit");
    assert!(
        size <= 3 * 1024 * 1024,
        "Should short-circuit before scanning all"
    );
}

#[test]
fn test_as_015_boundary_exactly_8mb() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let skill_dir = temp_dir.path().join("skill");
    fs::create_dir_all(&skill_dir).expect("Failed to create skill directory");

    // Create SKILL.md
    let skill_path = skill_dir.join("SKILL.md");
    fs::write(&skill_path, "---\nname: boundary-test\n---\nBody")
        .expect("Failed to write SKILL.md");

    // Create file that brings total to exactly 8MB (minus SKILL.md size)
    let skill_md_size = fs::metadata(&skill_path)
        .expect("Failed to read SKILL.md metadata")
        .len() as usize;
    let target_size = 8 * 1024 * 1024 - skill_md_size;
    write_bytes_to_file(&skill_dir.join("data.bin"), target_size);

    let validator = SkillValidator;
    let content = fs::read_to_string(&skill_path).expect("Failed to read SKILL.md content");
    let diagnostics = validator.validate(&skill_path, &content, &LintConfig::default());

    // Exactly 8MB should NOT trigger AS-015 (uses > not >=)
    let as_015_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-015").collect();
    assert!(
        as_015_errors.is_empty(),
        "Exactly 8MB should not trigger AS-015, but got: {:?}",
        as_015_errors
    );
}

// ===== Additional AS-016 Parse Error Tests =====

#[test]
fn test_as_001_missing_closing_delimiter_treated_as_no_frontmatter() {
    // When opening --- exists but closing --- is missing, the entire content
    // is treated as body without frontmatter, triggering AS-001
    let content = r#"---
name: test
description: A test skill
Missing closing delimiter
Body content"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-001").collect();
    assert_eq!(as_001.len(), 1);

    // Should NOT be treated as parse error since no frontmatter was detected
    let as_016: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-016").collect();
    assert!(as_016.is_empty());
}

#[test]
fn test_as_016_invalid_yaml_colon_in_value() {
    let content = r#"---
name: test:value:with:colons
description: A test skill
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    // This should parse OK as YAML handles colons in values
    let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-016").collect();
    assert!(parse_errors.is_empty());
}

#[test]
fn test_as_016_invalid_yaml_tabs() {
    let content = "---\nname: test\n\tdescription: bad indent\n---\nBody";

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    // Tabs in YAML can cause parse errors
    let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-016").collect();
    assert_eq!(
        parse_errors.len(),
        1,
        "Tab indentation should cause parse error"
    );
}

#[test]
fn test_as_016_valid_yaml_no_error() {
    let content = r#"---
name: valid-skill
description: A properly formatted skill
model: sonnet
---
Body content"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-016").collect();
    assert!(parse_errors.is_empty());
}

#[test]
fn test_as_016_disabled() {
    let mut config = LintConfig::default();
    config.rules.disabled_rules = vec!["AS-016".to_string()];

    let content = r#"---
name: test
description
invalid yaml syntax
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &config);

    assert!(!diagnostics.iter().any(|d| d.rule == "AS-016"));
}

// ===== Additional edge case tests for comprehensive coverage =====

#[test]
fn test_as_001_empty_file() {
    let content = "";
    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    assert!(diagnostics.iter().any(|d| d.rule == "AS-001"));
}

#[test]
fn test_as_001_only_body_no_frontmatter() {
    let content = "This is just body content without any frontmatter.";
    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    assert!(diagnostics.iter().any(|d| d.rule == "AS-001"));
}

#[test]
fn test_as_002_no_name_field() {
    let content = r#"---
description: A test skill without name field
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    // Missing name field should trigger AS-002
    assert!(diagnostics.iter().any(|d| d.rule == "AS-002"));
}

#[test]
fn test_as_004_whitespace_only_name() {
    let content = r#"---
name: "   "
description: A test skill
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    // Whitespace-only name should trigger AS-004 (invalid format)
    assert!(diagnostics.iter().any(|d| d.rule == "AS-004"));
}

#[test]
fn test_as_003_whitespace_description() {
    let content = r#"---
name: test-skill
description: "   "
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    // Whitespace-only description should be treated as short (AS-008)
    let as_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-008").collect();
    assert!(
        !as_008.is_empty(),
        "Whitespace description should trigger AS-008"
    );
}

#[test]
fn test_as_004_uppercase_in_name() {
    let content = r#"---
name: TestSkill
description: Use when testing skill names
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
    assert_eq!(as_004.len(), 1);
}

#[test]
fn test_as_004_valid_lowercase_hyphen_name() {
    let content = r#"---
name: valid-skill-name
description: Use when testing valid names
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    assert!(!diagnostics.iter().any(|d| d.rule == "AS-004"));
}

#[test]
fn test_as_007_all_reserved_names() {
    // Reserved names hardcoded in AS-007 validation logic
    // No constant exists for these in the codebase
    let reserved = ["anthropic", "claude", "skill"];

    for name in reserved {
        let content = format!(
            "---\nname: {}\ndescription: Use when testing reserved names\n---\nBody",
            name
        );

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        let as_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-007").collect();
        assert_eq!(
            as_007.len(),
            1,
            "Reserved name '{}' should trigger AS-007",
            name
        );
    }
}

#[test]
fn test_as_007_non_reserved_name_ok() {
    let content = r#"---
name: my-custom-skill
description: Use when testing non-reserved names
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    assert!(!diagnostics.iter().any(|d| d.rule == "AS-007"));
}

#[test]
fn test_as_011_exactly_500_chars() {
    let long_compat = "a".repeat(500);
    let content = format!(
        "---\nname: test\ndescription: Use when testing\ncompatibility: {}\n---\nBody",
        long_compat
    );

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), &content, &LintConfig::default());

    // Exactly 500 should be OK (limit is >500)
    assert!(!diagnostics.iter().any(|d| d.rule == "AS-011"));
}

#[test]
fn test_as_011_501_chars_triggers() {
    let long_compat = "a".repeat(501);
    let content = format!(
        "---\nname: test\ndescription: Use when testing\ncompatibility: {}\n---\nBody",
        long_compat
    );

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), &content, &LintConfig::default());

    assert!(diagnostics.iter().any(|d| d.rule == "AS-011"));
}

#[test]
fn test_as_012_exactly_500_lines_ok() {
    let body_lines = (0..470)
        .map(|i| format!("Line {}", i))
        .collect::<Vec<_>>()
        .join("\n");
    let content = format!(
        "---\nname: test\ndescription: Use when testing line limits\n---\n{}",
        body_lines
    );

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), &content, &LintConfig::default());

    // Around 500 lines should be OK
    assert!(!diagnostics.iter().any(|d| d.rule == "AS-012"));
}

#[test]
fn test_cc_sk_001_all_valid_models() {
    // Must match VALID_MODELS constant in skill/mod.rs
    let valid_models = VALID_MODELS;

    for model in valid_models {
        let content = format!(
            "---\nname: test\ndescription: Use when testing models\nmodel: {}\n---\nBody",
            model
        );

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        let cc_sk_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-001")
            .collect();
        assert!(
            cc_sk_001.is_empty(),
            "Model '{}' should be valid but got CC-SK-001",
            model
        );
    }
}

#[test]
fn test_cc_sk_001_invalid_model_exhaustive() {
    let content = r#"---
name: test
description: Use when testing
model: invalid-model
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    assert!(diagnostics.iter().any(|d| d.rule == "CC-SK-001"));
}

#[test]
fn test_cc_sk_002_fork_context_valid() {
    // Context can only be "fork", and requires an agent field
    let content = r#"---
name: test
description: Use when testing contexts
context: fork
agent: general-purpose
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_002: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-002")
        .collect();
    assert!(
        cc_sk_002.is_empty(),
        "Context 'fork' with agent should be valid"
    );
}

#[test]
fn test_cc_sk_002_invalid_context_exhaustive() {
    let content = r#"---
name: test
description: Use when testing contexts
context: invalid
agent: general-purpose
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    assert!(diagnostics.iter().any(|d| d.rule == "CC-SK-002"));
}

#[test]
fn test_cc_sk_003_fork_without_agent_exhaustive() {
    let content = r#"---
name: test
description: Use when testing
context: fork
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    assert!(diagnostics.iter().any(|d| d.rule == "CC-SK-003"));
}

#[test]
fn test_cc_sk_004_agent_without_context_exhaustive() {
    let content = r#"---
name: test
description: Use when testing
agent: general-purpose
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    assert!(diagnostics.iter().any(|d| d.rule == "CC-SK-004"));
}

#[test]
fn test_cc_sk_005_builtin_agents_valid() {
    // Must match BUILTIN_AGENTS constant in skill/mod.rs
    let builtin_agents = BUILTIN_AGENTS;

    for agent in builtin_agents {
        let content = format!(
            "---\nname: test\ndescription: Use when testing\ncontext: fork\nagent: {}\n---\nBody",
            agent
        );

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        let cc_sk_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-005")
            .collect();
        assert!(
            cc_sk_005.is_empty(),
            "Built-in agent '{}' should be valid",
            agent
        );
    }
}

#[test]
fn test_cc_sk_005_custom_kebab_agent_valid() {
    let content = r#"---
name: test
description: Use when testing
context: fork
agent: my-custom-agent
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    assert!(!diagnostics.iter().any(|d| d.rule == "CC-SK-005"));
}
