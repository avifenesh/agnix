//! Cursor project rules validation rules (CUR-001 to CUR-016)
//!
//! Validates:
//! - CUR-001: Empty .mdc rule file (HIGH) - files must have content
//! - CUR-002: Missing frontmatter (MEDIUM) - .mdc files should have frontmatter
//! - CUR-003: Invalid YAML frontmatter (HIGH) - frontmatter must be valid YAML
//! - CUR-004: Invalid glob pattern (HIGH) - globs field must contain valid patterns
//! - CUR-005: Unknown frontmatter keys (MEDIUM) - warn about unrecognized keys
//! - CUR-006: Legacy .cursorrules detected (MEDIUM) - migration warning
//! - CUR-007: alwaysApply with redundant globs (MEDIUM) - globs ignored when alwaysApply is true
//! - CUR-008: Invalid alwaysApply type (HIGH) - must be boolean, not string
//! - CUR-009: Missing description for agent-requested rule (MEDIUM) - agent needs description
//! - CUR-010: Invalid .cursor/hooks.json schema (HIGH)
//! - CUR-011: Unknown hook event name in .cursor/hooks.json (MEDIUM)
//! - CUR-012: Hook entry missing required command field (HIGH)
//! - CUR-013: Invalid hook type value (HIGH)
//! - CUR-014: Invalid Cursor subagent frontmatter (HIGH)
//! - CUR-015: Empty Cursor subagent body (MEDIUM)
//! - CUR-016: Invalid .cursor/environment.json schema (HIGH)

use crate::{
    FileType,
    config::LintConfig,
    diagnostics::{Diagnostic, Fix},
    parsers::frontmatter::split_frontmatter,
    rules::{Validator, ValidatorMetadata},
    schemas::cursor::{
        ParsedMdcFrontmatter, is_body_empty, is_content_empty, parse_mdc_frontmatter,
        validate_glob_pattern,
    },
};
use rust_i18n::t;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::path::Path;

const RULE_IDS: &[&str] = &[
    "CUR-001", "CUR-002", "CUR-003", "CUR-004", "CUR-005", "CUR-006", "CUR-007", "CUR-008",
    "CUR-009", "CUR-010", "CUR-011", "CUR-012", "CUR-013", "CUR-014", "CUR-015", "CUR-016",
];

const CURSOR_HOOK_EVENTS: &[&str] = &[
    "sessionStart",
    "sessionEnd",
    "preToolUse",
    "postToolUse",
    "postToolUseFailure",
    "subagentStart",
    "subagentStop",
    "beforeShellExecution",
    "afterShellExecution",
    "beforeMCPExecution",
    "afterMCPExecution",
    "beforeReadFile",
    "afterFileEdit",
    "beforeSubmitPrompt",
    "preCompact",
    "stop",
    "afterAgentResponse",
    "afterAgentThought",
    "beforeTabFileRead",
    "afterTabFileEdit",
];

const CURSOR_HOOK_TYPES: &[&str] = &["command", "prompt"];

pub struct CursorValidator;

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

/// Find the 1-indexed line number of a YAML field in parsed frontmatter.
fn find_field_line(parsed: &ParsedMdcFrontmatter, field_prefix: &str) -> usize {
    parsed
        .raw
        .lines()
        .enumerate()
        .find(|(_, line)| line.trim_start().starts_with(field_prefix))
        .map(|(idx, _)| parsed.start_line + 1 + idx)
        .unwrap_or(parsed.start_line)
}

/// Get the byte range of a YAML block (key line + indented continuation lines).
/// This is used for auto-fix deletion of multi-line fields like list-style `globs:`.
fn yaml_block_byte_range(content: &str, start_line: usize) -> Option<(usize, usize)> {
    let (block_start, mut block_end) = line_byte_range(content, start_line)?;

    // Extend to include subsequent indented lines (list items under the key)
    let mut current_line = start_line + 1;
    while let Some((next_start, next_end)) = line_byte_range(content, current_line) {
        let line_text = &content[next_start..next_end.min(content.len())];
        // Stop if the line is not indented (new top-level key or closing ---)
        if !line_text.starts_with(' ') && !line_text.starts_with('\t') {
            break;
        }
        block_end = next_end;
        current_line += 1;
    }

    Some((block_start, block_end))
}

/// Find the byte range of a quoted YAML value for a given key in frontmatter.
/// Returns the range including quotes (e.g., `"true"` or `'false'`).
/// Wrapper around the shared helper.
fn find_yaml_quoted_value_range(
    content: &str,
    parsed: &ParsedMdcFrontmatter,
    key: &str,
) -> Option<(usize, usize)> {
    crate::rules::find_yaml_value_range(content, parsed, key, true)
}

fn json_type_name(value: &JsonValue) -> &'static str {
    match value {
        JsonValue::Null => "null",
        JsonValue::Bool(_) => "boolean",
        JsonValue::Number(_) => "number",
        JsonValue::String(_) => "string",
        JsonValue::Array(_) => "array",
        JsonValue::Object(_) => "object",
    }
}

fn is_valid_cursor_agent_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_valid_cursor_model_id(model: &str) -> bool {
    !model.trim().is_empty()
        && model
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':' | '/'))
}

