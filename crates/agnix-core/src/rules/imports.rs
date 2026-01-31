//! @import reference validation

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    parsers::markdown::extract_imports,
    rules::Validator,
};
use std::path::{Path, PathBuf};

pub struct ImportsValidator;

impl Validator for ImportsValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Check both new category flag and legacy flag for backward compatibility
        if config.is_rule_enabled("imports::not_found") && config.rules.import_references {
            let imports = extract_imports(content);
            let base_dir = path.parent().unwrap_or(Path::new("."));

            for import in imports {
                // Resolve path relative to the file
                let import_path = if import.path.starts_with('~') {
                    // Home directory
                    if let Some(home) = dirs::home_dir() {
                        home.join(&import.path[2..])
                    } else {
                        PathBuf::from(&import.path)
                    }
                } else {
                    base_dir.join(&import.path)
                };

                if !import_path.exists() {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            import.line,
                            import.column,
                            "imports::not_found",
                            format!("Import not found: @{}", import.path),
                        )
                        .with_suggestion(format!(
                            "Check that the file exists: {}",
                            import_path.display()
                        )),
                    );
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
}
