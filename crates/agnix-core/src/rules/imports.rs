//! @import and markdown link reference validation
//!
//! This module validates:
//! - CC-MEM-001: @import references point to existing files (Claude Code specific)
//! - CC-MEM-002: Circular @import detection
//! - CC-MEM-003: @import depth exceeded
//! - REF-001: @import file not found (universal)
//! - REF-002: Broken markdown links (universal)

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    file_utils::safe_read_file,
    parsers::markdown::{extract_imports, extract_markdown_links, Import},
    rules::Validator,
};
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

pub struct ImportsValidator;

const MAX_IMPORT_DEPTH: usize = 5;

/// Check if a URL is a local file link (not external or anchor-only)
fn is_local_file_link(url: &str) -> bool {
    const EXTERNAL_PREFIXES: &[&str] = &[
        "http://", "https://", "mailto:", "tel:", "data:", "ftp://", "file://", "//",
    ];

    if EXTERNAL_PREFIXES.iter().any(|p| url.starts_with(p)) {
        return false;
    }

    !url.is_empty() && !url.starts_with('#')
}

/// Strip URL fragment (e.g., "file.md#section" -> "file.md")
fn strip_fragment(url: &str) -> &str {
    match url.find('#') {
        Some(idx) => &url[..idx],
        None => url,
    }
}

