//! @import reference validation

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    parsers::markdown::{extract_imports, Import},
    rules::Validator,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct ImportsValidator;

const MAX_IMPORT_DEPTH: usize = 5;

impl Validator for ImportsValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Check both new category flag and legacy flag for backward compatibility
        if !config.rules.imports || !config.rules.import_references {
            return diagnostics;
        }

        // Detect root file type for cycle/depth rules
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let is_claude_md = filename == "CLAUDE.md";

        let root_path = normalize_existing_path(path);
        let mut cache: HashMap<PathBuf, Vec<Import>> = HashMap::new();
        let mut visited_depth: HashMap<PathBuf, usize> = HashMap::new();
        let mut stack = Vec::new();

        cache.insert(root_path.clone(), extract_imports(content));
        visit_imports(
            &root_path,
            None,
            &mut cache,
            &mut visited_depth,
            &mut stack,
            &mut diagnostics,
            config,
            is_claude_md,
        );

        diagnostics
    }
}

fn visit_imports(
    file_path: &PathBuf,
    content_override: Option<&str>,
    cache: &mut HashMap<PathBuf, Vec<Import>>,
    visited_depth: &mut HashMap<PathBuf, usize>,
    stack: &mut Vec<PathBuf>,
    diagnostics: &mut Vec<Diagnostic>,
    config: &LintConfig,
    root_is_claude_md: bool,
) {
    let depth = stack.len();
    if let Some(prev_depth) = visited_depth.get(file_path) {
        if *prev_depth >= depth {
            return;
        }
    }
    visited_depth.insert(file_path.clone(), depth);

    let imports = get_imports_for_file(file_path, content_override, cache);
    let Some(imports) = imports else { return };

    let base_dir = file_path.parent().unwrap_or(Path::new("."));

    // Determine file type for current file to route its own diagnostics
    let filename = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let is_claude_md = filename == "CLAUDE.md";

    // Check rules based on CURRENT file type for missing imports
    // Check rules based on ROOT file type for cycles/depth (applies to entire chain)
    let check_not_found = (is_claude_md && config.is_rule_enabled("CC-MEM-001"))
        || (!is_claude_md && config.is_rule_enabled("REF-001"));
    let check_cycle = root_is_claude_md && config.is_rule_enabled("CC-MEM-002");
    let check_depth = root_is_claude_md && config.is_rule_enabled("CC-MEM-003");

    if !(check_not_found || check_cycle || check_depth) {
        return;
    }

    let rule_not_found = if is_claude_md { "CC-MEM-001" } else { "REF-001" };
    let rule_cycle = "CC-MEM-002";
    let rule_depth = "CC-MEM-003";

    stack.push(file_path.clone());

    for import in imports {
        let resolved = resolve_import_path(&import.path, base_dir);

        // Validate path to prevent traversal attacks
        // Reject absolute paths and paths that escape the project root
        if import.path.starts_with('/') || import.path.starts_with('~') {
            if check_not_found {
                diagnostics.push(
                    Diagnostic::error(
                        file_path.clone(),
                        import.line,
                        import.column,
                        rule_not_found,
                        format!("Absolute import paths not allowed: @{}", import.path),
                    )
                    .with_suggestion("Use relative paths only".to_string()),
                );
            }
            continue;
        }

        let normalized = if resolved.exists() {
            normalize_existing_path(&resolved)
        } else {
            resolved
        };

        if !normalized.exists() {
            if check_not_found {
                diagnostics.push(
                    Diagnostic::error(
                        file_path.clone(),
                        import.line,
                        import.column,
                        rule_not_found,
                        format!("Import not found: @{}", import.path),
                    )
                    .with_suggestion(format!(
                        "Check that the file exists: {}",
                        normalized.display()
                    )),
                );
            }
            continue;
        }

        // Always check for cycles/depth to prevent infinite recursion
        let has_cycle = stack.contains(&normalized);
        let exceeds_depth = depth + 1 > MAX_IMPORT_DEPTH;

        // Emit diagnostics if rules are enabled for this file type
        if check_cycle && has_cycle {
            let cycle = format_cycle(stack, &normalized);
            diagnostics.push(
                Diagnostic::error(
                    file_path.clone(),
                    import.line,
                    import.column,
                    rule_cycle,
                    format!("Circular @import detected: {}", cycle),
                )
                .with_suggestion("Remove or break the circular @import chain".to_string()),
            );
            continue;
        }

        if check_depth && exceeds_depth {
            diagnostics.push(
                Diagnostic::error(
                    file_path.clone(),
                    import.line,
                    import.column,
                    rule_depth,
                    format!(
                        "Import depth exceeds {} hops at @{}",
                        MAX_IMPORT_DEPTH, import.path
                    ),
                )
                .with_suggestion("Flatten or shorten the @import chain".to_string()),
            );
            continue;
        }

        // Only recurse if no cycle/depth issues
        if !has_cycle && !exceeds_depth {
            visit_imports(
                &normalized,
                None,
                cache,
                visited_depth,
                stack,
                diagnostics,
                config,
                root_is_claude_md,
            );
        }
    }

    stack.pop();
}

