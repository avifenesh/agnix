//! Hooks validation rules (CC-HK-001 to CC-HK-012)

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::Validator,
    schemas::hooks::{Hook, HooksSchema, SettingsSchema},
};
use rust_i18n::t;
use std::path::Path;

mod helpers;
use helpers::*;

pub struct HooksValidator;

/// Default timeout thresholds per hook type (from official Claude Code docs)
const COMMAND_HOOK_DEFAULT_TIMEOUT: u64 = 600; // 10 minutes
const PROMPT_HOOK_DEFAULT_TIMEOUT: u64 = 30; // 30 seconds

/// CC-HK-006: Missing command field
fn validate_cc_hk_006_command_field(
    command: &Option<String>,
    hook_location: &str,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if command.is_none() {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "CC-HK-006",
                t!("rules.cc_hk_006.message", location = hook_location),
            )
            .with_suggestion(t!("rules.cc_hk_006.suggestion")),
        );
    }
}

/// CC-HK-008: Script file not found
fn validate_cc_hk_008_script_exists(
    command: &str,
    project_dir: &Path,
    config: &LintConfig,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let fs = config.fs();
    for script_path in extract_script_paths(command) {
        if !has_unresolved_env_vars(&script_path) {
            let resolved = resolve_script_path(&script_path, project_dir);
            if !fs.exists(&resolved) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-HK-008",
                        t!(
                            "rules.cc_hk_008.message",
                            script = script_path.as_str(),
                            resolved = resolved.display().to_string()
                        ),
                    )
                    .with_suggestion(t!("rules.cc_hk_008.suggestion")),
                );
            }
        }
    }
}

/// CC-HK-009: Dangerous command patterns
fn validate_cc_hk_009_dangerous_patterns(
    command: &str,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some((pattern, reason)) = check_dangerous_patterns(command) {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                1,
                0,
                "CC-HK-009",
                t!("rules.cc_hk_009.message", reason = reason),
            )
            .with_suggestion(t!("rules.cc_hk_009.suggestion", pattern = pattern)),
        );
    }
}

/// CC-HK-010: Command hook timeout policy
fn validate_cc_hk_010_command_timeout(
    timeout: &Option<u64>,
    hook_location: &str,
    version_pinned: bool,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if timeout.is_none() {
        let mut diag = Diagnostic::warning(
            path.to_path_buf(),
            1,
            0,
            "CC-HK-010",
            t!(
                "rules.cc_hk_010.command_no_timeout",
                location = hook_location
            ),
        )
        .with_suggestion(t!("rules.cc_hk_010.command_no_timeout_suggestion"));

        if !version_pinned {
            diag = diag.with_assumption(t!("rules.cc_hk_010.assumption"));
        }

        diagnostics.push(diag);
    }
    if let Some(t) = timeout {
        if *t > COMMAND_HOOK_DEFAULT_TIMEOUT {
            let mut diag = Diagnostic::warning(
                path.to_path_buf(),
                1,
                0,
                "CC-HK-010",
                t!(
                    "rules.cc_hk_010.command_exceeds",
                    location = hook_location,
                    timeout = t,
                    default = COMMAND_HOOK_DEFAULT_TIMEOUT
                ),
            )
            .with_suggestion(t!(
                "rules.cc_hk_010.command_exceeds_suggestion",
                default = COMMAND_HOOK_DEFAULT_TIMEOUT
            ));

            if !version_pinned {
                diag = diag.with_assumption(t!("rules.cc_hk_010.assumption"));
            }

            diagnostics.push(diag);
        }
    }
}

/// CC-HK-002: Prompt hook on wrong event
fn validate_cc_hk_002_prompt_event_type(
    event: &str,
    hook_location: &str,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !HooksSchema::is_prompt_event(event) {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "CC-HK-002",
                t!(
                    "rules.cc_hk_002.message",
                    location = hook_location,
                    event = event
                ),
            )
            .with_suggestion(t!("rules.cc_hk_002.suggestion")),
        );
    }
}