impl Validator for ImportsValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Check both new category flag and legacy flag for backward compatibility
        if !config.rules.imports || !config.rules.import_references {
            return diagnostics;
        }

        // Detect root file type for cycle/depth rules
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let is_claude_md = matches!(filename, "CLAUDE.md" | "CLAUDE.local.md");

        let project_root = resolve_project_root(path, config);
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
            &project_root,
        );

        // Validate markdown links (REF-002)
        validate_markdown_links(path, content, config, &mut diagnostics);

        diagnostics
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_imports(
    file_path: &PathBuf,
    content_override: Option<&str>,
    cache: &mut HashMap<PathBuf, Vec<Import>>,
    visited_depth: &mut HashMap<PathBuf, usize>,
    stack: &mut Vec<PathBuf>,
    diagnostics: &mut Vec<Diagnostic>,
    config: &LintConfig,
    root_is_claude_md: bool,
    project_root: &Path,
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
    let normalized_base = normalize_existing_path(base_dir);
    let normalized_root = project_root;

    // Determine file type for current file to route its own diagnostics
    let filename = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let is_claude_md = matches!(filename, "CLAUDE.md" | "CLAUDE.local.md");

    // Check rules based on CURRENT file type for missing imports
    // Check rules based on ROOT file type for cycles/depth (applies to entire chain)
    let check_not_found = (is_claude_md && config.is_rule_enabled("CC-MEM-001"))
        || (!is_claude_md && config.is_rule_enabled("REF-001"));
    let check_cycle = root_is_claude_md && config.is_rule_enabled("CC-MEM-002");
    let check_depth = root_is_claude_md && config.is_rule_enabled("CC-MEM-003");

    if !(check_not_found || check_cycle || check_depth) {
        return;
    }

    let rule_not_found = if is_claude_md {
        "CC-MEM-001"
    } else {
        "REF-001"
    };
    let rule_cycle = "CC-MEM-002";
    let rule_depth = "CC-MEM-003";

    stack.push(file_path.clone());

    for import in imports {
        let resolved = resolve_import_path(&import.path, base_dir);

        // Validate path to prevent traversal attacks
        // Reject absolute paths and paths that escape the project root
        let raw_path = Path::new(&import.path);
        if raw_path.is_absolute()
            || import.path.starts_with('/')
            || import.path.starts_with('\\')
            || import.path.starts_with('~')
        {
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

        let normalized_resolved = normalize_join(&normalized_base, &import.path);
        if !normalized_resolved.starts_with(normalized_root) {
            if check_not_found {
                diagnostics.push(
                    Diagnostic::error(
                        file_path.clone(),
                        import.line,
                        import.column,
                        rule_not_found,
                        format!("Import path escapes project root: @{}", import.path),
                    )
                    .with_suggestion(
                        "Use relative paths that stay within the project root".to_string(),
                    ),
                );
            }
            continue;
        }

        let normalized = if resolved.exists() {
            let canonical_resolved = normalize_existing_path(&resolved);
            if !canonical_resolved.starts_with(normalized_root) {
                if check_not_found {
                    diagnostics.push(
                        Diagnostic::error(
                            file_path.clone(),
                            import.line,
                            import.column,
                            rule_not_found,
                            format!("Import path escapes project root: @{}", import.path),
                        )
                        .with_suggestion(
                            "Use relative paths that stay within the project root".to_string(),
                        ),
                    );
                }
                continue;
            }
            canonical_resolved
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
                project_root,
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
            // Silently skip files that can't be read (symlinks, too large, missing).
            // This is intentional: import chains often reference optional/external files,
            // and failing noisily on each would overwhelm the user.
            None => safe_read_file(file_path).ok()?,
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

fn normalize_join(base_dir: &Path, import_path: &str) -> PathBuf {
    let mut result = PathBuf::from(base_dir);
    for component in Path::new(import_path).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            Component::Normal(segment) => {
                result.push(segment);
            }
            Component::RootDir | Component::Prefix(_) => {
                result = PathBuf::from(component.as_os_str());
            }
        }
    }
    result
}

fn normalize_existing_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn resolve_project_root(path: &Path, config: &LintConfig) -> PathBuf {
    if let Some(root) = config.root_dir.as_deref() {
        return normalize_existing_path(root);
    }

    find_repo_root(path).unwrap_or_else(|| {
        let fallback = path.parent().unwrap_or(Path::new("."));
        normalize_existing_path(fallback)
    })
}

fn find_repo_root(path: &Path) -> Option<PathBuf> {
    for ancestor in path.ancestors() {
        if ancestor.as_os_str().is_empty() {
            continue;
        }
        let git_marker = ancestor.join(".git");
        if git_marker.is_dir() || git_marker.is_file() {
            return Some(ancestor.to_path_buf());
        }
    }
    None
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

/// Validate markdown links in content (REF-002)
fn validate_markdown_links(
    path: &Path,
    content: &str,
    config: &LintConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !config.is_rule_enabled("REF-002") {
        return;
    }

    let links = extract_markdown_links(content);
    let base_dir = path.parent().unwrap_or(Path::new("."));

    for link in links {
        // Skip non-local links (external URLs, anchors, etc.)
        if !is_local_file_link(&link.url) {
            continue;
        }

        // Strip fragment to get the file path
        let file_path = strip_fragment(&link.url);

        // Resolve the path relative to the file's directory
        let resolved = resolve_import_path(file_path, base_dir);

        // Security: Verify resolved path stays within project root
        // Normalize the resolved path to detect path traversal attempts
        if let Ok(canonical_resolved) = std::fs::canonicalize(&resolved) {
            if let Ok(canonical_base) = std::fs::canonicalize(base_dir) {
                if !canonical_resolved.starts_with(&canonical_base) {
                    // Path traversal attempt detected - skip this link
                    continue;
                }
            }
        }

        // Check if file exists
        if !resolved.exists() {
            let link_type = if link.is_image { "Image" } else { "Link" };
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    link.line,
                    link.column,
                    "REF-002",
                    format!("{} target not found: {}", link_type, link.url),
                )
                .with_suggestion(format!(
                    "Check that the file exists: {}",
                    resolved.display()
                )),
            );
        }
    }
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
        let diagnostics = validator.validate(&file_path, "See @missing.md", &LintConfig::default());

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
        for (i, path) in paths.iter().enumerate().take(5) {
            let content = format!("See @{}.md", i + 2);
            fs::write(path, content).unwrap();
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
        let diagnostics = validator.validate(&file_path, "See @missing.md", &LintConfig::default());

        assert!(diagnostics.iter().any(|d| d.rule == "REF-001"));
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-MEM-001"));
    }

    #[test]
    fn test_missing_import_in_agents_md() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("AGENTS.md");
        fs::write(&file_path, "See @missing.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @missing.md", &LintConfig::default());

        assert!(diagnostics.iter().any(|d| d.rule == "REF-001"));
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-MEM-001"));
    }

    #[test]
    fn test_missing_import_in_generic_md() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("README.md");
        fs::write(&file_path, "See @missing.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @missing.md", &LintConfig::default());

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
        for (i, path) in paths.iter().enumerate().take(5) {
            let content = format!("See @{}.md", i + 2);
            fs::write(path, content).unwrap();
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
        assert!(diagnostics
            .iter()
            .any(|d| d.rule == "CC-MEM-001" && d.file.ends_with("CLAUDE.md")));
        assert!(!diagnostics
            .iter()
            .any(|d| d.rule == "REF-001" && d.file.ends_with("CLAUDE.md")));
    }

    #[test]
    fn test_absolute_path_rejection() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("CLAUDE.md");
        fs::write(&file_path, "See @/etc/passwd").unwrap();

        let validator = ImportsValidator;
        let diagnostics =
            validator.validate(&file_path, "See @/etc/passwd", &LintConfig::default());

        // Absolute paths should be rejected
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("Absolute import paths not allowed")));
    }

    #[test]
    fn test_path_escape_rejection() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("root");
        let docs = root.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(temp.path().join("outside.md"), "Outside content").unwrap();

        let file_path = docs.join("CLAUDE.md");
        fs::write(&file_path, "See @../../outside.md").unwrap();

        let mut config = LintConfig::default();
        config.root_dir = Some(root);

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @../../outside.md", &config);

        assert!(diagnostics
            .iter()
            .any(|d| { d.rule == "CC-MEM-001" && d.message.contains("escapes project root") }));
    }

    #[cfg(unix)]
    #[test]
    fn test_symlink_escape_rejection() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().unwrap();
        let root = temp.path().join("root");
        let docs = root.join("docs");
        let outside = temp.path().join("outside");

        fs::create_dir_all(&docs).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("secret.md"), "Secret content").unwrap();

        let link_path = root.join("link");
        symlink(&outside, &link_path).unwrap();

        let file_path = docs.join("CLAUDE.md");
        fs::write(&file_path, "See @../link/secret.md").unwrap();

        let mut config = LintConfig::default();
        config.root_dir = Some(root);

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @../link/secret.md", &config);

        assert!(diagnostics
            .iter()
            .any(|d| { d.rule == "CC-MEM-001" && d.message.contains("escapes project root") }));
    }

    // ===== Helper Function Tests =====

    #[test]
    fn test_is_local_file_link_true() {
        assert!(is_local_file_link("file.md"));
        assert!(is_local_file_link("docs/guide.md"));
        assert!(is_local_file_link("./relative.md"));
        assert!(is_local_file_link("../parent.md"));
        assert!(is_local_file_link("file.md#section"));
    }

    #[test]
    fn test_is_local_file_link_false() {
        assert!(!is_local_file_link("https://example.com"));
        assert!(!is_local_file_link("http://example.com"));
        assert!(!is_local_file_link("mailto:test@example.com"));
        assert!(!is_local_file_link("tel:+1234567890"));
        assert!(!is_local_file_link("data:text/plain,hello"));
        assert!(!is_local_file_link("ftp://files.example.com"));
        assert!(!is_local_file_link("//cdn.example.com/file.js"));
        assert!(!is_local_file_link("#section"));
        assert!(!is_local_file_link(""));
    }

    #[test]
    fn test_strip_fragment() {
        assert_eq!(strip_fragment("file.md#section"), "file.md");
        assert_eq!(strip_fragment("file.md"), "file.md");
        assert_eq!(strip_fragment("#section"), "");
        assert_eq!(strip_fragment("docs/guide.md#heading"), "docs/guide.md");
    }

    // ===== REF-001 Tests =====

    #[test]
    fn test_ref_001_missing_import() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        fs::write(&file_path, "See @missing.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @missing.md", &LintConfig::default());

        // Non-CLAUDE.md files emit REF-001 only (not CC-MEM-001)
        assert!(diagnostics.iter().any(|d| d.rule == "REF-001"));
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-MEM-001"));
    }

    #[test]
    fn test_ref_001_existing_import() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("exists.md");
        let file_path = temp.path().join("test.md");
        fs::write(&target, "Target content").unwrap();
        fs::write(&file_path, "See @exists.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @exists.md", &LintConfig::default());

        // Should not emit any not-found errors
        assert!(!diagnostics.iter().any(|d| d.rule == "REF-001"));
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-MEM-001"));
    }

    #[test]
    fn test_ref_001_disabled() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        fs::write(&file_path, "See @missing.md").unwrap();

        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["REF-001".to_string()];

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @missing.md", &config);

        // Non-CLAUDE.md with REF-001 disabled emits nothing
        assert!(diagnostics.is_empty());
    }

    // ===== REF-002 Tests =====

    #[test]
    fn test_ref_002_broken_link() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        fs::write(&file_path, "See [guide](missing.md) for more.").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(
            &file_path,
            "See [guide](missing.md) for more.",
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "REF-002"));
        let ref_002 = diagnostics.iter().find(|d| d.rule == "REF-002").unwrap();
        assert!(ref_002.message.contains("Link target not found"));
    }

    #[test]
    fn test_ref_002_valid_link() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("exists.md");
        let file_path = temp.path().join("test.md");
        fs::write(&target, "Target content").unwrap();
        fs::write(&file_path, "See [guide](exists.md) for more.").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(
            &file_path,
            "See [guide](exists.md) for more.",
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "REF-002"));
    }

    #[test]
    fn test_ref_002_skips_external_links() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        let content = "See [GitHub](https://github.com) and [mail](mailto:test@example.com).";
        fs::write(&file_path, content).unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, content, &LintConfig::default());

        // External links should not trigger REF-002
        assert!(!diagnostics.iter().any(|d| d.rule == "REF-002"));
    }

    #[test]
    fn test_ref_002_skips_anchor_links() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        let content = "See [section](#section-name) for more.";
        fs::write(&file_path, content).unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, content, &LintConfig::default());

        // Pure anchor links should not trigger REF-002
        assert!(!diagnostics.iter().any(|d| d.rule == "REF-002"));
    }

    #[test]
    fn test_ref_002_link_with_fragment() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("exists.md");
        let file_path = temp.path().join("test.md");
        fs::write(&target, "# Section\nContent").unwrap();
        fs::write(&file_path, "See [section](exists.md#section) for more.").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(
            &file_path,
            "See [section](exists.md#section) for more.",
            &LintConfig::default(),
        );

        // File exists, fragment validation is not implemented - no error
        assert!(!diagnostics.iter().any(|d| d.rule == "REF-002"));
    }

    #[test]
    fn test_ref_002_missing_file_with_fragment() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        fs::write(&file_path, "See [section](missing.md#section) for more.").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(
            &file_path,
            "See [section](missing.md#section) for more.",
            &LintConfig::default(),
        );

        // File doesn't exist, should error
        assert!(diagnostics.iter().any(|d| d.rule == "REF-002"));
    }

    #[test]
    fn test_ref_002_broken_image() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        fs::write(&file_path, "![logo](images/logo.png)").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(
            &file_path,
            "![logo](images/logo.png)",
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "REF-002"));
        let ref_002 = diagnostics.iter().find(|d| d.rule == "REF-002").unwrap();
        assert!(ref_002.message.contains("Image target not found"));
    }

    #[test]
    fn test_ref_002_disabled() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        fs::write(&file_path, "See [guide](missing.md) for more.").unwrap();

        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["REF-002".to_string()];

        let validator = ImportsValidator;
        let diagnostics =
            validator.validate(&file_path, "See [guide](missing.md) for more.", &config);

        assert!(!diagnostics.iter().any(|d| d.rule == "REF-002"));
    }

    #[test]
    fn test_ref_002_imports_category_disabled() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        fs::write(&file_path, "See [guide](missing.md) for more.").unwrap();

        let mut config = LintConfig::default();
        config.rules.imports = false;

        let validator = ImportsValidator;
        let diagnostics =
            validator.validate(&file_path, "See [guide](missing.md) for more.", &config);

        // When imports category is disabled, no validation happens
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_ref_002_relative_path() {
        let temp = TempDir::new().unwrap();
        let subdir = temp.path().join("docs");
        fs::create_dir(&subdir).unwrap();
        let target = subdir.join("guide.md");
        let file_path = temp.path().join("test.md");
        fs::write(&target, "Guide content").unwrap();
        fs::write(&file_path, "See [guide](docs/guide.md) for more.").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(
            &file_path,
            "See [guide](docs/guide.md) for more.",
            &LintConfig::default(),
        );

        // Relative path should resolve correctly
        assert!(!diagnostics.iter().any(|d| d.rule == "REF-002"));
    }
}
