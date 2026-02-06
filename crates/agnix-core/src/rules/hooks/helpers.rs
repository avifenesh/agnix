use crate::diagnostics::{Diagnostic, Fix};
use crate::schemas::hooks::HooksSchema;
use regex::Regex;
use rust_i18n::t;
use std::path::Path;
use std::sync::OnceLock;

struct DangerousPattern {
    regex: Regex,
    pattern: &'static str,
    reason: &'static str,
}

static DANGEROUS_PATTERNS: OnceLock<Vec<DangerousPattern>> = OnceLock::new();
static SCRIPT_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

fn dangerous_patterns() -> &'static Vec<DangerousPattern> {
    DANGEROUS_PATTERNS.get_or_init(|| {
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
                let regex = Regex::new(&format!("(?i){}", pattern)).unwrap_or_else(|_| {
                    panic!("BUG: invalid dangerous pattern regex: {}", pattern)
                });
                DangerousPattern {
                    regex,
                    pattern,
                    reason,
                }
            })
            .collect()
    })
}

fn script_patterns() -> &'static Vec<Regex> {
    SCRIPT_PATTERNS.get_or_init(|| {
        [
            r#"["']?([^\s"']+\.sh)["']?\b"#,
            r#"["']?([^\s"']+\.bash)["']?\b"#,
            r#"["']?([^\s"']+\.py)["']?\b"#,
            r#"["']?([^\s"']+\.js)["']?\b"#,
            r#"["']?([^\s"']+\.ts)["']?\b"#,
        ]
        .iter()
        .map(|p| {
            Regex::new(p).unwrap_or_else(|_| panic!("BUG: invalid script pattern regex: {}", p))
        })
        .collect()
    })
}

/// CC-HK-005: Missing type field
pub(super) fn validate_cc_hk_005_missing_type_field(
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
                                        t!(
                                            "rules.cc_hk_005.message",
                                            location = hook_location.as_str()
                                        ),
                                    )
                                    .with_suggestion(t!("rules.cc_hk_005.suggestion")),
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
pub(super) fn validate_cc_hk_011_invalid_timeout_values(
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
                                            t!(
                                                "rules.cc_hk_011.message",
                                                location = hook_location.as_str()
                                            ),
                                        )
                                        .with_suggestion(t!("rules.cc_hk_011.suggestion")),
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

/// CC-HK-001: Invalid event name with auto-fix support
pub(super) fn validate_cc_hk_001_event_name(
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
        t!(
            "rules.cc_hk_001.message",
            event = event,
            valid = format!("{:?}", HooksSchema::VALID_EVENTS)
        ),
    )
    .with_suggestion(closest.suggestion);

    // Add auto-fix if we found a matching event
    if let Some(corrected) = closest.corrected_event {
        if let Some((start, end)) = find_event_key_position(content, event) {
            let replacement = format!("\"{}\"", corrected);
            let description = t!("rules.cc_hk_001.fix", old = event, new = corrected);
            // Case-only fixes are safe (high confidence)
            let fix = Fix::replace(start, end, replacement, description, closest.is_case_fix);
            diagnostic = diagnostic.with_fix(fix);
        }
    }

    diagnostics.push(diagnostic);
    false
}

/// CC-HK-003: Missing matcher for tool events
pub(super) fn validate_cc_hk_003_matcher_required(
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
                t!(
                    "rules.cc_hk_003.message",
                    event = event,
                    location = hook_location.as_str()
                ),
            )
            .with_suggestion(t!("rules.cc_hk_003.suggestion")),
        );
    }
}

/// CC-HK-004: Matcher on non-tool event
pub(super) fn validate_cc_hk_004_matcher_forbidden(
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
                t!(
                    "rules.cc_hk_004.message",
                    event = event,
                    location = hook_location.as_str()
                ),
            )
            .with_suggestion(t!("rules.cc_hk_004.suggestion")),
        );
    }
}

pub(super) fn check_dangerous_patterns(command: &str) -> Option<(&'static str, &'static str)> {
    for dp in dangerous_patterns() {
        if dp.regex.is_match(command) {
            return Some((dp.pattern, dp.reason));
        }
    }
    None
}

pub(super) fn extract_script_paths(command: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for re in script_patterns() {
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

pub(super) fn resolve_script_path(script_path: &str, project_dir: &Path) -> std::path::PathBuf {
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

pub(super) fn has_unresolved_env_vars(path: &str) -> bool {
    let after_claude = path
        .replace("$CLAUDE_PROJECT_DIR", "")
        .replace("${CLAUDE_PROJECT_DIR}", "");
    after_claude.contains('$')
}

pub(super) struct ClosestEventMatch {
    pub(super) suggestion: String,
    /// The correct event name if a good match was found
    pub(super) corrected_event: Option<String>,
    /// Whether this is a case-only difference (high confidence)
    pub(super) is_case_fix: bool,
}

pub(super) fn find_closest_event(invalid_event: &str) -> ClosestEventMatch {
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
pub(super) fn find_event_key_position(content: &str, event: &str) -> Option<(usize, usize)> {
    // Look for the event key in the "hooks" object
    // Pattern: capture the quoted event name, followed by : (with optional whitespace)
    let pattern = format!(r#"("{}")\s*:"#, regex::escape(event));
    let re = Regex::new(&pattern).ok()?;
    re.captures(content).and_then(|caps| {
        caps.get(1)
            .map(|key_match| (key_match.start(), key_match.end()))
    })
}
