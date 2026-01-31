//! Hooks validation rules (CC-HK-001 to CC-HK-009)

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::Validator,
    schemas::hooks::{Hook, HooksSchema, SettingsSchema},
};
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::Path;

pub struct HooksValidator;

struct DangerousPattern {
    regex: Regex,
    pattern: &'static str,
    reason: &'static str,
}

static DANGEROUS_PATTERNS: Lazy<Vec<DangerousPattern>> = Lazy::new(|| {
    let patterns: &[(&str, &str)] = &[
        (
            r"rm\s+-rf\s+/",
            "Recursive delete from root is extremely dangerous",
        ),
        (
            r"rm\s+-rf\s+\*",
            "Recursive delete with wildcard could delete unintended files",
        ),
        (
            r"rm\s+-rf\s+\.\.",
            "Recursive delete of parent directories is dangerous",
        ),
        (
            r"git\s+reset\s+--hard",
            "Hard reset discards uncommitted changes permanently",
        ),
        (
            r"git\s+clean\s+-fd",
            "Git clean -fd removes untracked files permanently",
        ),
        (
            r"git\s+push\s+.*--force",
            "Force push can overwrite remote history",
        ),
        (r"drop\s+database", "Dropping database is irreversible"),
        (r"drop\s+table", "Dropping table is irreversible"),
        (r"truncate\s+table", "Truncating table deletes all data"),
        (
            r"curl\s+.*\|\s*sh",
            "Piping curl to shell is a security risk",
        ),
        (
            r"curl\s+.*\|\s*bash",
            "Piping curl to bash is a security risk",
        ),
        (
            r"wget\s+.*\|\s*sh",
            "Piping wget to shell is a security risk",
        ),
        (r"chmod\s+777", "chmod 777 gives everyone full access"),
        (
            r">\s*/dev/sd[a-z]",
            "Writing directly to block devices can destroy data",
        ),
        (r"mkfs\.", "Formatting filesystem destroys all data"),
        (r"dd\s+if=.*of=/dev/", "dd to device can destroy data"),
    ];
    patterns
        .iter()
        .map(|&(pattern, reason)| {
            let regex =
                Regex::new(&format!("(?i){}", pattern)).expect("Invalid dangerous pattern regex");
            DangerousPattern {
                regex,
                pattern,
                reason,
            }
        })
        .collect()
});

static SCRIPT_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    [
        r#"["']?([^\s"']+\.sh)["']?\b"#,
        r#"["']?([^\s"']+\.bash)["']?\b"#,
        r#"["']?([^\s"']+\.py)["']?\b"#,
        r#"["']?([^\s"']+\.js)["']?\b"#,
        r#"["']?([^\s"']+\.ts)["']?\b"#,
    ]
    .iter()
    .map(|p| Regex::new(p).expect("Invalid script pattern regex"))
    .collect()
});

impl HooksValidator {
    fn check_dangerous_patterns(&self, command: &str) -> Option<(&'static str, &'static str)> {
        for dp in DANGEROUS_PATTERNS.iter() {
            if dp.regex.is_match(command) {
                return Some((dp.pattern, dp.reason));
            }
        }
        None
    }

    fn extract_script_paths(&self, command: &str) -> Vec<String> {
        let mut paths = Vec::new();
        for re in SCRIPT_PATTERNS.iter() {
            for caps in re.captures_iter(command) {
                if let Some(m) = caps.get(1) {
                    let path = m.as_str().trim_matches(|c| c == '"' || c == '\'');
                    if path.contains("://") || path.starts_with("http") {
                        continue;
                    }
                    paths.push(path.to_string());
                }
            }
        }
        paths
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

    fn has_unresolved_env_vars(&self, path: &str) -> bool {
        let after_claude = path
            .replace("$CLAUDE_PROJECT_DIR", "")
            .replace("${CLAUDE_PROJECT_DIR}", "");
        after_claude.contains('$')
    }
}

impl Validator for HooksValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Early return if hooks category is entirely disabled
        if !config.rules.hooks {
            return diagnostics;
        }

