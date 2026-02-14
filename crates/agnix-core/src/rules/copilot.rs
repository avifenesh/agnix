//! GitHub Copilot validation rules (COP-001 to COP-018)
//!
//! Validates:
//! - COP-001: Empty instruction file (HIGH) - files must have content
//! - COP-002: Invalid frontmatter (HIGH) - scoped files require valid YAML with applyTo
//! - COP-003: Invalid glob pattern (HIGH) - applyTo must contain valid globs
//! - COP-004: Unknown frontmatter keys (MEDIUM) - warn about unrecognized keys
//! - COP-005: Invalid excludeAgent value (HIGH) - must be "code-review" or "coding-agent"
//! - COP-006: File length limit (MEDIUM) - global files should not exceed ~4000 characters
//! - COP-007 to COP-012: Custom agent validation
//! - COP-013 to COP-015: Reusable prompt validation
//! - COP-017: Hooks schema validation
//! - COP-018: Setup workflow validation

use crate::{
    FileType,
    config::LintConfig,
    diagnostics::{Diagnostic, Fix},
    rules::{Validator, ValidatorMetadata},
    schemas::{
        copilot::{is_body_empty, is_content_empty, parse_frontmatter, validate_glob_pattern},
        copilot_agent::parse_agent_frontmatter,
        copilot_hooks::{
            has_copilot_setup_steps_job, parse_hooks_json, parse_setup_steps_yaml,
            validate_hooks_schema,
        },
        copilot_prompt::{
            VALID_AGENT_MODES, is_body_empty as is_prompt_body_empty, parse_prompt_frontmatter,
        },
    },
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &[
    "COP-001", "COP-002", "COP-003", "COP-004", "COP-005", "COP-006", "COP-007", "COP-008",
    "COP-009", "COP-010", "COP-011", "COP-012", "COP-013", "COP-014", "COP-015", "COP-017",
    "COP-018",
];

pub struct CopilotValidator;

fn line_byte_range(content: &str, line_number: usize) -> Option<(usize, usize)> {
    if line_number == 0 {
        return None;
    }

    let mut current_line = 1usize;
    let mut line_start = 0usize;

    for (idx, ch) in content.char_indices() {
        if current_line == line_number && ch == '\n' {
            return Some((line_start, idx + 1));
        }
        if ch == '\n' {
            current_line += 1;
            line_start = idx + 1;
        }
    }

    if current_line == line_number {
        Some((line_start, content.len()))
    } else {
        None
    }
}

fn frontmatter_key_line(raw: &str, start_line: usize, key: &str) -> usize {
    raw.lines()
        .enumerate()
        .find(|(_, line)| line.trim_start().starts_with(key))
        .map(|(idx, _)| start_line + 1 + idx)
        .unwrap_or(start_line)
}

fn is_setup_steps_workflow(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some("copilot-setup-steps.yml") | Some("copilot-setup-steps.yaml")
    )
}

