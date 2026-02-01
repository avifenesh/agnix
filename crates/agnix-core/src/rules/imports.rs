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
    parsers::markdown::{extract_imports, extract_markdown_links, Import},
    rules::Validator,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct ImportsValidator;

const MAX_IMPORT_DEPTH: usize = 5;

/// Check if a URL is a local file link (not external or anchor-only)
fn is_local_file_link(url: &str) -> bool {
    // Skip external URLs
    if url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("mailto:")
        || url.starts_with("tel:")
        || url.starts_with("data:")
        || url.starts_with("ftp://")
        || url.starts_with("//")
    {
        return false;
    }

    // Skip pure anchor links
    if url.starts_with('#') {
        return false;
    }

    // Skip empty URLs
    if url.is_empty() {
        return false;
    }

    true
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
        );

        // Validate markdown links (REF-002)
        validate_markdown_links(path, content, config, &mut diagnostics);

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
    let check_cc_mem_001 = config.is_rule_enabled("CC-MEM-001");
    let check_ref_001 = config.is_rule_enabled("REF-001");
    let check_not_found = check_cc_mem_001 || check_ref_001;
    let check_cycle = config.is_rule_enabled("CC-MEM-002");
    let check_depth = config.is_rule_enabled("CC-MEM-003");

    if !(check_not_found || check_cycle || check_depth) {
        return;
    }

    stack.push(file_path.clone());

    for import in imports {
        let resolved = resolve_import_path(&import.path, base_dir);
        let normalized = if resolved.exists() {
            normalize_existing_path(&resolved)
        } else {
            resolved
        };

        if !normalized.exists() {
            // Emit REF-001 (universal rule) if enabled
            if check_ref_001 {
                diagnostics.push(
                    Diagnostic::error(
                        file_path.clone(),
                        import.line,
                        import.column,
                        "REF-001",
                        format!("Import file not found: @{}", import.path),
                    )
                    .with_suggestion(format!(
                        "Check that the file exists: {}",
                        normalized.display()
                    )),
                );
            }
            // Also emit CC-MEM-001 (Claude Code specific) for backward compatibility
            if check_cc_mem_001 {
                diagnostics.push(
                    Diagnostic::error(
                        file_path.clone(),
                        import.line,
                        import.column,
                        "CC-MEM-001",
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

        if check_cycle && stack.contains(&normalized) {
            let cycle = format_cycle(stack, &normalized);
            diagnostics.push(
                Diagnostic::error(
                    file_path.clone(),
                    import.line,
                    import.column,
                    "CC-MEM-002",
                    format!("Circular @import detected: {}", cycle),
                )
                .with_suggestion("Remove or break the circular @import chain".to_string()),
            );
            continue;
        }

        if check_depth && depth + 1 > MAX_IMPORT_DEPTH {
            diagnostics.push(
                Diagnostic::error(
                    file_path.clone(),
                    import.line,
                    import.column,
                    "CC-MEM-003",
                    format!(
                        "Import depth exceeds {} hops at @{}",
                        MAX_IMPORT_DEPTH, import.path
                    ),
                )
                .with_suggestion("Flatten or shorten the @import chain".to_string()),
            );
            continue;
        }

        if check_cycle || check_depth {
            visit_imports(
                &normalized,
                None,
                cache,
                visited_depth,
                stack,
                diagnostics,
                config,
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

        // Skip if only fragment was left (e.g., "#section")
        if file_path.is_empty() {
            continue;
        }

        // Resolve the path relative to the file's directory
        let resolved = resolve_import_path(file_path, base_dir);

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
    fn test_missing_import() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("a.md");
        fs::write(&file_path, "See @missing.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @missing.md", &LintConfig::default());

        assert!(diagnostics.iter().any(|d| d.rule == "CC-MEM-001"));
    }

    #[test]
    fn test_cycle_detection() {
        let temp = TempDir::new().unwrap();
        let a = temp.path().join("a.md");
        let b = temp.path().join("b.md");
        fs::write(&a, "See @b.md").unwrap();
        fs::write(&b, "See @a.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&a, "See @b.md", &LintConfig::default());

        assert!(diagnostics.iter().any(|d| d.rule == "CC-MEM-002"));
    }

    #[test]
    fn test_depth_exceeded() {
        let temp = TempDir::new().unwrap();
        let paths: Vec<PathBuf> = (0..7)
            .map(|i| temp.path().join(format!("{}.md", i)))
            .collect();

        for i in 0..6 {
            let content = format!("See @{}.md", i + 1);
            fs::write(&paths[i], content).unwrap();
        }
        fs::write(&paths[6], "End").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&paths[0], "See @1.md", &LintConfig::default());

        assert!(diagnostics.iter().any(|d| d.rule == "CC-MEM-003"));
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

        // Should emit both REF-001 and CC-MEM-001
        assert!(diagnostics.iter().any(|d| d.rule == "REF-001"));
        assert!(diagnostics.iter().any(|d| d.rule == "CC-MEM-001"));
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

        // REF-001 should not be emitted, but CC-MEM-001 should
        assert!(!diagnostics.iter().any(|d| d.rule == "REF-001"));
        assert!(diagnostics.iter().any(|d| d.rule == "CC-MEM-001"));
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
