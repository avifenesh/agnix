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
    let check_not_found = config.is_rule_enabled("CC-MEM-001");
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
            if check_not_found {
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
}
