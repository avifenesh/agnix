//! CLAUDE.md validation

use crate::{
    config::LintConfig, diagnostics::Diagnostic, rules::Validator,
    schemas::claude_md::find_generic_instructions,
};
use std::path::Path;

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
}
