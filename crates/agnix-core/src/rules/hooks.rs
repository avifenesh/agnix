//! Hooks validation rules (CC-HK-001 to CC-HK-012)
//!
//! This module validates Claude Code hooks configuration files (.claude/settings.json).
//!
//! ## Rules Reference
//!
//! | Rule | Severity | Description |
//! |------|----------|-------------|
//! | CC-HK-001 | Error | Invalid event name |
//! | CC-HK-002 | Error | Prompt hook on wrong event type |
//! | CC-HK-003 | Error | Missing matcher for tool events |
//! | CC-HK-004 | Error | Matcher on non-tool event |
//! | CC-HK-005 | Error | Missing type field |
//! | CC-HK-006 | Error | Missing command field |
//! | CC-HK-007 | Error | Missing prompt field |
//! | CC-HK-008 | Error | Script file not found |
//! | CC-HK-009 | Warning | Dangerous command patterns |
//! | CC-HK-010 | Warning | Timeout policy violation |
//! | CC-HK-011 | Error | Invalid timeout value |
//! | CC-HK-012 | Error | JSON parse error |
//!
//! ## Architecture
//!
//! Validation functions are organized by rule number and grouped by validation phase:
//!
//! ### Pre-Parse Phase (raw JSON checks)
//! - `validate_cc_hk_005_missing_type_field` - Check for missing type field
//! - `validate_cc_hk_011_invalid_timeout_values` - Check for invalid timeout values
//!
//! ### Event-Level Validation
//! - `validate_cc_hk_001_event_name` - Validate event name with auto-fix
//! - `validate_cc_hk_003_matcher_required` - Check matcher required for tool events
//! - `validate_cc_hk_004_matcher_forbidden` - Check matcher forbidden on non-tool events
//!
//! ### Command Hook Validation
//! - `validate_cc_hk_006_command_field` - Check for missing command field
//! - `validate_cc_hk_008_script_exists` - Check script file exists
//! - `validate_cc_hk_009_dangerous_patterns` - Check for dangerous command patterns
//! - `validate_cc_hk_010_command_timeout` - Check command hook timeout policy
//!
//! ### Prompt Hook Validation
//! - `validate_cc_hk_002_prompt_event_type` - Check prompt hook on correct event
//! - `validate_cc_hk_007_prompt_field` - Check for missing prompt field
//! - `validate_cc_hk_010_prompt_timeout` - Check prompt hook timeout policy
//!
//! The main `validate()` method orchestrates these functions in sequence.

use crate::{
    config::LintConfig,
    diagnostics::{Diagnostic, Fix},
    rules::Validator,
    schemas::hooks::{Hook, HooksSchema, SettingsSchema},
};
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::Path;

pub struct HooksValidator;

/// Default timeout thresholds per hook type (from official Claude Code docs)
const COMMAND_HOOK_DEFAULT_TIMEOUT: u64 = 600; // 10 minutes
const PROMPT_HOOK_DEFAULT_TIMEOUT: u64 = 30; // 30 seconds

/// Version assumption note for CC-HK-010 when claude_code version is not pinned
const CC_HK_010_ASSUMPTION: &str = "Assumes Claude Code default timeout behavior. Pin claude_code version in .agnix.toml [tool_versions] for version-specific validation.";

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

// =============================================================================
// Pre-Parse Validation Functions (CC-HK-005, CC-HK-011)
// =============================================================================
// These functions must run before serde deserialization because they check
// raw JSON values that would be lost during parsing.

