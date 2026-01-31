//! Hooks validation rules (CC-HK-006 to CC-HK-009)

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::Validator,
    schemas::hooks::{Hook, SettingsSchema},
};
use regex::Regex;
use std::path::Path;

pub struct HooksValidator;

impl HooksValidator {
    const DANGEROUS_PATTERNS: &'static [(&'static str, &'static str)] = &[
        (r"rm\s+-rf\s+/", "Recursive delete from root is extremely dangerous"),
        (r"rm\s+-rf\s+\*", "Recursive delete with wildcard could delete unintended files"),
        (r"rm\s+-rf\s+\.\.", "Recursive delete of parent directories is dangerous"),
        (r"git\s+reset\s+--hard", "Hard reset discards uncommitted changes permanently"),
        (r"git\s+clean\s+-fd", "Git clean -fd removes untracked files permanently"),
        (r"git\s+push\s+.*--force", "Force push can overwrite remote history"),
        (r"drop\s+database", "Dropping database is irreversible"),
        (r"drop\s+table", "Dropping table is irreversible"),
        (r"truncate\s+table", "Truncating table deletes all data"),
        (r"curl\s+.*\|\s*sh", "Piping curl to shell is a security risk"),
        (r"curl\s+.*\|\s*bash", "Piping curl to bash is a security risk"),
        (r"wget\s+.*\|\s*sh", "Piping wget to shell is a security risk"),
        (r"chmod\s+777", "chmod 777 gives everyone full access"),
        (r">\s*/dev/sd[a-z]", "Writing directly to block devices can destroy data"),
        (r"mkfs\.", "Formatting filesystem destroys all data"),
        (r"dd\s+if=.*of=/dev/", "dd to device can destroy data"),
    ];

    fn check_dangerous_patterns(&self, command: &str) -> Option<(&'static str, &'static str)> {
        for (pattern, reason) in Self::DANGEROUS_PATTERNS {
            if let Ok(re) = Regex::new(&format!("(?i){}", pattern)) {
                if re.is_match(command) {
                    return Some((pattern, reason));
                }
            }
        }
        None
    }

    fn extract_script_path(&self, command: &str) -> Option<String> {
        let script_patterns = [
            r"([^\s]+\.sh)\b",
            r"([^\s]+\.bash)\b",
            r"([^\s]+\.py)\b",
            r"([^\s]+\.js)\b",
            r"([^\s]+\.ts)\b",
        ];

        for pattern in &script_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(caps) = re.captures(command) {
                    if let Some(m) = caps.get(1) {
                        let path = m.as_str();
                        if path.contains("://") || path.starts_with("http") {
                            continue;
                        }
                        return Some(path.to_string());
                    }
                }
            }
        }
        None
    }

    fn resolve_script_path(&self, script_path: &str, project_dir: &Path) -> std::path::PathBuf {
        let resolved = script_path
            .replace("$CLAUDE_PROJECT_DIR", &project_dir.display().to_string())
            .replace("${CLAUDE_PROJECT_DIR}", &project_dir.display().to_string());

        let path = std::path::PathBuf::from(&resolved);

        if path.is_relative() {
            project_dir.join(path)
        } else {
            path
        }
    }
}

impl Validator for HooksValidator {
    fn validate(&self, path: &Path, content: &str, _config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let settings: SettingsSchema = match serde_json::from_str(content) {
            Ok(s) => s,
            Err(e) => {
                diagnostics.push(Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "hooks::parse",
                    format!("Failed to parse hooks configuration: {}", e),
                ));
                return diagnostics;
            }
        };

        let project_dir = path
            .parent()
            .and_then(|p| {
                if p.ends_with(".claude") {
                    p.parent()
                } else {
                    Some(p)
                }
            })
            .unwrap_or_else(|| Path::new("."));