fn get_imports_for_file(
    file_path: &Path,
    content_override: Option<&str>,
    cache: &mut HashMap<PathBuf, Vec<Import>>,
) -> Option<Vec<Import>> {
    if !cache.contains_key(file_path) {
        let content = match content_override {
            Some(content) => content.to_string(),
            None => std::fs::read_to_string(file_path).ok()?,
        };
        let imports = extract_imports(&content);
        cache.insert(file_path.to_path_buf(), imports);
    }
    cache.get(file_path).cloned()
}

fn resolve_import_path(import_path: &str, base_dir: &Path) -> PathBuf {
    if import_path.starts_with("~/") || import_path.starts_with("~\\") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&import_path[2..]);
        }
    }

    let raw = PathBuf::from(import_path);
    if raw.is_absolute() {
        raw
    } else {
        base_dir.join(raw)
    }
}

fn normalize_existing_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn format_cycle(stack: &[PathBuf], target: &Path) -> String {
    let mut cycle = Vec::new();
    let mut in_cycle = false;
    for path in stack {
        if path == target {
            in_cycle = true;
        }
        if in_cycle {
            cycle.push(path.display().to_string());
        }
    }
    cycle.push(target.display().to_string());
    cycle.join(" -> ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_config_disabled_imports_category() {
        let mut config = LintConfig::default();
        config.rules.imports = false;

        let content = "@nonexistent-file.md";
        let validator = ImportsValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &config);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_legacy_import_references_flag() {
        let mut config = LintConfig::default();
        config.rules.import_references = false;

        let content = "@nonexistent-file.md";
        let validator = ImportsValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &config);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_missing_import_in_claude_md() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("CLAUDE.md");
        fs::write(&file_path, "See @missing.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics =
            validator.validate(&file_path, "See @missing.md", &LintConfig::default());

        assert!(diagnostics.iter().any(|d| d.rule == "CC-MEM-001"));
    }

    #[test]
    fn test_cycle_detection_in_claude_md() {
        let temp = TempDir::new().unwrap();
        let a = temp.path().join("CLAUDE.md");
        let b = temp.path().join("b.md");
        fs::write(&a, "See @b.md").unwrap();
        fs::write(&b, "See @CLAUDE.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&a, "See @b.md", &LintConfig::default());

        assert!(diagnostics.iter().any(|d| d.rule == "CC-MEM-002"));
    }

    #[test]
    fn test_depth_exceeded_in_claude_md() {
        let temp = TempDir::new().unwrap();
        let claude_md = temp.path().join("CLAUDE.md");
        let paths: Vec<PathBuf> = (1..7)
            .map(|i| temp.path().join(format!("{}.md", i)))
            .collect();

        fs::write(&claude_md, "See @1.md").unwrap();
        for i in 0..5 {
            let content = format!("See @{}.md", i + 2);
            fs::write(&paths[i], content).unwrap();
        }
        fs::write(&paths[5], "End").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&claude_md, "See @1.md", &LintConfig::default());

        assert!(diagnostics.iter().any(|d| d.rule == "CC-MEM-003"));
    }

    #[test]
    fn test_missing_import_in_skill_md() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("SKILL.md");
        fs::write(&file_path, "See @missing.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics =
            validator.validate(&file_path, "See @missing.md", &LintConfig::default());

        assert!(diagnostics.iter().any(|d| d.rule == "REF-001"));
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-MEM-001"));
    }

    #[test]
    fn test_missing_import_in_agents_md() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("AGENTS.md");
        fs::write(&file_path, "See @missing.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics =
            validator.validate(&file_path, "See @missing.md", &LintConfig::default());

        assert!(diagnostics.iter().any(|d| d.rule == "REF-001"));
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-MEM-001"));
    }

    #[test]
    fn test_missing_import_in_generic_md() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("README.md");
        fs::write(&file_path, "See @missing.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics =
            validator.validate(&file_path, "See @missing.md", &LintConfig::default());

        assert!(diagnostics.iter().any(|d| d.rule == "REF-001"));
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-MEM-001"));
    }

    #[test]
    fn test_cycle_in_skill_md() {
        let temp = TempDir::new().unwrap();
        let a = temp.path().join("SKILL.md");
        let b = temp.path().join("b.md");
        fs::write(&a, "See @b.md").unwrap();
        fs::write(&b, "See @SKILL.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&a, "See @b.md", &LintConfig::default());

        // Non-CLAUDE files don't check cycles, so no diagnostics expected
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_depth_exceeded_in_skill_md() {
        let temp = TempDir::new().unwrap();
        let skill_md = temp.path().join("SKILL.md");
        let paths: Vec<PathBuf> = (1..7)
            .map(|i| temp.path().join(format!("{}.md", i)))
            .collect();

        fs::write(&skill_md, "See @1.md").unwrap();
        for i in 0..5 {
            let content = format!("See @{}.md", i + 2);
            fs::write(&paths[i], content).unwrap();
        }
        fs::write(&paths[5], "End").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&skill_md, "See @1.md", &LintConfig::default());

        // Non-CLAUDE files don't check depth, so no diagnostics expected
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_ref_001_disabled_suppresses_skill_md_errors() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("SKILL.md");
        fs::write(&file_path, "See @missing.md").unwrap();

        let mut config = LintConfig::default();
        config.rules.disabled_rules.push("REF-001".to_string());

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @missing.md", &config);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_cc_mem_disabled_still_allows_ref() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("SKILL.md");
        fs::write(&file_path, "See @missing.md").unwrap();

        let mut config = LintConfig::default();
        config.rules.disabled_rules.push("CC-MEM-001".to_string());
        config.rules.disabled_rules.push("CC-MEM-002".to_string());
        config.rules.disabled_rules.push("CC-MEM-003".to_string());

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @missing.md", &config);

        assert!(diagnostics.iter().any(|d| d.rule == "REF-001"));
    }

    #[test]
    fn test_ref_disabled_still_allows_cc_mem() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("CLAUDE.md");
        fs::write(&file_path, "See @missing.md").unwrap();

        let mut config = LintConfig::default();
        config.rules.disabled_rules.push("REF-001".to_string());

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @missing.md", &config);

        // CLAUDE.md should still emit CC-MEM-001 even when REF-001 is disabled
        assert!(diagnostics.iter().any(|d| d.rule == "CC-MEM-001"));
    }

    #[test]
    fn test_nested_file_type_detection() {
        // Test for critical fix: file type should be determined per-file in recursion
        let temp = TempDir::new().unwrap();
        let skill_md = temp.path().join("SKILL.md");
        let claude_md = temp.path().join("CLAUDE.md");

        // SKILL.md imports CLAUDE.md which has a missing import
        fs::write(&skill_md, "See @CLAUDE.md").unwrap();
        fs::write(&claude_md, "See @missing.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&skill_md, "See @CLAUDE.md", &LintConfig::default());

        // CLAUDE.md's missing import should emit CC-MEM-001, not REF-001
        assert!(diagnostics.iter().any(|d| d.rule == "CC-MEM-001" && d.file.ends_with("CLAUDE.md")));
        assert!(!diagnostics.iter().any(|d| d.rule == "REF-001" && d.file.ends_with("CLAUDE.md")));
    }

    #[test]
    fn test_absolute_path_rejection() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("CLAUDE.md");
        fs::write(&file_path, "See @/etc/passwd").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @/etc/passwd", &LintConfig::default());

        // Absolute paths should be rejected
        assert!(diagnostics.iter().any(|d| d.message.contains("Absolute import paths not allowed")));
    }
}