/// CC-HK-005: Missing type field
///
/// Checks for hooks that are missing the required 'type' field.
/// This must be checked in raw JSON because invalid type values would cause
/// serde parsing to fail with a different error message.
fn validate_cc_hk_005_missing_type_field(
    raw_value: &serde_json::Value,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(hooks_obj) = raw_value.get("hooks").and_then(|h| h.as_object()) {
        for (event, matchers) in hooks_obj {
            if let Some(matchers_arr) = matchers.as_array() {
                for (matcher_idx, matcher) in matchers_arr.iter().enumerate() {
                    if let Some(hooks_arr) = matcher.get("hooks").and_then(|h| h.as_array()) {
                        for (hook_idx, hook) in hooks_arr.iter().enumerate() {
                            if hook.get("type").is_none() {
                                let hook_location =
                                    format!("hooks.{}[{}].hooks[{}]", event, matcher_idx, hook_idx);
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
                                        "Add 'type': 'command' or 'type': 'prompt'".to_string(),
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

/// CC-HK-011: Invalid timeout value
///
/// Checks for invalid timeout values (negative, zero, float, string, etc.).
/// This must be checked in raw JSON because negative numbers and floats cannot
/// be represented in `Option<u64>` after serde deserialization.
fn validate_cc_hk_011_invalid_timeout_values(
    raw_value: &serde_json::Value,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(hooks_obj) = raw_value.get("hooks").and_then(|h| h.as_object()) {
        for (event, matchers) in hooks_obj {
            if let Some(matchers_arr) = matchers.as_array() {
                for (matcher_idx, matcher) in matchers_arr.iter().enumerate() {
                    if let Some(hooks_arr) = matcher.get("hooks").and_then(|h| h.as_array()) {
                        for (hook_idx, hook) in hooks_arr.iter().enumerate() {
                            if let Some(timeout_val) = hook.get("timeout") {
                                let is_invalid = match timeout_val {
                                    serde_json::Value::Number(n) => {
                                        // A valid timeout must be a positive integer.
                                        // as_u64() returns Some only for non-negative integer
                                        // JSON numbers within the u64 range; it returns None
                                        // for negatives, any floats (including 30.0), or
                                        // out-of-range values.
                                        if let Some(val) = n.as_u64() {
                                            val == 0 // Zero is invalid
                                        } else {
                                            true // Negative, float, or out of range
                                        }
                                    }
                                    _ => true, // String, bool, null, object, array are invalid
                                };
                                if is_invalid {
                                    let hook_location = format!(
                                        "hooks.{}[{}].hooks[{}]",
                                        event, matcher_idx, hook_idx
                                    );
                                    diagnostics.push(
                                        Diagnostic::error(
                                            path.to_path_buf(),
                                            1,
                                            0,
                                            "CC-HK-011",
                                            format!(
                                                "Invalid timeout value at {}: must be a positive integer",
                                                hook_location
                                            ),
                                        )
                                        .with_suggestion(
                                            "Set timeout to a positive integer like 30".to_string(),
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
}

// =============================================================================
// Event-Level Validation Functions (CC-HK-001, CC-HK-003, CC-HK-004)
// =============================================================================

/// CC-HK-001: Invalid event name
///
/// Validates that the event name is one of the allowed hook events.
/// Returns `true` if the event is valid, `false` if invalid (caller should skip further validation).
fn validate_cc_hk_001_event_name(
    event: &str,
    path: &Path,
    content: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if HooksSchema::VALID_EVENTS.contains(&event) {
        return true;
    }

    let closest = find_closest_event(event);
    let mut diagnostic = Diagnostic::error(
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
    .with_suggestion(closest.suggestion);

    // Add auto-fix if we found a matching event
    if let Some(corrected) = closest.corrected_event {
        if let Some((start, end)) = find_event_key_position(content, event) {
            let replacement = format!("\"{}\"", corrected);
            let description = format!("Replace '{}' with '{}'", event, corrected);
            // Case-only fixes are safe (high confidence)
            let fix = Fix::replace(start, end, replacement, description, closest.is_case_fix);
            diagnostic = diagnostic.with_fix(fix);
        }
    }

    diagnostics.push(diagnostic);
    false
}

/// CC-HK-003: Missing matcher for tool events
///
/// Tool events (PreToolUse, PostToolUse, PermissionRequest) require a matcher field.
fn validate_cc_hk_003_matcher_required(
    event: &str,
    matcher: &Option<String>,
    matcher_idx: usize,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if HooksSchema::is_tool_event(event) && matcher.is_none() {
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
            .with_suggestion("Add 'matcher': '*' for all tools or specify a tool name".to_string()),
        );
    }
}

/// CC-HK-004: Matcher on non-tool event
///
/// Non-tool events (Stop, SubagentStop, SessionStart, etc.) must not have a matcher field.
fn validate_cc_hk_004_matcher_forbidden(
    event: &str,
    matcher: &Option<String>,
    matcher_idx: usize,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !HooksSchema::is_tool_event(event) && matcher.is_some() {
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
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Check if a command matches any dangerous patterns.
/// Returns (pattern, reason) if a match is found.
fn check_dangerous_patterns(command: &str) -> Option<(&'static str, &'static str)> {
    for dp in DANGEROUS_PATTERNS.iter() {
        if dp.regex.is_match(command) {
            return Some((dp.pattern, dp.reason));
        }
    }
    None
}

/// Extract script paths from a command string.
fn extract_script_paths(command: &str) -> Vec<String> {
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

/// Resolve a script path relative to the project directory.
fn resolve_script_path(script_path: &str, project_dir: &Path) -> std::path::PathBuf {
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

/// Check if a path contains unresolved environment variables.
fn has_unresolved_env_vars(path: &str) -> bool {
    let after_claude = path
        .replace("$CLAUDE_PROJECT_DIR", "")
        .replace("${CLAUDE_PROJECT_DIR}", "");
    after_claude.contains('$')
}

// =============================================================================
// Command Hook Validation Functions (CC-HK-006, CC-HK-008, CC-HK-009, CC-HK-010)
// =============================================================================

/// CC-HK-006: Missing command field
///
/// Command hooks must have a 'command' field specifying the command to execute.
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
                format!(
                    "Command hook at {} is missing required 'command' field",
                    hook_location
                ),
            )
            .with_suggestion("Add a 'command' field with the command to execute".to_string()),
        );
    }
}

/// CC-HK-008: Script file not found
///
/// Validates that referenced script files exist on the filesystem.
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
                        format!(
                            "Script file not found at '{}' (resolved to '{}')",
                            script_path,
                            resolved.display()
                        ),
                    )
                    .with_suggestion("Create the script file or correct the path".to_string()),
                );
            }
        }
    }
}

/// CC-HK-009: Dangerous command patterns
///
/// Warns about potentially dangerous commands like `rm -rf /`, `git reset --hard`, etc.
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
                format!("Potentially dangerous command pattern detected: {}", reason),
            )
            .with_suggestion(format!(
                "Review the command for safety. Pattern matched: {}",
                pattern
            )),
        );
    }
}

/// CC-HK-010: Command hook timeout policy
///
/// Warns if timeout is missing or exceeds the 600s default for command hooks.
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
            format!("Command hook at {} has no timeout specified", hook_location),
        )
        .with_suggestion("Add a \"timeout\" field (e.g., 600 for command hooks)".to_string());

        if !version_pinned {
            diag = diag.with_assumption(CC_HK_010_ASSUMPTION);
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
                format!(
                    "Command hook at {} has timeout {}s exceeding {}s default",
                    hook_location, t, COMMAND_HOOK_DEFAULT_TIMEOUT
                ),
            )
            .with_suggestion(format!(
                "Consider timeout <= {}s (10-minute default limit)",
                COMMAND_HOOK_DEFAULT_TIMEOUT
            ));

            if !version_pinned {
                diag = diag.with_assumption(CC_HK_010_ASSUMPTION);
            }

            diagnostics.push(diag);
        }
    }
}

// =============================================================================
// Prompt Hook Validation Functions (CC-HK-002, CC-HK-007, CC-HK-010)
// =============================================================================

/// CC-HK-002: Prompt hook on wrong event
///
/// Prompt hooks are only allowed for Stop and SubagentStop events.
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
}

/// CC-HK-007: Missing prompt field
///
/// Prompt hooks must have a 'prompt' field specifying the prompt text.
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
                format!(
                    "Prompt hook at {} is missing required 'prompt' field",
                    hook_location
                ),
            )
            .with_suggestion("Add a 'prompt' field with the prompt text".to_string()),
        );
    }
}