        for (event, matchers) in &settings.hooks {
            for (matcher_idx, matcher) in matchers.iter().enumerate() {
                for (hook_idx, hook) in matcher.hooks.iter().enumerate() {
                    let hook_location = format!(
                        "hooks.{}{}.hooks[{}]",
                        event,
                        matcher
                            .matcher
                            .as_ref()
                            .map(|m| format!("[matcher={}]", m))
                            .unwrap_or_else(|| format!("[{}]", matcher_idx)),
                        hook_idx
                    );

                    match hook {
                        Hook::Command { command, .. } => {
                            if command.is_none() {
                                diagnostics.push(
                                    Diagnostic::error(
                                        path.to_path_buf(),
                                        1,
                                        0,
                                        "CC-HK-006",
                                        format!(
                                            "Command hook at {} is missing required 'command' field",
                                            hook_location
                                        ),
                                    )
                                    .with_suggestion(
                                        "Add a 'command' field with the command to execute"
                                            .to_string(),
                                    ),
                                );
                            } else if let Some(cmd) = command {
                                if let Some(script_path) = self.extract_script_path(cmd) {
                                    let resolved =
                                        self.resolve_script_path(&script_path, project_dir);

                                    if !script_path.contains('$')
                                        || script_path.contains("CLAUDE_PROJECT_DIR")
                                    {
                                        if !resolved.exists() {
                                            diagnostics.push(
                                                Diagnostic::error(
                                                    path.to_path_buf(),
                                                    1,
                                                    0,
                                                    "CC-HK-008",
                                                    format!(
                                                        "Script file not found at '{}' (resolved to '{}')",
                                                        script_path,
                                                        resolved.display()
                                                    ),
                                                )
                                                .with_suggestion(format!(
                                                    "Create the script file or correct the path"
                                                )),
                                            );
                                        }
                                    }
                                }

                                if let Some((pattern, reason)) =
                                    self.check_dangerous_patterns(cmd)
                                {
                                    diagnostics.push(
                                        Diagnostic::warning(
                                            path.to_path_buf(),
                                            1,
                                            0,
                                            "CC-HK-009",
                                            format!(
                                                "Potentially dangerous command pattern detected: {}",
                                                reason
                                            ),
                                        )
                                        .with_suggestion(format!(
                                            "Review the command for safety. Pattern matched: {}",
                                            pattern
                                        )),
                                    );
                                }
                            }
                        }
                        Hook::Prompt { prompt, .. } => {
                            if prompt.is_none() {
                                diagnostics.push(
                                    Diagnostic::error(
                                        path.to_path_buf(),
                                        1,
                                        0,
                                        "CC-HK-007",
                                        format!(
                                            "Prompt hook at {} is missing required 'prompt' field",
                                            hook_location
                                        ),
                                    )
                                    .with_suggestion(
                                        "Add a 'prompt' field with the prompt text".to_string(),
                                    ),
                                );
                            }
                        }
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

    fn validate(content: &str) -> Vec<Diagnostic> {
        let validator = HooksValidator;
        validator.validate(
            Path::new("settings.json"),
            content,
            &LintConfig::default(),
        )
    }

    #[test]
    fn test_cc_hk_006_command_hook_missing_command() {
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-HK-006").collect();

        assert_eq!(cc_hk_006.len(), 1);
        assert_eq!(cc_hk_006[0].level, DiagnosticLevel::Error);
        assert!(cc_hk_006[0].message.contains("missing required 'command' field"));
    }

    #[test]
    fn test_cc_hk_006_command_hook_with_command_ok() {
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "echo hello" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-HK-006").collect();

        assert_eq!(cc_hk_006.len(), 0);
    }

    #[test]
    fn test_cc_hk_006_multiple_command_hooks_missing_command() {
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "*",
                        "hooks": [
                            { "type": "command" },
                            { "type": "command", "command": "valid" },
                            { "type": "command" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-HK-006").collect();

        assert_eq!(cc_hk_006.len(), 2);
    }

    #[test]
    fn test_cc_hk_007_prompt_hook_missing_prompt() {
        let content = r#"{
            "hooks": {
                "Stop": [
                    {
                        "hooks": [
                            { "type": "prompt" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-HK-007").collect();

        assert_eq!(cc_hk_007.len(), 1);
        assert_eq!(cc_hk_007[0].level, DiagnosticLevel::Error);
        assert!(cc_hk_007[0].message.contains("missing required 'prompt' field"));
    }

    #[test]
    fn test_cc_hk_007_prompt_hook_with_prompt_ok() {
        let content = r#"{
            "hooks": {
                "Stop": [
                    {
                        "hooks": [
                            { "type": "prompt", "prompt": "Summarize the session" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-HK-007").collect();

        assert_eq!(cc_hk_007.len(), 0);
    }

    #[test]
    fn test_cc_hk_007_mixed_hooks_one_missing_prompt() {
        let content = r#"{
            "hooks": {
                "SubagentStop": [
                    {
                        "hooks": [
                            { "type": "prompt", "prompt": "valid prompt" },
                            { "type": "prompt" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-HK-007").collect();

        assert_eq!(cc_hk_007.len(), 1);
    }

    #[test]
    fn test_cc_hk_008_script_file_not_found() {
        let content = r#"{
            "hooks": {
                "SessionStart": [
                    {
                        "hooks": [
                            { "type": "command", "command": "bash scripts/nonexistent.sh" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-HK-008").collect();

        assert_eq!(cc_hk_008.len(), 1);
        assert_eq!(cc_hk_008[0].level, DiagnosticLevel::Error);
        assert!(cc_hk_008[0].message.contains("Script file not found"));
    }

    #[test]
    fn test_cc_hk_008_system_command_no_script_ok() {
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "echo 'logging tool use'" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-HK-008").collect();

        assert_eq!(cc_hk_008.len(), 0);
    }

    #[test]
    fn test_cc_hk_008_env_var_with_unresolvable_path_skipped() {
        let content = r#"{
            "hooks": {
                "SessionStart": [
                    {
                        "hooks": [
                            { "type": "command", "command": "$HOME/scripts/setup.sh" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-HK-008").collect();

        assert_eq!(cc_hk_008.len(), 0);
    }

    #[test]
    fn test_cc_hk_008_python_script_not_found() {
        let content = r#"{
            "hooks": {
                "PostToolUse": [
                    {
                        "matcher": "*",
                        "hooks": [
                            { "type": "command", "command": "python hooks/logger.py" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-HK-008").collect();

        assert_eq!(cc_hk_008.len(), 1);
        assert!(cc_hk_008[0].message.contains("logger.py"));
    }

    #[test]
    fn test_cc_hk_008_url_not_treated_as_script() {
        let content = r#"{
            "hooks": {
                "Setup": [
                    {
                        "hooks": [
                            { "type": "command", "command": "curl https://example.com/install.sh | bash" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-HK-008").collect();

        assert_eq!(cc_hk_008.len(), 0);
    }

    #[test]
    fn test_cc_hk_009_rm_rf_root() {
        let content = r#"{
            "hooks": {
                "Stop": [
                    {
                        "hooks": [
                            { "type": "command", "command": "rm -rf /" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-HK-009").collect();

        assert_eq!(cc_hk_009.len(), 1);
        assert_eq!(cc_hk_009[0].level, DiagnosticLevel::Warning);
        assert!(cc_hk_009[0].message.contains("dangerous"));
    }

    #[test]
    fn test_cc_hk_009_git_reset_hard() {
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Write",
                        "hooks": [
                            { "type": "command", "command": "git reset --hard HEAD" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-HK-009").collect();

        assert_eq!(cc_hk_009.len(), 1);
        assert!(cc_hk_009[0].message.contains("Hard reset"));
    }

    #[test]
    fn test_cc_hk_009_curl_pipe_bash() {
        let content = r#"{
            "hooks": {
                "Setup": [
                    {
                        "hooks": [
                            { "type": "command", "command": "curl https://example.com/install.sh | bash" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-HK-009").collect();

        assert_eq!(cc_hk_009.len(), 1);
        assert!(cc_hk_009[0].message.contains("security risk"));
    }

    #[test]
    fn test_cc_hk_009_git_push_force() {
        let content = r#"{
            "hooks": {
                "PostToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "git push origin main --force" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-HK-009").collect();

        assert_eq!(cc_hk_009.len(), 1);
        assert!(cc_hk_009[0].message.contains("Force push"));
    }

    #[test]
    fn test_cc_hk_009_drop_database() {
        let content = r#"{
            "hooks": {
                "Stop": [
                    {
                        "hooks": [
                            { "type": "command", "command": "psql -c 'DROP DATABASE production'" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-HK-009").collect();

        assert_eq!(cc_hk_009.len(), 1);
        assert!(cc_hk_009[0].message.contains("irreversible"));
    }

    #[test]
    fn test_cc_hk_009_chmod_777() {
        let content = r#"{
            "hooks": {
                "Setup": [
                    {
                        "hooks": [
                            { "type": "command", "command": "chmod 777 /var/www" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-HK-009").collect();

        assert_eq!(cc_hk_009.len(), 1);
        assert!(cc_hk_009[0].message.contains("full access"));
    }

    #[test]
    fn test_cc_hk_009_safe_command_ok() {
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "echo 'logging'" },
                            { "type": "command", "command": "git status" },
                            { "type": "command", "command": "npm test" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CC-HK-009").collect();

        assert_eq!(cc_hk_009.len(), 0);
    }

    #[test]
    fn test_valid_hooks_no_errors() {
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "echo 'pre-bash'" }
                        ]
                    }
                ],
                "Stop": [
                    {
                        "hooks": [
                            { "type": "prompt", "prompt": "Summarize the work done" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);

        let rule_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("CC-HK-006") || d.rule.starts_with("CC-HK-007") || d.rule.starts_with("CC-HK-009"))
            .collect();

        assert_eq!(rule_errors.len(), 0);
    }

    #[test]
    fn test_empty_hooks_ok() {
        let content = r#"{ "hooks": {} }"#;

        let diagnostics = validate(content);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_settings_with_other_fields() {
        let content = r#"{
            "permissions": { "allow": ["Read"] },
            "hooks": {
                "SessionStart": [
                    {
                        "hooks": [
                            { "type": "command", "command": "echo 'started'" }
                        ]
                    }
                ]
            },
            "model": "sonnet"
        }"#;

        let diagnostics = validate(content);

        let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "hooks::parse").collect();
        assert_eq!(parse_errors.len(), 0);
    }

    #[test]
    fn test_invalid_json_parse_error() {
        let content = r#"{ invalid json }"#;

        let diagnostics = validate(content);

        let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "hooks::parse").collect();
        assert_eq!(parse_errors.len(), 1);
    }

    #[test]
    fn test_extract_script_path_sh() {
        let validator = HooksValidator;
        assert_eq!(
            validator.extract_script_path("bash scripts/hook.sh"),
            Some("scripts/hook.sh".to_string())
        );
    }

    #[test]
    fn test_extract_script_path_py() {
        let validator = HooksValidator;
        assert_eq!(
            validator.extract_script_path("python /path/to/script.py arg1 arg2"),
            Some("/path/to/script.py".to_string())
        );
    }

    #[test]
    fn test_extract_script_path_env_var() {
        let validator = HooksValidator;
        assert_eq!(
            validator.extract_script_path("$CLAUDE_PROJECT_DIR/hooks/setup.sh"),
            Some("$CLAUDE_PROJECT_DIR/hooks/setup.sh".to_string())
        );
    }

    #[test]
    fn test_extract_script_path_no_script() {
        let validator = HooksValidator;
        assert_eq!(
            validator.extract_script_path("echo 'hello world'"),
            None
        );
    }

    #[test]
    fn test_dangerous_pattern_case_insensitive() {
        let validator = HooksValidator;

        assert!(validator.check_dangerous_patterns("RM -RF /").is_some());
        assert!(validator.check_dangerous_patterns("Git Reset --Hard").is_some());
        assert!(validator.check_dangerous_patterns("DROP DATABASE test").is_some());
    }
}
