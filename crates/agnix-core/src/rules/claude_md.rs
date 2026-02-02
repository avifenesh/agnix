//! CLAUDE.md validation

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::Validator,
    schemas::claude_md::{
        check_readme_duplication, check_token_count, extract_npm_scripts, find_critical_in_middle,
        find_generic_instructions, find_negative_without_positive, find_weak_constraints,
    },
};
use std::fs;
use std::path::Path;

/// Maximum file size to read for validation (1MB)
const MAX_FILE_SIZE: u64 = 1_048_576;

/// Safely read a file with size limits to prevent DoS attacks.
/// Returns None if file doesn't exist, is too large, or can't be read.
fn safe_read_file(path: &Path) -> Option<String> {
    // Check file metadata first to avoid reading huge files
    let metadata = fs::metadata(path).ok()?;

    // Reject files larger than MAX_FILE_SIZE
    if metadata.len() > MAX_FILE_SIZE {
        return None;
    }

    // Reject non-regular files (symlinks, devices, etc.)
    if !metadata.is_file() {
        return None;
    }

    fs::read_to_string(path).ok()
}

pub struct ClaudeMdValidator;

impl Validator for ClaudeMdValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

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
                        format!(
                            "Generic instruction '{}' - Claude already knows this",
                            inst.text
                        ),
                    )
                    .with_suggestion(
                        "Remove generic instructions. Focus on project-specific context."
                            .to_string(),
                    ),
                );
            }
        }

        // CC-MEM-009: Token count exceeded
        if config.is_rule_enabled("CC-MEM-009") {
            if let Some(exceeded) = check_token_count(content) {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-MEM-009",
                        format!(
                            "File exceeds recommended token limit (~{} tokens, limit is {})",
                            exceeded.estimated_tokens, exceeded.limit
                        ),
                    )
                    .with_suggestion(
                        "Consider using @import to split content into multiple files.".to_string(),
                    ),
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
                        format!(
                            "Negative instruction '{}' without positive alternative",
                            neg.text
                        ),
                    )
                    .with_suggestion(
                        "Add a positive alternative: 'Instead, do...' or 'Use X instead.'"
                            .to_string(),
                    ),
                );
            }
        }

        // CC-MEM-007: Weak constraint language in critical sections
        if config.is_rule_enabled("CC-MEM-007") {
            let weak = find_weak_constraints(content);
            for w in weak {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        w.line,
                        w.column,
                        "CC-MEM-007",
                        format!(
                            "Weak constraint '{}' in critical section '{}'",
                            w.text, w.section
                        ),
                    )
                    .with_suggestion(
                        "Use strong language in critical sections: 'must', 'always', 'required'."
                            .to_string(),
                    ),
                );
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
                        format!(
                            "Critical keyword '{}' at {:.0}% of document (middle zone)",
                            c.keyword, c.position_percent
                        ),
                    )
                    .with_suggestion(
                        "Move critical content to the top or bottom of the document for better recall."
                            .to_string(),
                    ),
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
                    if let Some(pkg_content) = safe_read_file(&package_json_path) {
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
                                        "No scripts defined in package.json.".to_string()
                                    } else {
                                        format!(
                                            "Available scripts: {}",
                                            available_scripts.join(", ")
                                        )
                                    };

                                    diagnostics.push(
                                        Diagnostic::warning(
                                            path.to_path_buf(),
                                            npm_ref.line,
                                            npm_ref.column,
                                            "CC-MEM-004",
                                            format!(
                                                "npm script '{}' not found in package.json",
                                                npm_ref.script_name
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
                if let Some(readme_content) = safe_read_file(&readme_path) {
                    if let Some(dup) = check_readme_duplication(content, &readme_content) {
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                1,
                                0,
                                "CC-MEM-010",
                                format!(
                                    "CLAUDE.md has {:.0}% overlap with README.md (threshold: {:.0}%)",
                                    dup.overlap_percent, dup.threshold
                                ),
                            )
                            .with_suggestion(
                                "CLAUDE.md should complement README, not duplicate it. Remove duplicated sections."
                                    .to_string(),
                            ),
                        );
                    }
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;

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
}
