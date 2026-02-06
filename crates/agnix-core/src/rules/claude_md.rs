//! CLAUDE.md validation

use crate::{
    config::LintConfig,
    diagnostics::{Diagnostic, Fix},
    file_utils::safe_read_file,
    rules::Validator,
    schemas::claude_md::{
        check_readme_duplication, check_token_count, extract_npm_scripts, find_critical_in_middle,
        find_generic_instructions, find_negative_without_positive, find_weak_constraints,
    },
};
use rust_i18n::t;
use std::path::Path;

pub struct ClaudeMdValidator;

impl Validator for ClaudeMdValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Validate CLAUDE.md variants and cursor rule files.
        // Skip AGENTS.* files - CC-MEM rules are Claude-specific.
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let is_claude_md = matches!(filename, "CLAUDE.md" | "CLAUDE.local.md");
        let is_cursor_rules = filename == ".cursorrules"
            || filename == ".cursorrules.md"
            || filename.ends_with(".mdc");
        if !is_claude_md && !is_cursor_rules {
            return diagnostics;
        }

        // CC-MEM-005: Generic instructions detection
        // Also check legacy config flag for backward compatibility
        if config.is_rule_enabled("CC-MEM-005") && config.rules.generic_instructions {
            let generic_insts = find_generic_instructions(content);
            for inst in generic_insts {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        inst.line,
                        inst.column,
                        "CC-MEM-005",
                        t!("rules.cc_mem_005.message", text = inst.text.as_str()),
                    )
                    .with_suggestion(t!("rules.cc_mem_005.suggestion"))
                    .with_fix(Fix::delete(
                        inst.start_byte,
                        inst.end_byte,
                        t!("rules.cc_mem_005.fix"),
                        true,
                    )),
                );
            }
        }

        // CC-MEM-009: Token count exceeded
        #[allow(clippy::collapsible_if)]
        if config.is_rule_enabled("CC-MEM-009") {
            if let Some(exceeded) = check_token_count(content) {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-MEM-009",
                        t!(
                            "rules.cc_mem_009.message",
                            tokens = exceeded.estimated_tokens,
                            limit = exceeded.limit
                        ),
                    )
                    .with_suggestion(t!("rules.cc_mem_009.suggestion")),
                );
            }
        }

        // CC-MEM-006: Negative without positive
        if config.is_rule_enabled("CC-MEM-006") {
            let negatives = find_negative_without_positive(content);
            for neg in negatives {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        neg.line,
                        neg.column,
                        "CC-MEM-006",
                        t!("rules.cc_mem_006.message", text = neg.text.as_str()),
                    )
                    .with_suggestion(t!("rules.cc_mem_006.suggestion")),
                );
            }
        }

        // CC-MEM-007: Weak constraint language in critical sections
        if config.is_rule_enabled("CC-MEM-007") {
            let weak = find_weak_constraints(content);
            for w in weak {
                // Determine the replacement for weak language
                let (replacement, safe) = get_weak_constraint_replacement(&w.text);
                let mut diagnostic = Diagnostic::warning(
                    path.to_path_buf(),
                    w.line,
                    w.column,
                    "CC-MEM-007",
                    t!(
                        "rules.cc_mem_007.message",
                        text = w.text.as_str(),
                        section = w.section.as_str()
                    ),
                )
                .with_suggestion(t!("rules.cc_mem_007.suggestion"));

                // Add fix if we have a replacement
                if let Some(repl) = replacement {
                    diagnostic = diagnostic.with_fix(Fix::replace(
                        w.start_byte,
                        w.end_byte,
                        repl,
                        t!("rules.cc_mem_007.fix", text = w.text.as_str()),
                        safe,
                    ));
                }

                diagnostics.push(diagnostic);
            }
        }

        // CC-MEM-008: Critical content in middle
        if config.is_rule_enabled("CC-MEM-008") {
            let critical = find_critical_in_middle(content);
            for c in critical {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        c.line,
                        c.column,
                        "CC-MEM-008",
                        t!(
                            "rules.cc_mem_008.message",
                            keyword = c.keyword.as_str(),
                            percent = format!("{:.0}", c.position_percent)
                        ),
                    )
                    .with_suggestion(t!("rules.cc_mem_008.suggestion")),
                );
            }
        }

        // CC-MEM-004: Invalid npm script reference
        if config.is_rule_enabled("CC-MEM-004") {
            let npm_refs = extract_npm_scripts(content);
            if !npm_refs.is_empty() {
                // Try to find package.json relative to the CLAUDE.md file
                if let Some(parent) = path.parent() {
                    let package_json_path = parent.join("package.json");
                    // Use safe_read_file to prevent DoS and limit file size
                    if let Ok(pkg_content) = safe_read_file(&package_json_path) {
                        // Parse package.json and extract script names
                        if let Ok(pkg_json) =
                            serde_json::from_str::<serde_json::Value>(&pkg_content)
                        {
                            let available_scripts: Vec<String> = pkg_json
                                .get("scripts")
                                .and_then(|s| s.as_object())
                                .map(|scripts| scripts.keys().cloned().collect())
                                .unwrap_or_default();

                            for npm_ref in npm_refs {
                                if !available_scripts.contains(&npm_ref.script_name) {
                                    let suggestion = if available_scripts.is_empty() {
                                        t!("rules.cc_mem_004.suggestion_no_scripts").to_string()
                                    } else {
                                        t!(
                                            "rules.cc_mem_004.suggestion_available",
                                            scripts = available_scripts.join(", ")
                                        )
                                        .to_string()
                                    };

                                    diagnostics.push(
                                        Diagnostic::warning(
                                            path.to_path_buf(),
                                            npm_ref.line,
                                            npm_ref.column,
                                            "CC-MEM-004",
                                            t!(
                                                "rules.cc_mem_004.message",
                                                script = npm_ref.script_name.as_str()
                                            ),
                                        )
                                        .with_suggestion(suggestion),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // CC-MEM-010: README duplication
        if config.is_rule_enabled("CC-MEM-010") {
            if let Some(parent) = path.parent() {
                let readme_path = parent.join("README.md");
                // Use safe_read_file to prevent DoS and limit file size
                if let Ok(readme_content) = safe_read_file(&readme_path) {
                    if let Some(dup) = check_readme_duplication(content, &readme_content) {
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                1,
                                0,
                                "CC-MEM-010",
                                t!(
                                    "rules.cc_mem_010.message",
                                    overlap = format!("{:.0}", dup.overlap_percent),
                                    threshold = format!("{:.0}", dup.threshold)
                                ),
                            )
                            .with_suggestion(t!("rules.cc_mem_010.suggestion")),
                        );
                    }
                }
            }
        }

        diagnostics
    }
}

/// Get the replacement for weak constraint language
/// Returns (replacement_text, is_safe)
/// - "should" -> "must" (safe)
/// - "try to" -> "must" (safe)
/// - "consider" -> "ensure" (safe)
/// - "maybe" -> "" (delete, safe)
/// - "might want to" -> "must" (safe)
/// - "could" -> "must" (not safe - could have other meanings)
/// - "possibly" -> "" (delete, not safe)
fn get_weak_constraint_replacement(text: &str) -> (Option<&'static str>, bool) {
    match text.to_lowercase().as_str() {
        "should" => (Some("must"), true),
        "try to" => (Some("must"), true),
        "consider" => (Some("ensure"), true),
        "maybe" => (Some(""), true),
        "might want to" => (Some("must"), true),
        "could" => (Some("must"), false),
        "possibly" => (Some(""), false),
        _ => (None, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use std::fs;

    #[test]
    fn test_generic_instruction_detected() {
        let content = "Be helpful and accurate when responding.";
        let validator = ClaudeMdValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());

        assert!(!diagnostics.is_empty());
        // Verify rule ID is CC-MEM-005
        assert!(diagnostics.iter().any(|d| d.rule == "CC-MEM-005"));
    }

    #[test]
    fn test_skip_agents_files() {
        // CC-MEM rules should NOT apply to AGENTS.* files
        let content = "Be helpful and accurate when responding.";
        let validator = ClaudeMdValidator;

        // AGENTS.md should be skipped
        let diagnostics =
            validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default());
        assert!(
            diagnostics.is_empty(),
            "CC-MEM rules should not fire for AGENTS.md"
        );

        // AGENTS.local.md should be skipped
        let diagnostics = validator.validate(
            Path::new("AGENTS.local.md"),
            content,
            &LintConfig::default(),
        );
        assert!(
            diagnostics.is_empty(),
            "CC-MEM rules should not fire for AGENTS.local.md"
        );

        // AGENTS.override.md should be skipped
        let diagnostics = validator.validate(
            Path::new("AGENTS.override.md"),
            content,
            &LintConfig::default(),
        );
        assert!(
            diagnostics.is_empty(),
            "CC-MEM rules should not fire for AGENTS.override.md"
        );
    }

    #[test]
    fn test_claude_local_md_gets_rules() {
        // CLAUDE.local.md SHOULD get CC-MEM rules
        let content = "Be helpful and accurate when responding.";
        let validator = ClaudeMdValidator;
        let diagnostics = validator.validate(
            Path::new("CLAUDE.local.md"),
            content,
            &LintConfig::default(),
        );

        assert!(
            !diagnostics.is_empty(),
            "CC-MEM rules should fire for CLAUDE.local.md"
        );
        assert!(diagnostics.iter().any(|d| d.rule == "CC-MEM-005"));
    }

    #[test]
    fn test_config_disabled_memory_category() {
        let mut config = LintConfig::default();
        config.rules.memory = false;

        let content = "Be helpful and accurate when responding.";
        let validator = ClaudeMdValidator;
        let diagnostics = validator.validate(Path::new("CLAUDE.md"), content, &config);

        // Should be empty when memory category is disabled
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["CC-MEM-005".to_string()];

        let content = "Be helpful and accurate when responding.";
        let validator = ClaudeMdValidator;
        let diagnostics = validator.validate(Path::new("CLAUDE.md"), content, &config);

        // Should be empty when CC-MEM-005 is specifically disabled
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_config_cursor_target_disables_cc_mem_rules() {
        use crate::config::TargetTool;

        let mut config = LintConfig::default();
        config.target = TargetTool::Cursor;

        let content = "Be helpful and accurate when responding.";
        let validator = ClaudeMdValidator;
        let diagnostics = validator.validate(Path::new("CLAUDE.md"), content, &config);

        // CC-MEM-005 should not fire for Cursor target
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_legacy_generic_instructions_flag() {
        let mut config = LintConfig::default();
        config.rules.generic_instructions = false;

        let content = "Be helpful and accurate when responding.";
        let validator = ClaudeMdValidator;
        let diagnostics = validator.validate(Path::new("CLAUDE.md"), content, &config);

        // Legacy flag should still work
        assert!(diagnostics.is_empty());
    }

    // CC-MEM-009: Token count exceeded
    #[test]
    fn test_cc_mem_009_token_exceeded() {
        let content = "x".repeat(6100); // > 6000 chars = > 1500 tokens
        let validator = ClaudeMdValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), &content, &LintConfig::default());

        let mem009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-009")
            .collect();
        assert_eq!(mem009.len(), 1);
        assert!(mem009[0].message.contains("exceeds"));
    }

    #[test]
    fn test_cc_mem_009_under_limit() {
        let content = "Short content.";
        let validator = ClaudeMdValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());

        let mem009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-009")
            .collect();
        assert!(mem009.is_empty());
    }

    // CC-MEM-006: Negative without positive
    #[test]
    fn test_cc_mem_006_negative_without_positive() {
        let content = "Never use var in JavaScript.";
        let validator = ClaudeMdValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());

        let mem006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-006")
            .collect();
        assert_eq!(mem006.len(), 1);
        assert!(mem006[0].message.contains("Never"));
    }

    #[test]
    fn test_cc_mem_006_negative_with_positive() {
        let content = "Never use var, instead prefer const.";
        let validator = ClaudeMdValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());

        let mem006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-006")
            .collect();
        assert!(mem006.is_empty());
    }

    // CC-MEM-007: Weak constraint language
    #[test]
    fn test_cc_mem_007_weak_in_critical() {
        let content = "# Critical Rules\n\nYou should follow the coding style.";
        let validator = ClaudeMdValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());

        let mem007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-007")
            .collect();
        assert_eq!(mem007.len(), 1);
        assert!(mem007[0].message.contains("should"));
    }

    #[test]
    fn test_cc_mem_007_weak_outside_critical() {
        let content = "# General Info\n\nYou should follow the coding style.";
        let validator = ClaudeMdValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());

        let mem007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-007")
            .collect();
        assert!(mem007.is_empty());
    }

    // CC-MEM-008: Critical content in middle
    #[test]
    fn test_cc_mem_008_critical_in_middle() {
        // Create 20 lines with "critical" at line 10 (50%)
        let mut lines: Vec<String> = (0..20).map(|i| format!("Line {}", i)).collect();
        lines[10] = "This is critical information.".to_string();
        let content = lines.join("\n");

        let validator = ClaudeMdValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), &content, &LintConfig::default());

        let mem008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-008")
            .collect();
        assert_eq!(mem008.len(), 1);
        assert!(mem008[0].message.contains("middle zone"));
    }

    #[test]
    fn test_cc_mem_008_critical_at_top() {
        let mut lines: Vec<String> = (0..20).map(|i| format!("Line {}", i)).collect();
        lines[1] = "This is critical information.".to_string();
        let content = lines.join("\n");

        let validator = ClaudeMdValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), &content, &LintConfig::default());

        let mem008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-008")
            .collect();
        assert!(mem008.is_empty());
    }

    // CC-MEM-004: Invalid npm script (needs filesystem, tested via tempdir)
    #[test]
    fn test_cc_mem_004_invalid_npm_script() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let claude_md_path = temp_dir.path().join("CLAUDE.md");
        let package_json_path = temp_dir.path().join("package.json");

        // Write CLAUDE.md with npm run reference
        let mut claude_file = fs::File::create(&claude_md_path).unwrap();
        writeln!(claude_file, "Run tests with npm run nonexistent").unwrap();

        // Write package.json with different scripts
        let mut pkg_file = fs::File::create(&package_json_path).unwrap();
        writeln!(
            pkg_file,
            r#"{{"scripts": {{"test": "jest", "build": "tsc"}}}}"#
        )
        .unwrap();

        let content = fs::read_to_string(&claude_md_path).unwrap();
        let validator = ClaudeMdValidator;
        let diagnostics = validator.validate(&claude_md_path, &content, &LintConfig::default());

        let mem004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-004")
            .collect();
        assert_eq!(mem004.len(), 1);
        assert!(mem004[0].message.contains("nonexistent"));
    }

    #[test]
    fn test_cc_mem_004_valid_npm_script() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let claude_md_path = temp_dir.path().join("CLAUDE.md");
        let package_json_path = temp_dir.path().join("package.json");

        // Write CLAUDE.md with valid npm run reference
        let mut claude_file = fs::File::create(&claude_md_path).unwrap();
        writeln!(claude_file, "Run tests with npm run test").unwrap();

        // Write package.json with matching script
        let mut pkg_file = fs::File::create(&package_json_path).unwrap();
        writeln!(pkg_file, r#"{{"scripts": {{"test": "jest"}}}}"#).unwrap();

        let content = fs::read_to_string(&claude_md_path).unwrap();
        let validator = ClaudeMdValidator;
        let diagnostics = validator.validate(&claude_md_path, &content, &LintConfig::default());

        let mem004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-004")
            .collect();
        assert!(mem004.is_empty());
    }

    // CC-MEM-010: README duplication
    #[test]
    fn test_cc_mem_010_readme_duplication() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let claude_md_path = temp_dir.path().join("CLAUDE.md");
        let readme_path = temp_dir.path().join("README.md");

        let shared_content =
            "This project validates agent configurations using Rust for performance.";

        // Write identical content to both files
        let mut claude_file = fs::File::create(&claude_md_path).unwrap();
        writeln!(claude_file, "{}", shared_content).unwrap();

        let mut readme_file = fs::File::create(&readme_path).unwrap();
        writeln!(readme_file, "{}", shared_content).unwrap();

        let content = fs::read_to_string(&claude_md_path).unwrap();
        let validator = ClaudeMdValidator;
        let diagnostics = validator.validate(&claude_md_path, &content, &LintConfig::default());

        let mem010: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-010")
            .collect();
        assert_eq!(mem010.len(), 1);
        assert!(mem010[0].message.contains("overlap"));
    }

    #[test]
    fn test_cc_mem_010_no_duplication() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let claude_md_path = temp_dir.path().join("CLAUDE.md");
        let readme_path = temp_dir.path().join("README.md");

        // Write different content
        let mut claude_file = fs::File::create(&claude_md_path).unwrap();
        writeln!(
            claude_file,
            "Project-specific instructions for Claude. Focus on these guidelines."
        )
        .unwrap();

        let mut readme_file = fs::File::create(&readme_path).unwrap();
        writeln!(
            readme_file,
            "Welcome to the project. Installation: npm install. Usage: npm start."
        )
        .unwrap();

        let content = fs::read_to_string(&claude_md_path).unwrap();
        let validator = ClaudeMdValidator;
        let diagnostics = validator.validate(&claude_md_path, &content, &LintConfig::default());

        let mem010: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-010")
            .collect();
        assert!(mem010.is_empty());
    }

    #[test]
    fn test_all_new_rules_disabled_individually() {
        let content = r#"# Critical Rules

Don't do this without alternatives.
You should consider this approach.
"#
        .to_string()
            + &"x".repeat(6100);

        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec![
            "CC-MEM-004".to_string(),
            "CC-MEM-006".to_string(),
            "CC-MEM-007".to_string(),
            "CC-MEM-008".to_string(),
            "CC-MEM-009".to_string(),
            "CC-MEM-010".to_string(),
        ];

        let validator = ClaudeMdValidator;
        let diagnostics = validator.validate(Path::new("CLAUDE.md"), &content, &config);

        // Only CC-MEM-005 should remain (if present)
        for d in &diagnostics {
            assert!(
                !d.rule.starts_with("CC-MEM-00") || d.rule == "CC-MEM-005",
                "Rule {} should be disabled",
                d.rule
            );
        }
    }

    // ===== Auto-fix Tests for CC-MEM-005 =====

    #[test]
    fn test_cc_mem_005_has_fix() {
        let content = "Be helpful and accurate when responding.";
        let validator = ClaudeMdValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());

        let mem005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-005")
            .collect();
        assert!(!mem005.is_empty());
        assert!(mem005[0].has_fixes());

        let fix = &mem005[0].fixes[0];
        assert!(fix.is_deletion());
        assert!(fix.safe);
    }

    #[test]
    fn test_cc_mem_005_fix_byte_positions() {
        let content = "Line one.\nBe helpful and accurate.\nLine three.";
        let validator = ClaudeMdValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());

        let mem005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-005")
            .collect();
        assert_eq!(mem005.len(), 1);

        let fix = &mem005[0].fixes[0];
        // Line "Be helpful and accurate." starts at byte 10 (after "Line one.\n")
        assert_eq!(fix.start_byte, 10);
        // Ends at byte 35 (after "Be helpful and accurate.\n")
        assert_eq!(fix.end_byte, 35);
    }

    #[test]
    fn test_cc_mem_005_fix_application() {
        let content = "Line one.\nBe helpful and accurate.\nLine three.";
        let validator = ClaudeMdValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());

        let mem005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-005")
            .collect();
        assert_eq!(mem005.len(), 1);

        let fix = &mem005[0].fixes[0];
        let mut fixed = content.to_string();
        fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);

        assert_eq!(fixed, "Line one.\nLine three.");
    }

    #[test]
    fn test_cc_mem_005_fix_last_line_no_newline() {
        let content = "Be helpful and accurate.";
        let validator = ClaudeMdValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());

        let mem005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-005")
            .collect();
        assert_eq!(mem005.len(), 1);

        let fix = &mem005[0].fixes[0];
        // Last line without newline should have end_byte at content length
        assert_eq!(fix.start_byte, 0);
        assert_eq!(fix.end_byte, 24);
    }

    // ===== Auto-fix Tests for CC-MEM-007 =====

    #[test]
    fn test_cc_mem_007_has_fix() {
        let content = "# Critical Rules\n\nYou should follow the coding style.";
        let validator = ClaudeMdValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());

        let mem007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-007")
            .collect();
        assert_eq!(mem007.len(), 1);
        assert!(mem007[0].has_fixes());

        let fix = &mem007[0].fixes[0];
        assert_eq!(fix.replacement, "must");
        assert!(fix.safe);
    }

    #[test]
    fn test_cc_mem_007_fix_byte_positions() {
        let content = "# Critical Rules\n\nYou should follow the coding style.";
        let validator = ClaudeMdValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());

        let mem007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-007")
            .collect();
        assert_eq!(mem007.len(), 1);

        let fix = &mem007[0].fixes[0];
        // "should" starts at byte 22 (after "# Critical Rules\n\nYou ")
        assert_eq!(fix.start_byte, 22);
        // "should" ends at byte 28
        assert_eq!(fix.end_byte, 28);
    }

    #[test]
    fn test_cc_mem_007_fix_application() {
        let content = "# Critical Rules\n\nYou should follow the coding style.";
        let validator = ClaudeMdValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());

        let mem007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-007")
            .collect();
        assert_eq!(mem007.len(), 1);

        let fix = &mem007[0].fixes[0];
        let mut fixed = content.to_string();
        fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);

        assert_eq!(
            fixed,
            "# Critical Rules\n\nYou must follow the coding style."
        );
    }

    #[test]
    fn test_cc_mem_007_fix_replacements() {
        // Test all replacement mappings
        let test_cases = vec![
            ("# Critical Rules\n\nYou should do this.", "must", true),
            ("# Critical Rules\n\nTry to do this.", "must", true),
            ("# Critical Rules\n\nConsider doing this.", "ensure", true),
            ("# Critical Rules\n\nMaybe do this.", "", true),
            ("# Critical Rules\n\nYou could do this.", "must", false),
        ];

        let validator = ClaudeMdValidator;

        for (content, expected_replacement, expected_safe) in test_cases {
            let diagnostics =
                validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());

            let mem007: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-MEM-007")
                .collect();
            assert_eq!(mem007.len(), 1, "Expected one CC-MEM-007 for: {}", content);
            assert!(mem007[0].has_fixes());

            let fix = &mem007[0].fixes[0];
            assert_eq!(
                fix.replacement, expected_replacement,
                "Wrong replacement for: {}",
                content
            );
            assert_eq!(fix.safe, expected_safe, "Wrong safe flag for: {}", content);
        }
    }

    #[test]
    fn test_cc_mem_007_multiple_weak_words() {
        let content = "# Critical Rules\n\nYou should consider doing this.";
        let validator = ClaudeMdValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());

        let mem007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-007")
            .collect();

        // Each weak word generates a separate diagnostic with its own fix
        assert!(mem007.len() >= 1);
        for d in &mem007 {
            assert!(d.has_fixes());
        }
    }

    // ===== Additional CC-MEM rule tests =====

    #[test]
    fn test_cc_mem_004_all_known_commands() {
        // Test that known Claude commands don't trigger CC-MEM-004
        let known_commands = [
            "/help", "/compact", "/resume", "/memory", "/config", "/doctor",
        ];
        let validator = ClaudeMdValidator;

        for cmd in known_commands {
            let content = format!("# Commands\n\nUse {} for assistance.", cmd);
            let diagnostics =
                validator.validate(Path::new("CLAUDE.md"), &content, &LintConfig::default());

            let mem004: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-MEM-004")
                .collect();
            assert!(
                mem004.is_empty(),
                "Known command '{}' should not trigger CC-MEM-004",
                cmd
            );
        }
    }

    #[test]
    fn test_cc_mem_005_be_helpful_pattern() {
        // "Be helpful and accurate" is a known generic pattern
        let content = "Be helpful and accurate when responding.";
        let validator = ClaudeMdValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());

        let mem005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-005")
            .collect();
        assert!(
            !mem005.is_empty(),
            "Generic 'Be helpful and accurate' should trigger CC-MEM-005"
        );
    }

    #[test]
    fn test_cc_mem_006_all_negative_patterns() {
        let negative_patterns = [
            "Don't write bad code.",
            "Never use deprecated APIs.",
            "Avoid global variables.",
            "Do not skip tests.",
        ];
        let validator = ClaudeMdValidator;

        for pattern in negative_patterns {
            let content = format!("# Rules\n\n{}", pattern);
            let diagnostics =
                validator.validate(Path::new("CLAUDE.md"), &content, &LintConfig::default());

            let mem006: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-MEM-006")
                .collect();
            assert!(
                !mem006.is_empty(),
                "Negative pattern '{}' should trigger CC-MEM-006",
                pattern
            );
        }
    }

    #[test]
    fn test_cc_mem_008_edge_at_boundary() {
        // Test content with exactly 10 lines (boundary case)
        let lines: Vec<String> = (1..=10).map(|i| format!("Line {}", i)).collect();
        let content = lines.join("\n");

        let validator = ClaudeMdValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), &content, &LintConfig::default());

        // 10 lines is at boundary, CC-MEM-008 checks for critical in middle
        let mem008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-008")
            .collect();
        // No critical section, so no error
        assert!(mem008.is_empty());
    }

    #[test]
    fn test_cc_mem_009_exact_token_limit() {
        // Create content close to 4000 tokens (rough estimate: ~4 chars per token)
        let long_content = "word ".repeat(3900);
        let content = format!("# Project\n\n{}", long_content);

        let validator = ClaudeMdValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), &content, &LintConfig::default());

        let mem009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-009")
            .collect();
        // Should trigger token exceeded warning
        assert!(!mem009.is_empty());
    }

    #[test]
    fn test_cc_mem_010_unique_content_no_error() {
        // Content that doesn't duplicate README
        let content = "# Project Memory\n\nThis contains unique project-specific instructions.";

        let validator = ClaudeMdValidator;
        let diagnostics =
            validator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());

        let mem010: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-010")
            .collect();
        // No README duplication without access to README
        assert!(mem010.is_empty());
    }

    #[test]
    fn test_all_cc_mem_rules_can_be_disabled() {
        let rules = [
            "CC-MEM-004",
            "CC-MEM-005",
            "CC-MEM-006",
            "CC-MEM-007",
            "CC-MEM-008",
            "CC-MEM-009",
            "CC-MEM-010",
        ];

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules.disabled_rules = vec![rule.to_string()];

            // Content that could trigger each rule
            let content = "# Critical Rules\n\nAlways follow best practices. Don't write bad code.\nRun /unknown-cmd.\nYou should do this.";

            let validator = ClaudeMdValidator;
            let diagnostics = validator.validate(Path::new("CLAUDE.md"), content, &config);

            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }
}