        let raw_value: serde_json::Value = match serde_json::from_str(content) {
            Ok(v) => v,
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

        // CC-HK-005: Missing type field (pre-parse check)
        if config.is_rule_enabled("CC-HK-005") {
            if let Some(hooks_obj) = raw_value.get("hooks").and_then(|h| h.as_object()) {
                for (event, matchers) in hooks_obj {
                    if let Some(matchers_arr) = matchers.as_array() {
                        for (matcher_idx, matcher) in matchers_arr.iter().enumerate() {
                            if let Some(hooks_arr) = matcher.get("hooks").and_then(|h| h.as_array())
                            {
                                for (hook_idx, hook) in hooks_arr.iter().enumerate() {
                                    if hook.get("type").is_none() {
                                        let hook_location = format!(
                                            "hooks.{}[{}].hooks[{}]",
                                            event, matcher_idx, hook_idx
                                        );
                                        diagnostics.push(
                                            Diagnostic::error(
                                                path.to_path_buf(),
                                                1,
                                                0,
                                                "CC-HK-005",
                                                format!(
                                                    "Hook at {} is missing required 'type' field",
                                                    hook_location
                                                ),
                                            )
                                            .with_suggestion(
                                                "Add 'type': 'command' or 'type': 'prompt'"
                                                    .to_string(),
                                            ),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if diagnostics.iter().any(|d| d.rule == "CC-HK-005") {
            return diagnostics;
        }

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
            // CC-HK-001: Invalid event name
            if config.is_rule_enabled("CC-HK-001") {
                if !HooksSchema::VALID_EVENTS.contains(&event.as_str()) {
                    let suggestion = find_closest_event(event);
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CC-HK-001",
                            format!(
                                "Invalid hook event '{}', valid events: {:?}",
                                event,
                                HooksSchema::VALID_EVENTS
                            ),
                        )
                        .with_suggestion(suggestion),
                    );
                    continue; // Skip further validation for invalid events
                }
            } else if !HooksSchema::VALID_EVENTS.contains(&event.as_str()) {
                // Even if rule is disabled, skip invalid events to avoid runtime errors
                continue;
            }

            for (matcher_idx, matcher) in matchers.iter().enumerate() {
                // CC-HK-003: Missing matcher for tool events
                if config.is_rule_enabled("CC-HK-003")
                    && HooksSchema::is_tool_event(event)
                    && matcher.matcher.is_none()
                {
                    let hook_location = format!("hooks.{}[{}]", event, matcher_idx);
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CC-HK-003",
                            format!(
                                "Tool event '{}' at {} requires a matcher field",
                                event, hook_location
                            ),
                        )
                        .with_suggestion(
                            "Add 'matcher': '*' for all tools or specify a tool name".to_string(),
                        ),
                    );
                }

                // CC-HK-004: Matcher on non-tool event
                if config.is_rule_enabled("CC-HK-004")
                    && !HooksSchema::is_tool_event(event)
                    && matcher.matcher.is_some()
                {
                    let hook_location = format!("hooks.{}[{}]", event, matcher_idx);
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CC-HK-004",
                            format!(
                                "Non-tool event '{}' at {} must not have a matcher field",
                                event, hook_location
                            ),
                        )
                        .with_suggestion("Remove the 'matcher' field".to_string()),
                    );
                }

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
                            // CC-HK-006: Missing command field
                            if config.is_rule_enabled("CC-HK-006") && command.is_none() {
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
                            }

                            if let Some(cmd) = command {
                                // CC-HK-008: Script file not found
                                if config.is_rule_enabled("CC-HK-008") {
                                    for script_path in self.extract_script_paths(cmd) {
                                        if !self.has_unresolved_env_vars(&script_path) {
                                            let resolved =
                                                self.resolve_script_path(&script_path, project_dir);
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
                                                    .with_suggestion(
                                                        "Create the script file or correct the path"
                                                            .to_string(),
                                                    ),
                                                );
                                            }
                                        }
                                    }
                                }

                                // CC-HK-009: Dangerous command patterns
                                if config.is_rule_enabled("CC-HK-009") {
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
                        }
                        Hook::Prompt { prompt, .. } => {
                            // CC-HK-002: Prompt on wrong event
                            if config.is_rule_enabled("CC-HK-002")
                                && !HooksSchema::is_prompt_event(event)
                            {
                                diagnostics.push(
                                        Diagnostic::error(
                                            path.to_path_buf(),
                                            1,
                                            0,
                                            "CC-HK-002",
                                            format!(
                                                "Prompt hook at {} is only allowed for Stop and SubagentStop events, not '{}'",
                                                hook_location, event
                                            ),
                                        )
                                        .with_suggestion(
                                            "Use 'type': 'command' instead, or move this hook to Stop/SubagentStop".to_string(),
                                        ),
                                    );
                            }

                            // CC-HK-007: Missing prompt field
                            if config.is_rule_enabled("CC-HK-007") && prompt.is_none() {
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

fn find_closest_event(invalid_event: &str) -> String {
    let lower_event = invalid_event.to_lowercase();

    // Check for exact case-insensitive match first
    for valid in HooksSchema::VALID_EVENTS {
        if valid.to_lowercase() == lower_event {
            return format!("Did you mean '{}'? Event names are case-sensitive.", valid);
        }
    }

    // Check for partial matches
    for valid in HooksSchema::VALID_EVENTS {
        let valid_lower = valid.to_lowercase();
        if valid_lower.contains(&lower_event) || lower_event.contains(&valid_lower) {
            return format!("Did you mean '{}'?", valid);
        }
    }

    format!("Valid events are: {}", HooksSchema::VALID_EVENTS.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;

    fn validate(content: &str) -> Vec<Diagnostic> {
        let validator = HooksValidator;
        validator.validate(Path::new("settings.json"), content, &LintConfig::default())
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
        let cc_hk_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-006")
            .collect();

        assert_eq!(cc_hk_006.len(), 1);
        assert_eq!(cc_hk_006[0].level, DiagnosticLevel::Error);
        assert!(cc_hk_006[0]
            .message
            .contains("missing required 'command' field"));
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
        let cc_hk_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-006")
            .collect();

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
        let cc_hk_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-006")
            .collect();

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
        let cc_hk_007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-007")
            .collect();

        assert_eq!(cc_hk_007.len(), 1);
        assert_eq!(cc_hk_007[0].level, DiagnosticLevel::Error);
        assert!(cc_hk_007[0]
            .message
            .contains("missing required 'prompt' field"));
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
        let cc_hk_007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-007")
            .collect();

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
        let cc_hk_007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-007")
            .collect();

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
        let cc_hk_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-008")
            .collect();

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
        let cc_hk_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-008")
            .collect();

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
        let cc_hk_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-008")
            .collect();

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
        let cc_hk_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-008")
            .collect();

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
        let cc_hk_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-008")
            .collect();

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
        let cc_hk_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-009")
            .collect();

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
        let cc_hk_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-009")
            .collect();

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
        let cc_hk_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-009")
            .collect();

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
        let cc_hk_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-009")
            .collect();

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
        let cc_hk_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-009")
            .collect();

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
        let cc_hk_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-009")
            .collect();

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
        let cc_hk_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-009")
            .collect();

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
            .filter(|d| {
                d.rule.starts_with("CC-HK-006")
                    || d.rule.starts_with("CC-HK-007")
                    || d.rule.starts_with("CC-HK-009")
            })
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

        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "hooks::parse")
            .collect();
        assert_eq!(parse_errors.len(), 0);
    }

    #[test]
    fn test_invalid_json_parse_error() {
        let content = r#"{ invalid json }"#;

        let diagnostics = validate(content);

        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "hooks::parse")
            .collect();
        assert_eq!(parse_errors.len(), 1);
    }

    #[test]
    fn test_extract_script_paths_sh() {
        let validator = HooksValidator;
        let paths = validator.extract_script_paths("bash scripts/hook.sh");
        assert_eq!(paths, vec!["scripts/hook.sh"]);
    }

    #[test]
    fn test_extract_script_paths_py() {
        let validator = HooksValidator;
        let paths = validator.extract_script_paths("python /path/to/script.py arg1 arg2");
        assert_eq!(paths, vec!["/path/to/script.py"]);
    }

    #[test]
    fn test_extract_script_paths_env_var() {
        let validator = HooksValidator;
        let paths = validator.extract_script_paths("$CLAUDE_PROJECT_DIR/hooks/setup.sh");
        assert_eq!(paths, vec!["$CLAUDE_PROJECT_DIR/hooks/setup.sh"]);
    }

    #[test]
    fn test_extract_script_paths_no_script() {
        let validator = HooksValidator;
        let paths = validator.extract_script_paths("echo 'hello world'");
        assert!(paths.is_empty());
    }

    #[test]
    fn test_extract_script_paths_multiple() {
        let validator = HooksValidator;
        let paths = validator.extract_script_paths("./first.sh && ./second.sh");
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&"./first.sh".to_string()));
        assert!(paths.contains(&"./second.sh".to_string()));
    }

    #[test]
    fn test_extract_script_paths_quoted() {
        let validator = HooksValidator;
        let paths = validator.extract_script_paths("bash \"$CLAUDE_PROJECT_DIR/hooks/test.sh\"");
        assert_eq!(paths, vec!["$CLAUDE_PROJECT_DIR/hooks/test.sh"]);
    }

    #[test]
    fn test_has_unresolved_env_vars() {
        let validator = HooksValidator;
        assert!(!validator.has_unresolved_env_vars("./script.sh"));
        assert!(!validator.has_unresolved_env_vars("$CLAUDE_PROJECT_DIR/script.sh"));
        assert!(validator.has_unresolved_env_vars("$HOME/script.sh"));
        assert!(validator.has_unresolved_env_vars("$CLAUDE_PROJECT_DIR/$HOME/script.sh"));
    }

    #[test]
    fn test_dangerous_pattern_case_insensitive() {
        let validator = HooksValidator;

        assert!(validator.check_dangerous_patterns("RM -RF /").is_some());
        assert!(validator
            .check_dangerous_patterns("Git Reset --Hard")
            .is_some());
        assert!(validator
            .check_dangerous_patterns("DROP DATABASE test")
            .is_some());
    }

    #[test]
    fn test_fixture_valid_settings() {
        let content = include_str!("../../../../tests/fixtures/hooks/valid-settings.json");
        let diagnostics = validate(content);
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("CC-HK-00"))
            .collect();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_fixture_missing_command() {
        let content = include_str!("../../../../tests/fixtures/hooks/missing-command-field.json");
        let diagnostics = validate(content);
        let cc_hk_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-006")
            .collect();
        assert!(!cc_hk_006.is_empty());
    }

    #[test]
    fn test_fixture_missing_prompt() {
        let content = include_str!("../../../../tests/fixtures/hooks/missing-prompt-field.json");
        let diagnostics = validate(content);
        let cc_hk_007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-007")
            .collect();
        assert!(!cc_hk_007.is_empty());
    }

    #[test]
    fn test_fixture_dangerous_commands() {
        let content = include_str!("../../../../tests/fixtures/hooks/dangerous-commands.json");
        let diagnostics = validate(content);
        let cc_hk_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-009")
            .collect();
        assert!(cc_hk_009.len() >= 3);
    }

    // ===== CC-HK-001 Tests: Invalid Event Name =====

    #[test]
    fn test_cc_hk_001_invalid_event_name() {
        let content = r#"{
            "hooks": {
                "InvalidEvent": [
                    {
                        "hooks": [
                            { "type": "command", "command": "echo 'test'" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-001")
            .collect();

        assert_eq!(cc_hk_001.len(), 1);
        assert_eq!(cc_hk_001[0].level, DiagnosticLevel::Error);
        assert!(cc_hk_001[0].message.contains("Invalid hook event"));
        assert!(cc_hk_001[0].message.contains("InvalidEvent"));
    }

    #[test]
    fn test_cc_hk_001_wrong_case_event_name() {
        let content = r#"{
            "hooks": {
                "pretooluse": [
                    {
                        "matcher": "*",
                        "hooks": [
                            { "type": "command", "command": "echo 'test'" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-001")
            .collect();

        assert_eq!(cc_hk_001.len(), 1);
        // Should suggest the correct case
        assert!(cc_hk_001[0]
            .suggestion
            .as_ref()
            .unwrap()
            .contains("PreToolUse"));
        assert!(cc_hk_001[0]
            .suggestion
            .as_ref()
            .unwrap()
            .contains("case-sensitive"));
    }

    #[test]
    fn test_cc_hk_001_valid_event_name() {
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "echo 'test'" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-001")
            .collect();

        assert_eq!(cc_hk_001.len(), 0);
    }

    #[test]
    fn test_cc_hk_001_multiple_invalid_events() {
        let content = r#"{
            "hooks": {
                "InvalidOne": [
                    {
                        "hooks": [
                            { "type": "command", "command": "echo 'test'" }
                        ]
                    }
                ],
                "InvalidTwo": [
                    {
                        "hooks": [
                            { "type": "command", "command": "echo 'test'" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-001")
            .collect();

        assert_eq!(cc_hk_001.len(), 2);
    }

    #[test]
    fn test_fixture_invalid_event() {
        let content = include_str!("../../../../tests/fixtures/hooks/invalid-event.json");
        let diagnostics = validate(content);
        let cc_hk_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-001")
            .collect();
        // "InvalidEvent" and "pretooluse" are invalid
        assert_eq!(cc_hk_001.len(), 2);
    }

    // ===== CC-HK-002 Tests: Prompt Hook on Wrong Event =====

    #[test]
    fn test_cc_hk_002_prompt_on_pretooluse() {
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "prompt", "prompt": "not allowed here" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-002")
            .collect();

        assert_eq!(cc_hk_002.len(), 1);
        assert_eq!(cc_hk_002[0].level, DiagnosticLevel::Error);
        assert!(cc_hk_002[0]
            .message
            .contains("only allowed for Stop and SubagentStop"));
    }

    #[test]
    fn test_cc_hk_002_prompt_on_session_start() {
        let content = r#"{
            "hooks": {
                "SessionStart": [
                    {
                        "hooks": [
                            { "type": "prompt", "prompt": "not allowed here" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-002")
            .collect();

        assert_eq!(cc_hk_002.len(), 1);
    }

    #[test]
    fn test_cc_hk_002_prompt_on_stop_ok() {
        let content = r#"{
            "hooks": {
                "Stop": [
                    {
                        "hooks": [
                            { "type": "prompt", "prompt": "this is valid" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-002")
            .collect();

        assert_eq!(cc_hk_002.len(), 0);
    }

    #[test]
    fn test_cc_hk_002_prompt_on_subagent_stop_ok() {
        let content = r#"{
            "hooks": {
                "SubagentStop": [
                    {
                        "hooks": [
                            { "type": "prompt", "prompt": "this is valid" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-002")
            .collect();

        assert_eq!(cc_hk_002.len(), 0);
    }

    #[test]
    fn test_fixture_prompt_on_wrong_event() {
        let content = include_str!("../../../../tests/fixtures/hooks/prompt-on-wrong-event.json");
        let diagnostics = validate(content);
        let cc_hk_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-002")
            .collect();
        // PreToolUse and SessionStart should trigger errors, Stop and SubagentStop should not
        assert_eq!(cc_hk_002.len(), 2);
    }

    // ===== CC-HK-003 Tests: Missing Matcher for Tool Events =====

    #[test]
    fn test_cc_hk_003_missing_matcher_pretooluse() {
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "hooks": [
                            { "type": "command", "command": "echo 'test'" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-003")
            .collect();

        assert_eq!(cc_hk_003.len(), 1);
        assert_eq!(cc_hk_003[0].level, DiagnosticLevel::Error);
        assert!(cc_hk_003[0].message.contains("requires a matcher"));
    }

    #[test]
    fn test_cc_hk_003_missing_matcher_permission_request() {
        let content = r#"{
            "hooks": {
                "PermissionRequest": [
                    {
                        "hooks": [
                            { "type": "command", "command": "echo 'test'" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-003")
            .collect();

        assert_eq!(cc_hk_003.len(), 1);
    }

    #[test]
    fn test_cc_hk_003_missing_matcher_posttooluse() {
        let content = r#"{
            "hooks": {
                "PostToolUse": [
                    {
                        "hooks": [
                            { "type": "command", "command": "echo 'test'" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-003")
            .collect();

        assert_eq!(cc_hk_003.len(), 1);
    }

    #[test]
    fn test_cc_hk_003_with_matcher_ok() {
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "*",
                        "hooks": [
                            { "type": "command", "command": "echo 'test'" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-003")
            .collect();

        assert_eq!(cc_hk_003.len(), 0);
    }

    #[test]
    fn test_fixture_missing_matcher() {
        let content = include_str!("../../../../tests/fixtures/hooks/missing-matcher.json");
        let diagnostics = validate(content);
        let cc_hk_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-003")
            .collect();
        // All 4 tool events without matchers
        assert_eq!(cc_hk_003.len(), 4);
    }

    // ===== CC-HK-004 Tests: Matcher on Non-Tool Event =====

    #[test]
    fn test_cc_hk_004_matcher_on_stop() {
        let content = r#"{
            "hooks": {
                "Stop": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "echo 'test'" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-004")
            .collect();

        assert_eq!(cc_hk_004.len(), 1);
        assert_eq!(cc_hk_004[0].level, DiagnosticLevel::Error);
        assert!(cc_hk_004[0].message.contains("must not have a matcher"));
    }

    #[test]
    fn test_cc_hk_004_matcher_on_session_start() {
        let content = r#"{
            "hooks": {
                "SessionStart": [
                    {
                        "matcher": "Write",
                        "hooks": [
                            { "type": "command", "command": "echo 'test'" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-004")
            .collect();

        assert_eq!(cc_hk_004.len(), 1);
    }

    #[test]
    fn test_cc_hk_004_no_matcher_on_stop_ok() {
        let content = r#"{
            "hooks": {
                "Stop": [
                    {
                        "hooks": [
                            { "type": "command", "command": "echo 'test'" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-004")
            .collect();

        assert_eq!(cc_hk_004.len(), 0);
    }

    #[test]
    fn test_fixture_matcher_on_wrong_event() {
        let content = include_str!("../../../../tests/fixtures/hooks/matcher-on-wrong-event.json");
        let diagnostics = validate(content);
        let cc_hk_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-004")
            .collect();
        // Stop, SubagentStop, UserPromptSubmit, SessionStart all have matchers incorrectly
        assert_eq!(cc_hk_004.len(), 4);
    }

    // ===== CC-HK-005 Tests: Missing Type Field =====

    #[test]
    fn test_cc_hk_005_missing_type_field() {
        let content = r#"{
            "hooks": {
                "Stop": [
                    {
                        "hooks": [
                            { "command": "echo 'missing type'" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-005")
            .collect();

        assert_eq!(cc_hk_005.len(), 1);
        assert_eq!(cc_hk_005[0].level, DiagnosticLevel::Error);
        assert!(cc_hk_005[0]
            .message
            .contains("missing required 'type' field"));
    }

    #[test]
    fn test_cc_hk_005_multiple_missing_type() {
        let content = r#"{
            "hooks": {
                "Stop": [
                    {
                        "hooks": [
                            { "command": "echo 'missing type 1'" },
                            { "prompt": "missing type 2" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-005")
            .collect();

        assert_eq!(cc_hk_005.len(), 2);
    }

    #[test]
    fn test_cc_hk_005_with_type_ok() {
        let content = r#"{
            "hooks": {
                "Stop": [
                    {
                        "hooks": [
                            { "type": "command", "command": "echo 'has type'" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-005")
            .collect();

        assert_eq!(cc_hk_005.len(), 0);
    }

    #[test]
    fn test_fixture_missing_type_field() {
        let content = include_str!("../../../../tests/fixtures/hooks/missing-type-field.json");
        let diagnostics = validate(content);
        let cc_hk_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-005")
            .collect();
        // 3 hooks missing type field
        assert_eq!(cc_hk_005.len(), 3);
    }

    // ===== Helper Function Tests =====

    #[test]
    fn test_find_closest_event_exact_case_match() {
        let suggestion = find_closest_event("pretooluse");
        assert!(suggestion.contains("PreToolUse"));
        assert!(suggestion.contains("case-sensitive"));
    }

    #[test]
    fn test_find_closest_event_partial_match() {
        let suggestion = find_closest_event("tool");
        assert!(suggestion.contains("Did you mean"));
    }

    #[test]
    fn test_find_closest_event_no_match() {
        let suggestion = find_closest_event("CompletelyInvalid");
        assert!(suggestion.contains("Valid events are"));
    }

    // ===== Config Wiring Tests =====

    #[test]
    fn test_config_disabled_hooks_category_returns_empty() {
        let mut config = LintConfig::default();
        config.rules.hooks = false;

        let content = r#"{
            "hooks": {
                "InvalidEvent": [
                    {
                        "hooks": [
                            { "type": "command", "command": "echo test" }
                        ]
                    }
                ]
            }
        }"#;

        let validator = HooksValidator;
        let diagnostics = validator.validate(Path::new("settings.json"), content, &config);

        // CC-HK-001 should not fire when hooks category is disabled
        let cc_hk_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-001")
            .collect();
        assert_eq!(cc_hk_001.len(), 0);
    }

    #[test]
    fn test_config_disabled_specific_hook_rule() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["CC-HK-006".to_string()];

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

        let validator = HooksValidator;
        let diagnostics = validator.validate(Path::new("settings.json"), content, &config);

        // CC-HK-006 should not fire when specifically disabled
        let cc_hk_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-006")
            .collect();
        assert_eq!(cc_hk_006.len(), 0);
    }

    #[test]
    fn test_config_cursor_target_disables_hooks_rules() {
        use crate::config::TargetTool;

        let mut config = LintConfig::default();
        config.target = TargetTool::Cursor;

        let content = r#"{
            "hooks": {
                "InvalidEvent": [
                    {
                        "hooks": [
                            { "type": "command", "command": "echo test" }
                        ]
                    }
                ]
            }
        }"#;

        let validator = HooksValidator;
        let diagnostics = validator.validate(Path::new("settings.json"), content, &config);

        // CC-HK-* rules should not fire for Cursor target
        let hook_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("CC-HK-"))
            .collect();
        assert_eq!(hook_rules.len(), 0);
    }

    #[test]
    fn test_config_dangerous_pattern_disabled() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["CC-HK-009".to_string()];

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

        let validator = HooksValidator;
        let diagnostics = validator.validate(Path::new("settings.json"), content, &config);

        // CC-HK-009 should not fire when specifically disabled
        let cc_hk_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-009")
            .collect();
        assert_eq!(cc_hk_009.len(), 0);
    }
}