/// CC-HK-007: Missing prompt field
fn validate_cc_hk_007_prompt_field(
    prompt: &Option<String>,
    hook_location: &str,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if prompt.is_none() {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "CC-HK-007",
                t!("rules.cc_hk_007.message", location = hook_location),
            )
            .with_suggestion(t!("rules.cc_hk_007.suggestion")),
        );
    }
}

/// CC-HK-010: Prompt hook timeout policy
fn validate_cc_hk_010_prompt_timeout(
    timeout: &Option<u64>,
    hook_location: &str,
    version_pinned: bool,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if timeout.is_none() {
        let mut diag = Diagnostic::warning(
            path.to_path_buf(),
            1,
            0,
            "CC-HK-010",
            t!(
                "rules.cc_hk_010.prompt_no_timeout",
                location = hook_location
            ),
        )
        .with_suggestion(t!("rules.cc_hk_010.prompt_no_timeout_suggestion"));

        if !version_pinned {
            diag = diag.with_assumption(t!("rules.cc_hk_010.assumption"));
        }

        diagnostics.push(diag);
    }
    if let Some(t) = timeout {
        if *t > PROMPT_HOOK_DEFAULT_TIMEOUT {
            let mut diag = Diagnostic::warning(
                path.to_path_buf(),
                1,
                0,
                "CC-HK-010",
                t!(
                    "rules.cc_hk_010.prompt_exceeds",
                    location = hook_location,
                    timeout = t,
                    default = PROMPT_HOOK_DEFAULT_TIMEOUT
                ),
            )
            .with_suggestion(t!(
                "rules.cc_hk_010.prompt_exceeds_suggestion",
                default = PROMPT_HOOK_DEFAULT_TIMEOUT
            ));

            if !version_pinned {
                diag = diag.with_assumption(t!("rules.cc_hk_010.assumption"));
            }

            diagnostics.push(diag);
        }
    }
}

#[cfg(test)]
#[allow(dead_code)]
impl HooksValidator {
    fn check_dangerous_patterns(&self, command: &str) -> Option<(&'static str, &'static str)> {
        check_dangerous_patterns(command)
    }

    fn extract_script_paths(&self, command: &str) -> Vec<String> {
        extract_script_paths(command)
    }

    fn resolve_script_path(&self, script_path: &str, project_dir: &Path) -> std::path::PathBuf {
        resolve_script_path(script_path, project_dir)
    }

    fn has_unresolved_env_vars(&self, path: &str) -> bool {
        has_unresolved_env_vars(path)
    }
}

impl Validator for HooksValidator {
    /// Main validation entry point for hooks configuration.
    ///
    /// ## Validation Phases
    ///
    /// 1. **Category check** - Early return if hooks category disabled
    /// 2. **JSON parsing** - Parse raw JSON, report CC-HK-012 on failure
    /// 3. **Pre-parse validation** - Raw JSON checks (CC-HK-005, CC-HK-011)
    /// 4. **Typed parsing** - Parse into SettingsSchema
    /// 5. **Event iteration** - Validate each event and hook
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if !config.rules.hooks {
            return diagnostics;
        }