fn validate_cursor_hooks_file(path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let parsed = match serde_json::from_str::<JsonValue>(content) {
        Ok(value) => value,
        Err(error) => {
            if config.is_rule_enabled("CUR-010") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CUR-010",
                        t!("rules.cur_010.parse_error", error = error.to_string()),
                    )
                    .with_suggestion(t!("rules.cur_010.suggestion")),
                );
            }
            return diagnostics;
        }
    };

    let root = match parsed.as_object() {
        Some(obj) => obj,
        None => {
            if config.is_rule_enabled("CUR-010") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CUR-010",
                        t!("rules.cur_010.message"),
                    )
                    .with_suggestion(t!("rules.cur_010.suggestion")),
                );
            }
            return diagnostics;
        }
    };

    if config.is_rule_enabled("CUR-010")
        && root.get("version").and_then(JsonValue::as_i64).is_none()
    {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "CUR-010",
                t!("rules.cur_010.missing_version"),
            )
            .with_suggestion(t!("rules.cur_010.suggestion")),
        );
    }

    let hooks = match root.get("hooks") {
        Some(JsonValue::Object(map)) => map,
        Some(other) => {
            if config.is_rule_enabled("CUR-010") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CUR-010",
                        t!("rules.cur_010.invalid_hooks", got = json_type_name(other)),
                    )
                    .with_suggestion(t!("rules.cur_010.suggestion")),
                );
            }
            return diagnostics;
        }
        None => {
            if config.is_rule_enabled("CUR-010") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CUR-010",
                        t!("rules.cur_010.missing_hooks"),
                    )
                    .with_suggestion(t!("rules.cur_010.suggestion")),
                );
            }
            return diagnostics;
        }
    };

    for (event_name, hooks_value) in hooks {
        if config.is_rule_enabled("CUR-011") && !CURSOR_HOOK_EVENTS.contains(&event_name.as_str()) {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "CUR-011",
                    t!("rules.cur_011.message", event = event_name.as_str()),
                )
                .with_suggestion(t!("rules.cur_011.suggestion")),
            );
        }

        let hook_array = match hooks_value.as_array() {
            Some(entries) => entries,
            None => {
                if config.is_rule_enabled("CUR-010") {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CUR-010",
                            t!(
                                "rules.cur_010.invalid_event_hooks",
                                event = event_name.as_str(),
                                got = json_type_name(hooks_value)
                            ),
                        )
                        .with_suggestion(t!("rules.cur_010.suggestion")),
                    );
                }
                continue;
            }
        };

        for (index, hook_entry) in hook_array.iter().enumerate() {
            let hook_obj = match hook_entry.as_object() {
                Some(obj) => obj,
                None => {
                    if config.is_rule_enabled("CUR-010") {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
                                "CUR-010",
                                t!(
                                    "rules.cur_010.invalid_hook_entry",
                                    event = event_name.as_str(),
                                    index = index + 1
                                ),
                            )
                            .with_suggestion(t!("rules.cur_010.suggestion")),
                        );
                    }
                    continue;
                }
            };

            if config.is_rule_enabled("CUR-013")
                && let Some(type_value) = hook_obj.get("type")
            {
                let type_str = type_value.as_str();
                if type_str.is_none() || !CURSOR_HOOK_TYPES.contains(&type_str.unwrap_or_default())
                {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CUR-013",
                            t!(
                                "rules.cur_013.message",
                                event = event_name.as_str(),
                                index = index + 1
                            ),
                        )
                        .with_suggestion(t!("rules.cur_013.suggestion")),
                    );
                }
            }

            if config.is_rule_enabled("CUR-012") {
                let has_valid_command = hook_obj
                    .get("command")
                    .and_then(JsonValue::as_str)
                    .is_some_and(|command| !command.trim().is_empty());

                if !has_valid_command {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CUR-012",
                            t!(
                                "rules.cur_012.message",
                                event = event_name.as_str(),
                                index = index + 1
                            ),
                        )
                        .with_suggestion(t!("rules.cur_012.suggestion")),
                    );
                }
            }
        }
    }

    diagnostics
}

fn validate_cursor_agent_file(path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let parts = split_frontmatter(content);

    if config.is_rule_enabled("CUR-014") {
        if !parts.has_frontmatter || !parts.has_closing {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "CUR-014",
                    t!("rules.cur_014.message"),
                )
                .with_suggestion(t!("rules.cur_014.suggestion")),
            );
        } else {
            let frontmatter = match serde_yaml::from_str::<YamlValue>(&parts.frontmatter) {
                Ok(value) => Some(value),
                Err(_) => {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CUR-014",
                            t!("rules.cur_014.invalid_frontmatter"),
                        )
                        .with_suggestion(t!("rules.cur_014.suggestion")),
                    );
                    None
                }
            };

            if let Some(frontmatter_map) = frontmatter.as_ref().and_then(YamlValue::as_mapping) {
                let key = |name: &str| YamlValue::String(name.to_string());

                match frontmatter_map.get(key("name")) {
                    Some(YamlValue::String(name)) if is_valid_cursor_agent_name(name) => {}
                    Some(YamlValue::String(_)) => diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CUR-014",
                            t!("rules.cur_014.invalid_name"),
                        )
                        .with_suggestion(t!("rules.cur_014.suggestion")),
                    ),
                    Some(_) => diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CUR-014",
                            t!("rules.cur_014.name_not_string"),
                        )
                        .with_suggestion(t!("rules.cur_014.suggestion")),
                    ),
                    None => diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CUR-014",
                            t!("rules.cur_014.missing_name"),
                        )
                        .with_suggestion(t!("rules.cur_014.suggestion")),
                    ),
                }

                match frontmatter_map.get(key("description")) {
                    Some(YamlValue::String(_)) => {}
                    Some(_) => diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CUR-014",
                            t!("rules.cur_014.description_not_string"),
                        )
                        .with_suggestion(t!("rules.cur_014.suggestion")),
                    ),
                    None => diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CUR-014",
                            t!("rules.cur_014.missing_description"),
                        )
                        .with_suggestion(t!("rules.cur_014.suggestion")),
                    ),
                }

                if let Some(model_value) = frontmatter_map.get(key("model")) {
                    match model_value {
                        YamlValue::String(model)
                            if model == "fast"
                                || model == "inherit"
                                || is_valid_cursor_model_id(model) => {}
                        YamlValue::String(_) => diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
                                "CUR-014",
                                t!("rules.cur_014.invalid_model"),
                            )
                            .with_suggestion(t!("rules.cur_014.suggestion")),
                        ),
                        _ => diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
                                "CUR-014",
                                t!("rules.cur_014.model_not_string"),
                            )
                            .with_suggestion(t!("rules.cur_014.suggestion")),
                        ),
                    }
                }

                if let Some(readonly_value) = frontmatter_map.get(key("readonly"))
                    && !matches!(readonly_value, YamlValue::Bool(_))
                {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CUR-014",
                            t!("rules.cur_014.readonly_not_bool"),
                        )
                        .with_suggestion(t!("rules.cur_014.suggestion")),
                    );
                }

                if let Some(is_background_value) = frontmatter_map.get(key("is_background"))
                    && !matches!(is_background_value, YamlValue::Bool(_))
                {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CUR-014",
                            t!("rules.cur_014.is_background_not_bool"),
                        )
                        .with_suggestion(t!("rules.cur_014.suggestion")),
                    );
                }
            } else if frontmatter.is_some() && config.is_rule_enabled("CUR-014") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CUR-014",
                        t!("rules.cur_014.invalid_frontmatter"),
                    )
                    .with_suggestion(t!("rules.cur_014.suggestion")),
                );
            }
        }
    }

    if config.is_rule_enabled("CUR-015")
        && parts.has_frontmatter
        && parts.has_closing
        && parts.body.trim().is_empty()
    {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                1,
                0,
                "CUR-015",
                t!("rules.cur_015.message"),
            )
            .with_suggestion(t!("rules.cur_015.suggestion")),
        );
    }

    diagnostics
}