fn validate_custom_agent(path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let parsed = parse_agent_frontmatter(content);
    let body = parsed.as_ref().map_or(content, |p| p.body.as_str());

    if config.is_rule_enabled("COP-011") {
        const MAX_AGENT_BODY_CHARS: usize = 30_000;
        let body_len = body.chars().count();
        if body_len > MAX_AGENT_BODY_CHARS {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "COP-011",
                    format!(
                        "Custom agent prompt body exceeds {} characters (found {})",
                        MAX_AGENT_BODY_CHARS, body_len
                    ),
                )
                .with_suggestion("Reduce agent prompt size to 30000 characters or fewer."),
            );
        }
    }

    if let Some(parsed) = &parsed {
        if let Some(err) = &parsed.parse_error {
            if config.is_rule_enabled("COP-008") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        parsed.start_line,
                        0,
                        "COP-008",
                        format!("Custom agent frontmatter contains invalid YAML: {err}"),
                    )
                    .with_suggestion("Fix YAML syntax in custom agent frontmatter."),
                );
            }
            return diagnostics;
        }
    }

    if config.is_rule_enabled("COP-007") {
        let has_description = parsed
            .as_ref()
            .and_then(|p| p.schema.as_ref())
            .and_then(|s| s.description.as_ref())
            .is_some_and(|desc| !desc.trim().is_empty());
        if !has_description {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "COP-007",
                    "Custom agent frontmatter is missing required 'description' field",
                )
                .with_suggestion("Add a non-empty 'description' key in YAML frontmatter."),
            );
        }
    }

    if let Some(parsed) = &parsed {
        if config.is_rule_enabled("COP-008") {
            for unknown in &parsed.unknown_keys {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        unknown.line,
                        unknown.column,
                        "COP-008",
                        format!(
                            "Custom agent has unsupported frontmatter field '{}'",
                            unknown.key
                        ),
                    )
                    .with_suggestion(format!(
                        "Remove unknown frontmatter field '{}'.",
                        unknown.key
                    )),
                );
            }
        }

        if let Some(schema) = &parsed.schema {
            if config.is_rule_enabled("COP-009") {
                if let Some(target) = &schema.target {
                    const VALID_TARGETS: &[&str] = &["vscode", "github-copilot"];
                    if !VALID_TARGETS.contains(&target.as_str()) {
                        let line = frontmatter_key_line(&parsed.raw, parsed.start_line, "target:");
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                line,
                                0,
                                "COP-009",
                                format!(
                                    "Invalid custom agent target '{}'; expected 'vscode' or 'github-copilot'",
                                    target
                                ),
                            )
                            .with_suggestion("Set target to 'vscode' or 'github-copilot'."),
                        );
                    }
                }
            }

            if config.is_rule_enabled("COP-010") && schema.infer.is_some() {
                let line = frontmatter_key_line(&parsed.raw, parsed.start_line, "infer:");
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line,
                        0,
                        "COP-010",
                        "Custom agent uses deprecated 'infer' field",
                    )
                    .with_suggestion(
                        "Remove 'infer' and use user-invokable custom agents instead.",
                    ),
                );
            }

            if config.is_rule_enabled("COP-012") {
                let unsupported = [
                    ("model:", schema.model.is_some(), "model"),
                    (
                        "argument-hint:",
                        schema.argument_hint.is_some(),
                        "argument-hint",
                    ),
                    ("handoffs:", schema.handoffs.is_some(), "handoffs"),
                ];

                for (prefix, present, field_name) in unsupported {
                    if present {
                        let line = frontmatter_key_line(&parsed.raw, parsed.start_line, prefix);
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                line,
                                0,
                                "COP-012",
                                format!(
                                    "Field '{}' is unsupported on GitHub.com custom agents",
                                    field_name
                                ),
                            )
                            .with_suggestion(format!(
                                "Remove '{}' for GitHub.com compatibility.",
                                field_name
                            )),
                        );
                    }
                }
            }
        }
    }

    diagnostics
}

fn validate_reusable_prompt(path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let parsed = parse_prompt_frontmatter(content);
    let body = parsed.as_ref().map_or(content, |p| p.body.as_str());

    if config.is_rule_enabled("COP-013") && is_prompt_body_empty(body) {
        let line = parsed.as_ref().map_or(1, |p| p.end_line + 1);
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                line,
                0,
                "COP-013",
                "Prompt file body is empty",
            )
            .with_suggestion("Add prompt text below the optional frontmatter."),
        );
    }

    if let Some(parsed) = &parsed {
        if config.is_rule_enabled("COP-014") {
            if let Some(err) = &parsed.parse_error {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        parsed.start_line,
                        0,
                        "COP-014",
                        format!("Prompt frontmatter contains invalid YAML: {err}"),
                    )
                    .with_suggestion("Fix YAML syntax in prompt frontmatter."),
                );
                return diagnostics;
            }

            for unknown in &parsed.unknown_keys {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        unknown.line,
                        unknown.column,
                        "COP-014",
                        format!(
                            "Prompt file has unsupported frontmatter field '{}'",
                            unknown.key
                        ),
                    )
                    .with_suggestion(format!(
                        "Remove unknown frontmatter field '{}'.",
                        unknown.key
                    )),
                );
            }
        }

        if config.is_rule_enabled("COP-015") {
            if let Some(schema) = &parsed.schema {
                if let Some(agent_mode) = &schema.agent {
                    if !VALID_AGENT_MODES.contains(&agent_mode.as_str()) {
                        let line = frontmatter_key_line(&parsed.raw, parsed.start_line, "agent:");
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                line,
                                0,
                                "COP-015",
                                format!(
                                    "Invalid prompt agent mode '{}'; expected one of: none, ask, always",
                                    agent_mode
                                ),
                            )
                            .with_suggestion("Use agent mode 'none', 'ask', or 'always'."),
                        );
                    }
                }
            }
        }
    }

    diagnostics
}

fn validate_hooks_file(path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if is_setup_steps_workflow(path) {
        if !config.is_rule_enabled("COP-018") {
            return diagnostics;
        }

        match parse_setup_steps_yaml(content) {
            Ok(workflow) => {
                if !has_copilot_setup_steps_job(&workflow) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "COP-018",
                            "copilot-setup-steps workflow must define jobs.copilot-setup-steps with ubuntu runs-on and non-empty steps",
                        )
                        .with_suggestion(
                            "Define jobs.copilot-setup-steps with an Ubuntu runner and at least one step in .github/workflows/copilot-setup-steps.yml.",
                        ),
                    );
                }
            }
            Err(err) => diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "COP-018",
                    format!("Invalid copilot-setup-steps workflow YAML: {err}"),
                )
                .with_suggestion("Fix YAML syntax in copilot-setup-steps workflow."),
            ),
        }

        return diagnostics;
    }

    if !config.is_rule_enabled("COP-017") {
        return diagnostics;
    }

    match parse_hooks_json(content) {
        Ok(hooks) => {
            for error in validate_hooks_schema(&hooks) {
                diagnostics.push(
                    Diagnostic::error(path.to_path_buf(), 1, 0, "COP-017", error)
                        .with_suggestion("Fix hooks.json to match Copilot hooks schema."),
                );
            }
        }
        Err(err) => diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "COP-017",
                format!("Invalid hooks.json syntax: {err}"),
            )
            .with_suggestion("Fix JSON syntax in .github/hooks/hooks.json."),
        ),
    }

    diagnostics
}