        let raw_value: serde_json::Value = match serde_json::from_str(content) {
            Ok(v) => v,
            Err(e) => {
                if config.is_rule_enabled("CC-HK-012") {
                    diagnostics.push(Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-HK-012",
                        t!("rules.cc_hk_012.message", error = e.to_string()),
                    ));
                }
                return diagnostics;
            }
        };

        // CC-HK-005: Missing type field (early return on failure)
        if config.is_rule_enabled("CC-HK-005") {
            validate_cc_hk_005_missing_type_field(&raw_value, path, &mut diagnostics);
            if diagnostics.iter().any(|d| d.rule == "CC-HK-005") {
                return diagnostics;
            }
        }

        // CC-HK-011: Invalid timeout value
        if config.is_rule_enabled("CC-HK-011") {
            validate_cc_hk_011_invalid_timeout_values(&raw_value, path, content, &mut diagnostics);
        }

        let settings: SettingsSchema = match serde_json::from_value(raw_value) {
            Ok(s) => s,
            Err(e) => {
                if config.is_rule_enabled("CC-HK-012") {
                    diagnostics.push(Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-HK-012",
                        t!("rules.cc_hk_012.message", error = e.to_string()),
                    ));
                }
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
            // --- Event-level validation ---
            // CC-HK-001: Invalid event name
            if config.is_rule_enabled("CC-HK-001") {
                if !validate_cc_hk_001_event_name(event, path, content, &mut diagnostics) {
                    continue;
                }
            } else if !HooksSchema::VALID_EVENTS.contains(&event.as_str()) {
                continue; // Skip invalid events even if rule disabled
            }

            for (matcher_idx, matcher) in matchers.iter().enumerate() {
                // --- Matcher-level validation ---
                // CC-HK-003: Missing matcher for tool events
                if config.is_rule_enabled("CC-HK-003") {
                    validate_cc_hk_003_matcher_required(
                        event,
                        &matcher.matcher,
                        matcher_idx,
                        path,
                        &mut diagnostics,
                    );
                }

                // CC-HK-004: Matcher on non-tool event
                if config.is_rule_enabled("CC-HK-004") {
                    validate_cc_hk_004_matcher_forbidden(
                        event,
                        &matcher.matcher,
                        matcher_idx,
                        path,
                        content,
                        &mut diagnostics,
                    );
                }

                // --- Hook-level validation ---
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
                        Hook::Command {
                            command, timeout, ..
                        } => {
                            // CC-HK-010: Command timeout policy
                            if config.is_rule_enabled("CC-HK-010") {
                                validate_cc_hk_010_command_timeout(
                                    timeout,
                                    &hook_location,
                                    config.is_claude_code_version_pinned(),
                                    path,
                                    &mut diagnostics,
                                );
                            }

                            // CC-HK-006: Missing command field
                            if config.is_rule_enabled("CC-HK-006") {
                                validate_cc_hk_006_command_field(
                                    command,
                                    &hook_location,
                                    path,
                                    &mut diagnostics,
                                );
                            }

                            if let Some(cmd) = command {
                                // CC-HK-008: Script file not found
                                if config.is_rule_enabled("CC-HK-008") {
                                    validate_cc_hk_008_script_exists(
                                        cmd,
                                        project_dir,
                                        config,
                                        path,
                                        &mut diagnostics,
                                    );
                                }

                                // CC-HK-009: Dangerous command patterns
                                if config.is_rule_enabled("CC-HK-009") {
                                    validate_cc_hk_009_dangerous_patterns(
                                        cmd,
                                        path,
                                        &mut diagnostics,
                                    );
                                }
                            }
                        }
                        Hook::Prompt {
                            prompt, timeout, ..
                        } => {
                            // CC-HK-010: Prompt timeout policy
                            if config.is_rule_enabled("CC-HK-010") {
                                validate_cc_hk_010_prompt_timeout(
                                    timeout,
                                    &hook_location,
                                    config.is_claude_code_version_pinned(),
                                    path,
                                    &mut diagnostics,
                                );
                            }

                            // CC-HK-002: Prompt on wrong event
                            if config.is_rule_enabled("CC-HK-002") {
                                validate_cc_hk_002_prompt_event_type(
                                    event,
                                    &hook_location,
                                    path,
                                    &mut diagnostics,
                                );
                            }

                            // CC-HK-007: Missing prompt field
                            if config.is_rule_enabled("CC-HK-007") {
                                validate_cc_hk_007_prompt_field(
                                    prompt,
                                    &hook_location,
                                    path,
                                    &mut diagnostics,
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
mod tests;