/// CC-HK-010: Prompt hook timeout policy
///
/// Warns if timeout is missing or exceeds the 30s default for prompt hooks.
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
            format!("Prompt hook at {} has no timeout specified", hook_location),
        )
        .with_suggestion("Add a \"timeout\" field (e.g., 30 for prompt hooks)".to_string());

        if !version_pinned {
            diag = diag.with_assumption(CC_HK_010_ASSUMPTION);
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
                format!(
                    "Prompt hook at {} has timeout {}s exceeding {}s default",
                    hook_location, t, PROMPT_HOOK_DEFAULT_TIMEOUT
                ),
            )
            .with_suggestion(format!(
                "Consider timeout <= {}s (30-second default limit)",
                PROMPT_HOOK_DEFAULT_TIMEOUT
            ));

            if !version_pinned {
                diag = diag.with_assumption(CC_HK_010_ASSUMPTION);
            }

            diagnostics.push(diag);
        }
    }
}

// Keep HooksValidator methods for backward compatibility with existing tests.
// These delegate to the standalone functions above.
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

        // =====================================================================
        // Phase 1: Category check
        // =====================================================================
        if !config.rules.hooks {
            return diagnostics;
        }

        // =====================================================================
        // Phase 2: JSON parsing
        // =====================================================================
        let raw_value: serde_json::Value = match serde_json::from_str(content) {
            Ok(v) => v,
            Err(e) => {
                if config.is_rule_enabled("CC-HK-012") {
                    diagnostics.push(Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-HK-012",
                        format!("Failed to parse hooks configuration: {}", e),
                    ));
                }
                return diagnostics;
            }
        };

        // =====================================================================
        // Phase 3: Pre-parse validation (raw JSON checks)
        // =====================================================================
        // CC-HK-005: Missing type field (early return on failure)
        if config.is_rule_enabled("CC-HK-005") {
            validate_cc_hk_005_missing_type_field(&raw_value, path, &mut diagnostics);
            if diagnostics.iter().any(|d| d.rule == "CC-HK-005") {
                return diagnostics;
            }
        }

        // CC-HK-011: Invalid timeout value
        if config.is_rule_enabled("CC-HK-011") {
            validate_cc_hk_011_invalid_timeout_values(&raw_value, path, &mut diagnostics);
        }

        // =====================================================================
        // Phase 4: Typed parsing
        // =====================================================================
        let settings: SettingsSchema = match serde_json::from_str(content) {
            Ok(s) => s,
            Err(e) => {
                if config.is_rule_enabled("CC-HK-012") {
                    diagnostics.push(Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-HK-012",
                        format!("Failed to parse hooks configuration: {}", e),
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

        // =====================================================================
        // Phase 5: Event iteration with typed validation
        // =====================================================================
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

/// Result of finding the closest matching event
struct ClosestEventMatch {
    suggestion: String,
    /// The correct event name if a good match was found
    corrected_event: Option<String>,
    /// Whether this is a case-only difference (high confidence)
    is_case_fix: bool,
}

fn find_closest_event(invalid_event: &str) -> ClosestEventMatch {
    let lower_event = invalid_event.to_lowercase();

    // Check for exact case-insensitive match first (high confidence fix)
    for valid in HooksSchema::VALID_EVENTS {
        if valid.to_lowercase() == lower_event {
            return ClosestEventMatch {
                suggestion: format!("Did you mean '{}'? Event names are case-sensitive.", valid),
                corrected_event: Some(valid.to_string()),
                is_case_fix: true,
            };
        }
    }

    // Check for partial matches (lower confidence)
    for valid in HooksSchema::VALID_EVENTS {
        let valid_lower = valid.to_lowercase();
        if valid_lower.contains(&lower_event) || lower_event.contains(&valid_lower) {
            return ClosestEventMatch {
                suggestion: format!("Did you mean '{}'?", valid),
                corrected_event: Some(valid.to_string()),
                is_case_fix: false,
            };
        }
    }

    ClosestEventMatch {
        suggestion: format!("Valid events are: {}", HooksSchema::VALID_EVENTS.join(", ")),
        corrected_event: None,
        is_case_fix: false,
    }
}

/// Find the byte position of an event key in JSON content
/// Returns (start, end) byte positions of the event key (including quotes)
fn find_event_key_position(content: &str, event: &str) -> Option<(usize, usize)> {
    // Look for the event key in the "hooks" object
    // Pattern: capture the quoted event name, followed by : (with optional whitespace)
    let pattern = format!(r#"("{}")\s*:"#, regex::escape(event));
    let re = Regex::new(&pattern).ok()?;
    re.captures(content).and_then(|caps| {
        caps.get(1)
            .map(|key_match| (key_match.start(), key_match.end()))
    })
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
            .filter(|d| d.rule == "CC-HK-012")
            .collect();
        assert_eq!(parse_errors.len(), 0);
    }

    #[test]
    fn test_invalid_json_parse_error() {
        let content = r#"{ invalid json }"#;

        let diagnostics = validate(content);

        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-012")
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
        let content = include_str!("../../../../tests/fixtures/valid/hooks/settings.json");
        let diagnostics = validate(content);
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("CC-HK-00"))
            .collect();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_fixture_missing_command() {
        let content = include_str!(
            "../../../../tests/fixtures/invalid/hooks/missing-command-field/settings.json"
        );
        let diagnostics = validate(content);
        let cc_hk_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-006")
            .collect();
        assert!(!cc_hk_006.is_empty());
    }

    #[test]
    fn test_fixture_missing_prompt() {
        let content = include_str!(
            "../../../../tests/fixtures/invalid/hooks/missing-prompt-field/settings.json"
        );
        let diagnostics = validate(content);
        let cc_hk_007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-007")
            .collect();
        assert!(!cc_hk_007.is_empty());
    }

    #[test]
    fn test_fixture_dangerous_commands() {
        let content = include_str!(
            "../../../../tests/fixtures/invalid/hooks/dangerous-commands/settings.json"
        );
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
        let content =
            include_str!("../../../../tests/fixtures/invalid/hooks/invalid-event/settings.json");
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
        let content = include_str!(
            "../../../../tests/fixtures/invalid/hooks/prompt-on-wrong-event/settings.json"
        );
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
        let content =
            include_str!("../../../../tests/fixtures/invalid/hooks/missing-matcher/settings.json");
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
        let content = include_str!(
            "../../../../tests/fixtures/invalid/hooks/matcher-on-wrong-event/settings.json"
        );
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
        let content = include_str!(
            "../../../../tests/fixtures/invalid/hooks/missing-type-field/settings.json"
        );
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
        let closest = find_closest_event("pretooluse");
        assert!(closest.suggestion.contains("PreToolUse"));
        assert!(closest.suggestion.contains("case-sensitive"));
        assert_eq!(closest.corrected_event, Some("PreToolUse".to_string()));
        assert!(closest.is_case_fix);
    }

    #[test]
    fn test_find_closest_event_partial_match() {
        let closest = find_closest_event("tool");
        assert!(closest.suggestion.contains("Did you mean"));
        assert!(closest.corrected_event.is_some());
        assert!(!closest.is_case_fix);
    }

    #[test]
    fn test_find_closest_event_no_match() {
        let closest = find_closest_event("CompletelyInvalid");
        assert!(closest.suggestion.contains("Valid events are"));
        assert!(closest.corrected_event.is_none());
    }

    // ===== CC-HK-001 Auto-fix Tests =====

    #[test]
    fn test_cc_hk_001_case_fix_has_safe_fix() {
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
        assert!(cc_hk_001[0].has_fixes());

        let fix = &cc_hk_001[0].fixes[0];
        assert!(fix.safe); // Case-only fix is safe
        assert_eq!(fix.replacement, "\"PreToolUse\"");
    }

    #[test]
    fn test_cc_hk_001_typo_fix_not_safe() {
        // "tool" partially matches "PreToolUse"
        let content = r#"{
            "hooks": {
                "tool": [
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
        assert!(cc_hk_001[0].has_fixes());

        let fix = &cc_hk_001[0].fixes[0];
        assert!(!fix.safe); // Partial match is not safe
    }

    #[test]
    fn test_cc_hk_001_no_fix_for_completely_invalid() {
        let content = r#"{
            "hooks": {
                "XyzAbc123": [
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
        // No fix when there's no reasonable match
        assert!(!cc_hk_001[0].has_fixes());
    }

    #[test]
    fn test_cc_hk_001_fix_correct_byte_position() {
        let content =
            r#"{"hooks": {"stop": [{"hooks": [{"type": "command", "command": "echo"}]}]}}"#;

        let diagnostics = validate(content);
        let cc_hk_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-001")
            .collect();

        assert_eq!(cc_hk_001.len(), 1);
        assert!(cc_hk_001[0].has_fixes());

        let fix = &cc_hk_001[0].fixes[0];

        // Apply fix and verify
        let mut fixed = content.to_string();
        fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
        assert!(fixed.contains("\"Stop\""));
        assert!(!fixed.contains("\"stop\""));
    }

    #[test]
    fn test_find_event_key_position() {
        let content = r#"{"hooks": {"InvalidEvent": []}}"#;
        let pos = find_event_key_position(content, "InvalidEvent");
        assert!(pos.is_some());
        let (start, end) = pos.unwrap();
        assert_eq!(&content[start..end], "\"InvalidEvent\"");
    }

    #[test]
    fn test_find_event_key_position_not_found() {
        let content = r#"{"hooks": {"ValidEvent": []}}"#;
        let pos = find_event_key_position(content, "NotPresent");
        assert!(pos.is_none());
    }

    // ===== CC-HK-010 Tests: No Timeout Specified =====

    #[test]
    fn test_cc_hk_010_command_hook_no_timeout() {
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "echo test" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_010: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-010")
            .collect();

        assert_eq!(cc_hk_010.len(), 1);
        assert_eq!(cc_hk_010[0].level, DiagnosticLevel::Warning);
        assert!(cc_hk_010[0].message.contains("no timeout specified"));
    }

    #[test]
    fn test_cc_hk_010_prompt_hook_no_timeout() {
        let content = r#"{
            "hooks": {
                "Stop": [
                    {
                        "hooks": [
                            { "type": "prompt", "prompt": "Summarize session" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_010: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-010")
            .collect();

        assert_eq!(cc_hk_010.len(), 1);
        assert_eq!(cc_hk_010[0].level, DiagnosticLevel::Warning);
        assert!(cc_hk_010[0].message.contains("Prompt hook"));
    }

    #[test]
    fn test_cc_hk_010_with_timeout_ok() {
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "echo test", "timeout": 30 }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_010: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-010")
            .collect();

        assert_eq!(cc_hk_010.len(), 0);
    }

    #[test]
    fn test_cc_hk_010_command_timeout_exceeds_default() {
        // Command hooks have 600s default - 700 should warn
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "echo test", "timeout": 700 }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_010: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-010")
            .collect();

        assert_eq!(cc_hk_010.len(), 1);
        assert_eq!(cc_hk_010[0].level, DiagnosticLevel::Warning);
        assert!(cc_hk_010[0].message.contains("exceeding 600s default"));
    }

    #[test]
    fn test_cc_hk_010_command_timeout_at_default_ok() {
        // Command hooks have 600s default - 600 should NOT warn
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "echo test", "timeout": 600 }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_010: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-010")
            .collect();

        assert_eq!(cc_hk_010.len(), 0);
    }

    #[test]
    fn test_cc_hk_010_prompt_timeout_exceeds_default() {
        // Prompt hooks have 30s default - 45 should warn
        let content = r#"{
            "hooks": {
                "Stop": [
                    {
                        "hooks": [
                            { "type": "prompt", "prompt": "test prompt", "timeout": 45 }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_010: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-010")
            .collect();

        assert_eq!(cc_hk_010.len(), 1);
        assert_eq!(cc_hk_010[0].level, DiagnosticLevel::Warning);
        assert!(cc_hk_010[0].message.contains("exceeding 30s default"));
    }

    #[test]
    fn test_cc_hk_010_prompt_timeout_at_default_ok() {
        // Prompt hooks have 30s default - 30 should NOT warn
        let content = r#"{
            "hooks": {
                "Stop": [
                    {
                        "hooks": [
                            { "type": "prompt", "prompt": "test prompt", "timeout": 30 }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_010: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-010")
            .collect();

        assert_eq!(cc_hk_010.len(), 0);
    }

    #[test]
    fn test_cc_hk_010_multiple_hooks_mixed() {
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "*",
                        "hooks": [
                            { "type": "command", "command": "echo 'no timeout'" },
                            { "type": "command", "command": "echo 'with timeout'", "timeout": 30 },
                            { "type": "command", "command": "echo 'also no timeout'" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_010: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-010")
            .collect();

        assert_eq!(cc_hk_010.len(), 2);
    }

    #[test]
    fn test_fixture_no_timeout() {
        let content =
            include_str!("../../../../tests/fixtures/invalid/hooks/no-timeout/settings.json");
        let diagnostics = validate(content);
        let cc_hk_010: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-010")
            .collect();
        // PreToolUse command and Stop prompt are missing timeout
        assert_eq!(cc_hk_010.len(), 2);
    }

    // ===== CC-HK-011 Tests: Invalid Timeout Value =====

    #[test]
    fn test_cc_hk_011_negative_timeout() {
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "echo test", "timeout": -5 }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-011")
            .collect();

        assert_eq!(cc_hk_011.len(), 1);
        assert_eq!(cc_hk_011[0].level, DiagnosticLevel::Error);
        assert!(cc_hk_011[0].message.contains("Invalid timeout"));
    }

    #[test]
    fn test_cc_hk_011_zero_timeout() {
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "echo test", "timeout": 0 }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-011")
            .collect();

        assert_eq!(cc_hk_011.len(), 1);
        assert_eq!(cc_hk_011[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_cc_hk_011_float_zero_timeout() {
        // Edge case: 0.0 should be treated as invalid (zero is not positive)
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "echo test", "timeout": 0.0 }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-011")
            .collect();

        assert_eq!(cc_hk_011.len(), 1);
        assert_eq!(cc_hk_011[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_cc_hk_011_float_timeout() {
        // Non-integer floats like 30.5 should be invalid
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "echo test", "timeout": 30.5 }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-011")
            .collect();

        assert_eq!(cc_hk_011.len(), 1);
        assert_eq!(cc_hk_011[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_cc_hk_011_whole_float_invalid() {
        // Even whole floats like 30.0 are invalid - must be integer, not float
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "echo test", "timeout": 30.0 }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-011")
            .collect();

        // 30.0 is a float, not an integer - rule requires positive INTEGER
        assert_eq!(cc_hk_011.len(), 1);
        assert_eq!(cc_hk_011[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_cc_hk_011_string_timeout() {
        let content = r#"{
            "hooks": {
                "Stop": [
                    {
                        "hooks": [
                            { "type": "command", "command": "echo test", "timeout": "thirty" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-011")
            .collect();

        assert_eq!(cc_hk_011.len(), 1);
        assert_eq!(cc_hk_011[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_cc_hk_011_null_timeout() {
        let content = r#"{
            "hooks": {
                "Stop": [
                    {
                        "hooks": [
                            { "type": "command", "command": "echo test", "timeout": null }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-011")
            .collect();

        assert_eq!(cc_hk_011.len(), 1);
        assert_eq!(cc_hk_011[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_cc_hk_011_positive_timeout_ok() {
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "echo test", "timeout": 30 }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-011")
            .collect();

        assert_eq!(cc_hk_011.len(), 0);
    }

    #[test]
    fn test_cc_hk_011_multiple_invalid_timeouts() {
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "*",
                        "hooks": [
                            { "type": "command", "command": "echo 'negative'", "timeout": -5 },
                            { "type": "command", "command": "echo 'zero'", "timeout": 0 },
                            { "type": "command", "command": "echo 'valid'", "timeout": 30 }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-011")
            .collect();

        assert_eq!(cc_hk_011.len(), 2);
    }

    #[test]
    fn test_cc_hk_011_missing_timeout_not_triggered() {
        // CC-HK-011 should NOT trigger when timeout is missing (that's CC-HK-010's job)
        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "echo test" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);
        let cc_hk_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-011")
            .collect();

        assert_eq!(cc_hk_011.len(), 0);
    }

    #[test]
    fn test_fixture_invalid_timeout() {
        let content =
            include_str!("../../../../tests/fixtures/invalid/hooks/invalid-timeout/settings.json");
        let diagnostics = validate(content);
        let cc_hk_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-011")
            .collect();
        // zero (0) appears twice - 2 invalid timeouts
        assert_eq!(cc_hk_011.len(), 2);
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

    // ===== Version-Aware CC-HK-010 Tests =====

    #[test]
    fn test_cc_hk_010_assumption_when_version_not_pinned() {
        // Default config has no version pinned
        let config = LintConfig::default();
        assert!(!config.is_claude_code_version_pinned());

        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "echo test" }
                        ]
                    }
                ]
            }
        }"#;

        let validator = HooksValidator;
        let diagnostics = validator.validate(Path::new("settings.json"), content, &config);

        let cc_hk_010: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-010")
            .collect();

        assert_eq!(cc_hk_010.len(), 1);
        // Should have an assumption note when version not pinned
        assert!(cc_hk_010[0].assumption.is_some());
        let assumption = cc_hk_010[0].assumption.as_ref().unwrap();
        assert!(assumption.contains("Assumes Claude Code default timeout behavior"));
        assert!(assumption.contains("[tool_versions]"));
    }

    #[test]
    fn test_cc_hk_010_no_assumption_when_version_pinned() {
        let mut config = LintConfig::default();
        config.tool_versions.claude_code = Some("1.0.0".to_string());
        assert!(config.is_claude_code_version_pinned());

        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "echo test" }
                        ]
                    }
                ]
            }
        }"#;

        let validator = HooksValidator;
        let diagnostics = validator.validate(Path::new("settings.json"), content, &config);

        let cc_hk_010: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-010")
            .collect();

        assert_eq!(cc_hk_010.len(), 1);
        // Should NOT have an assumption note when version is pinned
        assert!(cc_hk_010[0].assumption.is_none());
    }

    #[test]
    fn test_cc_hk_010_prompt_assumption_when_version_not_pinned() {
        let config = LintConfig::default();

        let content = r#"{
            "hooks": {
                "Stop": [
                    {
                        "hooks": [
                            { "type": "prompt", "prompt": "Summarize session" }
                        ]
                    }
                ]
            }
        }"#;

        let validator = HooksValidator;
        let diagnostics = validator.validate(Path::new("settings.json"), content, &config);

        let cc_hk_010: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-010")
            .collect();

        assert_eq!(cc_hk_010.len(), 1);
        assert!(cc_hk_010[0].assumption.is_some());
    }

    #[test]
    fn test_cc_hk_010_exceeds_default_assumption_when_version_not_pinned() {
        let config = LintConfig::default();

        let content = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "echo test", "timeout": 700 }
                        ]
                    }
                ]
            }
        }"#;

        let validator = HooksValidator;
        let diagnostics = validator.validate(Path::new("settings.json"), content, &config);

        let cc_hk_010: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-010")
            .collect();

        assert_eq!(cc_hk_010.len(), 1);
        // Warning about exceeding default should also have assumption when unpinned
        assert!(cc_hk_010[0].assumption.is_some());
    }

    // ===== CC-HK-012: Hooks Parse Error =====

    #[test]
    fn test_cc_hk_012_invalid_json_syntax() {
        let content = r#"{ "hooks": { invalid syntax } }"#;

        let diagnostics = validate(content);

        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-012")
            .collect();
        assert_eq!(parse_errors.len(), 1);
        assert!(parse_errors[0].message.contains("Failed to parse"));
    }

    #[test]
    fn test_cc_hk_012_truncated_json() {
        let content = r#"{"hooks": {"Stop": [{"hooks": [{"type":"command""#;

        let diagnostics = validate(content);

        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-012")
            .collect();
        assert_eq!(parse_errors.len(), 1);
    }

    #[test]
    fn test_cc_hk_012_empty_file() {
        let content = "";

        let diagnostics = validate(content);

        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-012")
            .collect();
        assert_eq!(parse_errors.len(), 1);
    }

    #[test]
    fn test_cc_hk_012_valid_json_no_error() {
        let content = r#"{
            "hooks": {
                "Stop": [
                    {
                        "hooks": [
                            { "type": "command", "command": "echo done" }
                        ]
                    }
                ]
            }
        }"#;

        let diagnostics = validate(content);

        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-012")
            .collect();
        assert_eq!(parse_errors.len(), 0);
    }

    #[test]
    fn test_cc_hk_012_disabled() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["CC-HK-012".to_string()];

        let content = r#"{ invalid json }"#;
        let validator = HooksValidator;
        let diagnostics = validator.validate(Path::new("settings.json"), content, &config);

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-HK-012"));
    }

    #[test]
    fn test_cc_hk_012_missing_required_field_hooks_key() {
        // Valid JSON but missing the "hooks" key entirely - should NOT be CC-HK-012
        let content = r#"{"model": "sonnet"}"#;

        let diagnostics = validate(content);

        // This is valid JSON, just doesn't have hooks - no parse error
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-HK-012"));
    }

    #[test]
    fn test_cc_hk_012_null_value() {
        let content = r#"{"hooks": null}"#;

        let diagnostics = validate(content);

        // null for hooks triggers parse error since HooksSchema expects an object
        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-012")
            .collect();
        assert_eq!(parse_errors.len(), 1);
    }

    #[test]
    fn test_cc_hk_012_array_instead_of_object() {
        let content = r#"["hooks", "array"]"#;

        let diagnostics = validate(content);

        // This is valid JSON but wrong structure - should trigger parse error
        // because SettingsSchema expects an object
        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-012")
            .collect();
        assert_eq!(parse_errors.len(), 1);
    }

    // ===== Additional CC-HK edge case tests =====

    #[test]
    fn test_cc_hk_001_all_valid_events() {
        // Tool events that require matcher
        let tool_events = ["PreToolUse", "PostToolUse", "PermissionRequest"];
        // Non-tool events that don't require matcher
        let non_tool_events = ["Stop", "SubagentStop", "SessionStart"];

        // Test tool events WITH matcher (should be valid)
        for event in tool_events {
            let content = format!(
                r#"{{
                    "hooks": {{
                        "{}": [
                            {{
                                "matcher": "Bash",
                                "hooks": [{{ "type": "command", "command": "echo test" }}]
                            }}
                        ]
                    }}
                }}"#,
                event
            );

            let diagnostics = validate(&content);
            let hk_001: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-HK-001")
                .collect();
            assert!(
                hk_001.is_empty(),
                "Tool event '{}' with matcher should be valid",
                event
            );
        }

        // Test non-tool events WITHOUT matcher (should be valid)
        for event in non_tool_events {
            let content = format!(
                r#"{{
                    "hooks": {{
                        "{}": [
                            {{
                                "hooks": [{{ "type": "command", "command": "echo test" }}]
                            }}
                        ]
                    }}
                }}"#,
                event
            );

            let diagnostics = validate(&content);
            let hk_001: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-HK-001")
                .collect();
            assert!(
                hk_001.is_empty(),
                "Non-tool event '{}' without matcher should be valid",
                event
            );
        }
    }

    #[test]
    fn test_cc_hk_003_all_tool_events_require_matcher() {
        // Must match HooksSchema::TOOL_EVENTS constant
        let tool_events = HooksSchema::TOOL_EVENTS;

        for event in tool_events {
            let content = format!(
                r#"{{
                    "hooks": {{
                        "{}": [
                            {{
                                "hooks": [{{ "type": "command", "command": "echo test" }}]
                            }}
                        ]
                    }}
                }}"#,
                event
            );

            let diagnostics = validate(&content);
            let hk_003: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-HK-003")
                .collect();
            assert_eq!(
                hk_003.len(),
                1,
                "Event '{}' should require matcher but didn't get CC-HK-003",
                event
            );
        }
    }

    #[test]
    fn test_cc_hk_004_non_tool_events_reject_matcher() {
        let non_tool_events = ["Stop", "SubagentStop", "SessionStart"];

        for event in non_tool_events {
            let content = format!(
                r#"{{
                    "hooks": {{
                        "{}": [
                            {{
                                "matcher": "Bash",
                                "hooks": [{{ "type": "command", "command": "echo test" }}]
                            }}
                        ]
                    }}
                }}"#,
                event
            );

            let diagnostics = validate(&content);
            let hk_004: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-HK-004")
                .collect();
            assert_eq!(
                hk_004.len(),
                1,
                "Event '{}' should reject matcher but didn't get CC-HK-004",
                event
            );
        }
    }

    #[test]
    fn test_cc_hk_002_prompt_allowed_events() {
        // Must match HooksSchema::PROMPT_EVENTS constant
        let prompt_allowed = HooksSchema::PROMPT_EVENTS;

        for event in prompt_allowed {
            let content = format!(
                r#"{{
                    "hooks": {{
                        "{}": [
                            {{
                                "hooks": [{{ "type": "prompt", "prompt": "Summarize" }}]
                            }}
                        ]
                    }}
                }}"#,
                event
            );

            let diagnostics = validate(&content);
            let hk_002: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-HK-002")
                .collect();
            assert!(
                hk_002.is_empty(),
                "Prompt on '{}' should be valid but got CC-HK-002",
                event
            );
        }
    }

    #[test]
    fn test_cc_hk_002_prompt_disallowed_events() {
        let prompt_disallowed = [
            "PreToolUse",
            "PostToolUse",
            "SessionStart",
            "PermissionRequest",
        ];

        for event in prompt_disallowed {
            let content = format!(
                r#"{{
                    "hooks": {{
                        "{}": [
                            {{
                                "matcher": "Bash",
                                "hooks": [{{ "type": "prompt", "prompt": "Test" }}]
                            }}
                        ]
                    }}
                }}"#,
                event
            );

            let diagnostics = validate(&content);
            let hk_002: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-HK-002")
                .collect();
            assert_eq!(
                hk_002.len(),
                1,
                "Prompt on '{}' should be invalid but didn't get CC-HK-002",
                event
            );
        }
    }
}