fn validate_cursor_environment_file(
    path: &Path,
    content: &str,
    config: &LintConfig,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if !config.is_rule_enabled("CUR-016") {
        return diagnostics;
    }

    let parsed = match serde_json::from_str::<JsonValue>(content) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "CUR-016",
                    t!("rules.cur_016.parse_error", error = error.to_string()),
                )
                .with_suggestion(t!("rules.cur_016.suggestion")),
            );
            return diagnostics;
        }
    };

    let root = match parsed.as_object() {
        Some(obj) => obj,
        None => {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "CUR-016",
                    t!("rules.cur_016.message"),
                )
                .with_suggestion(t!("rules.cur_016.suggestion")),
            );
            return diagnostics;
        }
    };

    if root.get("snapshot").and_then(JsonValue::as_str).is_none() {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "CUR-016",
                t!("rules.cur_016.snapshot"),
            )
            .with_suggestion(t!("rules.cur_016.suggestion")),
        );
    }

    if root.get("install").and_then(JsonValue::as_str).is_none() {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "CUR-016",
                t!("rules.cur_016.install"),
            )
            .with_suggestion(t!("rules.cur_016.suggestion")),
        );
    }

    if let Some(start) = root.get("start")
        && start.as_str().is_none()
    {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "CUR-016",
                t!("rules.cur_016.start"),
            )
            .with_suggestion(t!("rules.cur_016.suggestion")),
        );
    }

    match root.get("terminals") {
        Some(JsonValue::Array(terminals)) => {
            for (index, terminal) in terminals.iter().enumerate() {
                if let Some(obj) = terminal.as_object() {
                    let has_name = obj.get("name").and_then(JsonValue::as_str).is_some();
                    let has_command = obj.get("command").and_then(JsonValue::as_str).is_some();
                    if !has_name || !has_command {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
                                "CUR-016",
                                t!("rules.cur_016.terminal", index = index + 1),
                            )
                            .with_suggestion(t!("rules.cur_016.suggestion")),
                        );
                    }
                } else {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CUR-016",
                            t!("rules.cur_016.terminal", index = index + 1),
                        )
                        .with_suggestion(t!("rules.cur_016.suggestion")),
                    );
                }
            }
        }
        Some(other) => diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "CUR-016",
                t!(
                    "rules.cur_016.invalid_terminals",
                    got = json_type_name(other)
                ),
            )
            .with_suggestion(t!("rules.cur_016.suggestion")),
        ),
        None => diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "CUR-016",
                t!("rules.cur_016.missing_terminals"),
            )
            .with_suggestion(t!("rules.cur_016.suggestion")),
        ),
    }

    diagnostics
}