impl Validator for CopilotValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let file_type = crate::detect_file_type(path);
        match file_type {
            FileType::CopilotAgent => return validate_custom_agent(path, content, config),
            FileType::CopilotPrompt => return validate_reusable_prompt(path, content, config),
            FileType::CopilotHooks => return validate_hooks_file(path, content, config),
            FileType::Copilot | FileType::CopilotScoped => {}
            _ => return diagnostics,
        }

        // Determine if this is global or scoped instruction file
        let is_scoped = file_type == FileType::CopilotScoped;
        let scoped_frontmatter = if is_scoped {
            parse_frontmatter(content)
        } else {
            None
        };

        // COP-001: Empty instruction file (ERROR)
        if config.is_rule_enabled("COP-001") {
            if is_scoped {
                // For scoped files, check body after frontmatter
                if let Some(parsed) = scoped_frontmatter.as_ref() {
                    if is_body_empty(&parsed.body) {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                parsed.end_line + 1,
                                0,
                                "COP-001",
                                t!("rules.cop_001.message_no_content"),
                            )
                            .with_suggestion(t!("rules.cop_001.suggestion_empty")),
                        );
                    }
                } else if is_content_empty(content) {
                    // Scoped file with no frontmatter and no content
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "COP-001",
                            t!("rules.cop_001.message_empty"),
                        )
                        .with_suggestion(t!("rules.cop_001.suggestion_scoped_empty")),
                    );
                }
            } else {
                // For global files, check entire content
                if is_content_empty(content) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "COP-001",
                            t!("rules.cop_001.message_empty"),
                        )
                        .with_suggestion(t!("rules.cop_001.suggestion_empty")),
                    );
                }
            }
        }

        // COP-006: File length limit for global files (WARNING)
        const COPILOT_GLOBAL_LENGTH_LIMIT: usize = 4000;
        if config.is_rule_enabled("COP-006") && !is_scoped {
            let char_count = content.chars().count();
            if char_count > COPILOT_GLOBAL_LENGTH_LIMIT {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        1,
                        0,
                        "COP-006",
                        t!("rules.cop_006.message", len = char_count),
                    )
                    .with_suggestion(t!("rules.cop_006.suggestion")),
                );
            }
        }

        // Rules COP-002, COP-003, COP-004, COP-005 only apply to scoped instruction files
        if !is_scoped {
            return diagnostics;
        }

        // Parse frontmatter for scoped files
        let parsed = match scoped_frontmatter {
            Some(p) => p,
            None => {
                // COP-002: Missing frontmatter in scoped file
                if config.is_rule_enabled("COP-002") && !is_content_empty(content) {
                    let mut diagnostic = Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "COP-002",
                        t!("rules.cop_002.message_missing"),
                    )
                    .with_suggestion(t!("rules.cop_002.suggestion_add_frontmatter"));

                    // Unsafe auto-fix: insert template frontmatter at start of file.
                    diagnostic = diagnostic.with_fix(Fix::insert(
                        0,
                        "---\napplyTo: \"**/*\"\n---\n",
                        t!("rules.cop_002.fix"),
                        false,
                    ));

                    diagnostics.push(diagnostic);
                }
                return diagnostics;
            }
        };

        // COP-002: Invalid frontmatter (YAML parse error)
        if config.is_rule_enabled("COP-002") {
            if let Some(ref error) = parsed.parse_error {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        parsed.start_line,
                        0,
                        "COP-002",
                        t!("rules.cop_002.message_invalid_yaml", error = error.as_str()),
                    )
                    .with_suggestion(t!("rules.cop_002.suggestion_fix_yaml")),
                );
                // Can't continue validating if YAML is broken
                return diagnostics;
            }

            // Check for missing applyTo field
            if let Some(ref schema) = parsed.schema {
                if schema.apply_to.is_none() {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            parsed.start_line,
                            0,
                            "COP-002",
                            t!("rules.cop_002.message_missing_apply_to"),
                        )
                        .with_suggestion(t!("rules.cop_002.suggestion_add_apply_to")),
                    );
                }
            }
        }

        // COP-003: Invalid glob pattern
        if config.is_rule_enabled("COP-003") {
            if let Some(ref schema) = parsed.schema {
                if let Some(ref apply_to) = schema.apply_to {
                    let validation = validate_glob_pattern(apply_to);
                    if !validation.valid {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                parsed.start_line + 1, // applyTo is typically on line 2
                                0,
                                "COP-003",
                                t!(
                                    "rules.cop_003.message",
                                    pattern = apply_to.as_str(),
                                    error = validation.error.unwrap_or_default()
                                ),
                            )
                            .with_suggestion(t!("rules.cop_003.suggestion")),
                        );
                    }
                }
            }
        }

        // COP-004: Unknown frontmatter keys (WARNING)
        if config.is_rule_enabled("COP-004") {
            for unknown in &parsed.unknown_keys {
                let mut diagnostic = Diagnostic::warning(
                    path.to_path_buf(),
                    unknown.line,
                    unknown.column,
                    "COP-004",
                    t!("rules.cop_004.message", key = unknown.key.as_str()),
                )
                .with_suggestion(t!("rules.cop_004.suggestion", key = unknown.key.as_str()));

                // Safe auto-fix: remove unknown top-level frontmatter key line.
                if let Some((start, end)) = line_byte_range(content, unknown.line) {
                    diagnostic = diagnostic.with_fix(Fix::delete(
                        start,
                        end,
                        format!("Remove unknown frontmatter key '{}'", unknown.key),
                        true,
                    ));
                }

                diagnostics.push(diagnostic);
            }
        }

        // COP-005: Invalid excludeAgent value (ERROR)
        if config.is_rule_enabled("COP-005") {
            if let Some(ref schema) = parsed.schema {
                if let Some(ref agent_value) = schema.exclude_agent {
                    const VALID_AGENTS: &[&str] = &["code-review", "coding-agent"];
                    if !VALID_AGENTS.contains(&agent_value.as_str()) {
                        // Find the line number of excludeAgent in raw frontmatter
                        let line = parsed
                            .raw
                            .lines()
                            .enumerate()
                            .find(|(_, l)| l.trim_start().starts_with("excludeAgent:"))
                            .map(|(i, _)| parsed.start_line + 1 + i)
                            .unwrap_or(parsed.start_line + 1);

                        let mut diagnostic = Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            0,
                            "COP-005",
                            t!("rules.cop_005.message", value = agent_value.as_str()),
                        )
                        .with_suggestion(t!("rules.cop_005.suggestion"));

                        // Unsafe auto-fix: replace with closest valid agent value
                        if let Some(closest) =
                            super::find_closest_value(agent_value.as_str(), VALID_AGENTS)
                        {
                            if let Some((start, end)) = crate::rules::find_yaml_value_range(
                                content,
                                &parsed,
                                "excludeAgent",
                                true,
                            ) {
                                let slice = content.get(start..end).unwrap_or("");
                                let replacement = if slice.starts_with('"') {
                                    format!("\"{}\"", closest)
                                } else if slice.starts_with('\'') {
                                    format!("'{}'", closest)
                                } else {
                                    closest.to_string()
                                };
                                diagnostic = diagnostic.with_fix(Fix::replace(
                                    start,
                                    end,
                                    replacement,
                                    t!("rules.cop_005.fix", fixed = closest),
                                    false,
                                ));
                            }
                        }

                        diagnostics.push(diagnostic);
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
    use crate::diagnostics::DiagnosticLevel;

    fn validate_global(content: &str) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/copilot-instructions.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_scoped(content: &str) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/instructions/typescript.instructions.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_scoped_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/instructions/typescript.instructions.md"),
            content,
            config,
        )
    }

    fn validate_agent(content: &str) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/agents/reviewer.agent.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_prompt(content: &str) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/prompts/refactor.prompt.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_hooks(content: &str) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/hooks/hooks.json"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_setup_steps(content: &str) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/workflows/copilot-setup-steps.yml"),
            content,
            &LintConfig::default(),
        )
    }

    // ===== COP-001: Empty Instruction File =====

    #[test]
    fn test_cop_001_empty_global_file() {
        let diagnostics = validate_global("");
        let cop_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-001").collect();
        assert_eq!(cop_001.len(), 1);
        assert_eq!(cop_001[0].level, DiagnosticLevel::Error);
        assert!(cop_001[0].message.contains("empty"));
    }

    #[test]
    fn test_cop_001_whitespace_only_global() {
        let diagnostics = validate_global("   \n\n\t  ");
        let cop_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-001").collect();
        assert_eq!(cop_001.len(), 1);
    }

    #[test]
    fn test_cop_001_valid_global_file() {
        let content = "# Copilot Instructions\n\nFollow the coding style guide.";
        let diagnostics = validate_global(content);
        let cop_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-001").collect();
        assert!(cop_001.is_empty());
    }

    #[test]
    fn test_cop_001_empty_scoped_body() {
        let content = r#"---
applyTo: "**/*.ts"
---
"#;
        let diagnostics = validate_scoped(content);
        let cop_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-001").collect();
        assert_eq!(cop_001.len(), 1);
        assert!(cop_001[0].message.contains("no content after frontmatter"));
    }

    #[test]
    fn test_cop_001_valid_scoped_file() {
        let content = r#"---
applyTo: "**/*.ts"
---
# TypeScript Instructions

Use strict mode.
"#;
        let diagnostics = validate_scoped(content);
        let cop_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-001").collect();
        assert!(cop_001.is_empty());
    }

    // ===== COP-002: Invalid Frontmatter =====

    #[test]
    fn test_cop_002_missing_frontmatter() {
        let content = "# Instructions without frontmatter";
        let diagnostics = validate_scoped(content);
        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        assert_eq!(cop_002.len(), 1);
        assert!(cop_002[0].message.contains("missing required frontmatter"));
    }

    #[test]
    fn test_cop_002_has_autofix_for_missing_frontmatter() {
        let content = "# Instructions without frontmatter";
        let diagnostics = validate_scoped(content);
        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        assert_eq!(cop_002.len(), 1);
        assert!(
            cop_002[0].has_fixes(),
            "COP-002 should have auto-fix for missing frontmatter"
        );
        let fix = &cop_002[0].fixes[0];
        assert!(!fix.safe, "COP-002 fix should be unsafe");
        assert_eq!(fix.start_byte, 0, "Fix should insert at start of file");
        assert_eq!(fix.end_byte, 0, "Fix should be an insert (start == end)");
        assert!(
            fix.replacement.contains("applyTo:"),
            "Fix should contain applyTo field"
        );
    }

    #[test]
    fn test_cop_002_no_autofix_for_yaml_error() {
        // YAML parse error should not get an insert-frontmatter fix
        let content = "---\napplyTo: [unclosed\n---\n# Body\n";
        let diagnostics = validate_scoped(content);
        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        assert_eq!(cop_002.len(), 1);
        // This diagnostic is about invalid YAML, not missing frontmatter, so no insert fix
        assert!(
            !cop_002[0].has_fixes(),
            "COP-002 should not have auto-fix for YAML parse errors"
        );
    }

    #[test]
    fn test_cop_002_invalid_yaml() {
        let content = r#"---
applyTo: [unclosed
---
# Body
"#;
        let diagnostics = validate_scoped(content);
        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        assert_eq!(cop_002.len(), 1);
        assert!(cop_002[0].message.contains("Invalid YAML"));
    }

    #[test]
    fn test_cop_002_missing_apply_to() {
        let content = r#"---
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        assert_eq!(cop_002.len(), 1);
        assert!(cop_002[0].message.contains("missing required 'applyTo'"));
    }

    #[test]
    fn test_cop_002_valid_frontmatter() {
        let content = r#"---
applyTo: "**/*.ts"
---
# TypeScript Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        assert!(cop_002.is_empty());
    }

    // ===== COP-003: Invalid Glob Pattern =====

    #[test]
    fn test_cop_003_invalid_glob() {
        let content = r#"---
applyTo: "[unclosed"
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-003").collect();
        assert_eq!(cop_003.len(), 1);
        assert!(cop_003[0].message.contains("Invalid glob pattern"));
    }

    #[test]
    fn test_cop_003_valid_glob_patterns() {
        let patterns = vec!["**/*.ts", "*.rs", "src/**/*.js", "tests/**/*.test.ts"];

        for pattern in patterns {
            let content = format!(
                r#"---
applyTo: "{}"
---
# Instructions
"#,
                pattern
            );
            let diagnostics = validate_scoped(&content);
            let cop_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-003").collect();
            assert!(cop_003.is_empty(), "Pattern '{}' should be valid", pattern);
        }
    }

    // ===== COP-004: Unknown Frontmatter Keys =====

    #[test]
    fn test_cop_004_unknown_keys() {
        let content = r#"---
applyTo: "**/*.ts"
unknownKey: value
anotherBadKey: 123
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-004").collect();
        assert_eq!(cop_004.len(), 2);
        assert_eq!(cop_004[0].level, DiagnosticLevel::Warning);
        assert!(cop_004.iter().any(|d| d.message.contains("unknownKey")));
        assert!(cop_004.iter().any(|d| d.message.contains("anotherBadKey")));
        assert!(
            cop_004.iter().all(|d| d.has_fixes()),
            "All unknown key diagnostics should include safe deletion fixes"
        );
        assert!(cop_004.iter().all(|d| d.fixes[0].safe));
    }

    #[test]
    fn test_cop_004_no_unknown_keys() {
        let content = r#"---
applyTo: "**/*.rs"
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-004").collect();
        assert!(cop_004.is_empty());
    }

    // ===== Global vs Scoped Behavior =====

    #[test]
    fn test_global_file_no_frontmatter_rules() {
        // Global files should not trigger COP-002/003/004
        let content = "# Instructions without frontmatter";
        let diagnostics = validate_global(content);

        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        let cop_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-003").collect();
        let cop_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-004").collect();

        assert!(cop_002.is_empty());
        assert!(cop_003.is_empty());
        assert!(cop_004.is_empty());
    }

    // ===== Config Integration =====

    #[test]
    fn test_config_disabled_copilot_category() {
        let mut config = LintConfig::default();
        config.rules_mut().copilot = false;

        let content = "";
        let diagnostics = validate_scoped_with_config(content, &config);

        let cop_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("COP-"))
            .collect();
        assert!(cop_rules.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["COP-001".to_string()];

        let content = "";
        let diagnostics = validate_scoped_with_config(content, &config);

        let cop_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-001").collect();
        assert!(cop_001.is_empty());
    }

    // ===== Combined Issues =====

    #[test]
    fn test_multiple_issues() {
        let content = r#"---
unknownKey: value
---
"#;
        let diagnostics = validate_scoped(content);

        // Should have:
        // - COP-001 for empty body
        // - COP-002 for missing applyTo
        // - COP-004 for unknown key
        assert!(
            diagnostics.iter().any(|d| d.rule == "COP-001"),
            "Expected COP-001"
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "COP-002"),
            "Expected COP-002"
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "COP-004"),
            "Expected COP-004"
        );
    }

    #[test]
    fn test_valid_scoped_no_issues() {
        let content = r#"---
applyTo: "**/*.ts"
---
# TypeScript Guidelines

Always use strict mode and explicit types.
"#;
        let diagnostics = validate_scoped(content);
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .collect();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    // ===== Additional COP rule tests =====

    #[test]
    fn test_cop_001_newlines_only() {
        let content = "\n\n\n";
        let diagnostics = validate_global(content);
        assert!(diagnostics.iter().any(|d| d.rule == "COP-001"));
    }

    #[test]
    fn test_cop_001_spaces_and_tabs() {
        let content = "   \t\t   ";
        let diagnostics = validate_global(content);
        assert!(diagnostics.iter().any(|d| d.rule == "COP-001"));
    }

    #[test]
    fn test_cop_002_yaml_with_tabs() {
        // YAML doesn't allow tabs for indentation
        let content = "---\n\tapplyTo: \"**/*.ts\"\n---\nBody";
        let diagnostics = validate_scoped(content);
        assert!(diagnostics.iter().any(|d| d.rule == "COP-002"));
    }

    #[test]
    fn test_cop_002_valid_frontmatter_no_error() {
        // Test that valid frontmatter doesn't trigger COP-002
        let content = r#"---
applyTo: "**/*.ts"
---
Body content"#;
        let diagnostics = validate_scoped(content);
        // Valid frontmatter should not trigger COP-002
        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        assert!(
            cop_002.is_empty(),
            "Valid frontmatter should not trigger COP-002"
        );
    }

    #[test]
    fn test_cop_003_all_valid_patterns() {
        let valid_patterns = [
            "**/*.ts",
            "*.rs",
            "src/**/*.py",
            "tests/*.test.js",
            "{src,lib}/**/*.ts",
        ];

        for pattern in valid_patterns {
            let content = format!("---\napplyTo: \"{}\"\n---\nBody", pattern);
            let diagnostics = validate_scoped(&content);
            let cop_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-003").collect();
            assert!(cop_003.is_empty(), "Pattern '{}' should be valid", pattern);
        }
    }

    #[test]
    fn test_cop_003_invalid_patterns() {
        let invalid_patterns = ["[invalid", "***", "**["];

        for pattern in invalid_patterns {
            let content = format!("---\napplyTo: \"{}\"\n---\nBody", pattern);
            let diagnostics = validate_scoped(&content);
            let cop_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-003").collect();
            assert!(
                !cop_003.is_empty(),
                "Pattern '{}' should be invalid",
                pattern
            );
        }
    }

    #[test]
    fn test_cop_004_all_known_keys() {
        let content = r#"---
applyTo: "**/*.ts"
---
Body"#;
        let diagnostics = validate_scoped(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "COP-004"));
    }

    #[test]
    fn test_cop_004_multiple_unknown_keys() {
        let content = r#"---
applyTo: "**/*.ts"
unknownKey1: value1
unknownKey2: value2
---
Body"#;
        let diagnostics = validate_scoped(content);
        let cop_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-004").collect();
        // Should report at least one unknown key warning
        assert!(!cop_004.is_empty());
    }

    #[test]
    fn test_all_cop_rules_can_be_disabled() {
        let rules = [
            "COP-001", "COP-002", "COP-003", "COP-004", "COP-005", "COP-006",
        ];
        let long_content = make_long_content();

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules_mut().disabled_rules = vec![rule.to_string()];

            // Content and path that could trigger each rule
            let (content, path): (&str, &str) = match rule {
                "COP-001" => ("", ".github/copilot-instructions.md"),
                "COP-002" => (
                    "Content without frontmatter",
                    ".github/instructions/test.instructions.md",
                ),
                "COP-003" => (
                    "---\napplyTo: \"[invalid\"\n---\nBody",
                    ".github/instructions/test.instructions.md",
                ),
                "COP-004" => (
                    "---\nunknown: value\n---\nBody",
                    ".github/instructions/test.instructions.md",
                ),
                "COP-005" => (
                    "---\napplyTo: \"**/*.ts\"\nexcludeAgent: \"invalid\"\n---\nBody",
                    ".github/instructions/test.instructions.md",
                ),
                "COP-006" => (&long_content, ".github/copilot-instructions.md"),
                _ => unreachable!("Unknown rule: {rule}"),
            };

            let validator = CopilotValidator;
            let diagnostics = validator.validate(Path::new(path), content, &config);

            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }

    /// Generate long content for COP-006 tests (>4000 chars)
    fn make_long_content() -> String {
        let mut s = String::from("# Copilot Instructions\n\n");
        while s.len() <= 4001 {
            s.push_str("Follow consistent naming conventions for variables and functions.\n");
        }
        s
    }

    fn validate_global_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/copilot-instructions.md"),
            content,
            config,
        )
    }

    // ===== COP-005: Invalid excludeAgent Value =====

    #[test]
    fn test_cop_005_invalid_exclude_agent() {
        let content = r#"---
applyTo: "**/*.ts"
excludeAgent: "invalid-agent"
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert_eq!(cop_005.len(), 1);
        assert_eq!(cop_005[0].level, DiagnosticLevel::Error);
        assert!(cop_005[0].message.contains("invalid-agent"));
    }

    #[test]
    fn test_cop_005_valid_code_review() {
        let content = r#"---
applyTo: "**/*.ts"
excludeAgent: "code-review"
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert!(cop_005.is_empty());
    }

    #[test]
    fn test_cop_005_valid_coding_agent() {
        let content = r#"---
applyTo: "**/*.ts"
excludeAgent: "coding-agent"
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert!(cop_005.is_empty());
    }

    #[test]
    fn test_cop_005_absent_exclude_agent() {
        let content = r#"---
applyTo: "**/*.ts"
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert!(cop_005.is_empty());
    }

    #[test]
    fn test_cop_005_global_file_no_trigger() {
        let content = r#"---
applyTo: "**/*.ts"
excludeAgent: "invalid-agent"
---
# Instructions
"#;
        // Global files should not trigger COP-005 (scoped-only rule)
        let diagnostics = validate_global(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert!(cop_005.is_empty());
    }

    #[test]
    fn test_cop_005_case_sensitive() {
        let content =
            "---\napplyTo: \"**/*.ts\"\nexcludeAgent: \"Code-Review\"\n---\n# Instructions\n";
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert_eq!(cop_005.len(), 1, "Mixed-case value should trigger COP-005");
        // Case-insensitive match should produce an auto-fix
        assert!(
            cop_005[0].has_fixes(),
            "COP-005 should have auto-fix for case mismatch"
        );
        let fix = &cop_005[0].fixes[0];
        assert!(!fix.safe, "COP-005 fix should be unsafe");
        assert!(
            fix.replacement.contains("code-review"),
            "Fix should suggest 'code-review', got: {}",
            fix.replacement
        );
    }

    #[test]
    fn test_cop_005_empty_string() {
        let content = "---\napplyTo: \"**/*.ts\"\nexcludeAgent: \"\"\n---\n# Instructions\n";
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert_eq!(
            cop_005.len(),
            1,
            "Empty excludeAgent should trigger COP-005"
        );
        // Empty string should NOT get a fix (no close match)
        assert!(
            !cop_005[0].has_fixes(),
            "COP-005 should not auto-fix empty string"
        );
    }

    #[test]
    fn test_cop_005_autofix_nonsense() {
        let content =
            "---\napplyTo: \"**/*.ts\"\nexcludeAgent: \"nonsense\"\n---\n# Instructions\n";
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert_eq!(cop_005.len(), 1);
        // "nonsense" has no close match - should NOT get a fix
        assert!(
            !cop_005[0].has_fixes(),
            "COP-005 should not auto-fix nonsense values"
        );
    }

    // ===== COP-006: File Length Limit =====

    #[test]
    fn test_cop_006_short_file() {
        let content = "# Short copilot instructions\n\nFollow the coding standards.";
        let diagnostics = validate_global(content);
        let cop_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-006").collect();
        assert!(cop_006.is_empty());
    }

    #[test]
    fn test_cop_006_long_file() {
        let long_content = make_long_content();
        let expected_len = long_content.len().to_string();
        let diagnostics = validate_global(&long_content);
        let cop_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-006").collect();
        assert_eq!(cop_006.len(), 1);
        assert_eq!(cop_006[0].level, DiagnosticLevel::Warning);
        assert!(
            cop_006[0].message.contains(&expected_len),
            "Diagnostic message should contain the file length"
        );
    }

    #[test]
    fn test_cop_006_exact_boundary() {
        // 4000 chars should pass
        let content_4000 = "x".repeat(4000);
        let diagnostics = validate_global(&content_4000);
        let cop_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-006").collect();
        assert!(cop_006.is_empty(), "4000 chars should not trigger COP-006");

        // 4001 chars should warn
        let content_4001 = "x".repeat(4001);
        let diagnostics = validate_global(&content_4001);
        let cop_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-006").collect();
        assert_eq!(cop_006.len(), 1, "4001 chars should trigger COP-006");
    }

    #[test]
    fn test_cop_006_scoped_file_no_trigger() {
        // Scoped files should not trigger COP-006
        let mut content = String::from("---\napplyTo: \"**/*.ts\"\n---\n# Instructions\n\n");
        while content.len() <= 5000 {
            content.push_str("Follow consistent naming conventions for all variables.\n");
        }
        let diagnostics = validate_scoped(&content);
        let cop_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-006").collect();
        assert!(cop_006.is_empty());
    }

    #[test]
    fn test_cop_006_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["COP-006".to_string()];

        let diagnostics = validate_global_with_config(&make_long_content(), &config);
        let cop_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-006").collect();
        assert!(cop_006.is_empty());
    }

    // ===== COP-007..COP-015, COP-017, COP-018 =====

    #[test]
    fn test_cop_007_missing_description() {
        let diagnostics = validate_agent(
            r#"---
target: vscode
---
Review pull requests.
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-007"));
    }

    #[test]
    fn test_cop_008_unknown_agent_field() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
unknown-field: true
---
Review pull requests.
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-008"));
    }

    #[test]
    fn test_cop_008_invalid_agent_frontmatter_yaml() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