impl Validator for CursorValidator {
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
            FileType::CursorHooks => return validate_cursor_hooks_file(path, content, config),
            FileType::CursorAgent => return validate_cursor_agent_file(path, content, config),
            FileType::CursorEnvironment => {
                return validate_cursor_environment_file(path, content, config);
            }
            _ => {}
        }

        // Determine if this is a .mdc rule file or legacy .cursorrules
        let is_legacy = file_type == FileType::CursorRulesLegacy;

        // CUR-006: Legacy .cursorrules detected (WARNING)
        if is_legacy && config.is_rule_enabled("CUR-006") {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "CUR-006",
                    t!("rules.cur_006.message"),
                )
                .with_suggestion(t!("rules.cur_006.suggestion")),
            );
            // For legacy files, just check if empty and return
            if config.is_rule_enabled("CUR-001") && is_content_empty(content) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CUR-001",
                        t!("rules.cur_006.legacy_empty"),
                    )
                    .with_suggestion(t!("rules.cur_001.suggestion_legacy_empty")),
                );
            }
            return diagnostics;
        }

        // CUR-001: Empty .mdc rule file (ERROR)
        let parsed_frontmatter = parse_mdc_frontmatter(content);

        if config.is_rule_enabled("CUR-001") {
            if let Some(parsed) = parsed_frontmatter.as_ref() {
                // Skip CUR-001 if there's a frontmatter parse error - CUR-003 will handle it
                if parsed.parse_error.is_none() && is_body_empty(&parsed.body) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            parsed.end_line + 1,
                            0,
                            "CUR-001",
                            t!("rules.cur_001.message_no_content"),
                        )
                        .with_suggestion(t!("rules.cur_001.suggestion_no_content")),
                    );
                }
            } else if is_content_empty(content) {
                // No frontmatter and no content
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CUR-001",
                        t!("rules.cur_001.message_empty"),
                    )
                    .with_suggestion(t!("rules.cur_001.suggestion_empty")),
                );
            }
        }

        // Parse frontmatter for further validation
        let parsed = match parsed_frontmatter {
            Some(p) => p,
            None => {
                // CUR-002: Missing frontmatter in .mdc file (WARNING)
                if config.is_rule_enabled("CUR-002") && !is_content_empty(content) {
                    let mut diagnostic = Diagnostic::warning(
                        path.to_path_buf(),
                        1,
                        0,
                        "CUR-002",
                        t!("rules.cur_002.message"),
                    )
                    .with_suggestion(t!("rules.cur_002.suggestion"));

                    // Unsafe auto-fix: insert template frontmatter at start of file.
                    diagnostic = diagnostic.with_fix(Fix::insert(
                        0,
                        "---\ndescription: \nglobs: \n---\n",
                        t!("rules.cur_002.fix"),
                        false,
                    ));

                    diagnostics.push(diagnostic);
                }
                return diagnostics;
            }
        };

        // CUR-003: Invalid YAML frontmatter (ERROR)
        if config.is_rule_enabled("CUR-003") {
            if let Some(ref error) = parsed.parse_error {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        parsed.start_line,
                        0,
                        "CUR-003",
                        t!("rules.cur_003.message", error = error.as_str()),
                    )
                    .with_suggestion(t!("rules.cur_003.suggestion")),
                );
                // Can't continue validating if YAML is broken
                return diagnostics;
            }
        }

        // CUR-004: Invalid glob pattern (ERROR)
        if config.is_rule_enabled("CUR-004") {
            if let Some(ref schema) = parsed.schema {
                if let Some(ref globs) = schema.globs {
                    let globs_line = find_field_line(&parsed, "globs:");

                    for pattern in globs.patterns() {
                        let validation = validate_glob_pattern(pattern);
                        if !validation.valid {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    globs_line,
                                    0,
                                    "CUR-004",
                                    t!(
                                        "rules.cur_004.message",
                                        pattern = pattern,
                                        error = validation.error.unwrap_or_default()
                                    ),
                                )
                                .with_suggestion(t!("rules.cur_004.suggestion")),
                            );
                        }
                    }
                }
            }
        }

        // CUR-005: Unknown frontmatter keys (WARNING)
        if config.is_rule_enabled("CUR-005") {
            for unknown in &parsed.unknown_keys {
                let mut diagnostic = Diagnostic::warning(
                    path.to_path_buf(),
                    unknown.line,
                    unknown.column,
                    "CUR-005",
                    t!("rules.cur_005.message", key = unknown.key.as_str()),
                )
                .with_suggestion(t!("rules.cur_005.suggestion", key = unknown.key.as_str()));

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

        // CUR-007, CUR-008, CUR-009 require a successfully parsed schema
        if let Some(ref schema) = parsed.schema {
            // CUR-008: Invalid alwaysApply type (ERROR)
            // Must be a boolean, not a quoted string like "true" or "false"
            if config.is_rule_enabled("CUR-008") {
                if let Some(crate::schemas::cursor::AlwaysApplyField::String(s)) =
                    schema.always_apply.as_ref()
                {
                    let always_apply_line = find_field_line(&parsed, "alwaysApply:");

                    let mut diagnostic = Diagnostic::error(
                        path.to_path_buf(),
                        always_apply_line,
                        0,
                        "CUR-008",
                        t!("rules.cur_008.message"),
                    )
                    .with_suggestion(t!("rules.cur_008.suggestion"));

                    // Safe auto-fix: convert quoted string to boolean
                    let lower = s.to_lowercase();
                    if lower == "true" || lower == "false" {
                        let bool_str = if lower == "true" { "true" } else { "false" };
                        // Find the quoted value in the raw frontmatter
                        if let Some((start, end)) =
                            find_yaml_quoted_value_range(content, &parsed, "alwaysApply")
                        {
                            diagnostic = diagnostic.with_fix(Fix::replace(
                                start,
                                end,
                                bool_str,
                                t!("rules.cur_008.fix", value = s.as_str(), fixed = bool_str),
                                true,
                            ));
                        }
                    }

                    diagnostics.push(diagnostic);
                }
            }

            // CUR-007: alwaysApply with redundant globs (WARNING)
            // When alwaysApply: true, globs are ignored
            if config.is_rule_enabled("CUR-007") {
                let is_always_apply = schema
                    .always_apply
                    .as_ref()
                    .and_then(|a| a.as_bool())
                    .unwrap_or(false);

                if is_always_apply && schema.globs.is_some() {
                    let globs_line = find_field_line(&parsed, "globs:");

                    let mut diagnostic = Diagnostic::warning(
                        path.to_path_buf(),
                        globs_line,
                        0,
                        "CUR-007",
                        t!("rules.cur_007.message"),
                    )
                    .with_suggestion(t!("rules.cur_007.suggestion"));

                    // Safe auto-fix: remove the entire globs block (key + indented children)
                    if let Some((start, end)) = yaml_block_byte_range(content, globs_line) {
                        diagnostic = diagnostic.with_fix(Fix::delete(
                            start,
                            end,
                            "Remove redundant globs field".to_string(),
                            true,
                        ));
                    }

                    diagnostics.push(diagnostic);
                }
            }

            // CUR-009: Missing description for agent-requested rule (WARNING)
            // If no alwaysApply and no globs, the rule is "agent-requested" and
            // the agent uses the description to decide when to apply it.
            if config.is_rule_enabled("CUR-009") {
                let has_always_apply = schema.always_apply.is_some();
                let has_globs = schema.globs.is_some();
                let has_description = !schema
                    .description
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .is_empty();

                if !has_always_apply && !has_globs && !has_description {
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            parsed.start_line,
                            0,
                            "CUR-009",
                            t!("rules.cur_009.message"),
                        )
                        .with_suggestion(t!("rules.cur_009.suggestion")),
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
    use crate::diagnostics::DiagnosticLevel;

    fn validate_mdc(content: &str) -> Vec<Diagnostic> {
        let validator = CursorValidator;
        validator.validate(
            Path::new(".cursor/rules/typescript.mdc"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_legacy(content: &str) -> Vec<Diagnostic> {
        let validator = CursorValidator;
        validator.validate(Path::new(".cursorrules"), content, &LintConfig::default())
    }

    fn validate_mdc_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = CursorValidator;
        validator.validate(Path::new(".cursor/rules/typescript.mdc"), content, config)
    }

    fn validate_cursor_hooks(content: &str) -> Vec<Diagnostic> {
        let validator = CursorValidator;
        validator.validate(
            Path::new(".cursor/hooks.json"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_cursor_agent(content: &str) -> Vec<Diagnostic> {
        let validator = CursorValidator;
        validator.validate(
            Path::new(".cursor/agents/reviewer.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_cursor_environment(content: &str) -> Vec<Diagnostic> {
        let validator = CursorValidator;
        validator.validate(
            Path::new(".cursor/environment.json"),
            content,
            &LintConfig::default(),
        )
    }

    // ===== CUR-001: Empty Rule File =====

    #[test]
    fn test_cur_001_empty_mdc_file() {
        let diagnostics = validate_mdc("");
        let cur_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-001").collect();
        assert_eq!(cur_001.len(), 1);
        assert_eq!(cur_001[0].level, DiagnosticLevel::Error);
        assert!(cur_001[0].message.contains("empty"));
    }

    #[test]
    fn test_cur_001_whitespace_only() {
        let diagnostics = validate_mdc("   \n\n\t  ");
        let cur_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-001").collect();
        assert_eq!(cur_001.len(), 1);
    }

    #[test]
    fn test_cur_001_valid_mdc_file() {
        let content = r#"---
description: TypeScript rules
globs: "**/*.ts"
---
# TypeScript Rules

Use strict mode.
"#;
        let diagnostics = validate_mdc(content);
        let cur_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-001").collect();
        assert!(cur_001.is_empty());
    }

    #[test]
    fn test_cur_001_empty_body_after_frontmatter() {
        let content = r#"---
description: Empty body
globs: "**/*.ts"
---
"#;
        let diagnostics = validate_mdc(content);
        let cur_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-001").collect();
        assert_eq!(cur_001.len(), 1);
        assert!(cur_001[0].message.contains("no content after frontmatter"));
    }

    #[test]
    fn test_cur_001_skips_when_parse_error() {
        // When frontmatter has parse error (missing closing ---),
        // CUR-001 should NOT trigger - CUR-003 handles it
        let content = r#"---
description: Unclosed frontmatter
# Missing closing ---
"#;
        let diagnostics = validate_mdc(content);
        let cur_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-001").collect();
        assert!(
            cur_001.is_empty(),
            "CUR-001 should not trigger when parse_error exists"
        );

        // Verify CUR-003 triggers instead
        let cur_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-003").collect();
        assert_eq!(cur_003.len(), 1);
        assert!(cur_003[0].message.contains("missing closing ---"));
    }

    // ===== CUR-002: Missing Frontmatter =====

    #[test]
    fn test_cur_002_missing_frontmatter() {
        let content = "# Rules without frontmatter";
        let diagnostics = validate_mdc(content);
        let cur_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-002").collect();
        assert_eq!(cur_002.len(), 1);
        assert_eq!(cur_002[0].level, DiagnosticLevel::Warning);
        assert!(
            cur_002[0]
                .message
                .contains("missing recommended frontmatter")
        );
    }

    #[test]
    fn test_cur_002_has_autofix() {
        let content = "# Rules without frontmatter";
        let diagnostics = validate_mdc(content);
        let cur_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-002").collect();
        assert_eq!(cur_002.len(), 1);
        assert!(
            cur_002[0].has_fixes(),
            "CUR-002 should have auto-fix for missing frontmatter"
        );
        let fix = &cur_002[0].fixes[0];
        assert!(!fix.safe, "CUR-002 fix should be unsafe");
        assert_eq!(fix.start_byte, 0, "Fix should insert at start of file");
        assert_eq!(fix.end_byte, 0, "Fix should be an insert (start == end)");
        assert!(
            fix.replacement.contains("---\n"),
            "Fix should contain frontmatter markers"
        );
        assert!(
            fix.replacement.contains("description:"),
            "Fix should contain description field"
        );
        assert!(
            fix.replacement.contains("globs:"),
            "Fix should contain globs field"
        );
    }

    #[test]
    fn test_cur_002_no_autofix_for_empty() {
        // Empty files should not trigger CUR-002 (CUR-001 handles them)
        let diagnostics = validate_mdc("");
        let cur_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-002").collect();
        assert!(cur_002.is_empty());
    }

    #[test]
    fn test_cur_002_has_frontmatter() {
        let content = r#"---
description: Valid
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-002").collect();
        assert!(cur_002.is_empty());
    }

    // ===== CUR-003: Invalid YAML Frontmatter =====

    #[test]
    fn test_cur_003_invalid_yaml() {
        let content = r#"---
globs: [unclosed
---
# Body
"#;
        let diagnostics = validate_mdc(content);
        let cur_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-003").collect();
        assert_eq!(cur_003.len(), 1);
        assert_eq!(cur_003[0].level, DiagnosticLevel::Error);
        assert!(cur_003[0].message.contains("Invalid YAML"));
    }

    #[test]
    fn test_cur_003_unclosed_frontmatter() {
        let content = r#"---
description: Test
# Missing closing ---
"#;
        let diagnostics = validate_mdc(content);
        let cur_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-003").collect();
        assert_eq!(cur_003.len(), 1);
        assert!(cur_003[0].message.contains("missing closing ---"));
    }

    #[test]
    fn test_cur_003_valid_yaml() {
        let content = r#"---
description: Valid YAML
globs: "**/*.ts"
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-003").collect();
        assert!(cur_003.is_empty());
    }

    // ===== CUR-004: Invalid Glob Pattern =====

    #[test]
    fn test_cur_004_invalid_glob() {
        let content = r#"---
description: Bad glob
globs: "[unclosed"
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-004").collect();
        assert_eq!(cur_004.len(), 1);
        assert_eq!(cur_004[0].level, DiagnosticLevel::Error);
        assert!(cur_004[0].message.contains("Invalid glob pattern"));
    }

    #[test]
    fn test_cur_004_invalid_glob_in_array() {
        let content = r#"---
description: Some bad globs
globs:
  - "**/*.ts"
  - "[unclosed"
  - "**/*.js"
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-004").collect();
        assert_eq!(cur_004.len(), 1);
        assert!(cur_004[0].message.contains("[unclosed"));
    }

    #[test]
    fn test_cur_004_valid_glob_patterns() {
        let patterns = vec!["**/*.ts", "*.rs", "src/**/*.js", "tests/**/*.test.ts"];

        for pattern in patterns {
            let content = format!(
                r#"---
description: Test
globs: "{}"
---
# Rules
"#,
                pattern
            );
            let diagnostics = validate_mdc(&content);
            let cur_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-004").collect();
            assert!(cur_004.is_empty(), "Pattern '{}' should be valid", pattern);
        }
    }

    #[test]
    fn test_cur_004_line_number_accuracy() {
        // Test that CUR-004 reports the line number of the globs field, not frontmatter start
        let content = r#"---
description: Bad glob
globs: "[unclosed"
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-004").collect();
        assert_eq!(cur_004.len(), 1);
        // globs: is on line 3 (line 1 is ---, line 2 is description, line 3 is globs)
        assert_eq!(
            cur_004[0].line, 3,
            "CUR-004 should point to the globs field line"
        );
    }

    // ===== CUR-005: Unknown Frontmatter Keys =====

    #[test]
    fn test_cur_005_unknown_keys() {
        let content = r#"---
description: Valid key
unknownKey: value
anotherBadKey: 123
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-005").collect();
        assert_eq!(cur_005.len(), 2);
        assert_eq!(cur_005[0].level, DiagnosticLevel::Warning);
        assert!(cur_005.iter().any(|d| d.message.contains("unknownKey")));
        assert!(cur_005.iter().any(|d| d.message.contains("anotherBadKey")));
        assert!(
            cur_005.iter().all(|d| d.has_fixes()),
            "All unknown key diagnostics should include safe deletion fixes"
        );
        assert!(cur_005.iter().all(|d| d.fixes[0].safe));
    }

    #[test]
    fn test_cur_005_no_unknown_keys() {
        let content = r#"---
description: Valid
globs: "**/*.rs"
alwaysApply: true
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-005").collect();
        assert!(cur_005.is_empty());
    }

    // ===== CUR-006: Legacy .cursorrules =====

    #[test]
    fn test_cur_006_legacy_file() {
        let content = "# Legacy rules content";
        let diagnostics = validate_legacy(content);
        let cur_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-006").collect();
        assert_eq!(cur_006.len(), 1);
        assert_eq!(cur_006[0].level, DiagnosticLevel::Warning);
        assert!(cur_006[0].message.contains("Legacy .cursorrules"));
        assert!(cur_006[0].message.contains("migrating"));
    }

    #[test]
    fn test_cur_006_legacy_empty() {
        let content = "";
        let diagnostics = validate_legacy(content);
        // Should have both CUR-006 (legacy warning) and CUR-001 (empty file)
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-006"));
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-001"));
    }

    #[test]
    fn test_mdc_file_no_cur_006() {
        // .mdc files should NOT trigger CUR-006
        let content = r#"---
description: Modern format
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-006").collect();
        assert!(cur_006.is_empty());
    }

    // ===== CUR-010 to CUR-016: Cursor hooks/agents/environment =====

    #[test]
    fn test_cur_010_hooks_schema_invalid() {
        let diagnostics = validate_cursor_hooks(r#"{"hooks": {}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-010"));
    }

    #[test]
    fn test_cur_010_hooks_json_parse_error() {
        let diagnostics = validate_cursor_hooks(r#"{"version":1,"hooks":{"sessionStart":[}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-010"));
    }

    #[test]
    fn test_cur_010_hooks_root_must_be_object() {
        let diagnostics = validate_cursor_hooks(r#"["not", "an", "object"]"#);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-010"));
    }

    #[test]
    fn test_cur_010_hooks_field_must_be_object() {
        let diagnostics = validate_cursor_hooks(r#"{"version":1,"hooks":[]}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-010"));
    }

    #[test]
    fn test_cur_010_event_hooks_must_be_array() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"sessionStart":{"type":"command","command":"echo hi"}}}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-010"));
    }

    #[test]
    fn test_cur_010_hook_entries_must_be_objects() {
        let diagnostics =
            validate_cursor_hooks(r#"{"version":1,"hooks":{"sessionStart":["echo hi"]}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-010"));
    }

    #[test]
    fn test_cur_011_unknown_hook_event() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"unknownEvent":[{"type":"command","command":"echo hi"}]}}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-011"));
    }

    #[test]
    fn test_cur_012_missing_hook_command() {
        let diagnostics =
            validate_cursor_hooks(r#"{"version":1,"hooks":{"sessionStart":[{"type":"command"}]}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-012"));
    }

    #[test]
    fn test_cur_012_command_must_be_non_empty_string() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"sessionStart":[{"type":"command","command":""}]}}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-012"));
    }

    #[test]
    fn test_cur_013_invalid_hook_type() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"sessionStart":[{"type":"agent","command":"echo hi"}]}}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-013"));
    }

    #[test]
    fn test_cur_014_invalid_cursor_agent_frontmatter() {
        let content = r#"---
name: ReviewerAgent
description: 123
readonly: "true"
---
Review code changes."#;
        let diagnostics = validate_cursor_agent(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-014"));
    }

    #[test]
    fn test_cur_014_malformed_yaml_reports_once() {
        let diagnostics = validate_cursor_agent(
            r#"---
name: reviewer-agent
description: [unclosed
---
Review changes."#,
        );
        let cur_014: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-014").collect();
        assert_eq!(cur_014.len(), 1);
    }

    #[test]
    fn test_cur_014_missing_required_fields() {
        let diagnostics = validate_cursor_agent(
            r#"---
model: fast
---
Review changes."#,
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "CUR-014"),
            "Expected CUR-014 when required fields are missing",
        );
    }

    #[test]
    fn test_cur_015_empty_cursor_agent_body() {
        let content = r#"---
name: reviewer-agent
description: Reviews pull requests
model: fast
readonly: true
is_background: false
---
"#;
        let diagnostics = validate_cursor_agent(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-015"));
    }

    #[test]
    fn test_cur_016_invalid_environment_schema() {
        let diagnostics = validate_cursor_environment(
            r#"{"snapshot":42,"install":"npm ci","terminals":[{"name":"main"}]}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-016"));
    }

    #[test]
    fn test_cur_016_environment_parse_error() {
        let diagnostics = validate_cursor_environment(r#"{"snapshot":"ubuntu","install":}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-016"));
    }

    #[test]
    fn test_cur_016_environment_root_must_be_object() {
        let diagnostics = validate_cursor_environment(r#"["snapshot","install","terminals"]"#);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-016"));
    }

    #[test]
    fn test_cur_016_environment_missing_or_invalid_terminals() {
        let missing = validate_cursor_environment(r#"{"snapshot":"ubuntu","install":"npm ci"}"#);
        assert!(missing.iter().any(|d| d.rule == "CUR-016"));

        let invalid_type = validate_cursor_environment(
            r#"{"snapshot":"ubuntu","install":"npm ci","terminals":{}}"#,
        );
        assert!(invalid_type.iter().any(|d| d.rule == "CUR-016"));
    }

    #[test]
    fn test_cur_016_environment_start_must_be_string() {
        let diagnostics = validate_cursor_environment(
            r#"{"snapshot":"ubuntu","install":"npm ci","start":42,"terminals":[{"name":"app","command":"npm run dev"}]}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-016"));
    }

    #[test]
    fn test_cursor_hooks_agents_environment_valid() {
        let hooks =
            r#"{"version":1,"hooks":{"sessionStart":[{"type":"command","command":"echo start"}]}}"#;
        let agent = r#"---
name: reviewer-agent
description: Reviews code quality
model: fast
readonly: true
is_background: false
---
Review the diff and suggest improvements."#;
        let environment = r#"{
  "snapshot": "ubuntu-24.04",
  "install": "npm ci",
  "start": "npm run dev",
  "terminals": [{"name": "app", "command": "npm run dev"}]
}"#;

        assert!(
            !validate_cursor_hooks(hooks)
                .iter()
                .any(|d| d.rule.starts_with("CUR-01")),
            "Valid hooks fixture should not trigger CUR-010..CUR-016",
        );
        assert!(
            !validate_cursor_agent(agent)
                .iter()
                .any(|d| d.rule == "CUR-014" || d.rule == "CUR-015"),
            "Valid cursor agent fixture should not trigger CUR-014/CUR-015",
        );
        assert!(
            !validate_cursor_environment(environment)
                .iter()
                .any(|d| d.rule == "CUR-016"),
            "Valid environment fixture should not trigger CUR-016",
        );
    }

    // ===== Config Integration =====

    #[test]
    fn test_config_disabled_cursor_category() {
        let mut config = LintConfig::default();
        config.rules_mut().cursor = false;

        let content = "";
        let diagnostics = validate_mdc_with_config(content, &config);

        let cur_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("CUR-"))
            .collect();
        assert!(cur_rules.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CUR-001".to_string()];

        let content = "";
        let diagnostics = validate_mdc_with_config(content, &config);

        let cur_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-001").collect();
        assert!(cur_001.is_empty());
    }

    // ===== Combined Issues =====

    #[test]
    fn test_multiple_issues() {
        let content = r#"---
unknownKey: value
---
"#;
        let diagnostics = validate_mdc(content);

        // Should have:
        // - CUR-001 for empty body
        // - CUR-005 for unknown key
        assert!(
            diagnostics.iter().any(|d| d.rule == "CUR-001"),
            "Expected CUR-001"
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "CUR-005"),
            "Expected CUR-005"
        );
    }

    #[test]
    fn test_valid_mdc_no_issues() {
        let content = r#"---
description: TypeScript Guidelines
globs: "**/*.ts"
alwaysApply: false
---
# TypeScript Guidelines

Always use strict mode and explicit types.
"#;
        let diagnostics = validate_mdc(content);
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .collect();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    // ===== Additional CUR rule tests =====

    #[test]
    fn test_cur_001_newlines_only() {
        let content = "\n\n\n";
        let diagnostics = validate_mdc(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-001"));
    }

    #[test]
    fn test_cur_001_frontmatter_only() {
        // File with just frontmatter and no body
        let content = "---\ndescription: test\n---\n";
        let diagnostics = validate_mdc(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-001"));
    }

    #[test]
    fn test_cur_002_no_frontmatter_in_cursorrules() {
        // .cursorrules should NOT require frontmatter (CUR-002 is for .mdc files)
        let content = "Just plain text rules without frontmatter.";
        let validator = CursorValidator;
        let diagnostics =
            validator.validate(Path::new(".cursorrules"), content, &LintConfig::default());
        assert!(!diagnostics.iter().any(|d| d.rule == "CUR-002"));
    }

    #[test]
    fn test_cur_003_yaml_with_tabs() {
        // YAML doesn't allow tabs for indentation
        let content = "---\n\tdescription: test\n---\nBody";
        let diagnostics = validate_mdc(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-003"));
    }

    #[test]
    fn test_cur_004_all_valid_patterns() {
        let valid_patterns = ["**/*.ts", "*.rs", "src/**/*.py", "{src,lib}/**/*.tsx"];

        for pattern in valid_patterns {
            let content = format!("---\nglobs: \"{}\"\n---\nBody", pattern);
            let diagnostics = validate_mdc(&content);
            let cur_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-004").collect();
            assert!(cur_004.is_empty(), "Pattern '{}' should be valid", pattern);
        }
    }

    #[test]
    fn test_cur_004_invalid_patterns() {
        let invalid_patterns = ["[invalid", "***", "**["];

        for pattern in invalid_patterns {
            let content = format!("---\nglobs: \"{}\"\n---\nBody", pattern);
            let diagnostics = validate_mdc(&content);
            let cur_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-004").collect();
            assert!(
                !cur_004.is_empty(),
                "Pattern '{}' should be invalid",
                pattern
            );
        }
    }

    #[test]
    fn test_cur_004_globs_as_array() {
        let content = r#"---
globs:
  - "**/*.ts"
  - "**/*.tsx"
---
Body"#;
        let diagnostics = validate_mdc(content);
        let cur_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-004").collect();
        assert!(cur_004.is_empty(), "Array globs should be valid");
    }

    #[test]
    fn test_cur_005_all_known_keys() {
        let content = r#"---
description: Test rule
globs: "**/*.ts"
alwaysApply: false
---
Body"#;
        let diagnostics = validate_mdc(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "CUR-005"));
    }

    #[test]
    fn test_cur_005_multiple_unknown_keys() {
        let content = r#"---
description: test
unknownKey1: value1
unknownKey2: value2
---
Body"#;
        let diagnostics = validate_mdc(content);
        let cur_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-005").collect();
        assert!(!cur_005.is_empty());
    }

    #[test]
    fn test_cur_006_legacy_file_with_content() {
        let content = "Some legacy cursor rules.";
        let validator = CursorValidator;
        let diagnostics =
            validator.validate(Path::new(".cursorrules"), content, &LintConfig::default());
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-006"));
    }

    #[test]
    fn test_mdc_file_no_cur_006_warning() {
        // .mdc files should not trigger CUR-006 (legacy warning)
        let content = "---\ndescription: test\n---\nRules content";
        let diagnostics = validate_mdc(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "CUR-006"));
    }

    #[test]
    fn test_all_cur_rules_can_be_disabled() {
        let rules = [
            "CUR-001", "CUR-002", "CUR-003", "CUR-004", "CUR-005", "CUR-006", "CUR-007", "CUR-008",
            "CUR-009", "CUR-010", "CUR-011", "CUR-012", "CUR-013", "CUR-014", "CUR-015", "CUR-016",
        ];

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules_mut().disabled_rules = vec![rule.to_string()];

            // Content that could trigger each rule
            let (content, path) = match rule {
                "CUR-001" => ("", ".cursor/rules/test.mdc"),
                "CUR-006" => ("content", ".cursorrules"),
                "CUR-007" => (
                    "---\nalwaysApply: true\nglobs: \"**/*.ts\"\n---\nBody",
                    ".cursor/rules/test.mdc",
                ),
                "CUR-008" => (
                    "---\nalwaysApply: \"true\"\n---\nBody",
                    ".cursor/rules/test.mdc",
                ),
                "CUR-009" => ("---\n\n---\nBody", ".cursor/rules/test.mdc"),
                "CUR-010" => ("{}", ".cursor/hooks.json"),
                "CUR-011" => (
                    r#"{"version":1,"hooks":{"unknownEvent":[{"type":"command","command":"echo hi"}]}}"#,
                    ".cursor/hooks.json",
                ),
                "CUR-012" => (
                    r#"{"version":1,"hooks":{"sessionStart":[{"type":"command"}]}}"#,
                    ".cursor/hooks.json",
                ),
                "CUR-013" => (
                    r#"{"version":1,"hooks":{"sessionStart":[{"type":"agent","command":"echo hi"}]}}"#,
                    ".cursor/hooks.json",
                ),
                "CUR-014" => (
                    "---\nname: BadName\ndescription: 1\nreadonly: \"true\"\n---\nbody",
                    ".cursor/agents/reviewer.md",
                ),
                "CUR-015" => (
                    "---\nname: reviewer-agent\ndescription: test\n---\n",
                    ".cursor/agents/reviewer.md",
                ),
                "CUR-016" => ("{}", ".cursor/environment.json"),
                _ => ("---\nunknown: value\n---\n", ".cursor/rules/test.mdc"),
            };

            let validator = CursorValidator;
            let diagnostics = validator.validate(Path::new(path), content, &config);

            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }

    // ===== CUR-007: alwaysApply with redundant globs =====

    #[test]
    fn test_cur_007_always_apply_with_globs() {
        let content = r#"---
description: TypeScript rules
alwaysApply: true
globs: "**/*.ts"
---
# Rules
Use strict mode.
"#;
        let diagnostics = validate_mdc(content);
        let cur_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-007").collect();
        assert_eq!(cur_007.len(), 1);
        assert_eq!(cur_007[0].level, DiagnosticLevel::Warning);
        assert!(cur_007[0].message.contains("redundant"));
    }

    #[test]
    fn test_cur_007_always_apply_with_globs_array() {
        let content = r#"---
alwaysApply: true
globs:
  - "**/*.ts"
  - "**/*.tsx"
---
# Rules
Body content.
"#;
        let diagnostics = validate_mdc(content);
        let cur_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-007").collect();
        assert_eq!(cur_007.len(), 1);
    }

    #[test]
    fn test_cur_007_always_apply_false_with_globs() {
        // alwaysApply: false with globs should NOT trigger CUR-007
        let content = r#"---
description: TypeScript rules
alwaysApply: false
globs: "**/*.ts"
---
# Rules
Use strict mode.
"#;
        let diagnostics = validate_mdc(content);
        let cur_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-007").collect();
        assert!(cur_007.is_empty());
    }

    #[test]
    fn test_cur_007_always_apply_without_globs() {
        // alwaysApply: true without globs should NOT trigger CUR-007
        let content = r#"---
description: Global rules
alwaysApply: true
---
# Rules
Always apply these.
"#;
        let diagnostics = validate_mdc(content);
        let cur_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-007").collect();
        assert!(cur_007.is_empty());
    }

    #[test]
    fn test_cur_007_has_autofix() {
        let content = r#"---
alwaysApply: true
globs: "**/*.ts"
---
# Rules
Body content.
"#;
        let diagnostics = validate_mdc(content);
        let cur_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-007").collect();
        assert_eq!(cur_007.len(), 1);
        assert!(cur_007[0].has_fixes(), "CUR-007 should include an auto-fix");
        assert!(cur_007[0].fixes[0].safe, "CUR-007 fix should be safe");
    }

    #[test]
    fn test_cur_007_line_number_accuracy() {
        let content = r#"---
description: Test
alwaysApply: true
globs: "**/*.ts"
---
# Rules
Body.
"#;
        let diagnostics = validate_mdc(content);
        let cur_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-007").collect();
        assert_eq!(cur_007.len(), 1);
        // globs is on line 4 (line 1 is ---, line 2 is description, line 3 is alwaysApply, line 4 is globs)
        assert_eq!(
            cur_007[0].line, 4,
            "CUR-007 should point to the globs field line"
        );
    }

    #[test]
    fn test_cur_007_autofix_deletes_array_globs_block() {
        // When globs is in array form, the auto-fix must delete the entire block
        // (key line + indented list items), not just the key line
        let content = "---\nalwaysApply: true\nglobs:\n  - \"**/*.ts\"\n  - \"**/*.tsx\"\n---\n# Rules\nBody.\n";
        let diagnostics = validate_mdc(content);
        let cur_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-007").collect();
        assert_eq!(cur_007.len(), 1);
        assert!(cur_007[0].has_fixes());

        // Apply the fix and verify the result is valid YAML
        let fix = &cur_007[0].fixes[0];
        let fixed = format!("{}{}", &content[..fix.start_byte], &content[fix.end_byte..]);
        // The fixed content should not contain any globs-related lines
        assert!(
            !fixed.contains("globs"),
            "Fix should remove entire globs block, got: {:?}",
            fixed
        );
        assert!(
            !fixed.contains("**/*.ts"),
            "Fix should remove globs list items, got: {:?}",
            fixed
        );
    }

    // ===== CUR-008: Invalid alwaysApply type =====

    #[test]
    fn test_cur_008_string_true() {
        let content = r#"---
description: Test
alwaysApply: "true"
---
# Rules
Body content.
"#;
        let diagnostics = validate_mdc(content);
        let cur_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-008").collect();
        assert_eq!(cur_008.len(), 1);
        assert_eq!(cur_008[0].level, DiagnosticLevel::Error);
        assert!(cur_008[0].message.contains("boolean"));
    }

    #[test]
    fn test_cur_008_string_false() {
        let content = r#"---
description: Test
alwaysApply: "false"
---
# Rules
Body content.
"#;
        let diagnostics = validate_mdc(content);
        let cur_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-008").collect();
        assert_eq!(cur_008.len(), 1);
    }

    #[test]
    fn test_cur_008_boolean_true() {
        // Proper boolean should NOT trigger CUR-008
        let content = r#"---
description: Test
alwaysApply: true
---
# Rules
Body content.
"#;
        let diagnostics = validate_mdc(content);
        let cur_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-008").collect();
        assert!(cur_008.is_empty());
    }

    #[test]
    fn test_cur_008_boolean_false() {
        // Proper boolean should NOT trigger CUR-008
        let content = r#"---
description: Test
alwaysApply: false
---
# Rules
Body content.
"#;
        let diagnostics = validate_mdc(content);
        let cur_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-008").collect();
        assert!(cur_008.is_empty());
    }

    #[test]
    fn test_cur_008_line_number_accuracy() {
        let content = r#"---
description: Test
alwaysApply: "true"
---
# Rules
Body.
"#;
        let diagnostics = validate_mdc(content);
        let cur_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-008").collect();
        assert_eq!(cur_008.len(), 1);
        assert_eq!(
            cur_008[0].line, 3,
            "CUR-008 should point to the alwaysApply field line"
        );
    }

    #[test]
    fn test_cur_008_arbitrary_string() {
        // alwaysApply with arbitrary string should also trigger
        let content = r#"---
description: Test
alwaysApply: "yes"
---
# Rules
Body content.
"#;
        let diagnostics = validate_mdc(content);
        let cur_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-008").collect();
        assert_eq!(cur_008.len(), 1);
        // Arbitrary string "yes" should NOT get a fix (not "true" or "false")
        assert!(
            !cur_008[0].has_fixes(),
            "CUR-008 should not auto-fix arbitrary strings"
        );
    }

    #[test]
    fn test_cur_008_autofix_string_true() {
        let content = "---\ndescription: Test\nalwaysApply: \"true\"\n---\n# Rules\nBody.\n";
        let diagnostics = validate_mdc(content);
        let cur_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-008").collect();
        assert_eq!(cur_008.len(), 1);
        assert!(
            cur_008[0].has_fixes(),
            "CUR-008 should have auto-fix for \"true\""
        );
        let fix = &cur_008[0].fixes[0];
        assert!(fix.safe, "CUR-008 fix should be safe");
        assert_eq!(
            fix.replacement, "true",
            "Fix should convert to unquoted boolean true"
        );
        // Verify the fix targets the quoted value
        let target = &content[fix.start_byte..fix.end_byte];
        assert_eq!(target, "\"true\"", "Fix should target the quoted string");
    }

    #[test]
    fn test_cur_008_autofix_string_false() {
        let content = "---\ndescription: Test\nalwaysApply: \"false\"\n---\n# Rules\nBody.\n";
        let diagnostics = validate_mdc(content);
        let cur_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-008").collect();
        assert_eq!(cur_008.len(), 1);
        assert!(
            cur_008[0].has_fixes(),
            "CUR-008 should have auto-fix for \"false\""
        );
        let fix = &cur_008[0].fixes[0];
        assert_eq!(
            fix.replacement, "false",
            "Fix should convert to unquoted boolean false"
        );
        let target = &content[fix.start_byte..fix.end_byte];
        assert_eq!(target, "\"false\"", "Fix should target the quoted string");
    }

    // ===== CUR-009: Missing description for agent-requested rule =====

    #[test]
    fn test_cur_009_no_description_no_globs_no_always_apply() {
        let content = r#"---

---
# Rules
Some content here.
"#;
        let diagnostics = validate_mdc(content);
        let cur_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-009").collect();
        assert_eq!(cur_009.len(), 1);
        assert_eq!(cur_009[0].level, DiagnosticLevel::Warning);
        assert!(cur_009[0].message.contains("description"));
    }

    #[test]
    fn test_cur_009_empty_description() {
        let content = r#"---
description: ""
---
# Rules
Some content here.
"#;
        let diagnostics = validate_mdc(content);
        let cur_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-009").collect();
        assert_eq!(cur_009.len(), 1);
    }

    #[test]
    fn test_cur_009_whitespace_only_description() {
        let content = r#"---
description: "   "
---
# Rules
Some content here.
"#;
        let diagnostics = validate_mdc(content);
        let cur_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-009").collect();
        assert_eq!(cur_009.len(), 1);
    }

    #[test]
    fn test_cur_009_with_description() {
        // Has description but no globs or alwaysApply - should NOT trigger
        let content = r#"---
description: TypeScript coding standards
---
# Rules
Some content here.
"#;
        let diagnostics = validate_mdc(content);
        let cur_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-009").collect();
        assert!(cur_009.is_empty());
    }

    #[test]
    fn test_cur_009_with_globs() {
        // Has globs but no description - should NOT trigger (not agent-requested)
        let content = r#"---
globs: "**/*.ts"
---
# Rules
Some content here.
"#;
        let diagnostics = validate_mdc(content);
        let cur_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-009").collect();
        assert!(cur_009.is_empty());
    }

    #[test]
    fn test_cur_009_with_always_apply() {
        // Has alwaysApply but no description - should NOT trigger (not agent-requested)
        let content = r#"---
alwaysApply: true
---
# Rules
Some content here.
"#;
        let diagnostics = validate_mdc(content);
        let cur_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-009").collect();
        assert!(cur_009.is_empty());
    }

    #[test]
    fn test_cur_009_with_always_apply_false() {
        // alwaysApply: false counts as "has alwaysApply" - should NOT trigger
        let content = r#"---
alwaysApply: false
---
# Rules
Some content here.
"#;
        let diagnostics = validate_mdc(content);
        let cur_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-009").collect();
        assert!(cur_009.is_empty());
    }
}