target: [vscode
---
Review pull requests.
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-008"));
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("invalid YAML"))
        );
        assert!(
            diagnostics.iter().all(|d| d.rule != "COP-007"),
            "Invalid YAML should not be reported as missing description"
        );
    }

    #[test]
    fn test_cop_009_invalid_target() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
target: desktop
---
Review pull requests.
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-009"));
    }

    #[test]
    fn test_cop_010_deprecated_infer() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
infer: true
---
Review pull requests.
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-010"));
    }

    #[test]
    fn test_cop_011_agent_body_length_limit() {
        let long_body = "x".repeat(30_001);
        let diagnostics =
            validate_agent(&format!("---\ndescription: Long agent\n---\n{}", long_body));
        assert!(diagnostics.iter().any(|d| d.rule == "COP-011"));
    }

    #[test]
    fn test_cop_012_github_unsupported_fields() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
model: gpt-4
---
Review pull requests.
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-012"));
    }

    #[test]
    fn test_cop_013_empty_prompt_body() {
        let diagnostics = validate_prompt(
            r#"---
description: Refactor selected code
---
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-013"));
    }

    #[test]
    fn test_cop_014_unknown_prompt_field() {
        let diagnostics = validate_prompt(
            r#"---
description: Refactor selected code
mystery: true
---
Refactor the selected code.
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-014"));
    }

    #[test]
    fn test_cop_014_invalid_prompt_frontmatter_yaml() {
        let diagnostics = validate_prompt(
            r#"---
description: Refactor selected code
agent: [ask
---
Refactor the selected code.
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-014"));
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("invalid YAML"))
        );
    }

    #[test]
    fn test_cop_015_invalid_prompt_agent_mode() {
        let diagnostics = validate_prompt(
            r#"---
description: Refactor selected code
agent: maybe
---
Refactor the selected code.
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-015"));
    }

    #[test]
    fn test_cop_017_hooks_schema_validation() {
        let diagnostics = validate_hooks(
            r#"{
  "version": 1,
  "hooks": [
    { "type": "command", "events": ["notReal"], "command": { "bash": "echo hi" } }
  ]
}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-017"));
    }

    #[test]
    fn test_cop_017_invalid_hooks_json_syntax() {
        let diagnostics = validate_hooks("{");
        assert!(diagnostics.iter().any(|d| d.rule == "COP-017"));
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("Invalid hooks.json syntax"))
        );
    }

    #[test]
    fn test_cop_018_setup_steps_requires_copilot_setup_steps_job() {
        let diagnostics = validate_setup_steps(
            r#"
name: Copilot Setup Steps
jobs:
  build:
    runs-on: ubuntu-latest
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-018"));
    }

    #[test]
    fn test_cop_018_invalid_setup_workflow_yaml() {
        let diagnostics = validate_setup_steps("name: Copilot Setup Steps\njobs: [");
        assert!(diagnostics.iter().any(|d| d.rule == "COP-018"));
        assert!(diagnostics.iter().any(|d| {
            d.message
                .contains("Invalid copilot-setup-steps workflow YAML")
        }));
    }
}
